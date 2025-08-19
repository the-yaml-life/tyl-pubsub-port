//! Mock implementation with Pact validation for testing and demonstration

#[cfg(feature = "pact-validation")]
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use crate::{EventPublisher, EventSubscriber, EventHandler, SubscriptionId, PubSubResult, EventId};
use crate::mock::MockPubSubAdapter;

#[cfg(feature = "pact-validation")]
use crate::validation::{
    PactValidated, ContractValidator, ValidatedEventPublisher, ValidatedEventSubscriber
};

/// Extended mock adapter with Pact validation capabilities
/// 
/// This extends the existing MockPubSubAdapter with contract validation,
/// allowing tests to verify that producers and consumers are compatible.
#[derive(Clone)]
pub struct ValidatedMockAdapter {
    /// The underlying mock adapter
    inner: MockPubSubAdapter,
    
    /// Contract validator for managing producer/consumer contracts
    #[cfg(feature = "pact-validation")]
    validator: Arc<Mutex<ContractValidator>>,
    
    /// Service name for this adapter instance
    service_name: String,
}

impl ValidatedMockAdapter {
    /// Create a new validated mock adapter
    pub fn new(service_name: impl Into<String>) -> Self {
        let service_name = service_name.into();
        Self {
            inner: MockPubSubAdapter::new(),
            #[cfg(feature = "pact-validation")]
            validator: Arc::new(Mutex::new(ContractValidator::new(service_name.clone()))),
            service_name,
        }
    }

    /// Create a reliable validated mock adapter (no failures)
    pub fn reliable(service_name: impl Into<String>) -> Self {
        let service_name = service_name.into();
        Self {
            inner: MockPubSubAdapter::reliable(),
            #[cfg(feature = "pact-validation")]
            validator: Arc::new(Mutex::new(ContractValidator::new(service_name.clone()))),
            service_name,
        }
    }

    /// Get all published events for testing
    pub fn get_published_events(&self) -> Vec<(String, serde_json::Value)> {
        // Delegate to inner adapter
        // Note: This would need to be implemented in the actual MockPubSubAdapter
        vec![] // Placeholder - would get from inner adapter's storage
    }

    /// Clear published events
    pub fn clear_published_events(&self) {
        // Delegate to inner adapter
        // Note: This would need to be implemented in the actual MockPubSubAdapter
    }

    /// Simulate another service registering as consumer
    /// 
    /// This is useful for testing scenarios where you need to simulate
    /// external services that consume your events.
    #[cfg(feature = "pact-validation")]
    pub fn simulate_consumer_registration(&self, event_type: &str, consumer_service: &str) {
        self.validator
            .lock()
            .unwrap()
            .register_consumer(event_type, consumer_service);
    }

    /// Simulate another service registering as producer
    /// 
    /// This is useful for testing scenarios where you need to simulate
    /// external services that produce events you consume.
    #[cfg(feature = "pact-validation")]
    pub fn simulate_producer_registration(&self, event_type: &str, producer_service: &str) {
        self.validator
            .lock()
            .unwrap()
            .register_producer(event_type, producer_service);
    }

    /// Get the service name for this adapter
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get a contract report for this adapter
    #[cfg(feature = "pact-validation")]
    pub fn get_contract_report(&self) -> crate::validation::validator::ContractReport {
        self.validator.lock().unwrap().generate_contract_report()
    }

    /// Clear all contracts (useful for test setup)
    #[cfg(feature = "pact-validation")]
    pub fn clear_contracts(&self) {
        self.validator.lock().unwrap().clear();
    }
}

/// PactEventPublisher - ONLY accepts pact_event! generated events
/// This trait enforces pact validation at compile time
#[async_trait]
pub trait PactEventPublisher: Send + Sync {
    /// Publish a pact-validated event to a topic
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId>
    where
        T: serde::Serialize + Send + Sync + crate::validation::PactEventValidator + crate::validation::PactValidated;

