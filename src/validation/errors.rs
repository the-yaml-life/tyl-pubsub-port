//! Error types for contract validation

use thiserror::Error;
use crate::TylError;

/// Contract validation specific errors
#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    #[error("No consumers registered for event type: {0}")]
    NoConsumersRegistered(String),

    #[error("Schema compilation failed: {0}")]
    SchemaCompilationFailed(String),

    #[error("Schema validation failed: {}", errors.join(", "))]
    SchemaValidationFailed { errors: Vec<String> },

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Contract incompatible: {0}")]
    ContractIncompatible(String),

    #[error("Producer not registered for event type: {0}")]
    ProducerNotRegistered(String),

    #[error("Consumer contract validation failed: {message}")]
    ConsumerValidationFailed { message: String },

    #[error("Producer contract validation failed: {message}")]
    ProducerValidationFailed { message: String },

    #[error("Pact broker communication failed: {0}")]
    PactBrokerFailed(String),

    #[error("Contract version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
}

impl From<ValidationError> for TylError {
    fn from(error: ValidationError) -> Self {
        match error {
            ValidationError::NoConsumersRegistered(event_type) => {
                TylError::validation("contract", format!("No consumers registered for event type: {event_type}"))
            }
            ValidationError::SchemaCompilationFailed(msg) => {
                TylError::validation("schema", format!("Schema compilation failed: {msg}"))
            }
            ValidationError::SchemaValidationFailed { errors } => {
                TylError::validation("schema", format!("Schema validation failed: {}", errors.join(", ")))
            }
            ValidationError::SerializationFailed(msg) => {
                TylError::serialization(msg)
            }
            ValidationError::ContractIncompatible(msg) => {
                TylError::validation("contract", format!("Contract incompatible: {msg}"))
            }
            ValidationError::ProducerNotRegistered(event_type) => {
                TylError::validation("contract", format!("Producer not registered for event type: {event_type}"))
            }
            ValidationError::ConsumerValidationFailed { message } => {
                TylError::validation("consumer", format!("Consumer contract validation failed: {message}"))
            }
            ValidationError::ProducerValidationFailed { message } => {
                TylError::validation("producer", format!("Producer contract validation failed: {message}"))
            }
            ValidationError::PactBrokerFailed(msg) => {
                TylError::network(format!("Pact broker communication failed: {msg}"))
            }
            ValidationError::VersionMismatch { expected, found } => {
                TylError::validation("version", format!("Contract version mismatch: expected {expected}, found {found}"))
            }
        }
    }
}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_conversion() {
        let validation_error = ValidationError::NoConsumersRegistered("test.event".to_string());
        let tyl_error: TylError = validation_error.into();
        
        // Should convert to TylError properly
        assert!(tyl_error.to_string().contains("No consumers registered"));
    }

    #[test]
    fn test_schema_validation_error_with_multiple_errors() {
        let errors = vec![
            "Field 'id' is required".to_string(),
            "Field 'value' must be positive".to_string(),
        ];
        let validation_error = ValidationError::SchemaValidationFailed { errors };
        let tyl_error: TylError = validation_error.into();
        
        let error_message = tyl_error.to_string();
        assert!(error_message.contains("Field 'id' is required"));
        assert!(error_message.contains("Field 'value' must be positive"));
    }

    #[test]
    fn test_version_mismatch_error() {
        let validation_error = ValidationError::VersionMismatch {
            expected: "v2".to_string(),
            found: "v1".to_string(),
        };
        let tyl_error: TylError = validation_error.into();
        
        let error_message = tyl_error.to_string();
        assert!(error_message.contains("expected v2"));
        assert!(error_message.contains("found v1"));
    }
}