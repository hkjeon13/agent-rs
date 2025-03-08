use async_openai::{
    config::OpenAIConfig,
    types::{CreateChatCompletionRequestArgs, ChatCompletionRequestUserMessageArgs},
    Client,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json,
    Router,
};
use futures::StreamExt;
use axum::body::Body; // axum의 Body를 사용합니다.
use bytes::Bytes;
use serde::Deserialize;
use std::{fs, net::SocketAddr, sync::Arc};
use tracing::info;
use tracing_subscriber;
use std::time::Instant;

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
}

struct AppState {
    client: Client<OpenAIConfig>,
}

#[tokio::main]
async fn main() {
    // tracing 구독자 초기화: 로그를 콘솔에 출력합니다.
    tracing_subscriber::fmt::init();

    let config_str = fs::read_to_string("config.toml")
        .expect("Failed to read config file");
    let config: Config = toml::from_str(&config_str)
        .expect("Failed to parse config file");

    let secrets_str = fs::read_to_string("secrets.toml")
        .expect("Failed to read secrets file");
    let secrets: Secrets = toml::from_str(&secrets_str)
        .expect("Failed to parse secrets file");

    let openai_config = OpenAIConfig::new().with_api_key(secrets.openai.api_key);
    let client: Client<OpenAIConfig> = Client::with_config(openai_config);

    let state = Arc::new(AppState { client });

    let app = Router::new()
        .route(&config.routes.chat, post(chat))
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port)
        .parse::<SocketAddr>()
        .unwrap();

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn chat(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ChatInput>,
) -> Result<impl IntoResponse, (StatusCode, String)> {

    let client = &state.client;

    info!("Session ID: {}, Chat ID: {}, Name: {}", input.session_id, input.chat_id, input.name);

    // 사용자 메시지 구성
    let user_message = ChatCompletionRequestUserMessageArgs::default()
        .content(input.query)
        .build()
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    // 스트리밍 옵션 활성화
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .messages(vec![user_message.into()])
        .stream(true)
        .build()
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    // create_stream 메서드를 사용하여 스트림을 받아옵니다.
    let stream = client
        .chat()
        .create_stream(request)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;


    // 스트림의 각 청크에서 텍스트만 추출하여 Bytes로 변환합니다.
    let body_stream = stream
        .map(|chunk_result| -> Result<Bytes, std::convert::Infallible> {
            match chunk_result {
                Ok(chunk) => {
                    let text = chunk.choices[0]
                        .delta
                        .content
                        .clone()
                        .unwrap_or_default();
                    Ok(Bytes::from(text))
                }
                Err(e) => Ok(Bytes::from(format!("\n[Error: {}]\n", e))),
            }
        })
        .boxed();

    // axum::body::Body::from_stream을 사용하여 스트림 바디를 생성합니다.
    let body = Body::from_stream(body_stream);
    let response = Response::builder()
        .header("Content-Type", "text/plain")
        .body(body)
        .unwrap();

    Ok(response)
}
