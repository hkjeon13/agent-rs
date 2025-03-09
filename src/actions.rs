use serde_json::{Value, Map, from_str};

pub struct State {
    step_number: u32,
    available_actions: Vec<Action>,
    selected_action: Option<Action>,
}

#[derive(Clone)]
pub struct ActionParameter {
    name: String,
    value_type: String,
    default: String,
}

pub struct Action {
    name: String,
    parameters: Vec<ActionParameter>,
    inputs: Vec<ActionParameter>,
    outputs: String,
}

impl State {
    pub fn new(step_number: u32, available_actions: Vec<Action>) -> Self {
        Self {
            step_number,
            available_actions,
            selected_action: None,
        }
    }

    pub fn select_action(&mut self, action: Action) {
        self.selected_action = Some(action);
    }
}


impl Action {
    fn new(name: String, parameters:Vec<ActionParameter>) -> Self {
        Self {
            name,
            parameters,
            inputs: Vec::new(),
            outputs: String::new(),
        }
    }

    pub fn set_outputs(&mut self, outputs: String) {
        self.outputs = outputs;
    }

    fn set_parameters(&mut self, parameters: Vec<ActionParameter>) {
        self.parameters = parameters;
    }

    fn parse_parameters(&self, inputs: String) -> Vec<ActionParameter> {
        let input_json:Value = from_str(&inputs).expect("Failed to parse inputs");
        let input_map:Map<String, Value> = input_json.as_object().expect("Failed to parse inputs").clone();
        let mut parameters:Vec<ActionParameter> = Vec::new();
        for parameter in self.parameters.iter() {
            let value = input_map.get(&parameter.name).expect("Failed to get parameter value");
            let value_type = value.as_str().expect("Failed to get value type");
            let default_value = parameter.default.clone();

            parameters.push(ActionParameter {
                name: parameter.name.clone(),
                value_type: value_type.to_string(),
                default: default_value,
            });
        }
        parameters
    }

    pub fn act(&self, inputs: String) -> Self {
        let parameters: Vec<ActionParameter> = self.parse_parameters(inputs);
        Self {
            name: self.name.clone(),
            parameters: self.parameters.clone(),
            inputs: parameters,
            outputs: self.outputs.clone(),
        }
    }
}



