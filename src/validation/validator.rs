//! Contract validator for managing Pact contracts between producers and consumers

use std::collections::{HashMap, HashSet};
use crate::validation::traits::{PactValidated, PactMessageExpectation, PactProviderVerification};
use crate::validation::errors::{ValidationError, ValidationResult};
use crate::PubSubResult;

/// Contract validator for managing Pact contracts
/// 
/// This validator tracks producers and consumers for different event types
/// and validates compatibility between them using Pact.io patterns.
#[derive(Debug, Clone)]
pub struct ContractValidator {
    service_name: String,
    /// Maps event_type -> set of consumer services
    registered_consumers: HashMap<String, HashSet<String>>,
    /// Maps event_type -> producer service
    registered_producers: HashMap<String, String>,
    /// Maps event_type -> provider verification data
    provider_verifications: HashMap<String, PactProviderVerification>,
    /// Maps event_type -> consumer expectations
    consumer_expectations: HashMap<String, Vec<PactMessageExpectation>>,
}

impl ContractValidator {
    /// Create a new contract validator for the given service
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            registered_consumers: HashMap::new(),
            registered_producers: HashMap::new(),
            provider_verifications: HashMap::new(),
            consumer_expectations: HashMap::new(),
        }
    }

    /// Register a consumer for an event type
    /// 
    /// This records that a specific service consumes events of this type
    pub fn register_consumer(&mut self, event_type: &str, consumer_service: &str) {
        self.registered_consumers
            .entry(event_type.to_string())
            .or_default()
            .insert(consumer_service.to_string());
        
        println!("📋 Registered {consumer_service} as consumer of {event_type}");
    }

    /// Register a producer for an event type
    /// 
    /// This records that a specific service produces events of this type
    pub fn register_producer(&mut self, event_type: &str, producer_service: &str) {
        self.registered_producers
            .insert(event_type.to_string(), producer_service.to_string());
        
        println!("📋 Registered {producer_service} as producer of {event_type}");
    }

    /// Register consumer expectation for contract testing
    /// 
    /// Stores what consumers expect to receive from producers
    pub fn register_consumer_expectation<T: PactValidated>(&mut self, event: &T, consumer_service: &str) {
        let event_type = event.event_type().to_string();
        let expectation = event.generate_consumer_expectation();
        
        self.consumer_expectations
            .entry(event_type.clone())
            .or_default()
            .push(expectation);
            
        self.register_consumer(&event_type, consumer_service);
        
        println!("📋 Registered consumer expectation for {event_type} from {consumer_service}");
    }

    /// Register provider verification for contract testing
    /// 
    /// Stores what producers can provide for verification
    pub fn register_provider_verification<T: PactValidated>(&mut self, event: &T, producer_service: &str) {
        let event_type = event.event_type().to_string();
        let verification = event.generate_provider_verification();
        
        self.provider_verifications
            .insert(event_type.clone(), verification);
            
        self.register_producer(&event_type, producer_service);
        
        println!("📋 Registered provider verification for {event_type} from {producer_service}");
    }

    /// Check if consumers can handle this event schema
    /// 
    /// Returns an error if no consumers are registered or if there are compatibility issues
    pub fn validate_consumer_compatibility<T: PactValidated>(&self, event: &T) -> PubSubResult<()> {
        let event_type = event.event_type();

        // Check if there are registered consumers
        let consumers = self.registered_consumers.get(event_type)
            .ok_or_else(|| ValidationError::NoConsumersRegistered(event_type.to_string()))?;

        if consumers.is_empty() {
            return Err(ValidationError::NoConsumersRegistered(event_type.to_string()).into());
        }

        // Validate the event schema
        event.validate_schema()?;

        // In a real implementation, this would check against stored consumer expectations
        // For now, we validate that the schema is consistent
        if let Some(provider_verification) = self.provider_verifications.get(event_type) {
            self.validate_schema_compatibility(event, provider_verification)?;
        }

        println!("✅ Event {} validated against {} consumers: {:?}",
            event_type, consumers.len(), consumers);

        Ok(())
    }

    /// Validate schema compatibility between producer and consumer expectations
    fn validate_schema_compatibility<T: PactValidated>(
        &self, 
        event: &T, 
        provider_verification: &PactProviderVerification
    ) -> ValidationResult<()> {
        // Check if event type matches
        if event.event_type() != provider_verification.event_type {
            return Err(ValidationError::ContractIncompatible(
                format!("Event type mismatch: expected {}, got {}", 
                    provider_verification.event_type, event.event_type())
            ));
        }

        // Validate against the provider's schema
        let current_schema = T::get_json_schema();
        
        // In a real implementation, we would do sophisticated schema compatibility checking
        // For now, we'll do a basic structural comparison
        if !self.schemas_compatible(&current_schema, &provider_verification.schema) {
            return Err(ValidationError::ContractIncompatible(
                "Schema structures are incompatible".to_string()
            ));
        }

        Ok(())
    }

    /// Check if two schemas are compatible (simplified implementation)
    fn schemas_compatible(
        &self,
        current: &schemars::schema::RootSchema,
        stored: &schemars::schema::RootSchema,
    ) -> bool {
        // This is a simplified compatibility check
        // In practice, you'd want more sophisticated schema evolution rules
        
        // For now, we'll just check that the schemas have the same structure
        serde_json::to_value(current).unwrap_or_default() == 
        serde_json::to_value(stored).unwrap_or_default()
    }

    /// Get all registered consumers for an event type
    pub fn get_consumers(&self, event_type: &str) -> Option<&HashSet<String>> {
        self.registered_consumers.get(event_type)
    }

    /// Get the registered producer for an event type
    pub fn get_producer(&self, event_type: &str) -> Option<&String> {
        self.registered_producers.get(event_type)
    }

    /// Get consumer expectations for an event type
    pub fn get_consumer_expectations(&self, event_type: &str) -> Option<&Vec<PactMessageExpectation>> {
        self.consumer_expectations.get(event_type)
    }

    /// Get provider verification for an event type
    pub fn get_provider_verification(&self, event_type: &str) -> Option<&PactProviderVerification> {
        self.provider_verifications.get(event_type)
    }

    /// Generate a contract report for all registered event types
    pub fn generate_contract_report(&self) -> ContractReport {
        let mut event_contracts = HashMap::new();

        // Collect all unique event types
        let mut all_event_types = HashSet::new();
        all_event_types.extend(self.registered_consumers.keys());
        all_event_types.extend(self.registered_producers.keys());

        for event_type in all_event_types {
            let consumers = self.registered_consumers.get(event_type)
                .cloned()
                .unwrap_or_default();
            let producer = self.registered_producers.get(event_type).cloned();
            let has_expectations = self.consumer_expectations.contains_key(event_type);
            let has_verification = self.provider_verifications.contains_key(event_type);

            event_contracts.insert(event_type.clone(), EventContract {
                event_type: event_type.clone(),
                producer,
                consumers,
                has_consumer_expectations: has_expectations,
                has_provider_verification: has_verification,
            });
        }

        ContractReport {
            service_name: self.service_name.clone(),
            event_contracts,
            generated_at: chrono::Utc::now(),
        }
    }

    /// Clear all registered contracts (useful for testing)
    pub fn clear(&mut self) {
        self.registered_consumers.clear();
        self.registered_producers.clear();
        self.provider_verifications.clear();
        self.consumer_expectations.clear();
    }
}

