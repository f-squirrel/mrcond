//! Metrics module for Prometheus integration

#[cfg(feature = "metrics")]
use prometheus::{Counter, CounterVec, Encoder, Gauge, GaugeVec, Opts, Registry, TextEncoder};
#[cfg(feature = "metrics")]
use std::collections::HashMap;
#[cfg(feature = "metrics")]
use std::sync::{Arc, Mutex};

// We need these types for the unified method signatures even when metrics are disabled
#[cfg(not(feature = "metrics"))]
use std::sync::Arc;
#[cfg(not(feature = "metrics"))]
pub struct Registry;

/// Inner metrics implementation containing the actual Prometheus metrics
#[cfg(feature = "metrics")]
#[derive(Clone)]
struct MetricsInner {
    registry: Option<Arc<Registry>>,
    running_servers: Option<Arc<Gauge>>,
    collection_servers: Option<Arc<GaugeVec>>,
    task_restarts: Option<Arc<CounterVec>>,
    task_failures: Option<Arc<CounterVec>>,
    task_total_started: Option<Arc<Counter>>,
    server_count: Arc<Mutex<usize>>,
    collection_counts: Arc<Mutex<HashMap<String, usize>>>,
}

/// Metrics collector for the MongoDB-RabbitMQ connector
#[derive(Clone)]
pub struct Metrics {
    #[cfg(feature = "metrics")]
    inner: MetricsInner,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        #[cfg(feature = "metrics")]
        {
            Self {
                inner: MetricsInner::new(),
            }
        }
        #[cfg(not(feature = "metrics"))]
        {
            Self {}
        }
    }

    /// Create a dummy metrics collector that ignores all calls
    pub fn dummy() -> Self {
        #[cfg(feature = "metrics")]
        {
            Self {
                inner: MetricsInner::new_dummy(),
            }
        }
        #[cfg(not(feature = "metrics"))]
        {
            Self {}
        }
    }

    /// Increment the total server count
    pub fn increment_servers(&self) {
        #[cfg(feature = "metrics")]
        {
            self.inner.increment_servers();
        }
    }

    /// Decrement the total server count
    pub fn decrement_servers(&self) {
        #[cfg(feature = "metrics")]
        {
            self.inner.decrement_servers();
        }
    }

    /// Set the total server count
    pub fn set_server_count(&self, count: usize) {
        #[cfg(feature = "metrics")]
        {
            self.inner.set_server_count(count);
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = count; // Suppress unused parameter warning
        }
    }

    /// Increment the server count for a specific collection
    pub fn increment_collection_server(&self, collection: &str, database: &str) {
        #[cfg(feature = "metrics")]
        {
            self.inner.increment_collection_server(collection, database);
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = (collection, database); // Suppress unused parameter warnings
        }
    }

    /// Decrement the server count for a specific collection
    pub fn decrement_collection_server(&self, collection: &str, database: &str) {
        #[cfg(feature = "metrics")]
        {
            self.inner.decrement_collection_server(collection, database);
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = (collection, database); // Suppress unused parameter warnings
        }
    }

    /// Set the server count for a specific collection
    pub fn set_collection_server_count(&self, collection: &str, database: &str, count: usize) {
        #[cfg(feature = "metrics")]
        {
            self.inner
                .set_collection_server_count(collection, database, count);
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = (collection, database, count); // Suppress unused parameter warnings
        }
    }

    /// Record a task restart
    pub fn record_task_restart(&self, collection: &str, database: &str, reason: &str) {
        #[cfg(feature = "metrics")]
        {
            self.inner.record_task_restart(collection, database, reason);
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = (collection, database, reason); // Suppress unused parameter warnings
        }
    }

    /// Record a task failure
    pub fn record_task_failure(&self, collection: &str, database: &str, error_type: &str) {
        #[cfg(feature = "metrics")]
        {
            self.inner
                .record_task_failure(collection, database, error_type);
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = (collection, database, error_type); // Suppress unused parameter warnings
        }
    }

    /// Record a task start
    pub fn record_task_start(&self) {
        #[cfg(feature = "metrics")]
        {
            self.inner.record_task_start();
        }
    }

    /// Get the current total server count
    pub fn get_server_count(&self) -> usize {
        #[cfg(feature = "metrics")]
        {
            self.inner.get_server_count()
        }
        #[cfg(not(feature = "metrics"))]
        {
            0
        }
    }

    /// Get the current server count for a specific collection
    pub fn get_collection_server_count(&self, collection: &str, database: &str) -> usize {
        #[cfg(feature = "metrics")]
        {
            self.inner.get_collection_server_count(collection, database)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = (collection, database); // Suppress unused parameter warnings
            0
        }
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(feature = "metrics")]
        {
            self.inner.export().map_err(|e| e.into())
        }
        #[cfg(not(feature = "metrics"))]
        {
            Ok(String::new())
        }
    }

    /// Get the registry for use with axum-prometheus
    /// Returns None if metrics are disabled (dummy mode)
    pub fn registry(&self) -> Option<Arc<Registry>> {
        #[cfg(feature = "metrics")]
        {
            self.inner.registry()
        }
        #[cfg(not(feature = "metrics"))]
        {
            None
        }
    }
}

