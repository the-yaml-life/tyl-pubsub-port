use crate::DlqConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main configuration for PubSub operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubConfig {
    /// Type of adapter to use (mock, redis, kafka, etc.)
    pub adapter_type: AdapterType,

    /// Connection configuration (adapter-specific)
    pub connection_config: ConnectionConfig,

    /// Default retry policy for all topics
    pub default_retry_policy: RetryPolicy,

    /// Dead Letter Queue configuration
    pub dlq_config: DlqConfig,

    /// Batch processing configuration
    pub batch_config: BatchConfig,

    /// Monitoring and observability configuration
    pub monitoring_config: MonitoringConfig,

    /// Event serialization settings
    pub serialization_config: SerializationConfig,
}

impl Default for PubSubConfig {
    fn default() -> Self {
        Self {
            adapter_type: AdapterType::Mock,
            connection_config: ConnectionConfig::default(),
            default_retry_policy: RetryPolicy::default(),
            dlq_config: DlqConfig::default(),
            batch_config: BatchConfig::default(),
            monitoring_config: MonitoringConfig::default(),
            serialization_config: SerializationConfig::default(),
        }
    }
}

/// Types of PubSub adapters available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdapterType {
    /// In-memory mock implementation for testing
    Mock,
    /// Redis Streams adapter
    Redis,
    /// Apache Kafka adapter
    Kafka,
    /// NATS adapter
    Nats,
    /// Custom adapter implementation
    Custom(String),
}

/// Connection configuration for adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection URL or endpoints
    pub urls: Vec<String>,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Keep-alive settings
    pub keep_alive: Option<Duration>,

    /// Maximum number of connections in the pool
    pub max_connections: u32,

    /// Minimum number of connections to maintain
    pub min_connections: u32,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// TLS/SSL configuration
    pub tls: Option<TlsConfig>,

    /// Additional adapter-specific settings
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            urls: vec!["localhost:6379".to_string()],
            connection_timeout: Duration::from_secs(30),
            keep_alive: Some(Duration::from_secs(60)),
            max_connections: 10,
            min_connections: 1,
            auth: None,
            tls: None,
            additional: std::collections::HashMap::new(),
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub certificate_path: Option<String>,
    pub key_path: Option<String>,
}

/// TLS/SSL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub verify_certificates: bool,
    pub ca_certificate_path: Option<String>,
    pub client_certificate_path: Option<String>,
    pub client_key_path: Option<String>,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Backoff strategy to use
    pub backoff_strategy: BackoffStrategy,

    /// Initial retry delay
    pub retry_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Whether to send to DLQ after max retries
    pub dlq_after_max_retries: bool,

    /// Specific error types that should trigger retries
    pub retriable_errors: Vec<String>,

    /// Specific error types that should never be retried
    pub non_retriable_errors: Vec<String>,

    /// Jitter factor to add randomness to retry delays
    pub jitter_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
            retry_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(60),
            dlq_after_max_retries: true,
            retriable_errors: vec![],
            non_retriable_errors: vec![],
            jitter_factor: 0.1,
        }
    }
}

impl RetryPolicy {
    /// Create a simple exponential backoff retry policy
    pub fn exponential_backoff(max_attempts: u32, initial_delay: Duration) -> Self {
        Self {
            max_attempts,
            retry_delay: initial_delay,
            backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
            ..Default::default()
        }
    }

    /// Create a linear backoff retry policy
    pub fn linear_backoff(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            retry_delay: delay,
            backoff_strategy: BackoffStrategy::Linear,
            ..Default::default()
        }
    }

    /// Create a fixed delay retry policy
    pub fn fixed_delay(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            retry_delay: delay,
            backoff_strategy: BackoffStrategy::Fixed,
            ..Default::default()
        }
    }

    /// Calculate the delay for a specific retry attempt
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = match self.backoff_strategy {
            BackoffStrategy::Fixed => self.retry_delay,
            BackoffStrategy::Linear => {
                Duration::from_millis(self.retry_delay.as_millis() as u64 * attempt as u64)
            }
            BackoffStrategy::Exponential { multiplier } => Duration::from_millis(
                (self.retry_delay.as_millis() as f64 * multiplier.powi(attempt as i32 - 1)) as u64,
            ),
        };

        // Apply jitter
        let jitter = if self.jitter_factor > 0.0 {
            let max_jitter = (base_delay.as_millis() as f64 * self.jitter_factor) as u64;
            Duration::from_millis(fastrand::u64(0..=max_jitter))
        } else {
            Duration::ZERO
        };

        let total_delay = base_delay + jitter;

        // Cap at max_delay
        if total_delay > self.max_delay {
            self.max_delay
        } else {
            total_delay
        }
    }

    /// Check if an error should be retried
    pub fn should_retry_error(&self, error_type: &str) -> bool {
        if self.non_retriable_errors.contains(&error_type.to_string()) {
            return false;
        }

        if self.retriable_errors.is_empty() {
            true // Default: retry all errors if no specific list
        } else {
            self.retriable_errors.contains(&error_type.to_string())
        }
    }
}

/// Backoff strategies for retry delays
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Linear increase in delay
    Linear,
    /// Exponential increase in delay
    Exponential { multiplier: f64 },
}

/// Batch processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of events in a batch
    pub max_batch_size: usize,

    /// Maximum time to wait before processing a partial batch
    pub batch_timeout: Duration,

    /// Maximum memory usage for batching
    pub max_batch_memory: usize,

    /// Whether to preserve event ordering within batches
    pub preserve_order: bool,

    /// Compression settings for batches
    pub compression_enabled: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            batch_timeout: Duration::from_millis(100),
            max_batch_memory: 1024 * 1024, // 1MB
            preserve_order: true,
            compression_enabled: false,
        }
    }
}

/// Monitoring and observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Whether to enable metrics collection
    pub metrics_enabled: bool,

    /// Metrics collection interval
    pub metrics_interval: Duration,

    /// Whether to enable distributed tracing
    pub tracing_enabled: bool,

    /// Tracing sample rate (0.0 to 1.0)
    pub tracing_sample_rate: f64,

    /// Whether to enable health checks
    pub health_checks_enabled: bool,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Whether to log all events (for debugging)
    pub log_events: bool,

    /// Log level for PubSub operations
    pub log_level: LogLevel,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            metrics_interval: Duration::from_secs(30),
            tracing_enabled: false,
            tracing_sample_rate: 0.1,
            health_checks_enabled: true,
            health_check_interval: Duration::from_secs(30),
            log_events: false,
            log_level: LogLevel::Info,
        }
    }
}

/// Log levels for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Event serialization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializationConfig {
    /// Default serialization format
    pub format: SerializationFormat,

    /// Whether to compress event payloads
    pub compression_enabled: bool,

    /// Compression algorithm to use
    pub compression_algorithm: CompressionAlgorithm,

    /// Maximum size for event payloads
    pub max_payload_size: usize,

    /// Whether to validate schemas
    pub schema_validation_enabled: bool,
}

impl Default for SerializationConfig {
    fn default() -> Self {
        Self {
            format: SerializationFormat::Json,
            compression_enabled: false,
            compression_algorithm: CompressionAlgorithm::Gzip,
            max_payload_size: 1024 * 1024, // 1MB
            schema_validation_enabled: false,
        }
    }
}

/// Supported serialization formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializationFormat {
    Json,
    MessagePack,
    Protobuf,
    Avro,
    Custom(String),
}

/// Supported compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    Lz4,
    Snappy,
    Zstd,
}
