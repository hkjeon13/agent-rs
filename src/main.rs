use axum::{
    routing::post,
    Router,
};
use serde::Deserialize;
use std::{fs, net::SocketAddr};

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Deserialize)]
struct RoutesConfig {
    indexing: String,
    search: String,
    delete: String,
}

#[derive(Deserialize)]
struct Config {
    server: ServerConfig,
    routes: RoutesConfig,
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
        .route(&config.routes.chat, post(indexing));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}


async fn chat() -> &'static str {
    "indexing endpoint"
}
