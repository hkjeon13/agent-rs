use axum::{
    routing::post,
    Router,Json,
    http::StatusCode,
};
use std::{fs, net::SocketAddr};
use serde::{Deserialize, Serialize};

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
struct Config {
    server: ServerConfig,
    routes: RoutesConfig,
}

#[derive(Deserialize)]
struct ChatInput {
    session_id: String,
    chat_id: String,
    name: String,
    query: String,
}

#[derive(Serialize)]
struct ChatOutput {
    session_id: String,
    chat_id: String,
    name: String,
    response: String,
}


#[tokio::main]
async fn main() {
    let config_str = fs::read_to_string("config.toml")
        .expect("Failed to read config file");
    let config: Config = toml::from_str(&config_str)
        .expect("Failed to parse config file");

    let addr = format!("{}:{}", config.server.host, config.server.port)
        .parse::<SocketAddr>()
        .unwrap();

    let app = Router::new()
        .route(&config.routes.chat, post(chat));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();

}


async fn chat(
    Json(input): Json<ChatInput>) -> (StatusCode, Json<ChatOutput>) {
    // Below is a simple echo response
    let output = ChatOutput {
        session_id: input.session_id,
        chat_id: input.chat_id,
        name: input.name,
        response: input.query,
    };

    // TODO: Implement chatbot logic here

    (StatusCode::OK, Json(output))

}
