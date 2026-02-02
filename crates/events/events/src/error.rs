//! Event error types.

use thiserror::Error;

/// Result type for event operations.
pub type EventResult<T> = Result<T, EventError>;

/// Error type for event handling.
#[derive(Debug, Error)]
pub enum EventError {
    /// Handler execution failed.
    #[error("Handler failed: {0}")]
    HandlerFailed(String),

    /// Event serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Event validation failed.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Event type not registered.
    #[error("Unknown event type: {0}")]
    UnknownEventType(String),

    /// Middleware rejected the event.
    #[error("Middleware rejected: {0}")]
    MiddlewareRejected(String),

    /// Event delivery timeout.
    #[error("Delivery timeout")]
    Timeout,

    /// Invalid input.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for EventError {
    fn from(err: serde_json::Error) -> Self {
        EventError::SerializationError(err.to_string())
    }
}
