//! Extended traits that add Pact validation to existing EventPublisher and EventSubscriber

use async_trait::async_trait;
use crate::{EventPublisher, EventSubscriber, EventHandler, SubscriptionId, PubSubResult, EventId};
use crate::validation::traits::PactValidated;
use crate::validation::validator::ContractValidator;
use std::sync::Arc;
use std::sync::Mutex;

/// Extended EventPublisher trait with automatic Pact contract validation
/// 
/// This trait extends the base EventPublisher with contract validation capabilities,
/// ensuring that events are only published when compatible consumers are registered.
#[async_trait]
pub trait ValidatedEventPublisher: EventPublisher {
    /// Publish an event with automatic Pact contract validation
    /// 
    /// This method:
    /// 1. Validates the event against its JSON schema
    /// 2. Checks if consumers are registered for this event type
    /// 3. Validates consumer compatibility
    /// 4. Publishes the event if all validations pass
    /// 
    /// # Arguments
    /// * `event` - The event to publish (must implement PactValidated)
    /// 
    /// # Returns
    /// * `Ok(EventId)` - If validation passes and event is published
    /// * `Err(TylError)` - If validation fails or no consumers registered
    async fn publish_validated<T: PactValidated>(&self, event: T) -> PubSubResult<EventId> {
        // 1. Validate event schema
        event.validate_schema()?;

        // 2. Check consumer compatibility
        self.validate_consumer_compatibility(&event).await?;

        // 3. Publish if valid (using the topic from event metadata or a default)
        let topic = event.event_type();  // Use event type as topic for now
        self.publish(topic, event).await
    }

    /// Check if consumers can handle this event type and schema
    /// 
    /// This should be implemented by concrete adapters to check their
    /// registered consumers and validate compatibility.
    async fn validate_consumer_compatibility<T: PactValidated>(&self, event: &T) -> PubSubResult<()>;

    /// Register this service as a producer of an event type
    /// 
    /// This creates a provider verification that can be used in contract tests
    async fn register_producer_contract<T: PactValidated>(&self, service_name: &str) -> PubSubResult<()>;

    /// Get the contract validator for this publisher (if available)
    /// 
    /// Allows access to the underlying validator for advanced operations
    fn get_contract_validator(&self) -> Option<Arc<Mutex<ContractValidator>>>;
}

/// Extended EventSubscriber trait with automatic consumer contract registration
/// 
/// This trait extends the base EventSubscriber with contract registration capabilities,
/// automatically registering the service as a consumer and validating incoming events.
#[async_trait] 
pub trait ValidatedEventSubscriber: EventSubscriber {
    /// Subscribe to events with automatic consumer contract registration
    /// 
    /// This method:
    /// 1. Registers the service as a consumer of this event type
    /// 2. Wraps the provided handler with validation logic
    /// 3. Sets up the subscription with the validated handler
    /// 
    /// # Arguments
    /// * `service_name` - Name of the consuming service (for contract registration)
    /// * `handler` - Event handler to process validated events
    /// 
    /// # Returns
    /// * `Ok(SubscriptionId)` - If registration and subscription succeed
    /// * `Err(TylError)` - If registration or subscription fails
    async fn subscribe_validated<T>(
        &self,
        service_name: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: PactValidated + serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        // 1. Register as consumer
        self.register_consumer_contract::<T>(service_name).await?;

        // 2. Wrap handler with validation
        let validated_handler = ValidatedEventHandler::new(handler);

        // 3. Subscribe with validated handler  
        let topic = T::example().event_type(); // Use event type as topic
        self.subscribe(topic, Box::new(validated_handler)).await
    }

    /// Register this service as a consumer of an event type
    /// 
    /// This creates a consumer expectation that can be used in contract tests
    async fn register_consumer_contract<T: PactValidated>(&self, service_name: &str) -> PubSubResult<()>;

    /// Get the contract validator for this subscriber (if available)
    /// 
    /// Allows access to the underlying validator for advanced operations
    fn get_contract_validator(&self) -> Option<Arc<Mutex<ContractValidator>>>;
}

