use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
use agentropic_cognition::{Belief, BeliefBase, ReasoningEngine, Rule};
use agentropic_runtime::prelude::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;

// ── Config ──────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
struct LlmConfig {
    llm_provider: String,
    llm_model: String,
    ollama_url: String,
    claude_url: String,
    api_key: String,
    confidence_threshold: f64,
    save_new_beliefs: bool,
}

fn load_config(path: &str) -> LlmConfig {
    let contents = fs::read_to_string(path).expect("Could not read config.json");
    serde_json::from_str(&contents).expect("Invalid config.json")
}

// ── Beliefs loader ──────────────────────────────────────────────────

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

fn load_beliefs(path: &str) -> BeliefBase {
    let mut base = BeliefBase::new();
    match fs::read_to_string(path) {
        Ok(contents) => {
            let file: BeliefsFile = serde_json::from_str(&contents)
                .expect("Invalid beliefs.json");
            for entry in file.beliefs {
                base.add(Belief::with_certainty(&entry.key, &entry.value, entry.certainty));
            }
            println!("  📂 Loaded {} beliefs from {}", base.len(), path);
        }
        Err(e) => {
            println!("  ⚠️  Could not load {}: {}", path, e);
        }
    }
    base
}

fn save_belief_to_file(path: &str, key: &str, value: &str) {
    let mut file: BeliefsFile = match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or(BeliefsFile { beliefs: vec![] }),
        Err(_) => BeliefsFile { beliefs: vec![] },
    };

    // Don't duplicate
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

// ── LLM caller ──────────────────────────────────────────────────────

async fn ask_llm(question: &str, config: &LlmConfig, beliefs: &BeliefBase) -> Option<String> {
    let client = reqwest::Client::new();

    // Build context from existing beliefs (limit to 10 to keep prompt small)
    let mut context = String::from("Here are verified facts about Agentropic:\n");
    for (i, belief) in beliefs.all().enumerate() {
        if i >= 10 { break; }
        context.push_str(&format!("- {}\n", belief.value()));
    }

    let system_prompt = format!(
        "You are a knowledgeable assistant for Agentropic, a multi-agent framework built in Rust.\n\
         {}\n\
         Use ONLY the facts above to answer. If the facts don't cover the question, say so honestly.\n\
         Always spell the name correctly: Agentropic.\n\
         Answer concisely in 1-3 sentences.",
        context
    );

    match config.llm_provider.as_str() {
        "ollama" => {
            let body = serde_json::json!({
                "model": config.llm_model,
                "prompt": format!(
                    "{}\n\nQuestion: {}\nAnswer:",
                    system_prompt, question
                ),
                "stream": false
            });

            match client.post(&config.ollama_url).json(&body).send().await {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        json["response"].as_str().map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                }
                Err(e) => {
                    println!("    ⚠️  Ollama error: {}", e);
                    None
                }
            }
        }
        "claude" => {
            let body = serde_json::json!({
                "model": config.llm_model,
                "max_tokens": 200,
                "system": system_prompt,
                "messages": [{
                    "role": "user",
                    "content": question
                }]
            });

            match client
                .post(&config.claude_url)
                .header("x-api-key", &config.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        json["content"][0]["text"].as_str().map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                }
                Err(e) => {
                    println!("    ⚠️  Claude error: {}", e);
                    None
                }
            }
        }
        other => {
            println!("    ⚠️  Unknown LLM provider: {}", other);
            None
        }
    }
}

// ── Reasoning rules ─────────────────────────────────────────────────

