use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for events
pub type EventId = String;

/// Unique identifier for transactions
pub type TransactionId = String;

/// Core event structure that wraps user payloads with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event<T> {
    /// Unique identifier for this event
    pub id: EventId,

    /// The topic this event belongs to
    pub topic: String,

    /// The actual event payload
    pub payload: T,

    /// Rich metadata about the event
    pub metadata: EventMetadata,

    /// When this event was created
    pub timestamp: DateTime<Utc>,

    /// Custom headers for additional context
    pub headers: HashMap<String, String>,
}

impl<T> Event<T> {
    /// Create a new event with the given payload and topic
    pub fn new(topic: impl Into<String>, payload: T) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.into(),
            payload,
            metadata: EventMetadata::default(),
            timestamp: Utc::now(),
            headers: HashMap::new(),
        }
    }

    /// Create a new event with custom metadata
    pub fn with_metadata(topic: impl Into<String>, payload: T, metadata: EventMetadata) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.into(),
            payload,
            metadata,
            timestamp: Utc::now(),
            headers: HashMap::new(),
        }
    }

    /// Add a custom header to this event
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add multiple headers to this event
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Set a custom event ID (useful for idempotency)
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set a custom timestamp
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Get a header value by key
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Check if this event has a specific header
    pub fn has_header(&self, key: &str) -> bool {
        self.headers.contains_key(key)
    }
}

/// Rich metadata associated with events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Distributed tracing ID for correlation across services
    pub trace_id: Option<String>,

    /// Correlation ID for linking related events
    pub correlation_id: Option<String>,

    /// Causation ID linking this event to its cause
    pub causation_id: Option<String>,

    /// The source service/component that generated this event
    pub source: String,

    /// The type/category of this event
    pub event_type: String,

    /// Partition key for ordered processing
    pub partition_key: Option<String>,

    /// Schema version for event evolution
    pub schema_version: String,

    /// User or system that initiated this event
    pub user_id: Option<String>,

    /// Session or request context
    pub session_id: Option<String>,

    /// Additional custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            trace_id: None,
            correlation_id: None,
            causation_id: None,
            source: "unknown".to_string(),
            event_type: "unknown".to_string(),
            partition_key: None,
            schema_version: "1.0".to_string(),
            user_id: None,
            session_id: None,
            custom: HashMap::new(),
        }
    }
}

impl EventMetadata {
    /// Create new metadata with source and event type
    pub fn new(source: impl Into<String>, event_type: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            event_type: event_type.into(),
            ..Default::default()
        }
    }

    /// Set the trace ID for distributed tracing
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Set the correlation ID
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Set the causation ID
    pub fn with_causation_id(mut self, causation_id: impl Into<String>) -> Self {
        self.causation_id = Some(causation_id.into());
        self
    }

    /// Set the partition key
    pub fn with_partition_key(mut self, partition_key: impl Into<String>) -> Self {
        self.partition_key = Some(partition_key.into());
        self
    }

    /// Set the user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Set the schema version
    pub fn with_schema_version(mut self, version: impl Into<String>) -> Self {
        self.schema_version = version.into();
        self
    }
}

/// A topic-event pair for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicEvent<T> {
    pub topic: String,
    pub event: T,
    pub partition_key: Option<String>,
}

impl<T> TopicEvent<T> {
    /// Create a new topic-event pair
    pub fn new(topic: impl Into<String>, event: T) -> Self {
        Self {
            topic: topic.into(),
            event,
            partition_key: None,
        }
    }

    /// Create a topic-event pair with a partition key
    pub fn with_key(topic: impl Into<String>, event: T, partition_key: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            event,
            partition_key: Some(partition_key.into()),
        }
    }
}

/// Generate a new unique event ID
pub fn generate_event_id() -> EventId {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a new unique transaction ID
pub fn generate_transaction_id() -> TransactionId {
    uuid::Uuid::new_v4().to_string()
}
