use serde::{Deserialize, Serialize};

/// Resource limits for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Max CPU usage (0.0 - 100.0)
    pub max_cpu: f64,
    /// Max memory in bytes
    pub max_memory: u64,
    /// Max file descriptors
    pub max_file_descriptors: u32,
    /// Max threads
    pub max_threads: u32,
}

impl ResourceLimits {
    /// Create new resource limits
    pub fn new() -> Self {
        Self {
            max_cpu: 100.0,
            max_memory: 512 * 1024 * 1024, // 512MB
            max_file_descriptors: 1024,
            max_threads: 10,
        }
    }

    /// Set max CPU
    pub fn with_max_cpu(mut self, cpu: f64) -> Self {
        self.max_cpu = cpu.clamp(0.0, 100.0);
        self
    }

    /// Set max memory
    pub fn with_max_memory(mut self, memory: u64) -> Self {
        self.max_memory = memory;
        self
    }

    /// Set max file descriptors
    pub fn with_max_file_descriptors(mut self, fds: u32) -> Self {
        self.max_file_descriptors = fds;
        self
    }

    /// Set max threads
    pub fn with_max_threads(mut self, threads: u32) -> Self {
        self.max_threads = threads;
        self
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::new()
    }
}
