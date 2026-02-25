use crate::{RuntimeConfig, RuntimeError, RuntimeHandle};
use agentropic_core::{Agent, AgentContext, AgentId, OutgoingMessage};
use agentropic_messaging::{Message, Performative, Router};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, mpsc, RwLock};
use tokio::task::JoinHandle;
use tracing::{info, error, warn};

/// Entry for a running agent
pub(crate) struct AgentEntry {
    pub id: AgentId,
    pub name: String,
    pub shutdown_tx: watch::Sender<bool>,
    pub task_handle: JoinHandle<()>,
}

/// Agent runtime engine — runs agents and routes messages between them
pub struct Runtime {
    config: RuntimeConfig,
    agents: Arc<RwLock<HashMap<AgentId, AgentEntry>>>,
    running: Arc<RwLock<bool>>,
    router: Router,
    /// Maps agent name → AgentId for name-based messaging
    name_registry: Arc<RwLock<HashMap<String, AgentId>>>,
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
            router: Router::new(),
            name_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get runtime configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Get reference to the router
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Get the name registry for resolving agent names to IDs
    pub fn name_registry(&self) -> &Arc<RwLock<HashMap<String, AgentId>>> {
        &self.name_registry
    }

    /// Spawn an agent — starts it running in a background task with messaging
    pub async fn spawn(
        &self,
        agent: Box<dyn Agent>,
        name: impl Into<String>,
    ) -> Result<AgentId, RuntimeError> {
        let agent_id = *agent.id();
        let agent_name = name.into();
        let ctx = AgentContext::new(agent_id);

        // Register agent with the router for message receiving
        let mailbox_rx = self.router.register(agent_id)
            .map_err(|e| RuntimeError::SpawnFailed(format!("Router registration failed: {}", e)))?;

        // Register name → id mapping
        {
            let mut names = self.name_registry.write().await;
            names.insert(agent_name.clone(), agent_id);
        }

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        info!("[{}] Spawning agent '{}'", agent_id, agent_name);

        // Clone what the task needs
        let router = self.router.clone();
        let name_reg = self.name_registry.clone();
        let task_name = agent_name.clone();

        // Spawn the agent execution loop
        let task_handle = tokio::spawn(async move {
            run_agent_loop(agent, ctx, shutdown_rx, mailbox_rx, router, name_reg, &task_name).await;
        });

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

        let _ = entry.shutdown_tx.send(true);

        // Unregister from router
        let _ = self.router.unregister(agent_id);

        // Remove from name registry
        {
            let mut names = self.name_registry.write().await;
            names.retain(|_, id| id != agent_id);
        }

        match tokio::time::timeout(
            std::time::Duration::from_secs(self.config.default_timeout_ms / 1000),
            entry.task_handle,
        )
        .await
        {
            Ok(Ok(())) => info!("[{}] Agent '{}' stopped cleanly", agent_id, entry.name),
            Ok(Err(e)) => error!("[{}] Agent '{}' task panicked: {}", agent_id, entry.name, e),
            Err(_) => warn!("[{}] Agent '{}' shutdown timed out", agent_id, entry.name),
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
    pub async fn stop(&self) -> Result<(), RuntimeError> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Runtime stopped");
        Ok(())
    }

    /// Check if runtime is running
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
            let _ = self.router.unregister(&id);

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

/// Convert a string performative name to the Performative enum
fn parse_performative(s: &str) -> Performative {
    match s.to_lowercase().as_str() {
        "inform" => Performative::Inform,
        "request" => Performative::Request,
        "query" => Performative::Query,
        "propose" => Performative::Propose,
        "accept" => Performative::Accept,
        "reject" => Performative::Reject,
        "confirm" => Performative::Confirm,
        "subscribe" => Performative::Subscribe,
        "cfp" => Performative::CFP,
        "refuse" => Performative::Refuse,
        _ => Performative::Inform,
    }
}

/// The actual agent execution loop with message handling
async fn run_agent_loop(
    mut agent: Box<dyn Agent>,
    ctx: AgentContext,
    mut shutdown_rx: watch::Receiver<bool>,
    mut mailbox_rx: mpsc::UnboundedReceiver<Message>,
    router: Router,
    name_registry: Arc<RwLock<HashMap<String, AgentId>>>,
    name: &str,
) {
    // Phase 1: Initialize
    info!("[{}] Agent '{}' initializing...", ctx.agent_id(), name);
    match agent.initialize(&ctx).await {
        Ok(()) => info!("[{}] Agent '{}' initialized", ctx.agent_id(), name),
        Err(e) => {
            error!("[{}] Agent '{}' initialization failed: {}", ctx.agent_id(), name, e);
            return;
        }
    }

    // Phase 2: Execute loop with message handling
    info!("[{}] Agent '{}' running", ctx.agent_id(), name);
    loop {
        tokio::select! {
            // Check for shutdown signal
            result = shutdown_rx.changed() => {
                match result {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!("[{}] Agent '{}' received shutdown signal", ctx.agent_id(), name);
                        break;
                    }
                    Err(_) => break,
                    _ => {}
                }
            }

            // Check for incoming messages
            Some(msg) = mailbox_rx.recv() => {
                let sender_str = msg.sender().to_string();
                let perf_str = msg.performative().to_string();
                let content = msg.content().to_string();

                match agent.handle_message(&ctx, &sender_str, &perf_str, &content).await {
                    Ok(()) => {}
                    Err(e) => {
                        error!("[{}] Agent '{}' message handling error: {}", ctx.agent_id(), name, e);
                    }
                }

                // Drain outbox after handling message
                deliver_outbox(&ctx, &router, &name_registry).await;
            }

            // Run agent execution
            result = agent.execute(&ctx) => {
                match result {
                    Ok(()) => {
                        // Drain outbox after execution
                        deliver_outbox(&ctx, &router, &name_registry).await;
                    }
                    Err(e) => {
                        error!("[{}] Agent '{}' execution error: {}", ctx.agent_id(), name, e);
                        break;
                    }
                }
            }
        }
    }

    // Phase 3: Shutdown
    info!("[{}] Agent '{}' shutting down...", ctx.agent_id(), name);
    match agent.shutdown(&ctx).await {
        Ok(()) => info!("[{}] Agent '{}' shutdown complete", ctx.agent_id(), name),
        Err(e) => error!("[{}] Agent '{}' shutdown error: {}", ctx.agent_id(), name, e),
    }
}

/// Deliver outgoing messages from agent's outbox via the router
async fn deliver_outbox(
    ctx: &AgentContext,
    router: &Router,
    name_registry: &Arc<RwLock<HashMap<String, AgentId>>>,
) {
    let messages = ctx.drain_outbox();
    if messages.is_empty() {
        return;
    }

    let names = name_registry.read().await;
    let sender_id = *ctx.agent_id();

    for outgoing in messages {
        let receiver_id = if let Some(id) = names.get(&outgoing.receiver) {
            *id
        } else {
            warn!("[{}] Cannot resolve receiver, dropped", sender_id);
            continue;
        };

        let performative = parse_performative(&outgoing.performative);
        let msg = Message::new(sender_id, receiver_id, performative, outgoing.content);

        match router.send(msg) {
            Ok(()) => {}
            Err(e) => {
                warn!("[{}] Message delivery failed: {}", sender_id, e);
            }
        }
    }
}
    

#[cfg(test)]
mod tests {
    use super::*;
    use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};

