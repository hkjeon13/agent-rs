use serde_json::{Map, Value};
use async_openai::{
    types::{
        ChatCompletionRequestMessage,
        ChatCompletionRequestUserMessageArgs,
        ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestToolMessageArgs,
    }
};
use tracing::info;
use crate::prompts::load_config;
use std::{
    fmt,
    any::{Any, TypeId},
    collections::HashMap
};

trait ToolBase {
    fn dict(&self) -> HashMap<String, Value>;
}

trait TimeBase {
    fn duration(&self) -> i32;
}

trait MemoryStep {
    fn dict(&self) -> HashMap<String, Value>;
    fn to_message(&self, summary_mode: bool) -> Vec<ChatCompletionRequestMessage>;
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait AgentMemoryBase {
    fn reset(&mut self);
    fn get_succinct_steps(&self) -> Vec<Value>;
    fn get_full_steps(&self) -> Vec<Value>;
    fn replay(&self);
    fn return_full_code(&self) -> String;
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
    pub start_time: i32,
    pub end_time: i32,
}

pub struct ActionStep {
    pub step_number: usize,
    pub timing: Timing,
    pub model_input_messages: Option<Vec<ChatCompletionRequestMessage>>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub error: Option<String>,
    pub model_output_message: Option<ChatCompletionRequestMessage>,
    pub model_output: Option<String>,
    pub code_action: Option<String>,
    pub observations: Option<String>,
    pub observations_images: Option<Vec<String>>,
    pub action_output: Option<Value>,
    pub token_usage: Option<TokenUsage>,
    pub is_final_answer: bool,
}

pub struct PlanningStep {
    model_input_messages: Vec<ChatCompletionRequestMessage>,
    model_output_message: Option<ChatCompletionRequestMessage>,
    plan: String,
    timing: Timing,
    token_usage: Option<TokenUsage>,
}

pub struct TaskStep {
    pub task: String,
    pub task_images: Option<Vec<String>>, // Assuming images are represented as strings (e.g., URLs or base64)
}

pub struct SystemPromptStep {
    pub system_prompt: String,
}

pub struct FinalAnswerStep {
    pub output: String,
}

pub enum Step {
    Task(TaskStep),
    Action(ActionStep),
    Planning(PlanningStep),
}

pub struct AgentMemory {
    pub system_prompt: SystemPromptStep,
    pub steps: Vec<Step>,
}

type Callback = Box<dyn Fn(&dyn MemoryStep)>;

pub struct CallbackRegistry {
    callbacks: HashMap<TypeId, Vec<Callback>>,
}

impl TimeBase for Timing {
    fn duration(&self) -> i32 {
        self.end_time - self.start_time
    }
}

impl fmt::Display for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Timing(start_time={}, end_time={}, duration={})",
            self.start_time,
            self.end_time,
            self.duration()
        )
    }
}

impl ToolBase for ToolCall {
    fn dict(&self) -> HashMap<String, Value> {
        let mut output = HashMap::new();
        output.insert("id".to_string(), Value::String(self.id.clone()));
        output.insert("type".to_string(), Value::String("function".into()));
        let mut func_obj = Map::new();
        func_obj.insert("name".to_string(), Value::String(self.name.clone()));
        func_obj.insert(
            "arguments".to_string(),
            Value::Object(
                Map::from_iter(self.arguments.iter().map(|(k, v)| (k.clone(), v.clone())))
            ),
        );
        output.insert("function".to_string(), Value::Object(func_obj));
        output
    }
}

impl ToolBase for Timing {
    fn dict(&self) -> HashMap<String, Value> {
        let mut output = HashMap::new();
        output.insert("start_time".to_string(), Value::Number(self.start_time.into()));
        output.insert("end_time".to_string(), Value::Number(self.end_time.into()));
        output.insert("duration".to_string(), Value::Number(self.duration().into()));
        output
    }
}

