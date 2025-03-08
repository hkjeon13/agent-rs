
trait Model {
    fn generate(&self, input: &str) -> String;
}

struct OpenAIModel {
    api_key: String,
    model_name: String,
}

struct AzureOpenAIModel {
    api_key: String,
    api_version: String,
    azure_endpoint: String,
    model_name: String,
}




