use agentropic_runtime::prelude::*;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    println!("=== Basic Runtime Example ===\n");

    // Create runtime with configuration
    let config = RuntimeConfig::new()
        .with_max_workers(4)
        .with_metrics(true)
        .with_timeout(30000);

    let runtime = Runtime::with_config(config);

    println!("Runtime created:");
    println!("  Max workers: {}", runtime.config().max_workers);
    println!("  Metrics enabled: {}", runtime.config().enable_metrics);
    println!("  Timeout: {}ms", runtime.config().default_timeout_ms);

    // Start runtime
    runtime.start().await?;
    println!("\n✓ Runtime started");

    // Spawn some agents
    let agent1 = AgentId::new();
    let agent2 = AgentId::new();
    let agent3 = AgentId::new();

    runtime.spawn(agent1, "trader_agent").await?;
    runtime.spawn(agent2, "monitor_agent").await?;
    runtime.spawn(agent3, "logger_agent").await?;

    println!("\n✓ Spawned {} agents", runtime.agent_count().await);

    // Get runtime handle
    let handle = runtime.handle();
    println!("\n✓ Runtime handle created");
    println!("  Agents via handle: {}", handle.agent_count().await);
    println!("  Runtime running: {}", handle.is_running().await);

    // Check agents
    println!("\nAgent checks:");
    println!("  agent1 exists: {}", runtime.has_agent(&agent1).await);
    println!("  agent2 exists: {}", runtime.has_agent(&agent2).await);
    println!("  agent3 exists: {}", runtime.has_agent(&agent3).await);

    // Stop runtime
    runtime.stop().await?;
    println!("\n✓ Runtime stopped");

    // Shutdown
    runtime.shutdown().await?;
    println!("✓ Runtime shutdown complete");

    Ok(())
}
