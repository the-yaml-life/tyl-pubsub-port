use crate::{DlqConfig, RetryPolicy};
use serde::{Deserialize, Serialize};

/// Unique identifier for subscriptions
pub type SubscriptionId = String;

/// Configuration options for event subscriptions
#[derive(Debug, Clone)]
pub struct SubscriptionOptions {
    /// Whether to automatically acknowledge events after processing
    pub auto_acknowledge: bool,

    /// Maximum number of events to process concurrently
    pub max_in_flight: usize,

    /// Consumer group name for load balancing
    pub consumer_group: Option<String>,

    /// Where to start consuming events from
    pub start_position: StartPosition,

    /// Custom retry policy for this subscription
    pub retry_policy: Option<RetryPolicy>,

    /// Dead Letter Queue configuration for this subscription
    pub dlq_config: Option<DlqConfig>,

    /// Whether to preserve event ordering
    pub preserve_order: bool,

    /// Subscription-specific metadata
    pub metadata: std::collections::HashMap<String, String>,

    /// Filter expression for selective event consumption
    pub filter: Option<EventFilter>,

    /// Backpressure configuration
    pub backpressure: BackpressureConfig,
}

impl Default for SubscriptionOptions {
    fn default() -> Self {
        Self {
            auto_acknowledge: true,
            max_in_flight: 10,
            consumer_group: None,
            start_position: StartPosition::Latest,
            retry_policy: None,
            dlq_config: None,
            preserve_order: false,
            metadata: std::collections::HashMap::new(),
            filter: None,
            backpressure: BackpressureConfig::default(),
        }
    }
}

impl SubscriptionOptions {
    /// Create subscription options for a consumer group
    pub fn consumer_group(group_name: impl Into<String>) -> Self {
        Self {
            consumer_group: Some(group_name.into()),
            start_position: StartPosition::Earliest,
            ..Default::default()
        }
    }

    /// Create subscription options that preserve event ordering
    pub fn ordered() -> Self {
        Self {
            preserve_order: true,
            max_in_flight: 1, // Required for ordering
            ..Default::default()
        }
    }

    /// Create subscription options for high-throughput scenarios
    pub fn high_throughput() -> Self {
        Self {
            max_in_flight: 100,
            auto_acknowledge: true,
            preserve_order: false,
            ..Default::default()
        }
    }

    /// Add metadata to the subscription
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set a custom retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Set a custom DLQ configuration
    pub fn with_dlq_config(mut self, config: DlqConfig) -> Self {
        self.dlq_config = Some(config);
        self
    }

    /// Set an event filter
    pub fn with_filter(mut self, filter: EventFilter) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// Where to start consuming events from when subscribing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StartPosition {
    /// Start from the earliest available event
    Earliest,
    /// Start from the latest event (only new events)
    Latest,
    /// Start from a specific timestamp
    Timestamp(chrono::DateTime<chrono::Utc>),
    /// Start from a specific event ID
    EventId(String),
    /// Start from a specific offset/position
    Offset(u64),
}

/// Event filtering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Filter by event type
    pub event_types: Option<Vec<String>>,

    /// Filter by source
    pub sources: Option<Vec<String>>,

    /// Filter by custom attributes
    pub attributes: Option<std::collections::HashMap<String, serde_json::Value>>,

    /// SQL-like filter expression
    pub expression: Option<String>,

    /// Whether to include or exclude matching events
    pub include_matching: bool,
}

impl EventFilter {
    /// Create a filter for specific event types
    pub fn event_types(types: Vec<String>) -> Self {
        Self {
            event_types: Some(types),
            sources: None,
            attributes: None,
            expression: None,
            include_matching: true,
        }
    }

    /// Create a filter for specific sources
    pub fn sources(sources: Vec<String>) -> Self {
        Self {
            event_types: None,
            sources: Some(sources),
            attributes: None,
            expression: None,
            include_matching: true,
        }
    }

    /// Create a filter with a custom expression
    pub fn expression(expr: impl Into<String>) -> Self {
        Self {
            event_types: None,
            sources: None,
            attributes: None,
            expression: Some(expr.into()),
            include_matching: true,
        }
    }
}

/// Backpressure configuration for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureConfig {
    /// Strategy to use when events are produced faster than consumed
    pub strategy: BackpressureStrategy,

    /// Buffer size for events
    pub buffer_size: usize,

    /// High water mark for triggering backpressure
    pub high_water_mark: usize,

    /// Low water mark for releasing backpressure
    pub low_water_mark: usize,

    /// Timeout for backpressure operations
    pub timeout: std::time::Duration,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            strategy: BackpressureStrategy::Block,
            buffer_size: 1000,
            high_water_mark: 800,
            low_water_mark: 200,
            timeout: std::time::Duration::from_secs(30),
        }
    }
}

/// Strategies for handling backpressure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackpressureStrategy {
    /// Block the producer until space is available
    Block,
    /// Drop the oldest events to make room for new ones
    DropOldest,
    /// Drop the newest events when buffer is full
    DropNewest,
    /// Use a custom strategy
    Custom(String),
}

/// Current status of a subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionStatus {
    /// The subscription ID
    pub subscription_id: SubscriptionId,

    /// Current state of the subscription
    pub state: SubscriptionState,

    /// Topic being subscribed to
    pub topic: String,

    /// Consumer group if applicable
    pub consumer_group: Option<String>,

    /// When the subscription was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When the subscription was last active
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,

    /// Current position in the event stream
    pub current_position: Option<String>,

    /// Number of events processed
    pub events_processed: u64,

    /// Number of events that failed processing
    pub events_failed: u64,

    /// Current lag behind the latest event
    pub lag: Option<u64>,

    /// Average processing time per event
    pub avg_processing_time: Option<std::time::Duration>,

    /// Health status of the subscription
    pub health: SubscriptionHealth,
}

/// Possible states of a subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionState {
    /// Subscription is active and processing events
    Active,
    /// Subscription is paused temporarily
    Paused,
    /// Subscription is stopped
    Stopped,
    /// Subscription has failed and needs attention
    Failed,
    /// Subscription is being created
    Creating,
    /// Subscription is being destroyed
    Destroying,
}

/// Health status of a subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionHealth {
    /// Subscription is healthy
    Healthy,
    /// Subscription is experiencing issues but still functioning
    Degraded,
    /// Subscription is unhealthy and may not be processing events
    Unhealthy,
}

/// Generate a new unique subscription ID
pub fn generate_subscription_id() -> SubscriptionId {
    uuid::Uuid::new_v4().to_string()
}
