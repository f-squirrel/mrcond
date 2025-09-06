//! Metrics module for Prometheus integration

use prometheus::{Counter, CounterVec, Encoder, Gauge, GaugeVec, Opts, Registry, TextEncoder};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Inner metrics implementation containing the actual Prometheus metrics
#[derive(Clone)]
struct MetricsInner {
    registry: Arc<Registry>,
    running_servers: Arc<Gauge>,
    collection_servers: Arc<GaugeVec>,
    task_restarts: Arc<CounterVec>,
    task_failures: Arc<CounterVec>,
    task_total_started: Arc<Counter>,
    server_count: Arc<Mutex<usize>>,
    collection_counts: Arc<Mutex<HashMap<String, usize>>>,
}

/// Metrics collector for the MongoDB-RabbitMQ connector
#[derive(Clone)]
pub struct Metrics {
    inner: Option<MetricsInner>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    /// Create a new metrics collector with Prometheus metrics enabled
    pub fn new() -> Self {
        Self {
            inner: Some(MetricsInner::new()),
        }
    }

    /// Create a dummy metrics collector that ignores all calls
    pub fn dummy() -> Self {
        Self { inner: None }
    }

    /// Increment the total server count
    pub fn increment_servers(&self) {
        if let Some(inner) = &self.inner {
            let mut count = inner.server_count.lock().unwrap();
            *count += 1;
            inner.running_servers.set(*count as f64);
        }
    }

    /// Decrement the total server count
    pub fn decrement_servers(&self) {
        if let Some(inner) = &self.inner {
            let mut count = inner.server_count.lock().unwrap();
            if *count > 0 {
                *count -= 1;
            }
            inner.running_servers.set(*count as f64);
        }
    }

    /// Set the total server count
    pub fn set_server_count(&self, count: usize) {
        if let Some(inner) = &self.inner {
            let mut current_count = inner.server_count.lock().unwrap();
            *current_count = count;
            inner.running_servers.set(count as f64);
        }
    }

    /// Increment the server count for a specific collection
    pub fn increment_collection_server(&self, collection: &str, database: &str) {
        if let Some(inner) = &self.inner {
            let key = format!("{}:{}", database, collection);
            let mut counts = inner.collection_counts.lock().unwrap();
            let count = counts.entry(key).or_insert(0);
            *count += 1;

            inner
                .collection_servers
                .with_label_values(&[collection, database])
                .set(*count as f64);
        }
    }

    /// Decrement the server count for a specific collection
    pub fn decrement_collection_server(&self, collection: &str, database: &str) {
        if let Some(inner) = &self.inner {
            let key = format!("{}:{}", database, collection);
            let mut counts = inner.collection_counts.lock().unwrap();
            if let Some(count) = counts.get_mut(&key) {
                if *count > 0 {
                    *count -= 1;
                }
                inner
                    .collection_servers
                    .with_label_values(&[collection, database])
                    .set(*count as f64);
            }
        }
    }

    /// Set the server count for a specific collection
    pub fn set_collection_server_count(&self, collection: &str, database: &str, count: usize) {
        if let Some(inner) = &self.inner {
            let key = format!("{}:{}", database, collection);
            let mut counts = inner.collection_counts.lock().unwrap();
            counts.insert(key, count);

            inner
                .collection_servers
                .with_label_values(&[collection, database])
                .set(count as f64);
        }
    }

    /// Record a task restart
    pub fn record_task_restart(&self, collection: &str, database: &str, reason: &str) {
        if let Some(inner) = &self.inner {
            inner
                .task_restarts
                .with_label_values(&[collection, database, reason])
                .inc();
        }
    }

    /// Record a task failure
    pub fn record_task_failure(&self, collection: &str, database: &str, error_type: &str) {
        if let Some(inner) = &self.inner {
            inner
                .task_failures
                .with_label_values(&[collection, database, error_type])
                .inc();
        }
    }

    /// Record a task start
    pub fn record_task_start(&self) {
        if let Some(inner) = &self.inner {
            inner.task_total_started.inc();
        }
    }

    /// Get the current total server count
    pub fn get_server_count(&self) -> usize {
        if let Some(inner) = &self.inner {
            *inner.server_count.lock().unwrap()
        } else {
            0
        }
    }

    /// Get the current server count for a specific collection
    pub fn get_collection_server_count(&self, collection: &str, database: &str) -> usize {
        if let Some(inner) = &self.inner {
            let key = format!("{}:{}", database, collection);
            let counts = inner.collection_counts.lock().unwrap();
            *counts.get(&key).unwrap_or(&0)
        } else {
            0
        }
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> Result<String, prometheus::Error> {
        if let Some(inner) = &self.inner {
            let encoder = TextEncoder::new();
            let metric_families = inner.registry.gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer)?;
            Ok(String::from_utf8_lossy(&buffer).to_string())
        } else {
            Ok(String::new())
        }
    }

