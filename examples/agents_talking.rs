use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
use agentropic_runtime::prelude::*;
use async_trait::async_trait;

/// An agent that asks a question on its first tick
struct AskerAgent {
    id: AgentId,
    asked: bool,
}

impl AskerAgent {
    fn new() -> Self {
        Self {
            id: AgentId::new(),
            asked: false,
        }
    }
}

#[async_trait]
impl Agent for AskerAgent {
    fn id(&self) -> &AgentId { &self.id }

    async fn initialize(&mut self, ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Asker] Ready. I have a question for Responder...");
        Ok(())
    }

    async fn execute(&mut self, ctx: &AgentContext) -> AgentResult<()> {
        if !self.asked {
            println!("  [Asker] → Sending: \"What patterns does Agentropic support?\"");
            ctx.send_message("responder", "query", "What patterns does Agentropic support?");
            self.asked = true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Asker] Goodbye!");
        Ok(())
    }

    async fn handle_message(
        &mut self,
        _ctx: &AgentContext,
        _sender: &str,
        performative: &str,
        content: &str,
    ) -> AgentResult<()> {
        println!("  [Asker] ← Received ({}):", performative);
        println!("           \"{}\"", content);
        println!("  [Asker] Thanks! That's what I needed.");
        Ok(())
    }
}

/// An agent that answers questions about Agentropic
struct ResponderAgent {
    id: AgentId,
}

impl ResponderAgent {
    fn new() -> Self {
        Self { id: AgentId::new() }
    }
}

#[async_trait]
impl Agent for ResponderAgent {
    fn id(&self) -> &AgentId { &self.id }

    async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Responder] Ready. Waiting for questions...");
        Ok(())
    }

    async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Responder] Goodbye!");
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &AgentContext,
        sender: &str,
        performative: &str,
        content: &str,
    ) -> AgentResult<()> {
        println!("  [Responder] ← Received ({}) from Asker:", performative);
        println!("               \"{}\"", content);

        // Think about it...
        println!("  [Responder] Thinking...");
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Reply
        let answer = "Agentropic supports 8 patterns: Hierarchy, Swarm, Coalition, Market, Federation, Team, Holarchy, and Blackboard.";
        println!("  [Responder] → Replying: \"{}\"", answer);
        ctx.send_message("asker", "inform", answer);

        Ok(())
    }
}

/// A third agent that just observes and comments
struct ObserverAgent {
    id: AgentId,
    ticks: u32,
}

impl ObserverAgent {
    fn new() -> Self {
        Self {
            id: AgentId::new(),
            ticks: 0,
        }
    }
}

#[async_trait]
impl Agent for ObserverAgent {
    fn id(&self) -> &AgentId { &self.id }

    async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Observer] Watching the conversation...");
        Ok(())
    }

    async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        self.ticks += 1;
        if self.ticks == 5 {
            println!("  [Observer] Interesting — the agents are communicating!");
        }
        if self.ticks == 10 {
            println!("  [Observer] Still running. All systems nominal.");
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        println!("  [Observer] Observed {} ticks total. Goodbye!", self.ticks);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    println!("╔══════════════════════════════════════════╗");
    println!("║   Agentropic — Agents Talking Demo       ║");
    println!("╚══════════════════════════════════════════╝\n");

    let runtime = Runtime::new();

    println!("Spawning 3 agents...\n");

    runtime.spawn(Box::new(ResponderAgent::new()), "responder").await?;
    runtime.spawn(Box::new(AskerAgent::new()), "asker").await?;
    runtime.spawn(Box::new(ObserverAgent::new()), "observer").await?;

    println!("\n--- Agents running for 3 seconds ---\n");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    println!("\n--- Shutting down ---\n");
    runtime.shutdown().await?;

    println!("\n✓ All agents stopped. Demo complete.");

    Ok(())
}
