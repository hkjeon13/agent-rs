use std::option::Option;

use crate::observation::Observation;

pub(crate) struct State {
    description: String,
    available_actions: Vec<Box<dyn crate::actions::Action>>,
    selected_action: Option<Box<dyn crate::actions::Action>>,
    observation: Option<Observation>,
}

impl State {
    fn new(self, description: String, available_actions: Vec<Box<dyn crate::actions::Action>>) -> Self {
        Self {
            description,
            available_actions,
            selected_action: Option::None,
            observation: Option::None,
        }
    }
}



