use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
use agentropic_runtime::prelude::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Minimal test agent
struct TestAgent {
    id: AgentId,
    counter: Arc<AtomicU32>,
}

impl TestAgent {
    fn new(counter: Arc<AtomicU32>) -> Self {
        Self {
            id: AgentId::new(),
            counter,
        }
    }
}

#[async_trait]
impl Agent for TestAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        Ok(())
    }

    async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn create_runtime() {
    let runtime = Runtime::new();
    assert_eq!(runtime.agent_count().await, 0);
}

#[tokio::test]
async fn spawn_agent() {
    let runtime = Runtime::new();
    let counter = Arc::new(AtomicU32::new(0));

    let agent = TestAgent::new(counter.clone());
    let agent_id = runtime.spawn(Box::new(agent), "test_agent").await.unwrap();

    assert_eq!(runtime.agent_count().await, 1);
    assert!(runtime.has_agent(&agent_id).await);

    // Let it run
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(counter.load(Ordering::SeqCst) >= 2);

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn runtime_handle() {
    let runtime = Runtime::new();
    let handle = runtime.handle();
    let counter = Arc::new(AtomicU32::new(0));

    let agent = TestAgent::new(counter);
    runtime.spawn(Box::new(agent), "test_agent").await.unwrap();

    assert_eq!(handle.agent_count().await, 1);

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn runtime_with_config() {
    let config = RuntimeConfig::new().with_max_workers(4).with_timeout(5000);
    let runtime = Runtime::with_config(config);

    assert_eq!(runtime.config().max_workers, 4);
    assert_eq!(runtime.config().default_timeout_ms, 5000);
}

#[tokio::test]
async fn stop_agent_individually() {
    let runtime = Runtime::new();
    let counter = Arc::new(AtomicU32::new(0));

    let agent = TestAgent::new(counter.clone());
    let agent_id = runtime.spawn(Box::new(agent), "stop_test").await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let count_before = counter.load(Ordering::SeqCst);
    assert!(count_before > 0);

    runtime.stop_agent(&agent_id).await.unwrap();
    assert_eq!(runtime.agent_count().await, 0);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let count_after = counter.load(Ordering::SeqCst);
    assert_eq!(count_before, count_after, "Agent should have stopped");
}
