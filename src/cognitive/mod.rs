//! Cognitive agent with knowledge base reasoning and LLM fallback
pub mod config;
pub mod llm;
pub mod agent;

pub use config::LlmConfig;
pub use agent::CognitiveAgent;
