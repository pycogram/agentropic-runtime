use agentropic_core::{Agent, AgentContext, AgentId, AgentResult};
use agentropic_runtime::prelude::*;
use async_trait::async_trait;

struct SimpleAgent {
    id: AgentId,
}

impl SimpleAgent {
    fn new() -> Self {
        Self { id: AgentId::new() }
    }
}

#[async_trait]
impl Agent for SimpleAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    async fn initialize(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        Ok(())
    }

    async fn execute(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &AgentContext) -> AgentResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn full_runtime_lifecycle() {
    let config = RuntimeConfig::new().with_max_workers(2);
    let runtime = Runtime::with_config(config);

    runtime.start().await.unwrap();
    assert!(runtime.is_running().await);

    let id1 = runtime.spawn(Box::new(SimpleAgent::new()), "agent1").await.unwrap();
    let id2 = runtime.spawn(Box::new(SimpleAgent::new()), "agent2").await.unwrap();

    assert_eq!(runtime.agent_count().await, 2);

    let handle = runtime.handle();
    assert_eq!(handle.agent_count().await, 2);

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn supervised_runtime() {
    let runtime = Runtime::new();
    let mut supervisor = Supervisor::new("main");

    let agent = SimpleAgent::new();
    let agent_id = *agent.id();
    let policy = RestartPolicy::new(RestartStrategy::OnFailure).with_max_retries(3);

    supervisor.supervise(agent_id, policy);
    runtime.spawn(Box::new(agent), "supervised_agent").await.unwrap();

    assert!(supervisor.get_policy(&agent_id).is_some());
    assert!(runtime.has_agent(&agent_id).await);

    runtime.shutdown().await.unwrap();
}
