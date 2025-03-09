use std::{
    convert::Infallible,
    pin::Pin,
    collections::HashMap
};

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage,
        ChatCompletionRequestUserMessageArgs,
        ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestSystemMessageArgs,
        CreateChatCompletionRequestArgs
    },
};
use async_trait::async_trait;
use axum::http::StatusCode;
use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt};


#[async_trait]
pub trait Model: Send + Sync {
    async fn async_generate_stream(
        &self,
        messages: Vec<HashMap<String, String>>,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item=Result<Bytes, Infallible>> + Send>>,
        (StatusCode, String),
    >;

    async fn async_generate(&self, messages:Vec<HashMap<String, String>>) -> String {
        let stream = self.async_generate_stream(messages)
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
    pub(crate) fn clone(&self) -> Self {
        Self {
            model_name: self.model_name.clone(),
            client: self.client.clone(),
        }
    }
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

    fn prepare_inputs(&self, inputs: Vec<HashMap<String, String>>) -> Vec<ChatCompletionRequestMessage> {
        let mut outputs:Vec<ChatCompletionRequestMessage> = Vec::new();
        for input in inputs {
            let message = match input.get("role") {
                Some(role) => {
                    match role.as_str() {
                        "user" => {
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(input.get("content").unwrap_or(&"".to_string()).to_string())
                                .build()
                                .expect("Failed to build user message")
                                .into()
                        }
                        "assistant" => {
                            ChatCompletionRequestAssistantMessageArgs::default()
                                .content(input.get("content").unwrap_or(&"".to_string()).to_string())
                                .build()
                                .expect("Failed to build assistant message")
                                .into()
                        }
                        "system" => {
                            ChatCompletionRequestSystemMessageArgs::default()
                                .content(input.get("content").unwrap_or(&"".to_string()).to_string())
                                .build()
                                .expect("Failed to build system message")
                                .into()
                        }
                        _ => {
                            panic!("Invalid role type(only 'user', 'assistant', 'system' are allowed)")
                        }
                    }
                }

                None => {
                    panic!("Role not found")
                }

            };
            outputs.push(message);
        }
        outputs
    }
}


#[async_trait]
impl Model for OpenAIModel {

    async fn async_generate_stream(
        &self,
        messages: Vec<HashMap<String, String>>,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item=Result<Bytes, Infallible>> + Send>>,
        (StatusCode, String),
    > {
        // 사용자 메시지 구성
        let input_messages = self.prepare_inputs(messages);
        // 스트리밍 요청 생성
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .messages(input_messages)
            .stream(true)
            .build()
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

        let stream = self.client
            .chat()
            .create_stream(request)
            .await
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

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
