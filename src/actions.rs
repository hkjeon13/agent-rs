use std::collections::HashMap;
use std::process::Command;

use async_trait::async_trait;
use tracing::info;

use crate::observation::Observation;

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub dtype: String,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct ActionInput {
    pub key: String,
    pub value: String,
    pub dtype: String,
}

#[async_trait]
pub trait Action {
    async fn act(&self, inputs: Vec<ActionInput>) -> Observation;
    fn get_parameters(&self) -> &Vec<Parameter>;
    fn prepare_inputs(&self, inputs: Vec<ActionInput>) -> HashMap<String, ActionInput> {
        self.get_parameters().iter().filter_map(|param| {
            inputs.iter().find(|input| {
                param.name == input.key && param.dtype.eq_ignore_ascii_case(&input.dtype)
            }).map(|input| (param.name.clone(), input.clone()))
        }).collect()
    }
}


pub struct NaverNewsSearchAction {
    pub name: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
    pub client_id: String,
    pub client_secret: String,
}

pub struct DuckDuckGoSearchAction {
    pub name: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
}


impl NaverNewsSearchAction {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            name: "NaverNewsSearchAction".to_string(),
            description: "Search News in www.naver.com".to_string(),
            parameters: vec![
                Parameter {
                    name: "query".to_string(),
                    dtype: "String".to_string(),
                    description: "Search query".to_string(),
                },
                Parameter {
                    name: "display".to_string(),
                    dtype: "Integer".to_string(),
                    description: "Number of results to display".to_string(),
                },
            ],
            client_id,
            client_secret,
        }
    }
}


impl DuckDuckGoSearchAction {
    pub fn new() -> Self {
        Self {
            name: "DuckDuckGoSearchAction".to_string(),
            description: "Search the web using DuckDuckGo".to_string(),
            parameters: vec![
                Parameter {
                    name: "query".to_string(),
                    dtype: "String".to_string(),
                    description: "Search query".to_string(),
                },
            ],
        }
    }
}


#[async_trait]
impl Action for NaverNewsSearchAction {
    fn get_parameters(&self) -> &Vec<Parameter> {
        &self.parameters
    }

    async fn act(&self, inputs: Vec<ActionInput>) -> Observation {
        info!("NaverNewsSearchAction.act() called");
        let matched_inputs = self.prepare_inputs(inputs);

        Observation {
            result: format!("Matched {} input(s)", matched_inputs.len()),
        }
    }
}


#[async_trait]
impl Action for DuckDuckGoSearchAction {
    fn get_parameters(&self) -> &Vec<Parameter> {
        &self.parameters
    }

    async fn act(&self, inputs: Vec<ActionInput>) -> Observation {
        info!("DuckDuckGoSearchAction.act() called");
        let matched_inputs = self.prepare_inputs(inputs);
        let query = matched_inputs.get("query").unwrap().value.clone();
        let output = Command::new("duckduckgo")
            .arg(format!("--query={}", query))
            .output()
            .expect("Failed to execute command");

        let stdout_str = String::from_utf8_lossy(&output.stdout);

        Observation {
            result: stdout_str.to_string(),
        }
    }
}