impl MemoryStep for ActionStep {
    fn dict(&self) -> HashMap<String, Value> {
        let mut output = HashMap::new();

        output.insert(
            "step_number".to_string(),
            Value::Number(self.step_number.into()),
        );
        output.insert(
            "timing".to_string(),
            Value::Object(Map::from_iter(self.timing.dict().into_iter())),
        );
        output.insert(
            "model_input_messages".to_string(),
            self.model_input_messages.as_ref().map_or(Value::Null, |msgs| {
                Value::Array(
                    msgs.iter()
                        .map(|msg| serde_json::to_value(msg).expect("serialize msg"))
                        .collect(),
                )
            }),
        );
        output.insert(
            "tool_calls".to_string(),
            self.tool_calls.as_ref().map_or(Value::Null, |calls| {
                Value::Array(
                    calls.iter()
                        .map(|call| Value::Object(Map::from_iter(call.dict().into_iter())))
                        .collect(),
                )
            }),
        );
        output.insert(
            "error".to_string(),
            self.error.clone().map_or(Value::Null, Value::String),
        );
        output.insert(
            "model_output_message".to_string(),
            self.model_output_message
                .as_ref()
                .map_or(Value::Null, |msg| {
                    serde_json::to_value(msg).expect("serialize model_output_message")
                }),
        );
        output.insert(
            "model_output".to_string(),
            self.model_output.clone().map_or(Value::Null, Value::String),
        );
        output.insert(
            "code_action".to_string(),
            self.code_action.clone().map_or(Value::Null, Value::String),
        );
        output.insert(
            "observations".to_string(),
            self.observations.clone().map_or(Value::Null, Value::String),
        );
        output.insert(
            "observations_images".to_string(),
            self.observations_images.as_ref().map_or(Value::Null, |images| {
                Value::Array(images.iter().map(|s| Value::String(s.clone())).collect())
            }),
        );
        output.insert(
            "action_output".to_string(),
            self.action_output.clone().map_or(Value::Null, |v| v),
        );
        output.insert(
            "token_usage".to_string(),
            self.token_usage.as_ref().map_or(Value::Null, |usage| {
                let mut usage_map = Map::new();
                usage_map.insert("prompt_tokens".to_string(), Value::Number(usage.prompt_tokens.into()));
                usage_map.insert("completion_tokens".to_string(), Value::Number(usage.completion_tokens.into()));
                usage_map.insert("total_tokens".to_string(), Value::Number(usage.total_tokens.into()));
                Value::Object(usage_map)
            }),
        );
        output.insert(
            "is_final_answer".to_string(),
            Value::Bool(self.is_final_answer),
        );

        output
    }

    fn to_message(&self, summary_mode: bool) -> Vec<ChatCompletionRequestMessage> {
        let mut messages = Vec::new();
        if let Some(output) = &self.model_output {
            if !summary_mode {
                messages.push(
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(output.clone())
                        .build()
                        .expect("Failed to build assistant message")
                        .into(),
                );
            }
        }

        if let Some(calls) = &self.tool_calls {
            for call in calls {
                messages.push(
                    ChatCompletionRequestToolMessageArgs::default()
                        .content(format!(
                            "Calling tools:\n{}",
                            serde_json::to_string(&call.dict()).unwrap_or_default()
                        ))
                        .build()
                        .expect("Failed to build tool message")
                        .into(),
                );
            }
        }

        if let Some(images) = &self.observations_images {
            for img in images {
                messages.push(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(format!("Observation image: {}", img))
                        .build()
                        .expect("Failed to build user message for image")
                        .into(),
                );
            }
        }

        if let Some(obs) = &self.observations {
            messages.push(
                ChatCompletionRequestToolMessageArgs::default()
                    .content(format!("Observations:\n{}", obs))
                    .build()
                    .expect("Failed to build tool message for observations")
                    .into(),
            );
        }

        if let Some(err) = &self.error {
            let call_id = self
                .tool_calls
                .as_ref()
                .and_then(|calls| calls.first().map(|c| c.id.clone()))
                .unwrap_or_else(|| "None".to_string());
            let error_msg = format!(
                "Error occurred: {}\nNow let's retry: take care not to repeat previous errors! If you have retried several times, try a completely different approach.\n",
                err
            );
            messages.push(
                ChatCompletionRequestToolMessageArgs::default()
                    .content(format!("Call id: {}\n{}", call_id, error_msg))
                    .build()
                    .expect("Failed to build tool message for error")
                    .into(),
            );
        }

        messages
    }
}


impl MemoryStep for PlanningStep {
    fn dict(&self) -> HashMap<String, Value> {
        let mut output = HashMap::new();
        output.insert(
            "model_input_messages".to_string(),
            Value::Array(
                self.model_input_messages
                    .iter()
                    .map(|msg| serde_json::to_value(msg).expect("serialize msg"))
                    .collect(),
            ),
        );
        output.insert(
            "model_output_message".to_string(),
            self.model_output_message
                .as_ref()
                .map_or(Value::Null, |msg| {
                    serde_json::to_value(msg).expect("serialize model_output_message")
                }),
        );
        output.insert("plan".to_string(), Value::String(self.plan.clone()));
        output.insert(
            "timing".to_string(),
            Value::Object(Map::from_iter(self.timing.dict().into_iter())),
        );
        output.insert(
            "token_usage".to_string(),
            self.token_usage.as_ref().map_or(Value::Null, |usage| {
                let mut usage_map = Map::new();
                usage_map.insert("prompt_tokens".to_string(), Value::Number(usage.prompt_tokens.into()));
                usage_map.insert("completion_tokens".to_string(), Value::Number(usage.completion_tokens.into()));
                usage_map.insert("total_tokens".to_string(), Value::Number(usage.total_tokens.into()));
                Value::Object(usage_map)
            }),
        );

        output
    }

