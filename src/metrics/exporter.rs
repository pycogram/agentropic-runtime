use super::MetricsRegistry;

/// Metrics exporter
pub struct MetricsExporter {
    registry: MetricsRegistry,
}

impl MetricsExporter {
    /// Create a new exporter
    pub fn new(registry: MetricsRegistry) -> Self {
        Self { registry }
    }

    /// Export metrics as JSON
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        let mut all_metrics = Vec::new();

        for (name, collector) in self.registry.collectors() {
            for metric in collector.metrics() {
                all_metrics.push(serde_json::json!({
                    "collector": name,
                    "name": metric.name(),
                    "type": format!("{:?}", metric.metric_type()),
                    "value": metric.value(),
                    "labels": metric.labels(),
                }));
            }
        }

        serde_json::to_string_pretty(&all_metrics)
    }

    /// Get registry
    pub fn registry(&self) -> &MetricsRegistry {
        &self.registry
    }
}
