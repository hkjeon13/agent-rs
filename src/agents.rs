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
    async fn plan(&self, state: &str, step: usize, is_initial: bool) -> String {
        let facts_messages = self.generate_facts(state, is_initial).await;
        self.generate_plan(state, step, &facts_messages, is_initial).await
    }
    async fn generate_facts(&self, state: &str, is_initial: bool) -> String;
    async fn generate_plan(&self, state: &str, step: usize, facts: &str, is_initial: bool) -> String;
}

pub struct Agent<M: Model> {
    model: M,
    max_steps: usize,
    prompt: Prompt,
    available_actions: Vec<Box<dyn Action>>,
}

impl<M: Model> Agent<M> {
    pub fn new(model: M, max_steps: usize, available_actions: Vec<Box<dyn Action>>) -> Self {
        let prompt = load_config("data/toolcalling_agent.yaml");
        Self { model, max_steps, prompt, available_actions }
    }

    // Helper function to generate messages
    fn generate_message(role: &str, content: &str) -> HashMap<String, String> {
        let mut message = HashMap::new();
        message.insert("role".to_string(), role.to_string());
        message.insert("content".to_string(), content.to_string());
        message
    }

    // Generate a list of facts messages based on whether it's the initial run or not
    fn generate_facts_messages(&self, state: &str, is_initial: bool) -> Vec<HashMap<String, String>> {
        if is_initial {
            vec![Self::generate_message("user", self.prompt.planning.initial_facts.clone().replace("{task}", state).as_str())]
        } else {
            vec![
                Self::generate_message("system", &self.prompt.planning.update_facts_pre_messages),
                Self::generate_message("user", &self.prompt.planning.update_facts_post_messages),
            ]
        }
    }
}


#[async_trait]
impl<M: Model + Send + Sync> AgentBase for Agent<M> {
    async fn run(&self, query: &str) -> String {
        let plan_text = self.plan(query, 0, true).await;
        info!("Plan: {}", plan_text);
        todo!("Run method needs to be implemented for Agent.")
    }

    async fn step(&self, state: &str) -> String {
        todo!("Step method needs to be implemented for Agent.")
    }

    async fn generate_facts(&self, state: &str, is_initial: bool) -> String {
        let facts_messages = self.generate_facts_messages(state, is_initial);
        self.model.async_generate(facts_messages).await
    }

    async fn generate_plan(&self, state: &str, step: usize, facts: &str, is_initial: bool) -> String {

        let tools_str = self.available_actions
            .iter()
            .map(|action| action.as_str())
            .collect::<Vec<String>>()
            .join("\n");

        let managed_agents = String::new(); // You can enhance this with real managed agents logic

        let plan_messages = if is_initial {
            vec![
            self.generate_message("user", self.prompt.planning.initial_plan.clone()
                .replace("{task}", state)
                .replace("{tools}", &tools_str)
                .replace("{managed_agents}", &managed_agents)
                .replace("{answer_facts}", facts)
            )]
        } else {
            vec![
                self.generate_message("system", &self.prompt.planning.update_plan_pre_messages),
                self.generate_message(
                    "user",
                    &self.prompt.planning.update_plan_post_messages.clone()
                        .replace("{task}", state)
                        .replace("{tools}", &tools_str)
                        .replace("{managed_agents}", &managed_agents)
                        .replace("{facts_update}", facts)
                        .replace("{remaining_steps}", &(self.max_steps - step).to_string())
                ),
            ]
        };

        self.model.async_generate(plan_messages).await
    }


}
