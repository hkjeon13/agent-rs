use models::Model;

trait Agent {
    fn run(&self, input: &str) -> String;
    fn step(&self, state: &str) -> String;
}

struct Agent<M: Model> {
    model: M,
    max_steps: usize,
}


impl Agent {
    fn new(model: M, max_steps: usize) -> Self {
        Self {
            model,
            max_steps,
        }
    }

    fn new(model: M) -> Self {
        Self {
            model,
            max_steps: 10,
        }
    }
}

impl Agent {
    fn run(&self, input: &str) -> String {
        self.model.generate(input)
    }

    fn step(&self, state: &str) -> String {
        self.model.generate(state)
    }
}




