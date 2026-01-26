use futures::Future;
use tokio::task::JoinHandle;

/// Task executor
pub struct Executor {
    name: String,
}

impl Executor {
    /// Create a new executor
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Spawn a task
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        tokio::spawn(future)
    }

    /// Get executor name
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new("default")
    }
}