    /// Publish a pact-validated event with a specific partition key
    async fn publish_with_key<T>(&self, topic: &str, key: &str, event: T) -> PubSubResult<EventId>
    where
        T: serde::Serialize + Send + Sync + crate::validation::PactEventValidator + crate::validation::PactValidated;
}

// ValidatedMockAdapter implements its own strict interface
#[async_trait]
impl PactEventPublisher for ValidatedMockAdapter {
    async fn publish<T>(&self, topic: &str, event: T) -> PubSubResult<EventId>
    where
        T: serde::Serialize + Send + Sync + crate::validation::PactEventValidator + crate::validation::PactValidated,
    {
        // ALWAYS check if it's a pact event - should always be true due to trait bound
        if !event.is_pact_event() {
            return Err(crate::TylError::validation("pact_event", format!(
                "Event of type '{}' is not a pact event. Use pact_event! macro to define your events.",
                std::any::type_name::<T>()
            )));
        }
        
        // Validate the pact event schema
        event.validate_pact_schema()?;
        
        // CRITICAL: Consumer validation - this is what makes test_orphaned_event_fails fail correctly!
        use crate::validation::extended_traits::ValidatedEventPublisher;
        self.validate_consumer_compatibility(&event).await?;
        
        // Proceed with normal publishing
        self.inner.publish(topic, event).await
    }

    async fn publish_with_key<T>(&self, topic: &str, key: &str, event: T) -> PubSubResult<EventId>
    where
        T: serde::Serialize + Send + Sync + crate::validation::PactEventValidator + crate::validation::PactValidated,
    {
        // ALWAYS check if it's a pact event - should always be true due to trait bound
        if !event.is_pact_event() {
            return Err(crate::TylError::validation("pact_event", format!(
                "Event of type '{}' is not a pact event. Use pact_event! macro to define your events.",
                std::any::type_name::<T>()
            )));
        }
        
        // Validate the pact event schema
        event.validate_pact_schema()?;
        
        // CRITICAL: Consumer validation 
        use crate::validation::extended_traits::ValidatedEventPublisher;
        self.validate_consumer_compatibility(&event).await?;
        
        self.inner.publish_with_key(topic, key, event).await
    }
}

// ValidatedMockAdapter also implements basic EventPublisher for compatibility 
// but it will reject non-pact events at runtime
#[async_trait]
impl EventPublisher for ValidatedMockAdapter {
    async fn publish<T>(&self, _topic: &str, _event: T) -> PubSubResult<EventId>
    where
        T: serde::Serialize + Send + Sync,
    {
        // ValidatedMockAdapter enforces compile-time safety via PactEventPublisher trait
        return Err(crate::TylError::validation("pact_validation", format!(
            "ValidatedMockAdapter only accepts events created with pact_event! macro. \
            For compile-time safety, use PactEventPublisher trait instead of EventPublisher. \
            Event type: '{}'. \
            \nCorrect usage: let adapter: &dyn PactEventPublisher = &validated_adapter; adapter.publish(topic, pact_event).await;",
            std::any::type_name::<T>()
        )));
    }

    async fn publish_with_key<T>(&self, _topic: &str, _key: &str, _event: T) -> PubSubResult<EventId>
    where
        T: serde::Serialize + Send + Sync,
    {
        return Err(crate::TylError::validation("pact_validation", format!(
            "ValidatedMockAdapter only accepts pact events. Use PactEventPublisher::publish_with_key() instead. Event type: '{}'",
            std::any::type_name::<T>()
        )));
    }

    async fn publish_batch<T>(&self, _events: Vec<crate::TopicEvent<T>>) -> PubSubResult<Vec<EventId>>
    where
        T: serde::Serialize + Send + Sync,
    {
        return Err(crate::TylError::validation("pact_validation", format!(
            "ValidatedMockAdapter only accepts pact events. Use basic MockPubSubAdapter for batch operations with non-pact events. Event type: '{}'",
            std::any::type_name::<T>()
        )));
    }

