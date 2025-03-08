use async_openai::{
    config::OpenAIConfig,
    types::{CreateChatCompletionRequestArgs, ChatCompletionRequestUserMessageArgs},
    Client,
};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{StreamExt, stream::BoxStream};
use std::{
    convert::Infallible,
    pin::Pin,
};
use axum::http::StatusCode;

#[async_trait]
pub trait Model: Send + Sync {
    async fn async_generate_stream(
        &self,
        input: &str,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<Bytes, Infallible>> + Send>>,
        (StatusCode, String),
    >;

    async fn async_generate(&self, input: &str) -> String {
        let stream = self.async_generate_stream(input)
            .await
            .expect("Failed to generate stream");

        let chunks = stream.collect::<Vec<_>>().await;
        let mut output = String::new();
        for chunk in chunks {
            let bytes = chunk.expect("Failed to get chunk");
            output.push_str(&String::from_utf8_lossy(&bytes));
        }
        output
    }
}


pub struct OpenAIModel {
    pub model_name: String,
    pub client: Client<OpenAIConfig>,
}

impl OpenAIModel {
    pub fn new(api_key: impl Into<String>, model_name: impl Into<String>) -> Self {
        let api_key_str = api_key.into();
        let model_name_str = model_name.into();
        let openai_config = OpenAIConfig::new().with_api_key(api_key_str.clone());
        let client: Client<OpenAIConfig> = Client::with_config(openai_config);
        Self {
            model_name: model_name_str,
            client,
        }
    }
}

#[async_trait]
impl Model for OpenAIModel {
    async fn async_generate_stream(
        &self,
        input: &str,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<Bytes, Infallible>> + Send>>,
        (StatusCode, String),
    > {
        // 사용자 메시지 구성
        let user_message = ChatCompletionRequestUserMessageArgs::default()
            .content(input)
            .build()
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

        // 스트리밍 요청 생성
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .messages(vec![user_message.into()])
            .stream(true)
            .build()
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

        // 미리 생성된 클라이언트를 사용하여 스트림 생성
        let stream = self.client
            .chat()
            .create_stream(request)
            .await
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

        // 각 청크에서 텍스트를 추출하여 Bytes로 변환하는 스트림 생성
        let body_stream: BoxStream<Result<Bytes, Infallible>> = stream
            .map(|chunk_result| -> Result<Bytes, Infallible> {
                match chunk_result {
                    Ok(chunk) => {
                        let text = chunk.choices[0].clone()
                            .delta
                            .content
                            .unwrap_or_default();
                        Ok(Bytes::from(text))
                    }
                    Err(e) => Ok(Bytes::from(format!("\n[Error: {}]\n", e))),
                }
            })
            .boxed();

        Ok(Box::pin(body_stream))
    }
}
