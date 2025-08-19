use crate::{EventHandler, PubSubResult, SubscriptionId, SubscriptionOptions};
use async_trait::async_trait;
use serde::de::DeserializeOwned;

/// Event subscription port - defines the contract for subscribing to events from topics
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Subscribe to a topic with a basic event handler
    ///
    /// # Arguments
    /// * `topic` - The topic name to subscribe to
    /// * `handler` - The event handler to process events
    ///
    /// # Returns
    /// Subscription ID for managing the subscription
    async fn subscribe<T>(
        &self,
        topic: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: DeserializeOwned + Send + Sync + 'static;

    /// Subscribe to a topic with advanced options
    ///
    /// # Arguments
    /// * `topic` - The topic name to subscribe to
    /// * `options` - Subscription configuration options
    /// * `handler` - The event handler to process events
    async fn subscribe_with_options<T>(
        &self,
        topic: &str,
        options: SubscriptionOptions,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: DeserializeOwned + Send + Sync + 'static;

    /// Subscribe as part of a consumer group for load balancing
    /// Events are distributed among all subscribers in the same group
    ///
    /// # Arguments
    /// * `topic` - The topic name to subscribe to
    /// * `consumer_group` - Name of the consumer group
    /// * `handler` - The event handler to process events
    async fn subscribe_consumer_group<T>(
        &self,
        topic: &str,
        consumer_group: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: DeserializeOwned + Send + Sync + 'static;

    /// Subscribe to multiple topics with the same handler
    /// Useful for related event types that need the same processing
    ///
    /// # Arguments
    /// * `topics` - Vector of topic names to subscribe to
    /// * `handler` - The event handler to process events from all topics
    async fn subscribe_multiple<T>(
        &self,
        topics: Vec<String>,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<Vec<SubscriptionId>>
    where
        T: DeserializeOwned + Send + Sync + 'static;

    /// Unsubscribe from a topic
    ///
    /// # Arguments
    /// * `subscription_id` - The subscription ID to cancel
    async fn unsubscribe(&self, subscription_id: SubscriptionId) -> PubSubResult<()>;

    /// Pause a subscription temporarily
    /// Events will be buffered but not processed until resumed
    ///
    /// # Arguments
    /// * `subscription_id` - The subscription ID to pause
    async fn pause_subscription(&self, subscription_id: SubscriptionId) -> PubSubResult<()>;

    /// Resume a paused subscription
    ///
    /// # Arguments
    /// * `subscription_id` - The subscription ID to resume
    async fn resume_subscription(&self, subscription_id: SubscriptionId) -> PubSubResult<()>;

    /// Get current subscription status and metrics
    ///
    /// # Arguments
    /// * `subscription_id` - The subscription ID to check
    async fn subscription_status(
        &self,
        subscription_id: SubscriptionId,
    ) -> PubSubResult<crate::SubscriptionStatus>;
}
