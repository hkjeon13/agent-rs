use std::collections::HashMap;
use std::fmt;
use serde_json::{Map, Value};
use async_openai::{
    types::{
        ChatCompletionRequestMessage,
    }
};

trait ToolBase {
    fn dict(&self) -> HashMap<String, Value>;
}

trait TimeBase {
    fn duration(&self) -> f32;
}

trait MemoryStep {
    fn dict(&self) -> HashMap<String, Value>;
    fn to_message(&self) -> Vec<ChatCompletionRequestMessage>;
}


pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, Value>,
}

pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

pub struct Timing {
    pub start_time: f32,
    pub end_time: f32
}


pub struct ActionStep {
    pub step_number: usize,
    pub timing: String, // Placeholder for Timing type
    pub model_input_messages: Option<Vec<ChatCompletionRequestMessage>>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub error: Option<String>, // Placeholder for AgentError type
    pub model_output_message: Option<ChatCompletionRequestMessage>,
    pub model_output: Option<String>, // Placeholder for output type
    pub code_action: Option<String>,
    pub observations: Option<String>,
    pub observations_images: Option<Vec<String>>, // Placeholder for image type
    pub action_output: Option<Value>, // Placeholder for action output type
    pub token_usage: Option<TokenUsage>, // Placeholder for TokenUsage type
    pub is_final_answer: bool,
}

impl TimeBase for Timing {
    fn duration(&self) -> f32 {
        self.end_time - self.start_time
    }
}

impl fmt::Display for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timing(start_time={}, end_time={}, duration={})", self.start_time, self.end_time, self.duration())
    }
}

impl ToolBase for ToolCall {
    fn dict(&self) -> HashMap<String, Value> {
        let mut output = HashMap::new();
        output.insert("id".to_string(), Value::String(self.id.clone()));
        output.insert("type".to_string(), "function".into());
        output.insert("function".to_string(), Value::Object(Map::from_iter(vec![
            ("name".to_string(), Value::String(self.name.clone())),
            ("arguments".to_string(), Value::Object(Map::from_iter(
                self.arguments.iter().map(|(k, v)| (k.clone(), v.clone()))
            ))),
        ])));
        output
    }
}







