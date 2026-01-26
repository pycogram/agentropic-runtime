use std::time::Instant;

/// Resource usage statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Number of threads
    pub thread_count: u32,
}

/// Resource monitor
pub struct ResourceMonitor {
    start_time: Instant,
    current_usage: ResourceUsage,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            current_usage: ResourceUsage::default(),
        }
    }

    /// Get current resource usage
    pub fn current_usage(&self) -> &ResourceUsage {
        &self.current_usage
    }

    /// Update resource usage
    pub fn update(&mut self, usage: ResourceUsage) {
        self.current_usage = usage;
    }

    /// Get uptime
    pub fn uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
