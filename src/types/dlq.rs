use crate::{EventId, EventMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an event that failed processing and its failure context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedEvent {
    /// The original event ID that failed
    pub original_event_id: EventId,

    /// The topic the event was published to
    pub topic: String,

    /// The event payload as raw JSON (since we don't know the original type)
    pub payload: serde_json::Value,

    /// The reason why this event failed
    pub failure_reason: FailureReason,

    /// How many times this event has been attempted
    pub attempt_count: u32,

    /// When this event first failed
    pub first_failure_time: DateTime<Utc>,

    /// When this event last failed
    pub last_failure_time: DateTime<Utc>,

    /// Original event metadata
    pub metadata: EventMetadata,

    /// Stack trace or detailed error information
    pub error_details: Option<String>,

    /// The handler that failed to process this event
    pub failed_handler: Option<String>,

    /// Additional context about the failure
    pub failure_context: std::collections::HashMap<String, serde_json::Value>,
}

impl FailedEvent {
    /// Create a new failed event
    pub fn new(
        original_event_id: EventId,
        topic: impl Into<String>,
        payload: serde_json::Value,
        failure_reason: FailureReason,
        metadata: EventMetadata,
    ) -> Self {
        let now = Utc::now();
        Self {
            original_event_id,
            topic: topic.into(),
            payload,
            failure_reason,
            attempt_count: 1,
            first_failure_time: now,
            last_failure_time: now,
            metadata,
            error_details: None,
            failed_handler: None,
            failure_context: std::collections::HashMap::new(),
        }
    }

    /// Record another failure attempt
    pub fn add_failure_attempt(&mut self, reason: FailureReason) {
        self.attempt_count += 1;
        self.last_failure_time = Utc::now();
        self.failure_reason = reason;
    }

    /// Set error details
    pub fn with_error_details(mut self, details: impl Into<String>) -> Self {
        self.error_details = Some(details.into());
        self
    }

    /// Set the handler that failed
    pub fn with_failed_handler(mut self, handler: impl Into<String>) -> Self {
        self.failed_handler = Some(handler.into());
        self
    }

    /// Add failure context
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.failure_context.insert(key.into(), value);
        self
    }

    /// Calculate how long this event has been failing
    pub fn failure_duration(&self) -> chrono::Duration {
        self.last_failure_time - self.first_failure_time
    }

    /// Check if this event has exceeded the maximum retry attempts
    pub fn has_exceeded_max_retries(&self, max_retries: u32) -> bool {
        self.attempt_count >= max_retries
    }
}

/// Various reasons why an event might fail processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureReason {
    /// The event handler threw an exception during processing
    HandlerError { error: String },

    /// Failed to deserialize the event payload
    SerializationError { error: String },

    /// The handler took too long to process the event
    TimeoutError,

    /// Network issues prevented processing
    NetworkError { error: String },

    /// The event was retried the maximum number of times
    MaxRetriesExceeded,

    /// The event format was invalid or corrupted
    InvalidEventFormat { details: String },

    /// A dependency service was unavailable
    DependencyUnavailable { service: String },

    /// Insufficient resources to process the event
    ResourceExhausted { resource: String },

    /// Authentication or authorization failed
    AuthenticationError { details: String },

    /// Business logic validation failed
    ValidationError { field: String, message: String },

    /// A custom failure reason
    Custom { reason: String },
}

impl std::fmt::Display for FailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureReason::HandlerError { error } => write!(f, "Handler error: {error}"),
            FailureReason::SerializationError { error } => {
                write!(f, "Serialization error: {error}")
            }
            FailureReason::TimeoutError => write!(f, "Operation timed out"),
            FailureReason::NetworkError { error } => write!(f, "Network error: {error}"),
            FailureReason::MaxRetriesExceeded => write!(f, "Maximum retry attempts exceeded"),
            FailureReason::InvalidEventFormat { details } => {
                write!(f, "Invalid event format: {details}")
            }
            FailureReason::DependencyUnavailable { service } => {
                write!(f, "Dependency unavailable: {service}")
            }
            FailureReason::ResourceExhausted { resource } => {
                write!(f, "Resource exhausted: {resource}")
            }
            FailureReason::AuthenticationError { details } => {
                write!(f, "Authentication error: {details}")
            }
            FailureReason::ValidationError { field, message } => {
                write!(f, "Validation error in field '{field}': {message}")
            }
            FailureReason::Custom { reason } => write!(f, "Custom error: {reason}"),
        }
    }
}

/// Statistics about events in the Dead Letter Queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqStats {
    /// The topic these statistics are for
    pub topic: String,

    /// Total number of events currently in the DLQ
    pub total_events: u64,

    /// Number of unique event types in the DLQ
    pub unique_event_types: u64,

    /// The oldest event in the DLQ
    pub oldest_event_age: Option<chrono::Duration>,

    /// The newest event in the DLQ
    pub newest_event_age: Option<chrono::Duration>,

    /// Average number of retry attempts for events in DLQ
    pub average_retry_attempts: f64,

    /// Most common failure reasons
    pub common_failure_reasons: Vec<FailureReasonStats>,

    /// Events grouped by failure reason
    pub events_by_reason: std::collections::HashMap<String, u64>,

    /// Events grouped by failed handler
    pub events_by_handler: std::collections::HashMap<String, u64>,

    /// When these statistics were generated
    pub generated_at: DateTime<Utc>,
}

/// Statistics for a specific failure reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureReasonStats {
    pub reason: String,
    pub count: u64,
    pub percentage: f64,
}

/// Configuration for Dead Letter Queue behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqConfig {
    /// Maximum number of events to keep in the DLQ per topic
    pub max_events_per_topic: Option<u64>,

    /// How long to keep events in the DLQ before auto-purging
    pub retention_period: Option<chrono::Duration>,

    /// Whether to automatically purge old events
    pub auto_purge_enabled: bool,

    /// Whether to preserve event ordering in the DLQ
    pub preserve_order: bool,

    /// Compression settings for DLQ storage
    pub compression_enabled: bool,

    /// Whether to store full stack traces with failed events
    pub store_stack_traces: bool,
}

impl Default for DlqConfig {
    fn default() -> Self {
        Self {
            max_events_per_topic: Some(10000),
            retention_period: Some(chrono::Duration::days(7)),
            auto_purge_enabled: true,
            preserve_order: true,
            compression_enabled: false,
            store_stack_traces: true,
        }
    }
}