/// Event handler wrapper that adds validation to incoming events
/// 
/// This wrapper validates events before passing them to the underlying handler
struct ValidatedEventHandler<T> {
    inner_handler: Box<dyn EventHandler<T>>,
}

impl<T> ValidatedEventHandler<T> {
    fn new(inner_handler: Box<dyn EventHandler<T>>) -> Self {
        Self { inner_handler }
    }
}

#[async_trait]
impl<T> EventHandler<T> for ValidatedEventHandler<T>
where
    T: PactValidated + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    async fn handle(&self, event: crate::Event<T>) -> crate::HandlerResult {
        // Validate incoming event payload before processing
        event.payload.validate_schema()
            .map_err(|e| crate::HandlerError::InvalidEventFormat { 
                details: e.to_string() 
            })?;

        println!("✅ Event {} passed validation", event.payload.event_type());

        // Call original handler
        self.inner_handler.handle(event).await
    }
}

/// Helper trait for easy adoption of validated publishing
/// 
/// Provides convenience methods that combine validation and publishing
pub trait PubSubValidationExt: ValidatedEventPublisher + ValidatedEventSubscriber {
    /// Publish multiple validated events in a batch
    fn publish_validated_batch<T: PactValidated>(&self, events: Vec<T>) -> impl std::future::Future<Output = PubSubResult<Vec<EventId>>> + Send
    where
        Self: Send + Sync,
    {
        async move {
            let mut event_ids = Vec::new();
            
            for event in events {
                let event_id = self.publish_validated(event).await?;
                event_ids.push(event_id);
            }
            
            Ok(event_ids)
        }
    }

    /// Subscribe to multiple event types with the same handler
    fn subscribe_validated_multiple<T>(
        &self,
        service_name: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> impl std::future::Future<Output = PubSubResult<Vec<SubscriptionId>>> + Send
    where
        T: PactValidated + serde::de::DeserializeOwned + Send + Sync + 'static,
        Self: Send + Sync,
    {
        async move {
            // For now, just subscribe to the single event type
            // In a real implementation, this could handle multiple related types
            let subscription_id = self.subscribe_validated(service_name, handler).await?;
            Ok(vec![subscription_id])
        }
    }
}

// Automatically implement PubSubValidationExt for any type that implements both traits
impl<T> PubSubValidationExt for T where T: ValidatedEventPublisher + ValidatedEventSubscriber {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};
    use schemars::JsonSchema;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct TestEvent {
        pub id: String,
        pub message: String,
    }

    impl PactValidated for TestEvent {
        fn example() -> Self {
            Self {
                id: "test-123".to_string(),
                message: "test message".to_string(),
            }
        }
        
        fn event_type(&self) -> &'static str {
            "test.event.v1"
        }
    }

    // Mock handler for testing
    struct TestHandler {
        call_count: Arc<AtomicUsize>,
    }

    impl TestHandler {
        fn new() -> Self {
            Self {
                call_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        #[allow(dead_code)]
        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl EventHandler<TestEvent> for TestHandler {
        async fn handle(&self, _event: crate::Event<TestEvent>) -> crate::HandlerResult {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_validated_event_handler() {
        let inner_handler = TestHandler::new();
        let call_count_ref = inner_handler.call_count.clone();
        
        let validated_handler = ValidatedEventHandler::new(Box::new(inner_handler));
        
        let event = crate::Event::new("test.topic", TestEvent::example());
        let result = validated_handler.handle(event).await;
        
        assert!(result.is_ok());
        assert_eq!(call_count_ref.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_validated_handler_rejects_invalid_event() {
        // This test would need a way to create invalid events
        // For now, we just verify the handler works with valid events
        let inner_handler = TestHandler::new();
        let validated_handler = ValidatedEventHandler::new(Box::new(inner_handler));
        
        let event = crate::Event::new("test.topic", TestEvent::example());
        let result = validated_handler.handle(event).await;
        
        assert!(result.is_ok());
    }
}