use serde::{Deserialize, Serialize};

/// A stored event in the event store with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    /// Unique identifier for this stored event
    pub event_id: String,

    /// The stream this event belongs to
    pub stream_id: String,

    /// Version number within the stream
    pub stream_version: u64,

    /// Global position across all streams
    pub global_position: GlobalPosition,

    /// Event type identifier
    pub event_type: String,

    /// Event data as JSON
    pub data: serde_json::Value,

    /// Additional metadata as JSON
    pub metadata: serde_json::Value,

    /// When this event was stored
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Correlation ID for linking related events
    pub correlation_id: Option<String>,

    /// Causation ID linking this event to its cause
    pub causation_id: Option<String>,
}

impl StoredEvent {
    /// Create a new stored event
    pub fn new(event_type: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            stream_id: String::new(), // Will be set when appending to stream
            stream_version: 0,        // Will be set when appending to stream
            global_position: GlobalPosition(0), // Will be set when appending
            event_type: event_type.into(),
            data,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: chrono::Utc::now(),
            correlation_id: None,
            causation_id: None,
        }
    }

    /// Create a stored event with metadata
    pub fn with_metadata(
        event_type: impl Into<String>,
        data: serde_json::Value,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            metadata,
            ..Self::new(event_type, data)
        }
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Set causation ID
    pub fn with_causation_id(mut self, causation_id: impl Into<String>) -> Self {
        self.causation_id = Some(causation_id.into());
        self
    }

    /// Set custom event ID
    pub fn with_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.event_id = event_id.into();
        self
    }
}

/// Global position in the event store across all streams
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GlobalPosition(pub u64);

impl GlobalPosition {
    /// Create a new global position
    pub fn new(position: u64) -> Self {
        Self(position)
    }

    /// Get the position value
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Get the next position
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    /// Check if this position is before another
    pub fn is_before(&self, other: &GlobalPosition) -> bool {
        self.0 < other.0
    }

    /// Check if this position is after another
    pub fn is_after(&self, other: &GlobalPosition) -> bool {
        self.0 > other.0
    }
}

impl std::fmt::Display for GlobalPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A stream of events with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStream {
    /// The stream identifier
    pub stream_id: String,

    /// Events in this stream
    pub events: Vec<StoredEvent>,

    /// Current version of the stream
    pub current_version: u64,

    /// Whether this is the complete stream or a partial read
    pub is_complete: bool,

    /// The position this stream starts from
    pub start_position: GlobalPosition,

    /// The position this stream ends at
    pub end_position: GlobalPosition,

    /// When this stream was read
    pub read_at: chrono::DateTime<chrono::Utc>,
}

impl EventStream {
    /// Create a new event stream
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
            events: Vec::new(),
            current_version: 0,
            is_complete: true,
            start_position: GlobalPosition(0),
            end_position: GlobalPosition(0),
            read_at: chrono::Utc::now(),
        }
    }

    /// Add an event to this stream
    pub fn add_event(&mut self, event: StoredEvent) {
        self.current_version = std::cmp::max(self.current_version, event.stream_version);
        self.end_position = std::cmp::max(self.end_position, event.global_position);
        self.events.push(event);
    }

    /// Get the number of events in this stream
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Check if this stream is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get events of a specific type
    pub fn events_of_type(&self, event_type: &str) -> Vec<&StoredEvent> {
        self.events
            .iter()
            .filter(|event| event.event_type == event_type)
            .collect()
    }

    /// Get events within a version range
    pub fn events_in_version_range(&self, start: u64, end: u64) -> Vec<&StoredEvent> {
        self.events
            .iter()
            .filter(|event| event.stream_version >= start && event.stream_version <= end)
            .collect()
    }

    /// Get the latest event
    pub fn latest_event(&self) -> Option<&StoredEvent> {
        self.events.iter().max_by_key(|event| event.stream_version)
    }

    /// Get the earliest event
    pub fn earliest_event(&self) -> Option<&StoredEvent> {
        self.events.iter().min_by_key(|event| event.stream_version)
    }
}

/// A snapshot of aggregate state at a specific version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// The stream this snapshot belongs to
    pub stream_id: String,

    /// The version at which this snapshot was taken
    pub version: u64,

    /// The snapshot data
    pub data: serde_json::Value,

    /// Additional metadata
    pub metadata: serde_json::Value,

    /// When this snapshot was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// The type of snapshot
    pub snapshot_type: String,

    /// Compression used for the snapshot data
    pub compression: Option<String>,

    /// Checksum for data integrity
    pub checksum: Option<String>,
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(
        stream_id: impl Into<String>,
        version: u64,
        snapshot_type: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            stream_id: stream_id.into(),
            version,
            data,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: chrono::Utc::now(),
            snapshot_type: snapshot_type.into(),
            compression: None,
            checksum: None,
        }
    }

    /// Create a snapshot with metadata
    pub fn with_metadata(
        stream_id: impl Into<String>,
        version: u64,
        snapshot_type: impl Into<String>,
        data: serde_json::Value,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            metadata,
            ..Self::new(stream_id, version, snapshot_type, data)
        }
    }

    /// Set compression information
    pub fn with_compression(mut self, compression: impl Into<String>) -> Self {
        self.compression = Some(compression.into());
        self
    }

    /// Set checksum
    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Get the age of this snapshot
    pub fn age(&self) -> chrono::Duration {
        chrono::Utc::now() - self.created_at
    }

    /// Check if this snapshot is recent (within the given duration)
    pub fn is_recent(&self, max_age: chrono::Duration) -> bool {
        self.age() <= max_age
    }
}

/// Event store query parameters
#[derive(Debug, Clone)]
pub struct EventQuery {
    /// Stream ID to query (None for all streams)
    pub stream_id: Option<String>,

    /// Event types to include
    pub event_types: Option<Vec<String>>,

    /// Start position (inclusive)
    pub start_position: Option<GlobalPosition>,

    /// End position (exclusive)
    pub end_position: Option<GlobalPosition>,

    /// Start version for specific stream
    pub start_version: Option<u64>,

    /// End version for specific stream
    pub end_version: Option<u64>,

    /// Start timestamp
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,

    /// End timestamp
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,

    /// Maximum number of events to return
    pub limit: Option<usize>,

    /// Direction to read events (forward or backward)
    pub direction: ReadDirection,

    /// Additional filters
    pub filters: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for EventQuery {
    fn default() -> Self {
        Self {
            stream_id: None,
            event_types: None,
            start_position: None,
            end_position: None,
            start_version: None,
            end_version: None,
            start_time: None,
            end_time: None,
            limit: None,
            direction: ReadDirection::Forward,
            filters: std::collections::HashMap::new(),
        }
    }
}

/// Direction for reading events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadDirection {
    /// Read events in chronological order
    Forward,
    /// Read events in reverse chronological order
    Backward,
}

/// Result of appending events to a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendResult {
    /// The stream ID that was appended to
    pub stream_id: String,

    /// The new version of the stream after appending
    pub new_version: u64,

    /// The previous version before appending
    pub previous_version: u64,

    /// Number of events that were appended
    pub events_appended: usize,

    /// Global positions of the appended events
    pub global_positions: Vec<GlobalPosition>,

    /// When the append operation completed
    pub appended_at: chrono::DateTime<chrono::Utc>,
}
