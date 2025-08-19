use crate::{DlqStats, EventId, FailedEvent, FailureReason, PubSubResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Dead Letter Queue management port - handles failed events
#[async_trait]
pub trait DeadLetterQueueManager: Send + Sync {
    /// Send a failed event to the Dead Letter Queue
    ///
    /// # Arguments
    /// * `failed_event` - The event that failed processing
    /// * `reason` - The reason for the failure
    ///
    /// # Returns
    /// Result of the DLQ operation
    async fn send_to_dlq(
        &self,
        failed_event: FailedEvent,
        reason: FailureReason,
    ) -> PubSubResult<()>;

    /// Retrieve events from the Dead Letter Queue for a specific topic
    ///
    /// # Arguments
    /// * `topic` - The topic to get DLQ events from
    /// * `limit` - Maximum number of events to retrieve (None for all)
    ///
    /// # Returns
    /// Vector of failed events in the DLQ
    async fn get_dlq_events(
        &self,
        topic: &str,
        limit: Option<usize>,
    ) -> PubSubResult<Vec<FailedEvent>>;

    /// Retry a specific event from the Dead Letter Queue
    ///
    /// # Arguments
    /// * `event_id` - The ID of the failed event to retry
    ///
    /// # Returns
    /// Result of the retry operation
    async fn retry_from_dlq(&self, event_id: EventId) -> PubSubResult<()>;

    /// Retry multiple events from the Dead Letter Queue in batch
    ///
    /// # Arguments
    /// * `event_ids` - Vector of failed event IDs to retry
    ///
    /// # Returns
    /// Vector of retry results for each event
    async fn retry_dlq_batch(&self, event_ids: Vec<EventId>) -> PubSubResult<Vec<DlqRetryResult>>;

    /// Purge old events from the Dead Letter Queue
    ///
    /// # Arguments
    /// * `topic` - The topic to purge DLQ events from
    /// * `before` - Only purge events older than this timestamp (None for all)
    ///
    /// # Returns
    /// Number of events purged
    async fn purge_dlq(&self, topic: &str, before: Option<DateTime<Utc>>) -> PubSubResult<usize>;

    /// Get statistics about the Dead Letter Queue for a topic
    ///
    /// # Arguments
    /// * `topic` - The topic to get DLQ statistics for
    ///
    /// # Returns
    /// DLQ statistics including count, oldest event, etc.
    async fn dlq_stats(&self, topic: &str) -> PubSubResult<DlqStats>;

    /// List all topics that have events in the Dead Letter Queue
    ///
    /// # Returns
    /// Vector of topic names with DLQ events
    async fn list_dlq_topics(&self) -> PubSubResult<Vec<String>>;

    /// Move an event permanently out of the DLQ without retrying
    /// Useful for events that should never be processed again
    ///
    /// # Arguments
    /// * `event_id` - The ID of the failed event to discard
    ///
    /// # Returns
    /// Result of the discard operation
    async fn discard_dlq_event(&self, event_id: EventId) -> PubSubResult<()>;

    /// Get detailed information about a specific failed event
    ///
    /// # Arguments
    /// * `event_id` - The ID of the failed event
    ///
    /// # Returns
    /// Detailed information about the failed event
    async fn get_dlq_event_details(&self, event_id: EventId) -> PubSubResult<Option<FailedEvent>>;

    /// Export DLQ events for external analysis or archiving
    ///
    /// # Arguments
    /// * `topic` - The topic to export DLQ events from
    /// * `format` - The export format (JSON, CSV, etc.)
    ///
    /// # Returns
    /// Serialized DLQ events in the requested format
    async fn export_dlq_events(&self, topic: &str, format: ExportFormat) -> PubSubResult<Vec<u8>>;
}

/// Result of a retry operation from DLQ
#[derive(Debug, Clone)]
pub struct DlqRetryResult {
    pub event_id: EventId,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Format for exporting DLQ events
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
    Xml,
    Custom(String),
}
