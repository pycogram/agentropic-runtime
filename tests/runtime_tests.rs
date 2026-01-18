use agentropic_runtime::prelude::*;

#[tokio::test]
async fn create_runtime() {
    let runtime = Runtime::new();
    assert_eq!(runtime.agent_count().await, 0);
    assert!(!runtime.is_running().await);
}

#[tokio::test]
async fn start_stop_runtime() {
    let runtime = Runtime::new();

    runtime.start().await.unwrap();
    assert!(runtime.is_running().await);

    runtime.stop().await.unwrap();
    assert!(!runtime.is_running().await);
}

#[tokio::test]
async fn spawn_agent() {
    let runtime = Runtime::new();
    let agent_id = AgentId::new();

    runtime.spawn(agent_id, "test_agent").await.unwrap();
    assert_eq!(runtime.agent_count().await, 1);
    assert!(runtime.has_agent(&agent_id).await);
}

#[tokio::test]
async fn runtime_handle() {
    let runtime = Runtime::new();
    let handle = runtime.handle();

    let agent_id = AgentId::new();
    runtime.spawn(agent_id, "test_agent").await.unwrap();

    assert_eq!(handle.agent_count().await, 1);
    assert!(handle.has_agent(&agent_id).await);
}

#[tokio::test]
async fn runtime_with_config() {
    let config = RuntimeConfig::new().with_max_workers(4).with_timeout(5000);

    let runtime = Runtime::with_config(config);
    assert_eq!(runtime.config().max_workers, 4);
    assert_eq!(runtime.config().default_timeout_ms, 5000);
}