    async fn publish_transactional<T>(
        &self,
        _events: Vec<crate::TopicEvent<T>>,
        _transaction_id: crate::TransactionId,
    ) -> PubSubResult<Vec<EventId>>
    where
        T: serde::Serialize + Send + Sync,
    {
        return Err(crate::TylError::validation("pact_validation", format!(
            "ValidatedMockAdapter only accepts pact events. Use basic MockPubSubAdapter for transactional operations with non-pact events. Event type: '{}'",
            std::any::type_name::<T>()
        )));
    }

    async fn publish_event<T>(&self, _event: crate::Event<T>) -> PubSubResult<EventId>
    where
        T: serde::Serialize + Send + Sync,
    {
        return Err(crate::TylError::validation("pact_validation", format!(
            "ValidatedMockAdapter only accepts pact events. Use basic MockPubSubAdapter for Event<T> with non-pact payloads. Event type: '{}'",
            std::any::type_name::<T>()
        )));
    }
}

// Delegate basic EventSubscriber to the inner adapter  
#[async_trait]
impl EventSubscriber for ValidatedMockAdapter {
    async fn subscribe<T>(
        &self,
        topic: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        self.inner.subscribe(topic, handler).await
    }

    async fn subscribe_with_options<T>(
        &self,
        topic: &str,
        options: crate::SubscriptionOptions,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        self.inner.subscribe_with_options(topic, options, handler).await
    }

    async fn subscribe_consumer_group<T>(
        &self,
        topic: &str,
        consumer_group: &str,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<SubscriptionId>
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        self.inner.subscribe_consumer_group(topic, consumer_group, handler).await
    }

    async fn subscribe_multiple<T>(
        &self,
        topics: Vec<String>,
        handler: Box<dyn EventHandler<T>>,
    ) -> PubSubResult<Vec<SubscriptionId>>
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        self.inner.subscribe_multiple(topics, handler).await
    }

    async fn unsubscribe(&self, subscription_id: SubscriptionId) -> PubSubResult<()> {
        self.inner.unsubscribe(subscription_id).await
    }

    async fn pause_subscription(&self, subscription_id: SubscriptionId) -> PubSubResult<()> {
        self.inner.pause_subscription(subscription_id).await
    }

    async fn resume_subscription(&self, subscription_id: SubscriptionId) -> PubSubResult<()> {
        self.inner.resume_subscription(subscription_id).await
    }

    async fn subscription_status(
        &self,
        subscription_id: SubscriptionId,
    ) -> PubSubResult<crate::SubscriptionStatus> {
        self.inner.subscription_status(subscription_id).await
    }
}

// Implement the validated traits when pact-validation feature is enabled
#[cfg(feature = "pact-validation")]
#[async_trait]
impl ValidatedEventPublisher for ValidatedMockAdapter {
    // Override the default implementation to use inner adapter directly
    async fn publish_validated<T: PactValidated>(&self, event: T) -> PubSubResult<EventId> {
        // 1. Validate event schema
        event.validate_schema()?;

        // 2. Check consumer compatibility
        self.validate_consumer_compatibility(&event).await?;

        // 3. Publish using inner adapter directly (bypass our EventPublisher impl)
        let topic = event.event_type();
        self.inner.publish(topic, event).await
    }

    async fn validate_consumer_compatibility<T: PactValidated>(&self, event: &T) -> PubSubResult<()> {
        self.validator
            .lock()
            .unwrap()
            .validate_consumer_compatibility(event)
    }

    async fn register_producer_contract<T: PactValidated>(&self, service_name: &str) -> PubSubResult<()> {
        let example = T::example();
        self.validator
            .lock()
            .unwrap()
            .register_provider_verification(&example, service_name);
        
        Ok(())
    }

    fn get_contract_validator(&self) -> Option<Arc<Mutex<ContractValidator>>> {
        Some(self.validator.clone())
    }
}