    fn to_message(&self, summary_mode: bool) -> Vec<ChatCompletionRequestMessage> {
        if summary_mode {
            vec![]
        } else {
            let mut messages = Vec::new();
            messages.push(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .content(self.plan.clone())
                    .build()
                    .expect("Failed to build assistant message")
                    .into(),
            );
            messages.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content("Now proceed and carry out this plan.".to_string())
                    .build()
                    .expect("Failed to build user message")
                    .into(),
            );
            messages
        }
    }
}

impl MemoryStep for TaskStep {
    fn dict(&self) -> HashMap<String, Value> {
        let mut output = HashMap::new();
        output.insert("task".to_string(), Value::String(self.task.clone()));
        output.insert(
            "task_images".to_string(),
            self.task_images.as_ref().map_or(Value::Null, |images| {
                Value::Array(images.iter().map(|s| Value::String(s.clone())).collect())
            }),
        );
        output
    }

    fn to_message(&self, summary_mode: bool) -> Vec<ChatCompletionRequestMessage> {
        info!("TaskStep to_message called with summary_mode={}", summary_mode);
        let mut messages = Vec::new();
        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(format!("New task:\n{}", self.task))
                .build()
                .expect("Failed to build user message for task")
                .into(),
        );
        if let Some(images) = &self.task_images {
            for img in images {
                messages.push(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(format!("Task image: {}", img))
                        .build()
                        .expect("Failed to build user message for image")
                        .into(),
                );
            }
        }

        messages
    }
}

impl MemoryStep for SystemPromptStep {
    fn dict(&self) -> HashMap<String, Value> {
        let mut output = HashMap::new();
        output.insert("system_prompt".to_string(), Value::String(self.system_prompt.clone()));
        output
    }

    fn to_message(&self, summary_mode: bool) -> Vec<ChatCompletionRequestMessage> {
        if summary_mode {
            vec![]
        } else {
            vec![ChatCompletionRequestUserMessageArgs::default()
                .content(self.system_prompt.clone())
                .build()
                .expect("Failed to build user message for system prompt")
                .into()]
        }
    }
}

impl MemoryStep for FinalAnswerStep {
    fn dict(&self) -> HashMap<String, Value> {
        let mut output = HashMap::new();
        output.insert("output".to_string(), Value::String(self.output.clone()));
        output
    }

    fn to_message(&self, summary_mode: bool) -> Vec<ChatCompletionRequestMessage> {
        if summary_mode {
            vec![]
        } else {
            vec![ChatCompletionRequestUserMessageArgs::default()
                .content(self.output.clone())
                .build()
                .expect("Failed to build user message for final answer")
                .into()]
        }
    }
}

impl AgentMemoryBase for AgentMemory {
    fn reset(&mut self) {
        self.steps.clear();
    }
    fn get_succinct_steps(&self) -> Vec<Value> {
        self.steps.iter().map(|step| {
            // 1) 원래 dict 생성
            let mut data = match step {
                Step::Task(ts)     => ts.dict(),
                Step::Action(as_)  => as_.dict(),
                Step::Planning(ps) => ps.dict(),
            };
            // 2) model_input_messages 키만 제거
            data.remove("model_input_messages");
            // 3) HashMap → serde_json::Map → Value::Object
            Value::Object(Map::from_iter(data.into_iter()))
        })
            .collect()
    }

    fn get_full_steps(&self) -> Vec<Value> {
        if self.steps.is_empty() {
            vec![]
        } else {
            self.steps.iter().map(|step| {
                match step {
                    Step::Task(ts)     => ts.dict(),
                    Step::Action(as_)  => as_.dict(),
                    Step::Planning(ps) => ps.dict(),
                }
            })
            .map(|data| Value::Object(Map::from_iter(data.into_iter())))
            .collect()
        }
    }

    fn replay(&self) {
        todo!()
    }

    fn return_full_code(&self) -> String {
        let full_code: Vec<String> = self.steps.iter()
            .filter_map(|step| {
                if let Step::Action(action_step) = step {
                    action_step.code_action.clone()
                } else {
                    None
                }
            })
            .collect();
        full_code.join("\n\n")
    }
}


impl CallbackRegistry {
    pub fn new() -> Self {
        Self { callbacks: HashMap::new() }
    }

    pub fn register<S, F>(&mut self, callback: F)
    where
        S: MemoryStep + 'static,
        F: Fn(&S) + 'static,
    {
        // Box<dyn Fn(&dyn MemoryStep)> 형태로 래핑
        let wrapped: Callback = Box::new(move |step: &dyn MemoryStep| {
            // 실제로는 S 타입인지 downcast 후 호출
            if let Some(s) = step.as_any().downcast_ref::<S>() {
                callback(s);
            }
        });

        self
            .callbacks
            .entry(TypeId::of::<S>())
            .or_default()
            .push(wrapped);
    }

    pub fn callback(&self, memory_step: &dyn MemoryStep) {
        let tid = memory_step.as_any().type_id();
        if let Some(cbs) = self.callbacks.get(&tid) {
            for cb in cbs {
                cb(memory_step);
            }
        }
    }
}