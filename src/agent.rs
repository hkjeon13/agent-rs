// Module: Agent, MultiStepAgent, CodeAgent
// This concepts of modules and traits is inspired by the github repo: https://github.com/huggingface/smolagents.git (MIT License, written in Python)


trait LMModel {
    fn predict(&self, input: &str) -> String;
}

trait Tokenizer {
    fn tokenize(&self, input: &str) -> Vec<String>;
}

struct Agent<M: LMModel, T: Tokenizer> {
    model: M,
    tokenizer: T,
    prompt: String,
}

struct MultiStepAgent<M: LMModel, T: Tokenizer> {
    agent: Agent<M, T>
}

struct CodeAgent<M: LMModel, T: Tokenizer> {
    agent: MultiStepAgent<M, T>
}




