use crate::{Event, FailedEvent, RetryPolicy};
use async_trait::async_trait;
use thiserror::Error;

/// Result type for event handler operations
pub type HandlerResult = Result<(), HandlerError>;
pub type PreProcessResult = Result<(), HandlerError>;
pub type PostProcessResult = Result<(), HandlerError>;

/// Errors that can occur during event handling
#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Processing failed: {message}")]
    ProcessingFailed { message: String },

    #[error("Retry required: {reason}")]
    RetryRequired { reason: String },

    #[error("Send to DLQ: {reason}")]
    SendToDlq { reason: String },

    #[error("Temporary failure - will retry: {reason}")]
    TemporaryFailure { reason: String },

    #[error("Permanent failure - will not retry: {reason}")]
    PermanentFailure { reason: String },

    #[error("Feature not implemented")]
    NotImplemented,

    #[error("Handler timeout: operation took too long")]
    Timeout,

    #[error("Invalid event format: {details}")]
    InvalidEventFormat { details: String },
}

/// Event handler port - defines the contract for processing events
#[async_trait]
pub trait EventHandler<T>: Send + Sync {
    /// Main event processing method
    /// This is called for each event received from a subscribed topic
    ///
    /// # Arguments
    /// * `event` - The event to process with complete metadata
    ///
    /// # Returns
    /// Result indicating success or failure with retry semantics
    async fn handle(&self, event: Event<T>) -> HandlerResult;

    /// Get the retry policy for this handler (optional override)
    /// If None, the subscription's default retry policy is used
    ///
    /// # Returns
    /// Optional retry policy specific to this handler
    fn retry_policy(&self) -> Option<RetryPolicy> {
        None
    }

    /// Handle events that have been sent to the Dead Letter Queue
    /// This is called when manually processing DLQ events
    ///
    /// # Arguments
    /// * `failed_event` - The failed event with failure context
    ///
    /// # Returns
    /// Result of DLQ event processing
    async fn handle_dlq_event(&self, _failed_event: FailedEvent) -> HandlerResult {
        Err(HandlerError::NotImplemented)
    }

    /// Pre-processing hook called before main event handling
    /// Useful for validation, authentication, or setup
    ///
    /// # Arguments
    /// * `event` - The event about to be processed
    ///
    /// # Returns
    /// Result indicating whether to proceed with processing
    async fn pre_handle(&self, _event: &Event<T>) -> PreProcessResult {
        Ok(())
    }

    /// Post-processing hook called after main event handling
    /// Useful for cleanup, metrics, or side effects
    ///
    /// # Arguments
    /// * `event` - The event that was processed
    /// * `result` - The result of the main handling
    ///
    /// # Returns
    /// Result of post-processing (doesn't affect main result)
    async fn post_handle(&self, _event: &Event<T>, _result: &HandlerResult) -> PostProcessResult {
        Ok(())
    }

    /// Get handler metadata for monitoring and debugging
    ///
    /// # Returns
    /// Handler information for observability
    fn handler_info(&self) -> HandlerInfo {
        HandlerInfo {
            name: std::any::type_name::<Self>().to_string(),
            version: "unknown".to_string(),
            description: None,
        }
    }

    /// Check if this handler can process the given event type
    /// Useful for dynamic routing and validation
    ///
    /// # Arguments
    /// * `event_type` - The event type identifier
    ///
    /// # Returns
    /// Whether this handler can process this event type
    fn can_handle(&self, _event_type: &str) -> bool {
        true // Default: handle all events
    }
}

/// Information about an event handler for monitoring and debugging
#[derive(Debug, Clone)]
pub struct HandlerInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}