/// Report of all contracts for a service
#[derive(Debug, Clone)]
pub struct ContractReport {
    pub service_name: String,
    pub event_contracts: HashMap<String, EventContract>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Contract information for a specific event type
#[derive(Debug, Clone)]
pub struct EventContract {
    pub event_type: String,
    pub producer: Option<String>,
    pub consumers: HashSet<String>,
    pub has_consumer_expectations: bool,
    pub has_provider_verification: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};
    use schemars::JsonSchema;

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    struct TestEvent {
        pub id: String,
        pub name: String,
        pub value: i32,
    }

    impl PactValidated for TestEvent {
        fn example() -> Self {
            Self {
                id: "test-123".to_string(),
                name: "Test Event".to_string(),
                value: 42,
            }
        }
        
        fn event_type(&self) -> &'static str {
            "test.event.v1"
        }
    }

    #[test]
    fn test_consumer_registration() {
        let mut validator = ContractValidator::new("test-service");
        validator.register_consumer("test.event", "consumer-service");
        
        let consumers = validator.get_consumers("test.event").unwrap();
        assert!(consumers.contains("consumer-service"));
    }

    #[test]
    fn test_producer_registration() {
        let mut validator = ContractValidator::new("test-service");
        validator.register_producer("test.event", "producer-service");
        
        let producer = validator.get_producer("test.event").unwrap();
        assert_eq!(producer, "producer-service");
    }

    #[test]
    fn test_consumer_compatibility_success() {
        let mut validator = ContractValidator::new("test-service");
        validator.register_consumer("test.event.v1", "consumer-service");
        
        let event = TestEvent::example();
        let result = validator.validate_consumer_compatibility(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_consumer_compatibility_fails_no_consumers() {
        let validator = ContractValidator::new("test-service");
        
        let event = TestEvent::example();
        let result = validator.validate_consumer_compatibility(&event);
        assert!(result.is_err());
        
        if let Err(tyl_error) = result {
            assert!(tyl_error.to_string().contains("No consumers registered"));
        }
    }

    #[test]
    fn test_contract_report_generation() {
        let mut validator = ContractValidator::new("test-service");
        validator.register_consumer("test.event.v1", "consumer1");
        validator.register_consumer("test.event.v1", "consumer2");
        validator.register_producer("test.event.v1", "producer1");
        
        let report = validator.generate_contract_report();
        assert_eq!(report.service_name, "test-service");
        assert_eq!(report.event_contracts.len(), 1);
        
        let event_contract = report.event_contracts.get("test.event.v1").unwrap();
        assert_eq!(event_contract.consumers.len(), 2);
        assert_eq!(event_contract.producer, Some("producer1".to_string()));
    }

    #[test]
    fn test_consumer_expectation_registration() {
        let mut validator = ContractValidator::new("test-service");
        let event = TestEvent::example();
        
        validator.register_consumer_expectation(&event, "consumer-service");
        
        let expectations = validator.get_consumer_expectations("test.event.v1").unwrap();
        assert_eq!(expectations.len(), 1);
        assert_eq!(expectations[0].description, "expects test.event.v1");
    }

    #[test]
    fn test_provider_verification_registration() {
        let mut validator = ContractValidator::new("test-service");
        let event = TestEvent::example();
        
        validator.register_provider_verification(&event, "producer-service");
        
        let verification = validator.get_provider_verification("test.event.v1").unwrap();
        assert_eq!(verification.event_type, "test.event.v1");
    }
}