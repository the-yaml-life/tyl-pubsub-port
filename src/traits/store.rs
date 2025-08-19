use crate::{EventStream, GlobalPosition, PubSubResult, Snapshot, StoredEvent};
use async_trait::async_trait;

/// Event store port - handles event sourcing and stream management
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append events to a specific stream
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream
    /// * `events` - Vector of events to append to the stream
    ///
    /// # Returns
    /// The new version number of the stream after appending
    async fn append_events(&self, stream_id: &str, events: Vec<StoredEvent>) -> PubSubResult<u64>;

    /// Read events from a specific stream starting from a given version
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream
    /// * `from_version` - The version to start reading from (0 for beginning)
    /// * `limit` - Maximum number of events to read (None for all)
    ///
    /// # Returns
    /// Event stream containing the requested events
    async fn read_events(
        &self,
        stream_id: &str,
        from_version: u64,
        limit: Option<usize>,
    ) -> PubSubResult<EventStream>;

    /// Read all events across all streams from a global position
    /// Useful for building read models or projections
    ///
    /// # Arguments
    /// * `from_position` - The global position to start reading from
    /// * `limit` - Maximum number of events to read (None for all)
    ///
    /// # Returns
    /// Event stream containing events from all streams
    async fn read_all_events(
        &self,
        from_position: GlobalPosition,
        limit: Option<usize>,
    ) -> PubSubResult<EventStream>;

    /// Read events from a specific stream within a version range
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream
    /// * `from_version` - The starting version (inclusive)
    /// * `to_version` - The ending version (inclusive, None for latest)
    ///
    /// # Returns
    /// Event stream containing events in the specified range
    async fn read_events_range(
        &self,
        stream_id: &str,
        from_version: u64,
        to_version: Option<u64>,
    ) -> PubSubResult<EventStream>;

    /// Save a snapshot for a specific stream at a given version
    /// Snapshots allow for faster reconstruction of aggregate state
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream
    /// * `version` - The version at which the snapshot was taken
    /// * `snapshot` - The snapshot data
    ///
    /// # Returns
    /// Result of saving the snapshot
    async fn save_snapshot(
        &self,
        stream_id: &str,
        version: u64,
        snapshot: Snapshot,
    ) -> PubSubResult<()>;

    /// Load the latest snapshot for a specific stream
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream
    ///
    /// # Returns
    /// The latest snapshot if available
    async fn load_snapshot(&self, stream_id: &str) -> PubSubResult<Option<Snapshot>>;

    /// Load a snapshot at or before a specific version
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream
    /// * `max_version` - The maximum version to consider for snapshot
    ///
    /// # Returns
    /// The latest snapshot at or before the specified version
    async fn load_snapshot_at_version(
        &self,
        stream_id: &str,
        max_version: u64,
    ) -> PubSubResult<Option<Snapshot>>;

    /// Replay events from a stream for rebuilding state
    /// This combines snapshot loading with event replay for efficiency
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream
    /// * `from_version` - The version to start replay from
    /// * `to_version` - The version to end replay at (None for latest)
    ///
    /// # Returns
    /// Event stream for replay operations
    async fn replay_events(
        &self,
        stream_id: &str,
        from_version: u64,
        to_version: Option<u64>,
    ) -> PubSubResult<EventStream>;

    /// Get metadata about a specific stream
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream
    ///
    /// # Returns
    /// Stream metadata including version, event count, etc.
    async fn get_stream_metadata(&self, stream_id: &str) -> PubSubResult<Option<StreamMetadata>>;

    /// List all stream IDs in the event store
    ///
    /// # Arguments
    /// * `prefix` - Optional prefix to filter streams (None for all)
    /// * `limit` - Maximum number of streams to return (None for all)
    ///
    /// # Returns
    /// Vector of stream IDs
    async fn list_streams(
        &self,
        prefix: Option<&str>,
        limit: Option<usize>,
    ) -> PubSubResult<Vec<String>>;

    /// Delete a stream and all its events
    /// WARNING: This is irreversible and should be used with caution
    ///
    /// # Arguments
    /// * `stream_id` - The unique identifier for the event stream to delete
    ///
    /// # Returns
    /// Result of the deletion operation
    async fn delete_stream(&self, stream_id: &str) -> PubSubResult<()>;

    /// Create a subscription to new events across all streams
    /// This enables real-time processing of events as they are appended
    ///
    /// # Arguments
    /// * `from_position` - The global position to start the subscription from
    /// * `handler` - Callback function to handle new events
    ///
    /// # Returns
    /// Subscription ID for managing the live subscription
    async fn subscribe_to_all(
        &self,
        from_position: GlobalPosition,
        handler: Box<dyn EventStreamHandler>,
    ) -> PubSubResult<crate::SubscriptionId>;

    /// Create a subscription to new events in a specific stream
    ///
    /// # Arguments
    /// * `stream_id` - The stream to subscribe to
    /// * `from_version` - The version to start the subscription from
    /// * `handler` - Callback function to handle new events
    ///
    /// # Returns
    /// Subscription ID for managing the stream subscription
    async fn subscribe_to_stream(
        &self,
        stream_id: &str,
        from_version: u64,
        handler: Box<dyn EventStreamHandler>,
    ) -> PubSubResult<crate::SubscriptionId>;
}

/// Metadata about an event stream
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamMetadata {
    pub stream_id: String,
    pub current_version: u64,
    pub event_count: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_snapshot_version: Option<u64>,
}

/// Handler for event stream subscriptions
#[async_trait]
pub trait EventStreamHandler: Send + Sync {
    /// Handle a new event from the stream subscription
    ///
    /// # Arguments
    /// * `event` - The new event from the stream
    ///
    /// # Returns
    /// Result of handling the event
    async fn handle_stream_event(&self, event: StoredEvent) -> PubSubResult<()>;

    /// Handle errors that occur during stream processing
    ///
    /// # Arguments
    /// * `error` - The error that occurred
    ///
    /// # Returns
    /// Whether to continue the subscription or stop
    async fn handle_stream_error(&self, _error: crate::TylError) -> bool {
        false // Default: stop on error
    }
}
