// src/main.rs

use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use futures::StreamExt;
use std::convert::Infallible;

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    response::{IntoResponse, Response},
    Router,
    routing::post,
};
use axum::body::Body;
use serde::Deserialize;
use tracing::info;
use tracing_subscriber;

use models::{Model, OpenAIModel};

mod models;
mod states;
mod memory;
mod actions;
mod observation;
mod agents;
mod prompts;

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
struct NaverSecrets {
    client_id: String,
    client_secret: String,
}

#[derive(Deserialize)]
struct Secrets {
    openai: OpenAISecrets,
    naver: NaverSecrets,
}

#[derive(Deserialize)]
struct ChatInput {
    session_id: String,
    chat_id: String,
    name: String,
    query: String,
    stream: bool,
}

struct AppState {
    agent: Arc<dyn agents::AgentBase + Send + Sync + 'static>,
    model: Box<dyn Model + Send + Sync>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // 설정 파일 로드
    let config: Config = toml::from_str(
        &fs::read_to_string("config.toml").expect("Failed to read config file"),
    )
        .expect("Failed to parse config file");

    let secrets: Secrets = toml::from_str(
        &fs::read_to_string("secrets.toml").expect("Failed to read secrets file"),
    )
        .expect("Failed to parse secrets file");

    // 모델 생성
    let openai_model = OpenAIModel::new(
        secrets.openai.api_key.clone(),
        config.model.model_name.clone(),
    );

    info!("Using OpenAI model: {} (type: {})", openai_model.model_name, config.model.model_type);

    let agent = agents::Agent::new(
        openai_model.clone(),
        3,
        vec![
            Box::new(actions::DuckDuckGoSearchAction::new()),
            Box::new(actions::NaverNewsSearchAction::new(
                secrets.naver.client_id.clone(), secrets.naver.client_secret.clone()
            ))
        ],
        true, // Enable streaming outputs
    );

    let state = Arc::new(AppState {
        agent: Arc::new(agent) as Arc<dyn agents::AgentBase + Send + Sync + 'static>,
        model: Box::new(openai_model),
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
    info!(
        "Processing chat: Session ID: {}, Chat ID: {}, Name: {}",
        input.session_id, input.chat_id, input.name
    );

    // Execute the agent, which yields a stream of text chunks
    let query = input.query.clone();
    let mut stream = state.agent.clone().run(query, true).await;

    if input.stream {
        // Stream chunks directly as SSE-like plain text
        let byte_stream = stream.map(|chunk| Ok::<_, Infallible>(chunk.into_bytes()));
        let response = Response::builder()
            .header("Content-Type", "text/plain")
            .body(Body::from_stream(byte_stream))
            .unwrap();
        return Ok(response);
    }

    // Otherwise, accumulate all chunks into a full text response
    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        full_text.push_str(&chunk);
    }
    let response = Response::builder()
        .header("Content-Type", "text/plain")
        .body(Body::from(full_text))
        .unwrap();
    Ok(response)
}
