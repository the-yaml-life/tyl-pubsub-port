use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Overall health status of the PubSub system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall system health
    pub status: Health,

    /// Health of individual components
    pub components: HashMap<String, ComponentHealth>,

    /// When this health check was performed
    pub checked_at: chrono::DateTime<chrono::Utc>,

    /// Any error messages or warnings
    pub messages: Vec<String>,

    /// System uptime
    pub uptime: std::time::Duration,

    /// Version information
    pub version: String,
}

/// Health status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Health {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health of an individual component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: Health,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub error_message: Option<String>,
    pub metrics: HashMap<String, serde_json::Value>,
}

/// Connection status to underlying infrastructure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    /// Whether the connection is active
    pub connected: bool,

    /// Connection details for each endpoint
    pub connections: Vec<ConnectionInfo>,

    /// Total number of active connections
    pub active_connections: u32,

    /// Connection pool statistics
    pub pool_stats: PoolStats,

    /// Last connection attempt timestamp
    pub last_connection_attempt: Option<chrono::DateTime<chrono::Utc>>,

    /// Connection error if any
    pub error: Option<String>,
}

/// Information about a specific connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub endpoint: String,
    pub connected: bool,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub error_count: u32,
}

/// Connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total_connections: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub pending_requests: u32,
    pub max_connections: u32,
    pub min_connections: u32,
}

/// Comprehensive metrics for a specific topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMetrics {
    /// The topic these metrics are for
    pub topic: String,

    /// Publishing metrics
    pub publishing: PublishingMetrics,

    /// Subscription metrics
    pub subscription: SubscriptionMetrics,

    /// Error and retry metrics
    pub errors: ErrorMetrics,

    /// Performance metrics
    pub performance: PerformanceMetrics,

    /// When these metrics were collected
    pub collected_at: chrono::DateTime<chrono::Utc>,
}

/// Publishing-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishingMetrics {
    /// Total events published
    pub events_published: u64,

    /// Events published in the last minute
    pub events_published_last_minute: u64,

    /// Events published in the last hour
    pub events_published_last_hour: u64,

    /// Average events per second
    pub events_per_second: f64,

    /// Peak events per second
    pub peak_events_per_second: f64,

    /// Total bytes published
    pub bytes_published: u64,

    /// Average event size
    pub average_event_size: f64,

    /// Failed publish attempts
    pub failed_publishes: u64,
}

/// Subscription-related metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionMetrics {
    /// Total events consumed
    pub events_consumed: u64,

    /// Events consumed in the last minute
    pub events_consumed_last_minute: u64,

    /// Events consumed in the last hour
    pub events_consumed_last_hour: u64,

    /// Average events consumed per second
    pub consumption_rate: f64,

    /// Number of active subscriptions
    pub active_subscriptions: u32,

    /// Number of consumer groups
    pub consumer_groups: u32,

    /// Current lag (events behind)
    pub lag: u64,

    /// Failed consumption attempts
    pub failed_consumptions: u64,
}

/// Error and retry metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors encountered
    pub total_errors: u64,

    /// Errors in the last minute
    pub errors_last_minute: u64,

    /// Errors in the last hour
    pub errors_last_hour: u64,

    /// Error rate (errors per minute)
    pub error_rate: f64,

    /// Events currently in DLQ
    pub dlq_events: u64,

    /// Total retry attempts
    pub retry_attempts: u64,

    /// Successful retries
    pub successful_retries: u64,

    /// Failed retries (sent to DLQ)
    pub failed_retries: u64,

    /// Most common error types
    pub common_errors: Vec<ErrorTypeCount>,
}

/// Count of errors by type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorTypeCount {
    pub error_type: String,
    pub count: u64,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average latency for event processing
    pub average_latency: std::time::Duration,

    /// 95th percentile latency
    pub p95_latency: std::time::Duration,

    /// 99th percentile latency
    pub p99_latency: std::time::Duration,

    /// Maximum observed latency
    pub max_latency: std::time::Duration,

    /// Minimum observed latency
    pub min_latency: std::time::Duration,

    /// Throughput (events per second)
    pub throughput: f64,

    /// Peak throughput observed
    pub peak_throughput: f64,

    /// Memory usage for this topic
    pub memory_usage_bytes: u64,

    /// CPU usage percentage
    pub cpu_usage_percent: f64,
}

/// Global metrics across all topics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetrics {
    /// Total number of topics
    pub total_topics: u32,

    /// Total events across all topics
    pub total_events: u64,

    /// Global publishing metrics
    pub publishing: PublishingMetrics,

    /// Global subscription metrics
    pub subscription: SubscriptionMetrics,

    /// Global error metrics
    pub errors: ErrorMetrics,

    /// Global performance metrics
    pub performance: PerformanceMetrics,

    /// System resource usage
    pub system: SystemMetrics,

    /// When these metrics were collected
    pub collected_at: chrono::DateTime<chrono::Utc>,
}

/// System-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Total memory usage
    pub memory_usage_bytes: u64,

    /// Memory usage percentage
    pub memory_usage_percent: f64,

    /// CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Disk usage for storage
    pub disk_usage_bytes: u64,

    /// Network bytes in
    pub network_bytes_in: u64,

    /// Network bytes out
    pub network_bytes_out: u64,

    /// Number of file descriptors in use
    pub file_descriptors: u32,

    /// System load average
    pub load_average: f64,
}

/// Retry statistics for a topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStats {
    /// The topic these statistics are for
    pub topic: String,

    /// Total retry attempts
    pub total_retries: u64,

    /// Successful retries
    pub successful_retries: u64,

    /// Failed retries (exhausted attempts)
    pub failed_retries: u64,

    /// Retry success rate (0.0 to 1.0)
    pub success_rate: f64,

    /// Average number of attempts before success
    pub average_attempts_to_success: f64,

    /// Most retried event types
    pub most_retried_event_types: Vec<EventTypeRetryCount>,

    /// Retry attempts by failure reason
    pub retries_by_reason: HashMap<String, u64>,

    /// When these statistics were generated
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Retry count for a specific event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTypeRetryCount {
    pub event_type: String,
    pub retry_count: u64,
    pub success_count: u64,
}

/// Configuration for distributed tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    /// Whether tracing is enabled
    pub enabled: bool,

    /// Sample rate (0.0 to 1.0)
    pub sample_rate: f64,

    /// Service name for tracing
    pub service_name: String,

    /// Additional tags to include
    pub tags: HashMap<String, String>,

    /// Export configuration
    pub export: TraceExportConfig,
}

/// Trace export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceExportConfig {
    /// Export endpoint
    pub endpoint: Option<String>,

    /// Export format
    pub format: TraceFormat,

    /// Export interval
    pub interval: std::time::Duration,

    /// Batch size for exports
    pub batch_size: usize,
}

/// Supported trace export formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceFormat {
    /// Jaeger format
    Jaeger,
    /// Zipkin format
    Zipkin,
    /// OpenTelemetry format
    OpenTelemetry,
    /// JSON format
    Json,
    /// Custom format
    Custom(String),
}
