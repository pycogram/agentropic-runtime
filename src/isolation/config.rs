use serde::{Deserialize, Serialize};

/// Isolation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationConfig {
    /// Enable isolation
    pub enabled: bool,
    /// CPU quota (percentage)
    pub cpu_quota: f64,
    /// Memory limit (bytes)
    pub memory_limit: u64,
    /// Enable network isolation
    pub network_isolation: bool,
}

impl IsolationConfig {
    /// Create a new isolation configuration
    pub fn new() -> Self {
        Self {
            enabled: true,
            cpu_quota: 100.0,
            memory_limit: 1024 * 1024 * 1024, // 1GB
            network_isolation: false,
        }
    }

    /// Set CPU quota
    pub fn with_cpu_quota(mut self, quota: f64) -> Self {
        self.cpu_quota = quota.max(0.0);
        self
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, limit: u64) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Enable network isolation
    pub fn with_network_isolation(mut self, enable: bool) -> Self {
        self.network_isolation = enable;
        self
    }
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self::new()
    }
}
