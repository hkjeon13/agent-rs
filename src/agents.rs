use crate::models::Model;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::info;
use crate::actions::Action;
use crate::prompts::{load_config, Prompt};


#[async_trait]
pub trait AgentBase {
    async fn run(&self, input: &str) -> String;
    async fn step(&self, state: &str) -> String;
    async fn plan(&self, state: &str, step:usize, is_initial: bool) -> String;
}


pub struct Agent<M: Model> {
    model: M,
    max_steps: usize,
    prompt: Prompt,
    available_actions: Vec<Box<dyn Action>>

}

impl<M: Model> Agent<M> {
    pub fn new(model: M, max_steps: usize, available_actions:Vec<Box<dyn Action>>) -> Self {
        let prompt = load_config("data/toolcalling_agent.yaml");
        Self { model, max_steps, prompt , available_actions }
    }
}


#[async_trait]
impl<M: Model + Send + Sync> AgentBase for Agent<M> {
    async fn run(&self, query: &str) -> String {
        let plan_text = self.plan(query, 0, true).await;
        info!("Plan: {}", plan_text);

        "Agent::run() not implemented".to_string()
    }

    async fn step(&self, state: &str) -> String {
        "Agent::step() not implemented".to_string()
    }

    async fn plan(&self, state: &str, step:usize, is_initial: bool) -> String {

        let facts_messages = match is_initial {
            true => vec![
                HashMap::from([
                    ("role".to_string(), "user".to_string()),
                    ("content".to_string(), self.prompt.planning.initial_facts.clone().replace("{task}", &state))
                ]),
            ],
            false => vec![
                HashMap::from([
                    ("role".to_string(), "system".to_string()),
                    ("content".to_string(), self.prompt.planning.update_facts_pre_messages.clone())
                ]),
                //todo: add Memory
                HashMap::from([
                    ("role".to_string(), "user".to_string()),
                    ("content".to_string(), self.prompt.planning.update_facts_post_messages.clone())
                ]),
            ]
        };

        let facts = self.model.async_generate(facts_messages).await;

        let tools_str = self.available_actions
            .iter().map(|action| {action.as_str() })
            .collect::<Vec<String>>().join("\n");

        //todo: managed_agents
        let managed_agents = "".to_string();

        let plan_messages = match is_initial {
            true => vec![
                HashMap::from([
                    ("role".to_string(), "user".to_string()),
                    (
                        "content".to_string(),
                        self.prompt.planning.initial_plan.clone()
                            .replace("{task}", &state)
                            .replace("{tools}", &tools_str)
                            .replace("{managed_agents}", &managed_agents)
                            .replace("{answer_facts}", &facts)
                    )
                ]),
            ],
            false => vec![
                HashMap::from([
                    ("role".to_string(), "system".to_string()),
                    ("content".to_string(), self.prompt.planning.update_plan_pre_messages.clone())
                ]),
                HashMap::from([
                    ("role".to_string(), "user".to_string()),
                    (
                        "content".to_string(),
                        self.prompt.planning.update_plan_post_messages.clone()
                            .replace("{task}", &state)
                            .replace("{tools}", &tools_str)
                            .replace("{managed_agents}", &managed_agents)
                            .replace("{facts_update}", &facts)
                            .replace("{remaining_steps}", &(self.max_steps - step).to_string())
                    )
                ]),
            ]
        };

        self.model.async_generate(plan_messages).await
    }
}

