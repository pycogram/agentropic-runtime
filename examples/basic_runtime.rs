use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
use agentropic_runtime::prelude::*;
use async_trait::async_trait;

/// A simple agent that prints a message each time it executes
struct PrintAgent {
    id: AgentId,
    name: String,
    count: u32,
}

impl PrintAgent {
    fn new(name: &str) -> Self {
        Self {
            id: AgentId::new(),
            name: name.to_string(),
            count: 0,
        }
    }
}

#[async_trait]
impl Agent for PrintAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn initialize(&mut self, ctx: &AgentContext) -> AgentResult<()> {
        ctx.log_info(&format!("{} ready!", self.name));
        Ok(())
    }

    async fn execute(&mut self, ctx: &AgentContext) -> AgentResult<()> {
        self.count += 1;
        ctx.log_info(&format!("{} tick #{}", self.name, self.count));
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        Ok(())
    }

    async fn shutdown(&mut self, ctx: &AgentContext) -> AgentResult<()> {
        ctx.log_info(&format!("{} done after {} ticks", self.name, self.count));
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    println!("=== Basic Runtime Example ===\n");

    let config = RuntimeConfig::new()
        .with_max_workers(4)
        .with_metrics(true)
        .with_timeout(30000);

    let runtime = Runtime::with_config(config);

    println!("Runtime created:");
    println!("  Max workers: {}", runtime.config().max_workers);
    println!("  Metrics enabled: {}", runtime.config().enable_metrics);
    println!("  Timeout: {}ms\n", runtime.config().default_timeout_ms);

    // Spawn real agents that actually run
    let id1 = runtime.spawn(Box::new(PrintAgent::new("Trader")), "trader_agent").await?;
    let id2 = runtime.spawn(Box::new(PrintAgent::new("Monitor")), "monitor_agent").await?;
    let id3 = runtime.spawn(Box::new(PrintAgent::new("Logger")), "logger_agent").await?;

    println!("Spawned {} agents\n", runtime.agent_count().await);

    // Let them run for 2 seconds
    println!("--- Agents running for 2 seconds ---\n");
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Check via handle
    let handle = runtime.handle();
    println!("\nRuntime handle:");
    println!("  Agent count: {}", handle.agent_count().await);
    for (id, name) in handle.agent_names().await {
        println!("  - {} ({})", name, id);
    }

    // Shutdown all
    println!("\n--- Shutting down ---\n");
    runtime.shutdown().await?;
    println!("\nDone!");

    Ok(())
}
