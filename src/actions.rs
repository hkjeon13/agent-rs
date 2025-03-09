pub struct Memory {
    states: Vec<State>,
}

struct State {
    step_number: u32,
    available_actions: Vec<Action>,
    selected_action: Option<Action>,
}

pub struct Action {
    name: String,
    inputs: String,
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
    pub fn new(name: String, inputs: String) -> Self {
        Self {
            name,
            inputs,
            outputs: String::new(),
        }
    }

    pub fn set_outputs(&mut self, outputs: String) {
        self.outputs = outputs;
    }

}

