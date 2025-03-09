use std::collections::HashMap;
use std::process::Command;
use async_trait::async_trait;
use tracing::info;
use std::fmt;
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

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {{ type: {}, description: {} }}", self.name, self.dtype, self.description)
    }
}


#[async_trait]
pub trait Action: Send + Sync {
    fn prepare_inputs(&self, inputs: Vec<ActionInput>) -> HashMap<String, ActionInput> {
        self.get_parameters()
            .iter()
            .filter_map(|param| {
                inputs.iter().find(|input| {
                    param.name == input.key && param.dtype.eq_ignore_ascii_case(&input.dtype)
                })
                    .map(|input| (param.name.clone(), input.clone()))
            })
            .collect()
    }
    fn as_str(&self) -> String;
    fn get_parameters(&self) -> &Vec<Parameter>;
    async fn act(&self, inputs: Vec<ActionInput>) -> Observation;
}

#[derive(Clone, Debug)]
pub struct ActionBase {
    pub name: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
    pub output_type: String,
}


pub struct NaverNewsSearchAction {
    pub info: ActionBase,
    pub client_id: String,
    pub client_secret: String,
}

pub struct DuckDuckGoSearchAction {
    pub info: ActionBase,
}

impl NaverNewsSearchAction {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            info: ActionBase {
                name: "NaverNewsSearchAction".to_string(),
                description: "Search the web using Naver News".to_string(),
                parameters: vec![
                    Parameter {
                        name: "query".to_string(),
                        dtype: "String".to_string(),
                        description: "Search query".to_string(),
                    },
                ],
                output_type: "String".to_string(),
            },
            client_id,
            client_secret,
        }
    }
}

impl DuckDuckGoSearchAction {
    pub fn new() -> Self {
        Self {
            info: ActionBase {
                name: "DuckDuckGoSearchAction".to_string(),
                description: "Search the web using DuckDuckGo".to_string(),
                parameters: vec![
                    Parameter {
                        name: "query".to_string(),
                        dtype: "String".to_string(),
                        description: "Search query".to_string(),
                    },
                ],
                output_type: "String".to_string(),
            },
        }
    }
}


#[async_trait]
impl Action for NaverNewsSearchAction {
    fn as_str(&self) -> String {
        format!("- {}: {}\n\tTakes inputs: {:?}\n\tReturns an output of type: {}", self.info.name, self.info.description, self.info.parameters, self.info.output_type)
    }

    fn get_parameters(&self) -> &Vec<Parameter> {
        &self.info.parameters
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
    fn as_str(&self) -> String {
        format!("- {}: {}\n\tTakes inputs: {:?}\n\tReturns an output of type: {}", self.info.name, self.info.description, self.info.parameters, self.info.output_type)
    }

    fn get_parameters(&self) -> &Vec<Parameter> {
        &self.info.parameters
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