#[cfg(feature = "metrics")]
impl MetricsInner {
    /// Create a new inner metrics implementation with full Prometheus functionality
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
            registry: Some(registry),
            running_servers: Some(running_servers),
            collection_servers: Some(collection_servers),
            task_restarts: Some(task_restarts),
            task_failures: Some(task_failures),
            task_total_started: Some(task_total_started),
            server_count: Arc::new(Mutex::new(0)),
            collection_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a dummy metrics implementation that ignores all calls
    fn new_dummy() -> Self {
        Self {
            registry: None,
            running_servers: None,
            collection_servers: None,
            task_restarts: None,
            task_failures: None,
            task_total_started: None,
            server_count: Arc::new(Mutex::new(0)),
            collection_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Increment the total server count
    fn increment_servers(&self) {
        if let Some(running_servers) = &self.running_servers {
            let mut count = self.server_count.lock().unwrap();
            *count += 1;
            running_servers.set(*count as f64);
        }
    }

    /// Decrement the total server count
    fn decrement_servers(&self) {
        if let Some(running_servers) = &self.running_servers {
            let mut count = self.server_count.lock().unwrap();
            if *count > 0 {
                *count -= 1;
            }
            running_servers.set(*count as f64);
        }
    }

    /// Set the total server count
    fn set_server_count(&self, count: usize) {
        if let Some(running_servers) = &self.running_servers {
            let mut current_count = self.server_count.lock().unwrap();
            *current_count = count;
            running_servers.set(count as f64);
        }
    }

    /// Increment the server count for a specific collection
    fn increment_collection_server(&self, collection: &str, database: &str) {
        if let Some(collection_servers) = &self.collection_servers {
            let key = format!("{}:{}", database, collection);
            let mut counts = self.collection_counts.lock().unwrap();
            let count = counts.entry(key).or_insert(0);
            *count += 1;

            collection_servers
                .with_label_values(&[collection, database])
                .set(*count as f64);
        }
    }

    /// Decrement the server count for a specific collection
    fn decrement_collection_server(&self, collection: &str, database: &str) {
        if let Some(collection_servers) = &self.collection_servers {
            let key = format!("{}:{}", database, collection);
            let mut counts = self.collection_counts.lock().unwrap();
            if let Some(count) = counts.get_mut(&key) {
                if *count > 0 {
                    *count -= 1;
                }
                collection_servers
                    .with_label_values(&[collection, database])
                    .set(*count as f64);
            }
        }
    }

    /// Set the server count for a specific collection
    fn set_collection_server_count(&self, collection: &str, database: &str, count: usize) {
        if let Some(collection_servers) = &self.collection_servers {
            let key = format!("{}:{}", database, collection);
            let mut counts = self.collection_counts.lock().unwrap();
            counts.insert(key, count);

            collection_servers
                .with_label_values(&[collection, database])
                .set(count as f64);
        }
    }

    /// Record a task restart
    fn record_task_restart(&self, collection: &str, database: &str, reason: &str) {
        if let Some(task_restarts) = &self.task_restarts {
            task_restarts
                .with_label_values(&[collection, database, reason])
                .inc();
        }
    }

    /// Record a task failure
    fn record_task_failure(&self, collection: &str, database: &str, error_type: &str) {
        if let Some(task_failures) = &self.task_failures {
            task_failures
                .with_label_values(&[collection, database, error_type])
                .inc();
        }
    }

    /// Record a task start
    fn record_task_start(&self) {
        if let Some(task_total_started) = &self.task_total_started {
            task_total_started.inc();
        }
    }

    /// Get the current total server count
    fn get_server_count(&self) -> usize {
        *self.server_count.lock().unwrap()
    }

    /// Get the current server count for a specific collection
    fn get_collection_server_count(&self, collection: &str, database: &str) -> usize {
        let key = format!("{}:{}", database, collection);
        let counts = self.collection_counts.lock().unwrap();
        *counts.get(&key).unwrap_or(&0)
    }

    /// Export metrics in Prometheus format
    fn export(&self) -> Result<String, prometheus::Error> {
        if let Some(registry) = &self.registry {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer)?;
            Ok(String::from_utf8_lossy(&buffer).to_string())
        } else {
            Ok(String::new())
        }
    }

    /// Get the registry for use with axum-prometheus
    fn registry(&self) -> Option<Arc<Registry>> {
        self.registry.as_ref().cloned()
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
        #[cfg(feature = "metrics")]
        assert!(metrics.registry().is_none());
        #[cfg(not(feature = "metrics"))]
        assert!(metrics.registry().is_none());
    }

    #[cfg(feature = "metrics")]
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

    #[cfg(feature = "metrics")]
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

    #[cfg(feature = "metrics")]
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

    #[cfg(not(feature = "metrics"))]
    #[test]
    fn test_no_metrics_feature() {
        let metrics = Metrics::new();

        // All operations should be no-ops
        metrics.increment_servers();
        metrics.decrement_servers();
        metrics.set_server_count(100);
        assert_eq!(metrics.get_server_count(), 0);

        metrics.increment_collection_server("test", "db");
        metrics.decrement_collection_server("test", "db");
        metrics.set_collection_server_count("test", "db", 50);
        assert_eq!(metrics.get_collection_server_count("test", "db"), 0);

        metrics.record_task_start();
        metrics.record_task_restart("test", "db", "reason");
        metrics.record_task_failure("test", "db", "error");

        assert_eq!(metrics.export().unwrap(), "");
        assert!(metrics.registry().is_none());
    }
}
