use crate::actions::Action;
use crate::models::Model;
use crate::prompts::{load_config, Prompt};
use async_stream::stream;
use async_trait::async_trait;
use futures::stream::once;
use futures::stream::Stream;
use futures::StreamExt;
use std::collections::HashMap;
use std::pin::Pin;
use std::time::Instant;
use tracing::info;

/// Represents either a streaming or text result from planning.
pub enum PlanOutput {
    Stream(Pin<Box<dyn Stream<Item = String> + Send>>),
    Text(String),
}

#[async_trait]
pub trait AgentBase {
    async fn run(&self, input: String) -> Pin<Box<dyn Stream<Item = String> + Send>>;
    async fn _run_stream(
        &self,
        task: String,
        max_steps: usize,
        images: Vec<String>,
    ) -> Pin<Box<dyn Stream<Item = String> + Send>>;
    async fn step(&self, state: &str) -> String;
    async fn plan(&self, state: &str, step: usize, is_initial: bool) -> PlanOutput;
}

pub struct Agent<M: Model> {
    model: M,
    max_steps: usize,
    prompt: Prompt,
    available_actions: Vec<Box<dyn Action>>,
    stream_outputs: bool,
    interrupt_switch: bool,
    planning_interval: Option<usize>,
}

impl<M: Model> Agent<M> {
    pub fn new(
        model: M,
        max_steps: usize,
        available_actions: Vec<Box<dyn Action>>,
        stream_outputs: bool,
    ) -> Self {
        let prompt = load_config("data/toolcalling_agent.yaml");
        Self {
            model,
            max_steps,
            prompt,
            available_actions,
            stream_outputs,
            interrupt_switch: false,
            planning_interval: None, // Default to None, can be set later
        }
    }
}

#[async_trait]
impl<M: Model + Send + Sync + Clone + 'static> AgentBase for Agent<M> {
    async fn run(&self, query: String) -> Pin<Box<dyn Stream<Item = String> + Send>> {
        info!("Agent::run() called with query: {}", query);
        self._run_stream(query.clone(), self.max_steps, vec![]).await
    }

    async fn _run_stream(
        &self,
        task: String,
        max_steps: usize,
        _images: Vec<String>,
    ) -> Pin<Box<dyn Stream<Item = String> + Send>> {
        // Clone model and flag so closure owns them
        let model = self.model.clone();
        let stream_outputs = self.stream_outputs;
        let task_str = task.clone();
        // Get the raw planning output as a stream
        let plan_output = self.plan(&task_str, 1, true).await;
        let mut plan_stream = match plan_output {
            PlanOutput::Stream(s) => s,
            PlanOutput::Text(t) => Box::pin(stream! { yield t.clone() }),
        };
        let combined = stream! {
            // Accumulate plan text while streaming
            let mut buf = String::new();
            while let Some(chunk) = plan_stream.next().await {
                buf.push_str(&chunk);
                yield chunk;
            }
            // Build the full plan_for_generation
            let plan_for_generation = format!(
                "Here are the facts I know and the plan of action that I will follow to solve the task:\n```\n{}\n```",
                buf
            );
            info!("Plan for generation: {}", plan_for_generation);

            // Prepare messages
            let mut messages = Vec::new();
            messages.push(HashMap::from([
                ("role".to_string(), "system".to_string()),
                ("content".to_string(), plan_for_generation.clone()),
            ]));
            messages.push(HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), task_str.to_string()),
            ]));

            // Stream or generate model output
            if stream_outputs {
                // Unified boxed stream
                let mut model_stream = match model.async_generate_stream(messages.clone()).await {
                    Ok(raw) => raw.map(|res| {
                        let bytes = res.unwrap_or_default();
                        String::from_utf8_lossy(&bytes).to_string()
                    }).boxed(),
                    Err(err) => {
                        info!("Generation stream error: {:?}", err);
                        stream! { yield String::new() }.boxed()
                    }
                };
                while let Some(chunk) = model_stream.next().await {
                    yield chunk;
                }
            } else {
                let text = model.async_generate(messages).await;
                yield text;
            }
        };
        Box::pin(combined)
    }

    async fn step(&self, _state: &str) -> String {
        "Agent::step() not implemented".to_string()
    }

    async fn plan(&self, state: &str, step: usize, is_initial: bool) -> PlanOutput {
        let start = Instant::now();

        let tools_str = self
            .available_actions
            .iter()
            .map(|a| a.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let managed_agents = ""; // 필요 시 채우기

        let input_messages = if is_initial {
            vec![HashMap::from([
                ("role".into(), "user".into()),
                (
                    "content".into(),
                    self.prompt
                        .planning
                        .initial_plan
                        .replace("{task}", state)
                        .replace("{tools}", &tools_str)
                        .replace("{managed_agents}", managed_agents),
                ),
            ])]
        } else {
            vec![
                HashMap::from([
                    ("role".into(), "system".into()),
                    (
                        "content".into(),
                        self.prompt.planning.update_plan_pre_messages.clone(),
                    ),
                ]),
                // TODO: memory 메시지 삽입
                HashMap::from([
                    ("role".into(), "user".into()),
                    (
                        "content".into(),
                        self.prompt
                            .planning
                            .update_plan_post_messages
                            .replace("{task}", state),
                    ),
                ]),
            ]
        };
        if self.stream_outputs {
            let raw_stream = match self.model.async_generate_stream(input_messages).await {
                Ok(s) => s,
                Err(err) => {
                    info!("Stream generation error: {:?}", err);
                    return PlanOutput::Text(String::new());
                }
            };
            let mapped = raw_stream.map(|chunk_res| {
                let bytes = chunk_res.unwrap_or_default();
                String::from_utf8_lossy(&bytes).to_string()
            });
            // Box and pin the stream
            let boxed: Pin<Box<dyn Stream<Item = String> + Send>> = Box::pin(mapped);
            info!("Plan generated in {} ms", start.elapsed().as_millis());
            PlanOutput::Stream(boxed)
        } else {
            let plan_text = self.model.async_generate(input_messages).await;
            info!("Plan generated in {} ms", start.elapsed().as_millis());
            PlanOutput::Text(plan_text)
        }
    }
}
