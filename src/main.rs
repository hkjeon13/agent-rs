// src/main.rs

mod models;
mod actions;
mod state;
mod tools;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json,
    Router,
};
use axum::body::Body;
use serde::Deserialize;
use std::{
    fs,
    net::SocketAddr,
    sync::Arc,
};
use tracing::info;
use tracing_subscriber;
use models::{Model, OpenAIModel};

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Deserialize)]
struct RoutesConfig {
    chat: String,
}

#[derive(Deserialize)]
struct ModelConfig {
    model_type: String,
    model_name: String,
}

#[derive(Deserialize)]
struct Config {
    server: ServerConfig,
    routes: RoutesConfig,
    model: ModelConfig,
}

#[derive(Deserialize)]
struct OpenAISecrets {
    api_key: String,
}

#[derive(Deserialize)]
struct Secrets {
    openai: OpenAISecrets,
}

#[derive(Deserialize)]
struct ChatInput {
    session_id: String,
    chat_id: String,
    name: String,
    query: String,
    stream: bool,
}

// AppState now holds a Box<dyn Model>
struct AppState {
    model: Box<dyn Model>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config_str = fs::read_to_string("config.toml")
        .expect("Failed to read config file");
    let config: Config = toml::from_str(&config_str)
        .expect("Failed to parse config file");

    let secrets_str = fs::read_to_string("secrets.toml")
        .expect("Failed to read secrets file");
    let secrets: Secrets = toml::from_str(&secrets_str)
        .expect("Failed to parse secrets file");

    // Choose the model based on configuration.
    let model = match config.model.model_type.as_str() {
        "openai" => {
            info!("OpenAI model selected");
            OpenAIModel::new(secrets.openai.api_key, config.model.model_name)
        }
        _ => panic!("Unsupported model type"),
    };

    let state = Arc::new(AppState {
        model: Box::new(model),
    });

    let app = Router::new()
        .route(&config.routes.chat, post(chat))
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port)
        .parse::<SocketAddr>()
        .unwrap();

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn chat(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ChatInput>,
) -> Result<impl IntoResponse, (StatusCode, String)> {

    info!("Session ID: {}, Chat ID: {}, Name: {}", input.session_id, input.chat_id, input.name);

    if input.stream {
        let body_stream = state.model.async_generate_stream(&input.query).await?;
        let body = Body::from_stream(body_stream);
        let response = Response::builder()
            .header("Content-Type", "text/plain")
            .body(body)
            .unwrap();
        Ok(response)

    } else {
        let output = state.model.async_generate(&input.query).await;
        let response = Response::builder()
            .header("Content-Type", "text/plain")
            .body(Body::from(output))
            .unwrap();
        Ok(response)
    }
}