fn build_rules() -> ReasoningEngine {
    let mut engine = ReasoningEngine::new();

    engine.add_rule(Rule::new("topic:what_is")
        .with_condition("what").with_condition("agentropic").with_condition("about").with_condition("explain").with_condition("tell")
        .with_conclusion("what_is_agentropic"));

    engine.add_rule(Rule::new("topic:crates")
        .with_condition("crate").with_condition("module").with_condition("component").with_condition("layer").with_condition("architecture")
        .with_conclusion("crates"));

    engine.add_rule(Rule::new("topic:patterns")
        .with_condition("pattern").with_condition("organization").with_condition("structure").with_condition("support")
        .with_conclusion("patterns"));

    engine.add_rule(Rule::new("topic:messaging")
        .with_condition("message").with_condition("router").with_condition("communicate").with_condition("performative").with_condition("fipa").with_condition("send")
        .with_conclusion("messaging"));

    engine.add_rule(Rule::new("topic:bdi")
        .with_condition("bdi").with_condition("belief").with_condition("desire").with_condition("intention").with_condition("reasoning")
        .with_conclusion("bdi"));

    engine.add_rule(Rule::new("topic:runtime")
        .with_condition("runtime").with_condition("supervisor").with_condition("restart").with_condition("crash").with_condition("lifecycle")
        .with_conclusion("runtime"));

    engine.add_rule(Rule::new("topic:rust")
        .with_condition("rust").with_condition("performance").with_condition("safe").with_condition("fast").with_condition("async").with_condition("why")
        .with_conclusion("rust"));

    engine.add_rule(Rule::new("topic:swarm")
        .with_condition("swarm").with_condition("flock").with_condition("foraging").with_condition("decentralized").with_condition("consensus")
        .with_conclusion("swarm"));

    engine.add_rule(Rule::new("topic:market")
        .with_condition("market").with_condition("auction").with_condition("bid").with_condition("english").with_condition("dutch").with_condition("vickrey")
        .with_conclusion("market"));

    engine.add_rule(Rule::new("topic:hierarchy")
        .with_condition("hierarchy").with_condition("command").with_condition("chain").with_condition("top-down").with_condition("strategic")
        .with_conclusion("hierarchy"));

    engine.add_rule(Rule::new("topic:coalition")
        .with_condition("coalition").with_condition("alliance").with_condition("temporary").with_condition("join")
        .with_conclusion("coalition"));

    engine.add_rule(Rule::new("topic:federation")
        .with_condition("federation").with_condition("voting").with_condition("governance").with_condition("policy")
        .with_conclusion("federation"));

    engine.add_rule(Rule::new("topic:team")
        .with_condition("team").with_condition("role").with_condition("leader").with_condition("coordinator")
        .with_conclusion("team"));

    engine.add_rule(Rule::new("topic:holarchy")
        .with_condition("holarchy").with_condition("holon").with_condition("nested").with_condition("autonomous")
        .with_conclusion("holarchy"));

    engine.add_rule(Rule::new("topic:blackboard")
        .with_condition("blackboard").with_condition("shared").with_condition("knowledge").with_condition("collaborative")
        .with_conclusion("blackboard"));

    engine.add_rule(Rule::new("topic:getting_started")
        .with_condition("start").with_condition("install").with_condition("setup").with_condition("begin").with_condition("how").with_condition("tutorial")
        .with_conclusion("getting_started"));

    engine.add_rule(Rule::new("topic:agent_trait")
        .with_condition("trait").with_condition("implement").with_condition("interface").with_condition("methods")
        .with_conclusion("agent_trait"));

    engine.add_rule(Rule::new("topic:handle_message")
        .with_condition("handle").with_condition("receive").with_condition("incoming").with_condition("handle_message")
        .with_conclusion("handle_message"));

    engine.add_rule(Rule::new("topic:supervisor")
        .with_condition("supervisor").with_condition("monitor").with_condition("restart").with_condition("policy").with_condition("crash")
        .with_conclusion("supervisor"));

    engine.add_rule(Rule::new("topic:performatives")
        .with_condition("performative").with_condition("inform").with_condition("request").with_condition("query").with_condition("propose").with_condition("cfp")
        .with_conclusion("performatives"));

    engine.add_rule(Rule::new("topic:xbot")
        .with_condition("xbot").with_condition("twitter").with_condition("bot").with_condition("tweet")
        .with_conclusion("xbot"));

    engine.add_rule(Rule::new("topic:vision")
        .with_condition("vision").with_condition("future").with_condition("roadmap").with_condition("goal").with_condition("plan")
        .with_conclusion("vision"));

    engine.add_rule(Rule::new("topic:open_source")
        .with_condition("license").with_condition("open").with_condition("source").with_condition("free").with_condition("mit")
        .with_conclusion("open_source"));

    engine.add_rule(Rule::new("topic:context")
        .with_condition("context").with_condition("agentcontext").with_condition("log").with_condition("send_message")
        .with_conclusion("agent_context"));

    engine.add_rule(Rule::new("topic:cognition")
        .with_condition("cognition").with_condition("thinking").with_condition("intelligence").with_condition("decide").with_condition("brain")
        .with_conclusion("cognition_crate"));

    engine
}

// ── Cognitive Agent ─────────────────────────────────────────────────

struct CognitiveAgent {
    id: AgentId,
    beliefs: BeliefBase,
    engine: ReasoningEngine,
    config: LlmConfig,
    beliefs_path: String,
}

impl CognitiveAgent {
    fn new(beliefs_path: &str, config_path: &str) -> Self {
        Self {
            id: AgentId::new(),
            beliefs: load_beliefs(beliefs_path),
            engine: build_rules(),
            config: load_config(config_path),
            beliefs_path: beliefs_path.to_string(),
        }
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .filter(|w| !w.is_empty() && w.len() > 1)
            .collect()
    }

