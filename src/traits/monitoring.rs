use crate::{
    ConnectionStatus, GlobalMetrics, HealthStatus, PubSubResult, TopicMetrics, TraceConfig,
    TraceFormat,
};
use async_trait::async_trait;

/// Monitoring and observability port - handles health checks, metrics, and tracing
#[async_trait]
pub trait EventMonitoring: Send + Sync {
    /// Perform a health check on the PubSub system
    ///
    /// # Returns
    /// Overall health status of the system
    async fn health_check(&self) -> PubSubResult<HealthStatus>;

    /// Check the connection status to the underlying infrastructure
    ///
    /// # Returns
    /// Current connection status and details
    async fn connection_status(&self) -> PubSubResult<ConnectionStatus>;

    /// Get comprehensive metrics for a specific topic
    ///
    /// # Arguments
    /// * `topic` - The topic to get metrics for
    ///
    /// # Returns
    /// Detailed metrics for the specified topic
    async fn get_metrics(&self, topic: &str) -> PubSubResult<TopicMetrics>;

    /// Get global metrics across all topics and operations
    ///
    /// # Returns
    /// System-wide metrics and statistics
    async fn get_global_metrics(&self) -> PubSubResult<GlobalMetrics>;

    /// Get metrics for multiple topics in a single call
    ///
    /// # Arguments
    /// * `topics` - Vector of topic names to get metrics for
    ///
    /// # Returns
    /// Map of topic names to their metrics
    async fn get_bulk_metrics(
        &self,
        topics: Vec<String>,
    ) -> PubSubResult<std::collections::HashMap<String, TopicMetrics>>;

    /// Start distributed tracing for debugging and observability
    ///
    /// # Arguments
    /// * `trace_config` - Configuration for tracing collection
    ///
    /// # Returns
    /// Result of starting the tracing system
    async fn start_tracing(&self, trace_config: TraceConfig) -> PubSubResult<()>;

    /// Stop distributed tracing
    ///
    /// # Returns
    /// Result of stopping the tracing system
    async fn stop_tracing(&self) -> PubSubResult<()>;

    /// Export collected traces in the specified format
    ///
    /// # Arguments
    /// * `format` - The format to export traces in
    ///
    /// # Returns
    /// Serialized trace data
    async fn export_traces(&self, format: TraceFormat) -> PubSubResult<Vec<u8>>;

    /// Get real-time performance statistics
    ///
    /// # Returns
    /// Current performance metrics
    async fn get_performance_stats(&self) -> PubSubResult<PerformanceStats>;

    /// Set up alerts for specific conditions
    ///
    /// # Arguments
    /// * `alert_config` - Configuration for alert conditions
    ///
    /// # Returns
    /// Alert ID for managing the alert
    async fn create_alert(&self, alert_config: AlertConfig) -> PubSubResult<AlertId>;

    /// Remove an existing alert
    ///
    /// # Arguments
    /// * `alert_id` - The ID of the alert to remove
    ///
    /// # Returns
    /// Result of removing the alert
    async fn remove_alert(&self, alert_id: AlertId) -> PubSubResult<()>;

    /// Get the current status of all configured alerts
    ///
    /// # Returns
    /// Vector of alert statuses
    async fn get_alert_status(&self) -> PubSubResult<Vec<AlertStatus>>;

    /// Get historical metrics for trend analysis
    ///
    /// # Arguments
    /// * `topic` - The topic to get historical data for (None for global)
    /// * `time_range` - The time range to query
    /// * `interval` - The aggregation interval
    ///
    /// # Returns
    /// Time series data for the specified parameters
    async fn get_historical_metrics(
        &self,
        topic: Option<String>,
        time_range: TimeRange,
        interval: Duration,
    ) -> PubSubResult<TimeSeriesData>;
}

/// Time range for historical queries
#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Duration for aggregation intervals
#[derive(Debug, Clone)]
pub enum Duration {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
}

/// Time series data for historical analysis
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeSeriesData {
    pub metric_name: String,
    pub data_points: Vec<DataPoint>,
    pub aggregation_interval: String,
}

/// A single data point in a time series
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
}

/// Performance statistics for real-time monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
    pub network_bytes_in: u64,
    pub network_bytes_out: u64,
    pub active_connections: u32,
    pub queue_depth: u32,
    pub processing_latency_ms: f64,
}

/// Alert configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertConfig {
    pub name: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub duration: std::time::Duration,
    pub actions: Vec<AlertAction>,
}

/// Conditions that can trigger alerts
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AlertCondition {
    HighErrorRate,
    HighLatency,
    LowThroughput,
    HighMemoryUsage,
    HighCpuUsage,
    QueueBacklog,
    ConnectionFailures,
    Custom(String),
}

/// Actions to take when an alert is triggered
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AlertAction {
    Log,
    Email(String),
    Webhook(String),
    Custom(String),
}

/// Alert ID for managing alerts
pub type AlertId = String;

/// Current status of an alert
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertStatus {
    pub alert_id: AlertId,
    pub name: String,
    pub active: bool,
    pub last_triggered: Option<chrono::DateTime<chrono::Utc>>,
    pub trigger_count: u64,
}