    /// Get the registry for use with axum-prometheus
    /// Returns None if metrics are disabled (dummy mode)
    pub fn registry(&self) -> Option<Arc<Registry>> {
        self.inner.as_ref().map(|inner| inner.registry.clone())
    }
}

impl MetricsInner {
    /// Create a new inner metrics implementation
    fn new() -> Self {
        let registry = Arc::new(Registry::new());

        // Total number of running servers
        let running_servers = Arc::new(
            Gauge::with_opts(Opts::new(
                "mrcon_running_servers_total",
                "Total number of running connector servers",
            ))
            .expect("Failed to create running_servers gauge"),
        );

        // Per-collection server count
        let collection_servers = Arc::new(
            GaugeVec::new(
                Opts::new(
                    "mrcon_collection_servers",
                    "Number of connector servers per collection",
                ),
                &["collection", "database"],
            )
            .expect("Failed to create collection_servers gauge"),
        );

        // Task restart counter
        let task_restarts = Arc::new(
            CounterVec::new(
                Opts::new(
                    "mrcon_task_restarts_total",
                    "Total number of task restarts per collection",
                ),
                &["collection", "database", "reason"],
            )
            .expect("Failed to create task_restarts counter"),
        );

        // Task failure counter
        let task_failures = Arc::new(
            CounterVec::new(
                Opts::new(
                    "mrcon_task_failures_total",
                    "Total number of task failures per collection",
                ),
                &["collection", "database", "error_type"],
            )
            .expect("Failed to create task_failures counter"),
        );

        // Total tasks started counter
        let task_total_started = Arc::new(
            Counter::with_opts(Opts::new(
                "mrcon_tasks_started_total",
                "Total number of tasks started since server startup",
            ))
            .expect("Failed to create task_total_started counter"),
        );

        // Register metrics
        registry
            .register(Box::new((*running_servers).clone()))
            .expect("Failed to register running_servers metric");
        registry
            .register(Box::new((*collection_servers).clone()))
            .expect("Failed to register collection_servers metric");
        registry
            .register(Box::new((*task_restarts).clone()))
            .expect("Failed to register task_restarts metric");
        registry
            .register(Box::new((*task_failures).clone()))
            .expect("Failed to register task_failures metric");
        registry
            .register(Box::new((*task_total_started).clone()))
            .expect("Failed to register task_total_started metric");

        Self {
            registry,
            running_servers,
            collection_servers,
            task_restarts,
            task_failures,
            task_total_started,
            server_count: Arc::new(Mutex::new(0)),
            collection_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new();
        assert_eq!(metrics.get_server_count(), 0);
    }

    #[test]
    fn test_dummy_metrics() {
        let metrics = Metrics::dummy();
        assert_eq!(metrics.get_server_count(), 0);

        // All operations should be no-ops and return sensible defaults
        metrics.increment_servers();
        assert_eq!(metrics.get_server_count(), 0);

        metrics.record_task_start();
        metrics.record_task_restart("test", "db", "reason");
        metrics.record_task_failure("test", "db", "error");

        assert_eq!(metrics.get_collection_server_count("test", "db"), 0);
        assert_eq!(metrics.export().unwrap(), "");
        assert!(metrics.registry().is_none());
    }

    #[test]
    fn test_server_count() {
        let metrics = Metrics::new();

        metrics.increment_servers();
        assert_eq!(metrics.get_server_count(), 1);

        metrics.increment_servers();
        assert_eq!(metrics.get_server_count(), 2);

        metrics.decrement_servers();
        assert_eq!(metrics.get_server_count(), 1);

        metrics.set_server_count(5);
        assert_eq!(metrics.get_server_count(), 5);
    }

    #[test]
    fn test_collection_server_count() {
        let metrics = Metrics::new();

        metrics.increment_collection_server("users", "mydb");
        assert_eq!(metrics.get_collection_server_count("users", "mydb"), 1);

        metrics.increment_collection_server("users", "mydb");
        assert_eq!(metrics.get_collection_server_count("users", "mydb"), 2);

        metrics.decrement_collection_server("users", "mydb");
        assert_eq!(metrics.get_collection_server_count("users", "mydb"), 1);

        metrics.set_collection_server_count("orders", "mydb", 3);
        assert_eq!(metrics.get_collection_server_count("orders", "mydb"), 3);
    }

    #[test]
    fn test_export() {
        let metrics = Metrics::new();
        metrics.set_server_count(2);
        metrics.set_collection_server_count("users", "mydb", 1);
        metrics.record_task_restart("users", "mydb", "mongo_error");
        metrics.record_task_failure("users", "mydb", "connection_failed");
        metrics.record_task_start();

        let export = metrics.export().unwrap();
        assert!(export.contains("mrcon_running_servers_total"));
        assert!(export.contains("mrcon_collection_servers"));
        assert!(export.contains("mrcon_task_restarts_total"));
        assert!(export.contains("mrcon_task_failures_total"));
        assert!(export.contains("mrcon_tasks_started_total"));
    }
}
