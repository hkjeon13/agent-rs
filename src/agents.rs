use crate::models::Model;
use async_trait::async_trait;
use std::{collections::HashMap, time::Instant};
use futures::StreamExt;
use tracing::info;
use crate::actions::Action;
use crate::prompts::{load_config, Prompt};

#[async_trait]
pub trait AgentBase {
    async fn run(&self, input: &str) -> String;
    async fn step(&self, state: &str) -> String;
    async fn plan(&self, state: &str, step: usize, is_initial: bool) -> String;
}

pub struct Agent<M: Model> {
    model: M,
    max_steps: usize,
    prompt: Prompt,
    available_actions: Vec<Box<dyn Action>>,
    stream_outputs: bool,
}

impl<M: Model> Agent<M> {
    pub fn new(
        model: M,
        max_steps: usize,
        available_actions: Vec<Box<dyn Action>>,
    ) -> Self {
        let prompt = load_config("data/toolcalling_agent.yaml");
        // 기본값으로 스트리밍 비활성화
        let stream_outputs = false;
        Self {
            model,
            max_steps,
            prompt,
            available_actions,
            stream_outputs,
        }
    }
}

#[async_trait]
impl<M: Model + Send + Sync> AgentBase for Agent<M> {
    async fn run(&self, query: &str) -> String {
        let plan_text = self.plan(query, 0, true).await;
        info!("Final plan: {}", plan_text);
        plan_text
    }

    async fn step(&self, _state: &str) -> String {
        "Agent::step() not implemented".to_string()
    }

    async fn plan(&self, state: &str, step: usize, is_initial: bool) -> String {
        let start = Instant::now();

        // 1) facts 메시지 준비
        let facts_msgs = if is_initial {
            vec![HashMap::from([
                ("role".into(), "user".into()),
                ("content".into(), self.prompt.planning.initial_plan.replace("{task}", state)),
            ])]
        } else {
            vec![
                HashMap::from([
                    ("role".into(), "system".into()),
                    ("content".into(), self.prompt.planning.update_plan_pre_messages.clone()),
                ]),
                // TODO: memory 메시지 삽입
                HashMap::from([
                    ("role".into(), "user".into()),
                    ("content".into(), self.prompt.planning.update_plan_post_messages.replace("{task}", state)),
                ]),
            ]
        };

        // 2) facts 생성 (단일 String 반환)
        let facts = self.model.async_generate(facts_msgs).await;

        // 3) tools & managed_agents 문자열 준비
        let tools_str = self
            .available_actions
            .iter()
            .map(|a| a.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let managed_agents = ""; // 필요 시 채우기

        // 4) plan 메시지 준비
        let plan_msgs = if is_initial {
            vec![HashMap::from([
                ("role".into(), "user".into()),
                (
                    "content".into(),
                    self.prompt
                        .planning
                        .initial_plan
                        .replace("{task}", state)
                        .replace("{tools}", &tools_str)
                        .replace("{managed_agents}", managed_agents)
                        .replace("{answer_facts}", &facts),
                ),
            ])]
        } else {
            vec![
                HashMap::from([
                    ("role".into(), "system".into()),
                    ("content".into(), self.prompt.planning.update_plan_pre_messages.clone()),
                ]),
                HashMap::from([
                    ("role".into(), "user".into()),
                    (
                        "content".into(),
                        self.prompt
                            .planning
                            .update_plan_post_messages
                            .replace("{task}", state)
                            .replace("{tools}", &tools_str)
                            .replace("{managed_agents}", managed_agents)
                            .replace("{facts_update}", &facts)
                            .replace("{remaining_steps}", &(self.max_steps - step).to_string()),
                    ),
                ]),
            ]
        };

        // 5) 스트리밍 vs 일괄 호출 분기
        let plan_text = if self.stream_outputs {
            match self.model.async_generate_stream(plan_msgs).await {
                Ok(mut stream) => {
                    let mut acc = String::new();
                    while let Some(Ok(bytes)) = stream.next().await {
                        let s = String::from_utf8_lossy(&bytes).to_string();
                        print!("{}", s);
                        acc.push_str(&s);
                    }
                    acc
                }
                Err((status, err)) => {
                    eprintln!("Stream error ({}): {}", status, err);
                    String::new()
                }
            }
        } else {
            self.model.async_generate(plan_msgs).await
        };

        // 6) 완료 로그
        info!(
            "{} plan completed in {:.2?}",
            if is_initial { "Initial" } else { "Update" },
            start.elapsed()
        );

        plan_text
    }
}
