//! # TYL PubSub Port
//!
//! Event-driven PubSub port for TYL Framework with future-proof signatures and comprehensive functionality.
//!
//! ## Features
//!
//! - Async publish/subscribe patterns with type safety
//! - Dead Letter Queue (DLQ) management for failed events
//! - Sophisticated retry logic with exponential backoff
//! - Event sourcing capabilities with stream management
//! - Comprehensive monitoring and observability
//! - Hexagonal architecture with ports and adapters
//! - Complete mock implementation for testing and development
//!
//! ## Quick Start
//!
//! ```rust
//! use tyl_pubsub_port::{EventPublisher, EventSubscriber, MockPubSubAdapter};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct UserEvent {
//!     user_id: String,
//!     action: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let pubsub = MockPubSubAdapter::new();
//!     
//!     // Publish an event
//!     let event = UserEvent {
//!         user_id: "123".to_string(),
//!         action: "registered".to_string(),
//!     };
//!     
//!     let event_id = pubsub.publish("user.events", event).await?;
//!     println!("Published event: {}", event_id);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! This module follows hexagonal architecture:
//!
//! - **Ports (Interfaces)**: `EventPublisher`, `EventSubscriber`, `DeadLetterQueueManager`, etc.
//! - **Adapters**: Mock implementation included, future adapters for Redis, Kafka, NATS
//! - **Domain Logic**: Event management, retry policies, DLQ handling independent of infrastructure
//!
//! ## Examples
//!
//! See the `examples/` directory for complete usage examples including:
//! - Basic pub/sub operations
//! - Dead letter queue handling
//! - Event sourcing patterns
//! - Monitoring and metrics

// Re-export minimal TYL dependencies
pub use tyl_errors::{TylError, TylResult};

// Core result type for PubSub operations
pub type PubSubResult<T> = TylResult<T>;

// Module declarations
pub mod macros;
pub mod mock;
pub mod traits;
pub mod types;
pub mod validation;

// Re-exports for public API - specific to avoid ambiguity
// Traits
pub use traits::dlq::*;
pub use traits::handler::{EventHandler, HandlerError, HandlerInfo, HandlerResult};
pub use traits::monitoring::*;
pub use traits::publisher::*;
pub use traits::retry::*;
pub use traits::store::*;
pub use traits::subscriber::*;

// Types
pub use types::config::*;
pub use types::dlq::*;
pub use types::event::*;
pub use types::monitoring::*;
pub use types::store::*;
pub use types::subscription::*;

// Mock implementation
pub use mock::{MockPubSubAdapter, ValidatedMockAdapter};

// Pact validation publisher (compile-time safety)
#[cfg(feature = "pact-validation")]
pub use mock::validated_adapter::PactEventPublisher;

// Validation re-exports (conditional on feature)
#[cfg(feature = "pact-validation")]
pub use validation::{
    ContractValidator, PactValidated, PubSubValidationExt, ValidatedEventPublisher,
    ValidatedEventSubscriber, ValidationError,
};

// Auto-validation re-exports (always available)
pub use validation::PactEventValidator;

// Re-export macros
// Note: Macros are automatically available when using the crate

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubsub_result_type_alias() {
        let result: PubSubResult<String> = Ok("test".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_error_integration() {
        let error = TylError::validation("topic", "invalid topic name");
        let result: PubSubResult<()> = Err(error);
        assert!(result.is_err());
    }
}
