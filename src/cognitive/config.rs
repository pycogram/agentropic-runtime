use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone, Debug)]
pub struct LlmConfig {
    pub llm_provider: String,
    pub llm_model: String,
    pub ollama_url: String,
    pub claude_url: String,
    pub api_key: String,
    pub confidence_threshold: f64,
    pub save_new_beliefs: bool,
}

impl LlmConfig {
    pub fn from_file(path: &str) -> Self {
        let contents = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Could not read config file '{}': {}", path, e));
        serde_json::from_str(&contents)
            .unwrap_or_else(|e| panic!("Invalid config JSON in '{}': {}", path, e))
    }

    pub fn default_ollama() -> Self {
        Self {
            llm_provider: "ollama".to_string(),
            llm_model: "mistral".to_string(),
            ollama_url: "http://localhost:11434/api/generate".to_string(),
            claude_url: "https://api.anthropic.com/v1/messages".to_string(),
            api_key: String::new(),
            confidence_threshold: 0.35,
            save_new_beliefs: true,
        }
    }

    pub fn default_claude(api_key: &str) -> Self {
        Self {
            llm_provider: "claude".to_string(),
            llm_model: "claude-sonnet-4-20250514".to_string(),
            ollama_url: "http://localhost:11434/api/generate".to_string(),
            claude_url: "https://api.anthropic.com/v1/messages".to_string(),
            api_key: api_key.to_string(),
            confidence_threshold: 0.35,
            save_new_beliefs: true,
        }
    }
}
