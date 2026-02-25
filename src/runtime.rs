use crate::{RuntimeConfig, RuntimeError, RuntimeHandle};
use agentropic_core::{Agent, AgentContext, AgentId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tokio::task::JoinHandle;
use tracing::{info, error, warn};

/// Entry for a running agent
pub(crate) struct AgentEntry {
    pub id: AgentId,
    pub name: String,
    pub shutdown_tx: watch::Sender<bool>,
    pub task_handle: JoinHandle<()>,
}

/// Agent runtime engine — actually runs agents
pub struct Runtime {
    config: RuntimeConfig,
    agents: Arc<RwLock<HashMap<AgentId, AgentEntry>>>,
    running: Arc<RwLock<bool>>,
}

impl Runtime {
    /// Create a new runtime with default configuration
    pub fn new() -> Self {
        Self::with_config(RuntimeConfig::default())
    }

    /// Create a runtime with custom configuration
    pub fn with_config(config: RuntimeConfig) -> Self {
        Self {
            config,
            agents: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get runtime configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Spawn an agent — starts it running in a background task
    /// The agent goes through: initialize → execute loop → shutdown
    pub async fn spawn(
        &self,
        agent: Box<dyn Agent>,
        name: impl Into<String>,
    ) -> Result<AgentId, RuntimeError> {
        let agent_id = *agent.id();
        let agent_name = name.into();
        let ctx = AgentContext::new(agent_id);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        info!("[{}] Spawning agent '{}'", agent_id, agent_name);

        // Spawn the agent execution loop as a tokio task
        let task_name = agent_name.clone();
        let task_handle = tokio::spawn(async move {
            run_agent_loop(agent, ctx, shutdown_rx, &task_name).await;
        });

        // Store the entry
        let entry = AgentEntry {
            id: agent_id,
            name: agent_name,
            shutdown_tx,
            task_handle,
        };

        let mut agents = self.agents.write().await;
        agents.insert(agent_id, entry);

        Ok(agent_id)
    }

    /// Stop a specific agent gracefully
    pub async fn stop_agent(&self, agent_id: &AgentId) -> Result<(), RuntimeError> {
        let mut agents = self.agents.write().await;

        let entry = agents
            .remove(agent_id)
            .ok_or_else(|| RuntimeError::AgentNotFound(agent_id.to_string()))?;

        info!("[{}] Stopping agent '{}'", agent_id, entry.name);

        // Signal shutdown
        let _ = entry.shutdown_tx.send(true);

        // Wait for the task to finish (with timeout)
        match tokio::time::timeout(
            std::time::Duration::from_secs(self.config.default_timeout_ms / 1000),
            entry.task_handle,
        )
        .await
        {
            Ok(Ok(())) => {
                info!("[{}] Agent '{}' stopped cleanly", agent_id, entry.name);
            }
            Ok(Err(e)) => {
                error!("[{}] Agent '{}' task panicked: {}", agent_id, entry.name, e);
            }
            Err(_) => {
                warn!("[{}] Agent '{}' shutdown timed out", agent_id, entry.name);
            }
        }

        Ok(())
    }

    /// Get agent count
    pub async fn agent_count(&self) -> usize {
        self.agents.read().await.len()
    }

    /// Check if agent exists
    pub async fn has_agent(&self, agent_id: &AgentId) -> bool {
        self.agents.read().await.contains_key(agent_id)
    }

    /// Start the runtime
    pub async fn start(&self) -> Result<(), RuntimeError> {
        let mut running = self.running.write().await;
        *running = true;
        info!("Runtime started");
        Ok(())
    }

    /// Stop the runtime (marks as not running)
    /// Stop the runtime (keeps agents, just marks as not running)
    pub async fn stop(&self) -> Result<(), RuntimeError> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Runtime stopped");
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get runtime handle
    pub fn handle(&self) -> RuntimeHandle {
        RuntimeHandle::new(self.agents.clone(), self.running.clone())
    }

    /// Shutdown the runtime — stops all agents gracefully
    pub async fn shutdown(self) -> Result<(), RuntimeError> {
        info!("Runtime shutting down...");

        let mut agents = self.agents.write().await;
        let timeout_ms = self.config.default_timeout_ms;

        for (id, entry) in agents.drain() {
            info!("[{}] Shutting down agent '{}'", id, entry.name);
            let _ = entry.shutdown_tx.send(true);

            match tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                entry.task_handle,
            )
            .await
            {
                Ok(Ok(())) => info!("[{}] Agent '{}' shutdown complete", id, entry.name),
                Ok(Err(e)) => error!("[{}] Agent '{}' panicked: {}", id, entry.name, e),
                Err(_) => warn!("[{}] Agent '{}' shutdown timed out", id, entry.name),
            }
        }

        let mut running = self.running.write().await;
        *running = false;

        info!("Runtime shutdown complete");
        Ok(())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// The actual agent execution loop — runs inside a tokio task
async fn run_agent_loop(
    mut agent: Box<dyn Agent>,
    ctx: AgentContext,
    mut shutdown_rx: watch::Receiver<bool>,
    name: &str,
) {
    // Phase 1: Initialize
    info!("[{}] Agent '{}' initializing...", ctx.agent_id(), name);
    match agent.initialize(&ctx).await {
        Ok(()) => {
            info!("[{}] Agent '{}' initialized", ctx.agent_id(), name);
        }
        Err(e) => {
            error!(
                "[{}] Agent '{}' initialization failed: {}",
                ctx.agent_id(),
                name,
                e
            );
            return;
        }
    }

    // Phase 2: Execute loop
    info!("[{}] Agent '{}' running", ctx.agent_id(), name);
    loop {
        tokio::select! {
            // Check for shutdown signal
            result = shutdown_rx.changed() => {
                match result {
                    Ok(()) => {
                        if *shutdown_rx.borrow() {
                            info!("[{}] Agent '{}' received shutdown signal", ctx.agent_id(), name);
                            break;
                        }
                    }
                    Err(_) => {
                        // Sender dropped, shutdown
                        break;
                    }
                }
            }
            // Run agent execution
            result = agent.execute(&ctx) => {
                match result {
                    Ok(()) => {
                        // Agent executed successfully, continue loop
                    }
                    Err(e) => {
                        error!("[{}] Agent '{}' execution error: {}", ctx.agent_id(), name, e);
                        // Break on error — supervisor will handle restart later
                        break;
                    }
                }
            }
        }
    }

    // Phase 3: Shutdown
    info!("[{}] Agent '{}' shutting down...", ctx.agent_id(), name);
    match agent.shutdown(&ctx).await {
        Ok(()) => {
            info!("[{}] Agent '{}' shutdown complete", ctx.agent_id(), name);
        }
        Err(e) => {
            error!(
                "[{}] Agent '{}' shutdown error: {}",
                ctx.agent_id(),
                name,
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// A test agent that counts how many times execute() is called
    struct CountingAgent {
        id: AgentId,
        counter: Arc<AtomicU32>,
    }

    impl CountingAgent {
        fn new(counter: Arc<AtomicU32>) -> Self {
            Self {
                id: AgentId::new(),
                counter,
            }
        }
    }

    #[async_trait]
    impl Agent for CountingAgent {
        fn id(&self) -> &AgentId {
            &self.id
        }

        async fn initialize(&mut self, ctx: &AgentContext) -> AgentResult<()> {
            ctx.log_info("CountingAgent initialized");
            Ok(())
        }

        async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            // Sleep to avoid busy-loop
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            Ok(())
        }

        async fn shutdown(&mut self, ctx: &AgentContext) -> AgentResult<()> {
            ctx.log_info("CountingAgent shutdown");
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_runtime_spawns_and_runs_agent() {
        let runtime = Runtime::new();
        let counter = Arc::new(AtomicU32::new(0));

        let agent = CountingAgent::new(counter.clone());
        let agent_id = runtime
            .spawn(Box::new(agent), "counter-agent")
            .await
            .unwrap();

        assert!(runtime.has_agent(&agent_id).await);
        assert_eq!(runtime.agent_count().await, 1);

        // Let it run for 200ms
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Should have executed multiple times
        let count = counter.load(Ordering::SeqCst);
        assert!(count >= 2, "Agent executed {} times, expected >= 2", count);

        // Stop it
        runtime.stop_agent(&agent_id).await.unwrap();
        assert_eq!(runtime.agent_count().await, 0);
    }

    #[tokio::test]
    async fn test_runtime_runs_multiple_agents() {
        let runtime = Runtime::new();

        let counter1 = Arc::new(AtomicU32::new(0));
        let counter2 = Arc::new(AtomicU32::new(0));

        let agent1 = CountingAgent::new(counter1.clone());
        let agent2 = CountingAgent::new(counter2.clone());

        let id1 = runtime.spawn(Box::new(agent1), "agent-1").await.unwrap();
        let id2 = runtime.spawn(Box::new(agent2), "agent-2").await.unwrap();

        assert_eq!(runtime.agent_count().await, 2);

        // Let them run
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Both should have executed
        assert!(counter1.load(Ordering::SeqCst) >= 2, "Agent 1 didn't run enough");
        assert!(counter2.load(Ordering::SeqCst) >= 2, "Agent 2 didn't run enough");

        // Shutdown all
        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_runtime_shutdown_stops_all() {
        let runtime = Runtime::new();
        let counter = Arc::new(AtomicU32::new(0));

        let agent = CountingAgent::new(counter.clone());
        runtime.spawn(Box::new(agent), "shutdown-test").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let count_before = counter.load(Ordering::SeqCst);
        assert!(count_before > 0, "Agent should have run at least once");

        runtime.shutdown().await.unwrap();

        // After shutdown, count should stop increasing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let count_after = counter.load(Ordering::SeqCst);

        assert_eq!(count_before, count_after, "Agent should have stopped after shutdown");
    }
}
