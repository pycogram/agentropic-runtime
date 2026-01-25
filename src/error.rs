use thiserror::Error;

/// Runtime errors
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Failed to spawn agent: {0}")]
    SpawnFailed(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Scheduling error: {0}")]
    SchedulingError(String),

    #[error("Supervision error: {0}")]
    SupervisionError(String),

    #[error("Isolation error: {0}")]
    IsolationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Runtime error: {0}")]
    Other(String),
}