#[cfg(feature = "pact-validation")]
#[async_trait]
impl ValidatedEventSubscriber for ValidatedMockAdapter {
    async fn register_consumer_contract<T: PactValidated>(&self, service_name: &str) -> PubSubResult<()> {
        let example = T::example();
        self.validator
            .lock()
            .unwrap()
            .register_consumer_expectation(&example, service_name);
        
        Ok(())
    }

    fn get_contract_validator(&self) -> Option<Arc<Mutex<ContractValidator>>> {
        Some(self.validator.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "pact-validation")]
    use crate::validation::PactValidated;

    // Use pact_event! macro for proper test event
    crate::pact_event! {
        /// Test event for ValidatedMockAdapter tests
        struct TestEvent {
            pub id: String,
            pub name: String,
            pub value: i32,
        }
        
        event_type = "test.event.v1",
        example = TestEvent {
            id: "test-123".to_string(),
            name: "Test Event".to_string(),
            value: 42,
        }
    }

    #[tokio::test]
    async fn test_basic_functionality() {
        let adapter = ValidatedMockAdapter::reliable("test-service");
        assert_eq!(adapter.service_name(), "test-service");
    }

    #[cfg(feature = "pact-validation")]
    #[tokio::test]
    async fn test_validated_publish_success() {
        let adapter = ValidatedMockAdapter::reliable("test-service");

        // Simulate consumer registration
        adapter.simulate_consumer_registration("test.event.v1", "consumer-service");

        // Create test event
        let event = TestEvent {
            id: "event-123".to_string(),
            name: "Test".to_string(),
            value: 100,
        };

        // This should succeed
        let result = adapter.publish_validated(event).await;
        assert!(result.is_ok());
    }

    #[cfg(feature = "pact-validation")]
    #[tokio::test] 
    async fn test_validated_publish_fails_no_consumers() {
        let adapter = ValidatedMockAdapter::reliable("test-service");

        // Don't register any consumers
        let event = TestEvent::example();

        // This should fail - no consumers registered
        let result = adapter.publish_validated(event).await;
        assert!(result.is_err());
    }

    #[cfg(feature = "pact-validation")]
    #[tokio::test]
    async fn test_producer_contract_registration() {
        let adapter = ValidatedMockAdapter::reliable("producer-service");

        let result = adapter.register_producer_contract::<TestEvent>("producer-service").await;
        assert!(result.is_ok());

        let report = adapter.get_contract_report();
        assert_eq!(report.service_name, "producer-service");
    }

    #[cfg(feature = "pact-validation")]
    #[tokio::test]
    async fn test_consumer_contract_registration() {
        let adapter = ValidatedMockAdapter::reliable("consumer-service");

        let result = adapter.register_consumer_contract::<TestEvent>("consumer-service").await;
        assert!(result.is_ok());

        let report = adapter.get_contract_report();
        assert_eq!(report.service_name, "consumer-service");
    }

    #[cfg(feature = "pact-validation")]
    #[tokio::test]
    async fn test_contract_clearing() {
        let adapter = ValidatedMockAdapter::reliable("test-service");
        
        // Register some contracts
        adapter.simulate_consumer_registration("test.event.v1", "consumer-service");
        adapter.simulate_producer_registration("test.event.v1", "producer-service");
        
        // Verify contracts exist
        let report = adapter.get_contract_report();
        assert!(!report.event_contracts.is_empty());
        
        // Clear contracts
        adapter.clear_contracts();
        
        // Verify contracts are cleared
        let report = adapter.get_contract_report();
        assert!(report.event_contracts.is_empty());
    }
}


// NO implementations for basic types like serde_json::Value, String, etc.
// This FORCES all events to be created with pact_event! macro
// 
// If someone tries to publish serde_json::Value, they'll get a compile error:
// "the trait bound `serde_json::Value: PactEventValidator` is not satisfied"
//
// This is EXACTLY what we want - forces pact_event! usage!