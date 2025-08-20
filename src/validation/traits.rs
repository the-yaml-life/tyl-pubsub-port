//! Traits for contract validation using Pact.io patterns

use crate::validation::errors::*;
use crate::PubSubResult;
use schemars::JsonSchema;
use serde::Serialize;

/// Trait for events that support Pact contract validation
///
/// This trait extends events with automatic schema validation and
/// contract generation capabilities for testing consumer compatibility.
pub trait PactValidated: Serialize + JsonSchema + Clone + Send + Sync {
    /// Get JSON schema for this event type
    ///
    /// Used to validate event instances and generate contracts
    fn get_json_schema() -> schemars::schema::RootSchema {
        schemars::schema_for!(Self)
    }

    /// Example instance for contract generation
    ///
    /// This should return a valid, realistic example of this event
    /// that can be used in Pact contracts and documentation
    fn example() -> Self;

    /// Get the event type identifier for this event
    ///
    /// Should be a stable identifier like "user.registered.v1"
    fn event_type(&self) -> &'static str;

    /// Validate this instance against its JSON schema
    ///
    /// Returns an error if the event doesn't match its expected schema
    fn validate_schema(&self) -> PubSubResult<()> {
        let schema = Self::get_json_schema();
        let instance = serde_json::to_value(self)
            .map_err(|e| ValidationError::SerializationFailed(e.to_string()))?;

        // Convert schema to JSON Value for jsonschema compilation
        let schema_value = serde_json::to_value(&schema.schema).map_err(|e| {
            ValidationError::SchemaCompilationFailed(format!("Failed to serialize schema: {e}"))
        })?;

        let compiled = jsonschema::JSONSchema::compile(&schema_value)
            .map_err(|e| ValidationError::SchemaCompilationFailed(e.to_string()))?;

        let result = compiled.validate(&instance);
        match result {
            Ok(()) => Ok(()),
            Err(errors) => {
                let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
                Err(ValidationError::SchemaValidationFailed {
                    errors: error_messages,
                }
                .into())
            }
        }
    }

    /// Generate Pact consumer expectation for this event
    ///
    /// Creates a Pact message expectation that can be used in consumer tests
    fn generate_consumer_expectation(&self) -> PactMessageExpectation {
        let example_json =
            serde_json::to_value(self.clone()).expect("Event should be serializable");

        PactMessageExpectation {
            description: format!("expects {}", self.event_type()),
            provider_state: Some(format!("{} is available", self.event_type())),
            contents: example_json,
        }
    }

    /// Generate Pact provider verification for this event
    ///
    /// Creates provider-side verification data for contract testing
    fn generate_provider_verification(&self) -> PactProviderVerification {
        PactProviderVerification {
            event_type: self.event_type().to_string(),
            schema: Self::get_json_schema(),
            example: serde_json::to_value(self.clone()).expect("Event should be serializable"),
        }
    }
}

/// Pact message expectation structure
///
/// Represents what a consumer expects to receive from a producer
#[derive(Debug, Clone)]
pub struct PactMessageExpectation {
    pub description: String,
    pub provider_state: Option<String>,
    pub contents: serde_json::Value,
}

/// Pact provider verification structure
///
/// Contains information needed for provider-side contract verification
#[derive(Debug, Clone)]
pub struct PactProviderVerification {
    pub event_type: String,
    pub schema: schemars::schema::RootSchema,
    pub example: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

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
    fn test_schema_generation() {
        let schema = TestEvent::get_json_schema();

        // Basic schema validation - just verify it's not empty
        let schema_json = serde_json::to_value(&schema.schema).unwrap();
        assert!(!schema_json.is_null());

        // Try to access properties if it's an object
        if let Some(obj) = schema_json.as_object() {
            // Should have some structure
            assert!(!obj.is_empty());
        }
    }

    #[test]
    fn test_valid_event_validation() {
        let event = TestEvent::example();
        assert!(event.validate_schema().is_ok());
    }

    #[test]
    fn test_consumer_expectation_generation() {
        let event = TestEvent::example();
        let expectation = event.generate_consumer_expectation();

        assert_eq!(expectation.description, "expects test.event.v1");
        assert!(expectation.provider_state.is_some());
        assert!(expectation.contents.is_object());
    }

    #[test]
    fn test_provider_verification_generation() {
        let event = TestEvent::example();
        let verification = event.generate_provider_verification();

        assert_eq!(verification.event_type, "test.event.v1");
        assert!(verification.example.is_object());
    }
}
