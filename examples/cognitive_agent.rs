use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
use agentropic_cognition::Rule;
use agentropic_runtime::prelude::*;
use agentropic_runtime::{CognitiveAgent, LlmConfig};
use async_trait::async_trait;

/// Agent that asks questions
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
                "What is Agentropic?",
                "What patterns does it support?",
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
        println!("  [Curious] I have {} questions!", self.questions.len());
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
        println!("  [Curious] Done!");
        Ok(())
    }

    async fn handle_message(
        &mut self, _ctx: &AgentContext, _sender: &str, _perf: &str, content: &str,
    ) -> AgentResult<()> {
        println!("  [Curious] ← Answer: \"{}\"", content);
        self.index += 1;
        self.waiting = false;
        if self.index >= self.questions.len() {
            println!("\n  [Curious] All questions answered! ✓");
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║   Agentropic — Cognitive Agent Demo                    ║");
    println!("║   Using CognitiveAgent from framework                  ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // Build a cognitive agent in just a few lines
    let mut thinker = CognitiveAgent::from_config("data/beliefs.json", "data/config.json");

    // Add reasoning rules
    thinker.add_rule(Rule::new("topic:what_is")
        .with_condition("what").with_condition("agentropic").with_condition("about")
        .with_conclusion("what_is_agentropic"));
    thinker.add_rule(Rule::new("topic:patterns")
        .with_condition("pattern").with_condition("support").with_condition("organization")
        .with_conclusion("patterns"));
    thinker.add_rule(Rule::new("topic:bdi")
        .with_condition("bdi").with_condition("belief").with_condition("desire").with_condition("reasoning")
        .with_conclusion("bdi"));
    thinker.add_rule(Rule::new("topic:messaging")
        .with_condition("message").with_condition("router").with_condition("communicate")
        .with_conclusion("messaging"));
    thinker.add_rule(Rule::new("topic:runtime")
        .with_condition("runtime").with_condition("supervisor").with_condition("restart")
        .with_conclusion("runtime"));

    println!("  Built CognitiveAgent: {} beliefs, {} rules\n",
        thinker.belief_count(), thinker.rule_count());

    let runtime = Runtime::new();
    runtime.spawn(Box::new(thinker), "thinker").await?;
    runtime.spawn(Box::new(CuriousAgent::new()), "curious").await?;

    tokio::time::sleep(std::time::Duration::from_secs(120)).await;

    println!("\n--- Shutting down ---\n");
    runtime.shutdown().await?;
    println!("\n✓ Demo complete.");
    Ok(())
}
