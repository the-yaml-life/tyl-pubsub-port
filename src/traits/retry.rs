use crate::{EventId, FailedEvent, PubSubResult, RetryPolicy, RetryStats};
use async_trait::async_trait;

/// Retry management port - handles retry logic and policies
#[async_trait]
pub trait RetryManager: Send + Sync {
    /// Set the retry policy for a specific topic
    ///
    /// # Arguments
    /// * `topic` - The topic to set the retry policy for
    /// * `policy` - The retry policy configuration
    ///
    /// # Returns
    /// Result of setting the retry policy
    async fn set_retry_policy(&self, topic: &str, policy: RetryPolicy) -> PubSubResult<()>;

    /// Get the current retry policy for a topic
    ///
    /// # Arguments
    /// * `topic` - The topic to get the retry policy for
    ///
    /// # Returns
    /// The retry policy if set, None if using default
    async fn get_retry_policy(&self, topic: &str) -> PubSubResult<Option<RetryPolicy>>;

    /// Remove the custom retry policy for a topic (revert to default)
    ///
    /// # Arguments
    /// * `topic` - The topic to remove the custom retry policy from
    ///
    /// # Returns
    /// Result of removing the retry policy
    async fn remove_retry_policy(&self, topic: &str) -> PubSubResult<()>;

    /// Manually retry a specific failed event
    ///
    /// # Arguments
    /// * `event_id` - The ID of the failed event to retry
    ///
    /// # Returns
    /// Result of the retry operation
    async fn retry_failed_event(&self, event_id: EventId) -> PubSubResult<RetryResult>;

    /// Retry multiple failed events in batch
    ///
    /// # Arguments
    /// * `event_ids` - Vector of failed event IDs to retry
    ///
    /// # Returns
    /// Vector of retry results for each event
    async fn retry_failed_events_batch(
        &self,
        event_ids: Vec<EventId>,
    ) -> PubSubResult<Vec<RetryResult>>;

    /// Get retry statistics for a topic
    ///
    /// # Arguments
    /// * `topic` - The topic to get retry statistics for
    ///
    /// # Returns
    /// Retry statistics including success rate, average attempts, etc.
    async fn get_retry_stats(&self, topic: &str) -> PubSubResult<RetryStats>;

    /// Get currently failing events for a topic
    /// These are events that are in the retry queue but haven't exhausted all attempts
    ///
    /// # Arguments
    /// * `topic` - The topic to get failing events for
    /// * `limit` - Maximum number of events to retrieve (None for all)
    ///
    /// # Returns
    /// Vector of events currently being retried
    async fn get_failing_events(
        &self,
        topic: &str,
        limit: Option<usize>,
    ) -> PubSubResult<Vec<FailedEvent>>;

    /// Get global retry statistics across all topics
    ///
    /// # Returns
    /// Global retry statistics
    async fn get_global_retry_stats(&self) -> PubSubResult<GlobalRetryStats>;

    /// Reset retry count for a specific event
    /// Allows giving an event fresh retry attempts
    ///
    /// # Arguments
    /// * `event_id` - The ID of the event to reset retry count for
    ///
    /// # Returns
    /// Result of resetting the retry count
    async fn reset_retry_count(&self, event_id: EventId) -> PubSubResult<()>;

    /// Set a global default retry policy
    /// This policy is used for topics that don't have a specific policy set
    ///
    /// # Arguments
    /// * `policy` - The default retry policy
    ///
    /// # Returns
    /// Result of setting the default retry policy
    async fn set_default_retry_policy(&self, policy: RetryPolicy) -> PubSubResult<()>;

    /// Get the current global default retry policy
    ///
    /// # Returns
    /// The default retry policy
    async fn get_default_retry_policy(&self) -> PubSubResult<RetryPolicy>;

    /// Pause retry processing for a topic temporarily
    /// Failed events will accumulate but won't be retried until resumed
    ///
    /// # Arguments
    /// * `topic` - The topic to pause retry processing for
    ///
    /// # Returns
    /// Result of pausing retry processing
    async fn pause_retry_processing(&self, topic: &str) -> PubSubResult<()>;

    /// Resume retry processing for a topic
    ///
    /// # Arguments
    /// * `topic` - The topic to resume retry processing for
    ///
    /// # Returns
    /// Result of resuming retry processing
    async fn resume_retry_processing(&self, topic: &str) -> PubSubResult<()>;
}

/// Result of a retry operation
#[derive(Debug, Clone)]
pub struct RetryResult {
    pub event_id: EventId,
    pub success: bool,
    pub attempt_number: u32,
    pub error_message: Option<String>,
    pub next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Global retry statistics across all topics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlobalRetryStats {
    pub total_retries_attempted: u64,
    pub total_retries_succeeded: u64,
    pub total_events_sent_to_dlq: u64,
    pub average_retry_attempts: f64,
    pub most_retried_topics: Vec<String>,
    pub retry_success_rate: f64,
}
