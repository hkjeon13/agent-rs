use serde::Deserialize;
use std::fs;
use serde_yaml;

#[derive(Debug, Deserialize)]
pub struct Prompt {
    pub system_prompt: String,
    pub planning: Planning,
    pub managed_agent: ManagedAgent,
    pub final_answer: FinalAnswer,
}

#[derive(Debug, Deserialize)]
pub struct Planning {
    pub initial_facts: String,
    pub initial_plan: String,
    pub update_facts_pre_messages: String,
    pub update_facts_post_messages: String,
    pub update_plan_pre_messages: String,
    pub update_plan_post_messages: String,
}

#[derive(Debug, Deserialize)]
pub struct ManagedAgent {
    pub task: String,
    pub report: String,
}

#[derive(Debug, Deserialize)]
pub struct FinalAnswer {
    pub pre_messages: String,
    pub post_messages: String,
}

pub fn load_config(file_path: &str) -> Prompt {
    let file_content = fs::read_to_string(file_path)
        .expect("YAML 파일을 읽어오지 못했습니다.");
    serde_yaml::from_str(&file_content)
        .expect("YAML 파일 파싱에 실패했습니다.")
}
