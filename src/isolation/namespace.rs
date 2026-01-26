use agentropic_core::AgentId;

/// Namespace for agent isolation
pub struct Namespace {
    agent_id: AgentId,
    name: String,
}

impl Namespace {
    /// Create a new namespace
    pub fn new(agent_id: AgentId, name: impl Into<String>) -> Self {
        Self {
            agent_id,
            name: name.into(),
        }
    }

    /// Get agent ID
    pub fn agent_id(&self) -> &AgentId {
        &self.agent_id
    }

    /// Get namespace name
    pub fn name(&self) -> &str {
        &self.name
    }
}