    /// A test agent that counts executions
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
        fn id(&self) -> &AgentId { &self.id }

        async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }

        async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            Ok(())
        }

        async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }
    }

    /// An agent that receives messages and logs them
    struct EchoAgent {
        id: AgentId,
        received: Arc<Mutex<Vec<String>>>,
    }

    impl EchoAgent {
        fn new(received: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                id: AgentId::new(),
                received,
            }
        }
    }

    #[async_trait]
    impl Agent for EchoAgent {
        fn id(&self) -> &AgentId { &self.id }

        async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }

        async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            Ok(())
        }

        async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }

        async fn handle_message(
            &mut self,
            _ctx: &AgentContext,
            sender: &str,
            performative: &str,
            content: &str,
        ) -> AgentResult<()> {
            let msg = format!("[{}:{}] {}", sender, performative, content);
            if let Ok(mut received) = self.received.lock() {
                received.push(msg);
            }
            Ok(())
        }
    }

    /// An agent that sends a message on first execute, then sleeps
    struct SenderAgent {
        id: AgentId,
        target_name: String,
        sent: bool,
    }

    impl SenderAgent {
        fn new(target_name: &str) -> Self {
            Self {
                id: AgentId::new(),
                target_name: target_name.to_string(),
                sent: false,
            }
        }
    }

    #[async_trait]
    impl Agent for SenderAgent {
        fn id(&self) -> &AgentId { &self.id }

        async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }

        async fn execute(&mut self, ctx: &AgentContext) -> AgentResult<()> {
            if !self.sent {
                ctx.send_message(&self.target_name, "inform", "Hello from sender!");
                self.sent = true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            Ok(())
        }

        async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }
    }

    #[tokio::test]
    async fn test_runtime_spawns_and_runs_agent() {
        let runtime = Runtime::new();
        let counter = Arc::new(AtomicU32::new(0));

        let agent = CountingAgent::new(counter.clone());
        let agent_id = runtime.spawn(Box::new(agent), "counter").await.unwrap();

        assert!(runtime.has_agent(&agent_id).await);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let count = counter.load(Ordering::SeqCst);
        assert!(count >= 2, "Agent executed {} times, expected >= 2", count);

        runtime.stop_agent(&agent_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_runtime_runs_multiple_agents() {
        let runtime = Runtime::new();
        let c1 = Arc::new(AtomicU32::new(0));
        let c2 = Arc::new(AtomicU32::new(0));

        runtime.spawn(Box::new(CountingAgent::new(c1.clone())), "a1").await.unwrap();
        runtime.spawn(Box::new(CountingAgent::new(c2.clone())), "a2").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        assert!(c1.load(Ordering::SeqCst) >= 2);
        assert!(c2.load(Ordering::SeqCst) >= 2);

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_agent_messaging() {
        let runtime = Runtime::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        // Spawn receiver first
        let receiver = EchoAgent::new(received.clone());
        runtime.spawn(Box::new(receiver), "receiver").await.unwrap();

        // Spawn sender that targets "receiver" by name
        let sender = SenderAgent::new("receiver");
        runtime.spawn(Box::new(sender), "sender").await.unwrap();

        // Wait for message delivery
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Check that receiver got the message
        let msgs = received.lock().unwrap();
        assert!(!msgs.is_empty(), "Receiver should have gotten a message");
        assert!(msgs[0].contains("Hello from sender!"), "Got: {}", msgs[0]);

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_runtime_shutdown_stops_all() {
        let runtime = Runtime::new();
        let counter = Arc::new(AtomicU32::new(0));

        runtime.spawn(Box::new(CountingAgent::new(counter.clone())), "test").await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let count_before = counter.load(Ordering::SeqCst);
        assert!(count_before > 0);

        runtime.shutdown().await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let count_after = counter.load(Ordering::SeqCst);
        assert_eq!(count_before, count_after);
    }
}