    async fn think(&mut self, question: &str) -> String {
        let facts = self.tokenize(question);
        println!("    💭 Tokenized: {:?}", facts);

        // Step 1: Try reasoning engine
        if let Some(inference) = self.engine.best_match(&facts) {
            if inference.confidence >= self.config.confidence_threshold {
                let topic = &inference.conclusions[0];
                println!("    💭 Matched: {} (confidence: {:.0}%)", inference.rule_name, inference.confidence * 100.0);

                if let Some(belief) = self.beliefs.get(topic) {
                    println!("    💭 Source: Knowledge Base ✓");
                    return belief.value().to_string();
                }
            } else {
                println!("    💭 Low confidence match: {:.0}% (threshold: {:.0}%)",
                    inference.confidence * 100.0, self.config.confidence_threshold * 100.0);
            }
        } else {
            println!("    💭 No matching rule found");
        }

        // Step 2: Fall back to LLM
        println!("    🤖 Asking {} ({})...", self.config.llm_provider, self.config.llm_model);

        match ask_llm(question, &self.config, &self.beliefs).await {
            Some(answer) => {
                println!("    🤖 Source: LLM ({}) ✓", self.config.llm_provider);

                // Step 3: Store as new belief
                if self.config.save_new_beliefs {
                    let key = format!("llm:{}", question.to_lowercase().replace(' ', "_").chars().take(50).collect::<String>());
                    self.beliefs.add(Belief::with_certainty(&key, &answer, 0.8));
                    save_belief_to_file(&self.beliefs_path, &key, &answer);
                    println!("    💾 Saved as new belief: {} (beliefs now: {})", key, self.beliefs.len());
                }

                answer
            }
            None => {
                println!("    ⚠️  LLM unavailable");
                "I don't have enough knowledge to answer that right now, and my LLM fallback is unavailable.".to_string()
            }
        }
    }
}

#[async_trait]
impl Agent for CognitiveAgent {
    fn id(&self) -> &AgentId { &self.id }

    async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Thinker] Initialized: {} beliefs, {} rules, LLM: {} ({})",
            self.beliefs.len(), self.engine.rules().len(),
            self.config.llm_provider, self.config.llm_model);
        println!("  [Thinker] Confidence threshold: {:.0}%", self.config.confidence_threshold * 100.0);
        Ok(())
    }

    async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Thinker] Final belief count: {}", self.beliefs.len());
        println!("  [Thinker] Shutting down.");
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &AgentContext,
        _sender: &str,
        performative: &str,
        content: &str,
    ) -> AgentResult<()> {
        println!("  [Thinker] ← Received ({}): \"{}\"", performative, content);
        println!("  [Thinker] 🧠 Reasoning...");

        let answer = self.think(content).await;

        println!("  [Thinker] → Replying: \"{}\"", answer);
        ctx.send_message("curious", "inform", &answer);
        Ok(())
    }
}

// ── Curious Agent ───────────────────────────────────────────────────

struct CuriousAgent {
    id: AgentId,
    questions: Vec<&'static str>,
    index: usize,
    waiting: bool,
}

impl CuriousAgent {
    fn new() -> Self {
        Self {
            id: AgentId::new(),
            questions: vec![
                // These hit BeliefBase (instant)
                "What is Agentropic?",
                "What patterns does it support?",
                // This falls through to LLM (slow on CPU)
                "Can Agentropic agents collaborate with external APIs?",
            ],
            index: 0,
            waiting: false,
        }
    }
}

#[async_trait]
impl Agent for CuriousAgent {
    fn id(&self) -> &AgentId { &self.id }

    async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Curious] I have {} questions — 2 from knowledge base, 1 needs LLM!", self.questions.len());
        Ok(())
    }

    async fn execute(&mut self, ctx: &AgentContext) -> AgentResult<()> {
        if !self.waiting && self.index < self.questions.len() {
            let question = self.questions[self.index];
            println!("\n  [Curious] → Asking: \"{}\"", question);
            ctx.send_message("thinker", "query", question);
            self.waiting = true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Curious] Done asking questions!");
        Ok(())
    }

    async fn handle_message(
        &mut self,
        _ctx: &AgentContext,
        _sender: &str,
        _performative: &str,
        content: &str,
    ) -> AgentResult<()> {
        println!("  [Curious] ← Answer: \"{}\"", content);
        self.index += 1;
        self.waiting = false;

        if self.index >= self.questions.len() {
            println!("\n  [Curious] All {} questions answered! ✓", self.questions.len());
        }
        Ok(())
    }
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║   Agentropic — Cognitive Agent with LLM Fallback      ║");
    println!("║   Knowledge Base first, LLM when needed               ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    let runtime = Runtime::new();

    runtime.spawn(
        Box::new(CognitiveAgent::new("data/beliefs.json", "data/config.json")),
        "thinker"
    ).await?;
    runtime.spawn(Box::new(CuriousAgent::new()), "curious").await?;

    // Give enough time — LLM calls take a few seconds on CPU
    tokio::time::sleep(std::time::Duration::from_secs(120)).await;

    println!("\n--- Shutting down ---\n");
    runtime.shutdown().await?;

    println!("\n✓ Demo complete.");
    Ok(())
}
