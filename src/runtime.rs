use crate::{RuntimeConfig, RuntimeError, RuntimeHandle};
use crate::supervisor::{RestartPolicy, RestartStrategy};
use agentropic_core::{Agent, AgentContext, AgentId};
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
    name_registry: Arc<RwLock<HashMap<String, AgentId>>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self::with_config(RuntimeConfig::default())
    }

    pub fn with_config(config: RuntimeConfig) -> Self {
        Self {
            config,
            agents: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            router: Router::new(),
            name_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    pub fn router(&self) -> &Router {
        &self.router
    }

    pub fn name_registry(&self) -> &Arc<RwLock<HashMap<String, AgentId>>> {
        &self.name_registry
    }

    /// Spawn an agent with default restart policy (Never)
    pub async fn spawn(
        &self,
        agent: Box<dyn Agent>,
        name: impl Into<String>,
    ) -> Result<AgentId, RuntimeError> {
        self.spawn_with_policy(agent, name, RestartPolicy::new(RestartStrategy::Never)).await
    }

    /// Spawn an agent with a specific restart policy
    pub async fn spawn_with_policy(
        &self,
        agent: Box<dyn Agent>,
        name: impl Into<String>,
        policy: RestartPolicy,
    ) -> Result<AgentId, RuntimeError> {
        let agent_id = *agent.id();
        let agent_name = name.into();
        let ctx = AgentContext::new(agent_id);

        let mailbox_rx = self.router.register(agent_id)
            .map_err(|e| RuntimeError::SpawnFailed(format!("Router registration failed: {}", e)))?;

        {
            let mut names = self.name_registry.write().await;
            names.insert(agent_name.clone(), agent_id);
            // Also register UUID string so agents can reply by sender ID
            names.insert(agent_id.to_string(), agent_id);
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        info!("[{}] Spawning agent '{}' (restart: {:?})", agent_id, agent_name, policy.strategy());

        let router = self.router.clone();
        let name_reg = self.name_registry.clone();
        let task_name = agent_name.clone();

        let task_handle = tokio::spawn(async move {
            run_agent_loop(agent, ctx, shutdown_rx, mailbox_rx, router, name_reg, &task_name, policy).await;
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

    pub async fn stop_agent(&self, agent_id: &AgentId) -> Result<(), RuntimeError> {
        let mut agents = self.agents.write().await;

        let entry = agents
            .remove(agent_id)
            .ok_or_else(|| RuntimeError::AgentNotFound(agent_id.to_string()))?;

        info!("[{}] Stopping agent '{}'", agent_id, entry.name);

        let _ = entry.shutdown_tx.send(true);
        let _ = self.router.unregister(agent_id);

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

    pub async fn agent_count(&self) -> usize {
        self.agents.read().await.len()
    }

    pub async fn has_agent(&self, agent_id: &AgentId) -> bool {
        self.agents.read().await.contains_key(agent_id)
    }

    pub async fn start(&self) -> Result<(), RuntimeError> {
        let mut running = self.running.write().await;
        *running = true;
        info!("Runtime started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), RuntimeError> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Runtime stopped");
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub fn handle(&self) -> RuntimeHandle {
        RuntimeHandle::new(self.agents.clone(), self.running.clone())
    }

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

/// The agent execution loop with message handling and supervised restarts
async fn run_agent_loop(
    mut agent: Box<dyn Agent>,
    ctx: AgentContext,
    mut shutdown_rx: watch::Receiver<bool>,
    mut mailbox_rx: mpsc::UnboundedReceiver<Message>,
    router: Router,
    name_registry: Arc<RwLock<HashMap<String, AgentId>>>,
    name: &str,
    policy: RestartPolicy,
) {
    let max_retries = policy.max_retries().unwrap_or(u32::MAX);
    let mut attempt: u32 = 0;

    loop {
        // Check if shutdown was requested before starting
        if *shutdown_rx.borrow() {
            break;
        }

        if attempt > 0 {
            // Calculate backoff delay
            let delay_secs = match policy.strategy() {
                RestartStrategy::ExponentialBackoff => {
                    let base = policy.backoff_seconds();
                    base * 2u64.saturating_pow(attempt - 1)
                }
                _ => policy.backoff_seconds(),
            };

            info!(
                "[{}] Agent '{}' restarting (attempt {}/{}) after {}s...",
                ctx.agent_id(), name, attempt, max_retries, delay_secs
            );

            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

            // Check shutdown again after sleeping
            if *shutdown_rx.borrow() {
                break;
            }
        }

        // Phase 1: Initialize
        info!("[{}] Agent '{}' initializing...", ctx.agent_id(), name);
        match agent.initialize(&ctx).await {
            Ok(()) => info!("[{}] Agent '{}' initialized", ctx.agent_id(), name),
            Err(e) => {
                error!("[{}] Agent '{}' initialization failed: {}", ctx.agent_id(), name, e);
                if should_restart(&policy, attempt, max_retries, true) {
                    attempt += 1;
                    continue;
                }
                return;
            }
        }

        // Phase 2: Execute loop with message handling
        info!("[{}] Agent '{}' running", ctx.agent_id(), name);
        let mut failed = false;

        loop {
            tokio::select! {
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

                    deliver_outbox(&ctx, &router, &name_registry).await;
                }

                result = agent.execute(&ctx) => {
                    match result {
                        Ok(()) => {
                            deliver_outbox(&ctx, &router, &name_registry).await;
                        }
                        Err(e) => {
                            error!("[{}] Agent '{}' execution error: {}", ctx.agent_id(), name, e);
                            failed = true;
                            break;
                        }
                    }
                }
            }
        }

        // If shutdown was requested, do clean shutdown and exit
        if !failed {
            info!("[{}] Agent '{}' shutting down...", ctx.agent_id(), name);
            match agent.shutdown(&ctx).await {
                Ok(()) => info!("[{}] Agent '{}' shutdown complete", ctx.agent_id(), name),
                Err(e) => error!("[{}] Agent '{}' shutdown error: {}", ctx.agent_id(), name, e),
            }
            return;
        }

        // Agent failed — check restart policy
        if should_restart(&policy, attempt, max_retries, true) {
            attempt += 1;
            warn!(
                "[{}] Agent '{}' failed, will restart (attempt {})",
                ctx.agent_id(), name, attempt
            );
        } else {
            error!(
                "[{}] Agent '{}' failed, restart policy exhausted after {} attempts",
                ctx.agent_id(), name, attempt
            );
            // Still try to shutdown cleanly
            let _ = agent.shutdown(&ctx).await;
            return;
        }
    }

    // Shutdown on exit
    info!("[{}] Agent '{}' shutting down...", ctx.agent_id(), name);
    let _ = agent.shutdown(&ctx).await;
}

/// Decide whether to restart based on policy
fn should_restart(policy: &RestartPolicy, attempt: u32, max_retries: u32, was_failure: bool) -> bool {
    if attempt >= max_retries {
        return false;
    }

    match policy.strategy() {
        RestartStrategy::Never => false,
        RestartStrategy::Always => true,
        RestartStrategy::OnFailure => was_failure,
        RestartStrategy::ExponentialBackoff => was_failure,
    }
}

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
    use agentropic_core::{Agent, AgentContext, AgentId, AgentError, AgentResult};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};

    /// Agent that counts executions
    struct CountingAgent {
        id: AgentId,
        counter: Arc<AtomicU32>,
    }

    impl CountingAgent {
        fn new(counter: Arc<AtomicU32>) -> Self {
            Self { id: AgentId::new(), counter }
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

    /// Agent that fails after N executions, then succeeds
    struct FlakyAgent {
        id: AgentId,
        counter: Arc<AtomicU32>,
        fail_until: u32,
    }

    impl FlakyAgent {
        fn new(counter: Arc<AtomicU32>, fail_until: u32) -> Self {
            Self { id: AgentId::new(), counter, fail_until }
        }
    }

    #[async_trait]
    impl Agent for FlakyAgent {
        fn id(&self) -> &AgentId { &self.id }
        async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }
        async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
            let count = self.counter.fetch_add(1, Ordering::SeqCst);
            if count < self.fail_until {
                return Err(AgentError::ExecutionFailed(
                    format!("Flaky failure #{}", count + 1),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            Ok(())
        }
        async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }
    }

    /// Agent that receives messages
    struct EchoAgent {
        id: AgentId,
        received: Arc<Mutex<Vec<String>>>,
    }

    impl EchoAgent {
        fn new(received: Arc<Mutex<Vec<String>>>) -> Self {
            Self { id: AgentId::new(), received }
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
        async fn handle_message(&mut self, _ctx: &AgentContext, _sender: &str, _perf: &str, content: &str) -> AgentResult<()> {
            if let Ok(mut r) = self.received.lock() { r.push(content.to_string()); }
            Ok(())
        }
    }

    /// Agent that sends a message once
    struct SenderAgent {
        id: AgentId,
        target: String,
        sent: bool,
    }

    impl SenderAgent {
        fn new(target: &str) -> Self {
            Self { id: AgentId::new(), target: target.to_string(), sent: false }
        }
    }

    #[async_trait]
    impl Agent for SenderAgent {
        fn id(&self) -> &AgentId { &self.id }
        async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> { Ok(()) }
        async fn execute(&mut self, ctx: &AgentContext) -> AgentResult<()> {
            if !self.sent {
                ctx.send_message(&self.target, "inform", "Hello from sender!");
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
        assert!(counter.load(Ordering::SeqCst) >= 2);

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

        runtime.spawn(Box::new(EchoAgent::new(received.clone())), "receiver").await.unwrap();
        runtime.spawn(Box::new(SenderAgent::new("receiver")), "sender").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let msgs = received.lock().unwrap();
        assert!(!msgs.is_empty(), "Receiver should have gotten a message");
        assert!(msgs[0].contains("Hello from sender!"));

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_restart_on_failure() {
        let runtime = Runtime::new();
        let counter = Arc::new(AtomicU32::new(0));

        // Agent fails on first 2 executions, then succeeds
        let agent = FlakyAgent::new(counter.clone(), 2);
        let policy = RestartPolicy::new(RestartStrategy::OnFailure)
            .with_max_retries(5)
            .with_backoff_seconds(0);

        runtime.spawn_with_policy(Box::new(agent), "flaky", policy).await.unwrap();

        // Wait for restarts and successful execution
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Should have: fail #1, restart, fail #2, restart, then succeed multiple times
        let count = counter.load(Ordering::SeqCst);
        assert!(count > 2, "Agent should have restarted and run. Count: {}", count);

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_never_restart() {
        let runtime = Runtime::new();
        let counter = Arc::new(AtomicU32::new(0));

        // Agent fails immediately, no restart
        let agent = FlakyAgent::new(counter.clone(), 100);

        runtime.spawn(Box::new(agent), "no-restart").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Should have only executed once (failed, no restart)
        let count = counter.load(Ordering::SeqCst);
        assert_eq!(count, 1, "Should fail once and stop. Count: {}", count);

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
        assert_eq!(count_before, counter.load(Ordering::SeqCst));
    }
}
