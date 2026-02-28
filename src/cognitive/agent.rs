use super::config::LlmConfig;
use super::llm::ask_llm;
use agentropic_cognition::{Belief, BeliefBase, ReasoningEngine, Rule};
use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{info, warn};

#[derive(Deserialize, Serialize)]
struct BeliefsFile {
    beliefs: Vec<BeliefEntry>,
}

#[derive(Deserialize, Serialize, Clone)]
struct BeliefEntry {
    key: String,
    value: String,
    certainty: f64,
}

/// A reusable agent that reasons from a knowledge base and falls back to an LLM
pub struct CognitiveAgent {
    id: AgentId,
    beliefs: BeliefBase,
    engine: ReasoningEngine,
    config: LlmConfig,
    beliefs_path: String,
}

impl CognitiveAgent {
    /// Create from config files
    pub fn from_config(beliefs_path: &str, config_path: &str) -> Self {
        Self {
            id: AgentId::new(),
            beliefs: load_beliefs(beliefs_path),
            engine: ReasoningEngine::new(),
            config: LlmConfig::from_file(config_path),
            beliefs_path: beliefs_path.to_string(),
        }
    }

    /// Create with explicit config
    pub fn new(beliefs_path: &str, config: LlmConfig) -> Self {
        Self {
            id: AgentId::new(),
            beliefs: load_beliefs(beliefs_path),
            engine: ReasoningEngine::new(),
            config,
            beliefs_path: beliefs_path.to_string(),
        }
    }

    /// Add a reasoning rule that maps keywords to a belief key
    pub fn add_rule(&mut self, rule: Rule) {
        self.engine.add_rule(rule);
    }

    /// Add a belief directly
    pub fn add_belief(&mut self, key: &str, value: &str) {
        self.beliefs.add(Belief::new(key, value));
    }

    /// Get belief count
    pub fn belief_count(&self) -> usize {
        self.beliefs.len()
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.engine.rules().len()
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .filter(|w| !w.is_empty() && w.len() > 1)
            .collect()
    }

    /// Reason about a question: try knowledge base first, fall back to LLM
    pub async fn think(&mut self, question: &str) -> (String, &'static str) {
        let facts = self.tokenize(question);

        // Try reasoning engine first
        if let Some(inference) = self.engine.best_match(&facts) {
            println!("    💭 Matched: {} (confidence: {:.0}%)", inference.rule_name, inference.confidence * 100.0);
            if inference.confidence >= self.config.confidence_threshold {
                let topic = &inference.conclusions[0];
                if let Some(belief) = self.beliefs.get(topic) {
                    return (belief.value().to_string(), "knowledge_base");
                }
            } else {
                println!("    💭 Below threshold ({:.0}%)", self.config.confidence_threshold * 100.0);
            }
        } else {
            println!("    💭 No matching rule");
        }

        // Fall back to LLM
        match ask_llm(question, &self.config, &self.beliefs).await {
            Some(answer) => {
                // Save as new belief
                if self.config.save_new_beliefs {
                    let key = format!(
                        "llm:{}",
                        question.to_lowercase().replace(' ', "_").chars().take(50).collect::<String>()
                    );
                    self.beliefs.add(Belief::with_certainty(&key, &answer, 0.8));
                    save_belief_to_file(&self.beliefs_path, &key, &answer);
                    info!("Saved new belief: {} (total: {})", key, self.beliefs.len());
                }
                (answer, "llm")
            }
            None => {
                ("I don't have enough knowledge to answer that right now.".to_string(), "none")
            }
        }
    }
}

#[async_trait]
impl Agent for CognitiveAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        info!(
            "CognitiveAgent initialized: {} beliefs, {} rules, LLM: {} ({})",
            self.beliefs.len(),
            self.engine.rules().len(),
            self.config.llm_provider,
            self.config.llm_model
        );
        Ok(())
    }

    async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        info!("CognitiveAgent shutdown. Final belief count: {}", self.beliefs.len());
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &AgentContext,
        sender: &str,
        _performative: &str,
        content: &str,
    ) -> AgentResult<()> {
        let (answer, source) = self.think(content).await;
        info!("Answered from {}: \"{}\"", source, &answer[..answer.len().min(80)]);
        ctx.send_message(sender, "inform", &answer);
        Ok(())
    }
}

fn load_beliefs(path: &str) -> BeliefBase {
    let mut base = BeliefBase::new();
    match fs::read_to_string(path) {
        Ok(contents) => {
            let file: BeliefsFile =
                serde_json::from_str(&contents).unwrap_or_else(|e| panic!("Invalid beliefs.json: {}", e));
            for entry in file.beliefs {
                base.add(Belief::with_certainty(&entry.key, &entry.value, entry.certainty));
            }
        }
        Err(e) => {
            warn!("Could not load {}: {}", path, e);
        }
    }
    base
}

fn save_belief_to_file(path: &str, key: &str, value: &str) {
    let mut file: BeliefsFile = match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or(BeliefsFile { beliefs: vec![] }),
        Err(_) => BeliefsFile { beliefs: vec![] },
    };

    if file.beliefs.iter().any(|b| b.key == key) {
        return;
    }

    file.beliefs.push(BeliefEntry {
        key: key.to_string(),
        value: value.to_string(),
        certainty: 0.8,
    });

    if let Ok(json) = serde_json::to_string_pretty(&file) {
        let _ = fs::write(path, json);
    }
}
