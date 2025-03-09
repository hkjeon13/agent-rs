# agent-rs
LLM Agent Built in Rust

## Setup
### OpenAI API Key
Create a `secrets.toml` file in the project root and add:
```toml
[openai]
api_key= "sk-<your-api-key>"
```

### Running the Agent
```bash
cargo run --release
```

## Status
Current version supports only OpenAI's `/chat` API (not fully agentic).
