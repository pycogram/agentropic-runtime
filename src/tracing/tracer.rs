use std::time::Instant;

/// Span for tracing
#[derive(Debug)]
pub struct Span {
    name: String,
    start: Instant,
}

impl Span {
    /// Create a new span
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
        }
    }

    /// Get span name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

/// Tracer for distributed tracing
pub struct Tracer {
    enabled: bool,
}

impl Tracer {
    /// Create a new tracer
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Create a span
    pub fn span(&self, name: impl Into<String>) -> Option<Span> {
        if self.enabled {
            Some(Span::new(name))
        } else {
            None
        }
    }

    /// Check if tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new(true)
    }
}
