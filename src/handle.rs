use crate::runtime::AgentEntry;
use agentropic_core::AgentId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Handle to interact with the runtime
#[derive(Clone)]
pub struct RuntimeHandle {
    agents: Arc<RwLock<HashMap<AgentId, AgentEntry>>>,
    running: Arc<RwLock<bool>>,
}

impl RuntimeHandle {
    /// Create a new runtime handle
    pub(crate) fn new(
        agents: Arc<RwLock<HashMap<AgentId, AgentEntry>>>,
        running: Arc<RwLock<bool>>,
    ) -> Self {
        Self { agents, running }
    }

    /// Get agent count
    pub async fn agent_count(&self) -> usize {
        self.agents.read().await.len()
    }

    /// Check if runtime is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Check if agent exists
    pub async fn has_agent(&self, agent_id: &AgentId) -> bool {
        self.agents.read().await.contains_key(agent_id)
    }

    /// Get names of all running agents
    pub async fn agent_names(&self) -> Vec<(AgentId, String)> {
        self.agents
            .read()
            .await
            .values()
            .map(|entry| (entry.id, entry.name.clone()))
            .collect()
    }
}
