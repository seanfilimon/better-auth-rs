//! Webhook error types.

use thiserror::Error;

/// Result type for webhook operations.
pub type WebhookResult<T> = Result<T, WebhookError>;

/// Error type for webhook operations.
#[derive(Debug, Error)]
pub enum WebhookError {
    /// Invalid signature.
    #[error("Invalid signature")]
    InvalidSignature,

    /// Signature expired.
    #[error("Signature expired")]
    ExpiredSignature,

    /// Invalid payload.
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    /// Delivery failed.
    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),

    /// Queue error.
    #[error("Queue error: {0}")]
    QueueError(String),

    /// Endpoint not found.
    #[error("Endpoint not found: {0}")]
    EndpointNotFound(String),

    /// HTTP error.
    #[error("HTTP error: {0}")]
    HttpError(String),

    /// Timeout.
    #[error("Request timeout")]
    Timeout,

    /// Max retries exceeded.
    #[error("Max retries exceeded")]
    MaxRetriesExceeded,

    /// Storage error.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Circuit breaker is open.
    #[error("Circuit breaker is open - endpoint is temporarily unavailable")]
    CircuitOpen,

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for WebhookError {
    fn from(err: serde_json::Error) -> Self {
        WebhookError::InvalidPayload(err.to_string())
    }
}

#[cfg(feature = "http-client")]
impl From<reqwest::Error> for WebhookError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            WebhookError::Timeout
        } else {
            WebhookError::HttpError(err.to_string())
        }
    }
}
