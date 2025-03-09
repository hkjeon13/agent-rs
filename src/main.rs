// src/main.rs

use std::{
    fs,
    net::SocketAddr,
    sync::Arc,
    collections::HashMap,
};

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
    agent: Box<dyn agents::AgentBase + Send + Sync>,
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

    let agent = agents::Agent::new(
        openai_model.clone(),
        1,
        vec![
            Box::new(actions::DuckDuckGoSearchAction::new()),
            Box::new(actions::NaverNewsSearchAction::new(secrets.naver.client_id.clone(), secrets.naver.client_secret.clone()))
        ]
    );

    let state = Arc::new(AppState {
        agent: Box::new(agent),
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

    let agent_outputs = state.agent.run(&input.query).await;
    info!("Agent response: {}", agent_outputs);

    let inputs = vec![HashMap::from([
        ("role".to_string(), "user".to_string()),
        ("content".to_string(), input.query.clone()),
    ])];

    if input.stream {
        let body_stream = state.model.async_generate_stream(inputs).await?;
        let response = Response::builder()
            .header("Content-Type", "text/plain")
            .body(Body::from_stream(body_stream))
            .unwrap();
        Ok(response)
    } else {
        let output = state.model.async_generate(inputs).await;
        let response = Response::builder()
            .header("Content-Type", "text/plain")
            .body(Body::from(output))
            .unwrap();
        Ok(response)
    }
}
