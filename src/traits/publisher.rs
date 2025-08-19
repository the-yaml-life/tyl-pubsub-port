use crate::{Event, EventId, PubSubResult, TopicEvent, TransactionId};
use async_trait::async_trait;
use serde::Serialize;

/// Event publishing port - defines the contract for publishing events to topics
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a single event to a topic
    ///
    /// # Arguments
    /// * `topic` - The topic name to publish to
    /// * `event` - The event payload to publish
    ///
    /// # Returns
    /// The unique event ID for tracking
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync;

    /// Publish an event with a specific partition key for ordered processing
    ///
    /// # Arguments
    /// * `topic` - The topic name to publish to
    /// * `key` - The partition key for routing and ordering
    /// * `event` - The event payload to publish
    async fn publish_with_key<T>(&self, topic: &str, key: &str, event: T) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync;

    /// Publish multiple events in a single batch operation
    ///
    /// # Arguments
    /// * `events` - Vector of topic-event pairs to publish
    ///
    /// # Returns
    /// Vector of event IDs in the same order as input
    async fn publish_batch<T>(&self, events: Vec<TopicEvent<T>>) -> PubSubResult<Vec<EventId>>
    where
        T: Serialize + Send + Sync;

    /// Publish events within a transaction (future implementation)
    /// This ensures all events are published together or none at all
    ///
    /// # Arguments
    /// * `events` - Vector of events to publish transactionally
    /// * `transaction_id` - Transaction identifier for coordination
    async fn publish_transactional<T>(
        &self,
        events: Vec<TopicEvent<T>>,
        transaction_id: TransactionId,
    ) -> PubSubResult<Vec<EventId>>
    where
        T: Serialize + Send + Sync;

    /// Publish an event with complete metadata control
    /// Allows setting custom headers, correlation IDs, etc.
    ///
    /// # Arguments
    /// * `event` - Complete event with all metadata
    async fn publish_event<T>(&self, event: Event<T>) -> PubSubResult<EventId>
    where
        T: Serialize + Send + Sync;
}
