//! Contract validation module for PubSub events using Pact.io patterns
//!
//! This module provides automatic schema and contract validation for events,
//! ensuring compatibility between producers and consumers.

#[cfg(feature = "pact-validation")]
pub mod traits;
#[cfg(feature = "pact-validation")]
pub mod validator;
#[cfg(feature = "pact-validation")]
pub mod errors;
#[cfg(feature = "pact-validation")]
pub mod extended_traits;

// Re-exports for convenience when pact-validation feature is enabled
#[cfg(feature = "pact-validation")]
pub use traits::PactValidated;
#[cfg(feature = "pact-validation")]
pub use validator::ContractValidator;
#[cfg(feature = "pact-validation")]
pub use errors::*;
#[cfg(feature = "pact-validation")]
pub use extended_traits::{ValidatedEventPublisher, ValidatedEventSubscriber, PubSubValidationExt};

/// Simple trait to detect and validate pact events
pub trait PactEventValidator {
    fn is_pact_event(&self) -> bool { false }
    fn validate_pact_schema(&self) -> crate::PubSubResult<()> { Ok(()) }
}

// Manual implementations needed for specific non-pact types
// The pact_event! macro will implement this trait automatically

// Stub implementations when pact-validation feature is disabled
#[cfg(not(feature = "pact-validation"))]
pub mod stub {
    use crate::PubSubResult;
    use serde::Serialize;
    
    /// Placeholder trait when pact validation is disabled
    pub trait PactValidated: Serialize + Clone + Send + Sync {
        fn validate_schema(&self) -> PubSubResult<()> {
            Ok(())
        }
        
        fn example() -> Self;
        
        fn event_type(&self) -> &'static str;
    }
    
    /// Placeholder validator when pact validation is disabled  
    pub struct ContractValidator;
    
    impl ContractValidator {
        pub fn new(_service_name: impl Into<String>) -> Self {
            Self
        }
    }
}

#[cfg(not(feature = "pact-validation"))]
pub use stub::*;