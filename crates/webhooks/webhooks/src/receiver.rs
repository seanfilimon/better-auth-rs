//! Webhook receiver for verifying incoming webhooks.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{WebhookError, WebhookResult};
use crate::signature::WebhookSigner;

/// Parsed webhook payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Webhook ID.
    pub id: String,
    /// Event type.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Event data.
    pub data: Value,
    /// Timestamp.
    pub timestamp: String,
    /// Correlation ID.
    pub correlation_id: Option<String>,
}

/// Webhook receiver for verifying incoming webhooks.
pub struct WebhookReceiver {
    signer: WebhookSigner,
    /// Tolerance for timestamp validation (in seconds).
    tolerance_secs: i64,
}

impl WebhookReceiver {
    /// Creates a new webhook receiver.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            signer: WebhookSigner::new(secret),
            tolerance_secs: 300, // 5 minutes
        }
    }

    /// Sets the timestamp tolerance.
    pub fn with_tolerance(mut self, tolerance_secs: i64) -> Self {
        self.tolerance_secs = tolerance_secs;
        self
    }

    /// Verifies a webhook signature and parses the payload.
    pub fn verify(&self, signature: &str, payload: &[u8]) -> WebhookResult<WebhookPayload> {
        // Verify signature
        self.signer
            .verify_header(signature, payload, self.tolerance_secs)
            .map_err(|e| match e {
                crate::signature::SignatureError::InvalidFormat => WebhookError::InvalidSignature,
                crate::signature::SignatureError::Invalid => WebhookError::InvalidSignature,
                crate::signature::SignatureError::Expired => WebhookError::ExpiredSignature,
            })?;

        // Parse payload
        let payload: WebhookPayload =
            serde_json::from_slice(payload).map_err(|e| WebhookError::InvalidPayload(e.to_string()))?;

        Ok(payload)
    }

    /// Verifies a webhook and returns the raw JSON value.
    pub fn verify_raw(&self, signature: &str, payload: &[u8]) -> WebhookResult<Value> {
        // Verify signature
        self.signer
            .verify_header(signature, payload, self.tolerance_secs)
            .map_err(|e| match e {
                crate::signature::SignatureError::InvalidFormat => WebhookError::InvalidSignature,
                crate::signature::SignatureError::Invalid => WebhookError::InvalidSignature,
                crate::signature::SignatureError::Expired => WebhookError::ExpiredSignature,
            })?;

        // Parse as raw JSON
        serde_json::from_slice(payload).map_err(|e| WebhookError::InvalidPayload(e.to_string()))
    }

    /// Verifies only the signature without parsing.
    pub fn verify_signature(&self, signature: &str, payload: &[u8]) -> WebhookResult<()> {
        self.signer
            .verify_header(signature, payload, self.tolerance_secs)
            .map_err(|e| match e {
                crate::signature::SignatureError::InvalidFormat => WebhookError::InvalidSignature,
                crate::signature::SignatureError::Invalid => WebhookError::InvalidSignature,
                crate::signature::SignatureError::Expired => WebhookError::ExpiredSignature,
            })
    }
}

/// Builder for webhook receivers with custom configuration.
pub struct WebhookReceiverBuilder {
    secret: String,
    tolerance_secs: i64,
}

impl WebhookReceiverBuilder {
    /// Creates a new builder.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            tolerance_secs: 300,
        }
    }

    /// Sets the timestamp tolerance.
    pub fn tolerance(mut self, secs: i64) -> Self {
        self.tolerance_secs = secs;
        self
    }

    /// Builds the receiver.
    pub fn build(self) -> WebhookReceiver {
        WebhookReceiver::new(self.secret).with_tolerance(self.tolerance_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_payload_parsing() {
        let json = r#"{
            "id": "123",
            "type": "user.created",
            "data": {"user_id": "456"},
            "timestamp": "2024-01-01T00:00:00Z",
            "correlation_id": "trace-789"
        }"#;

        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.id, "123");
        assert_eq!(payload.event_type, "user.created");
        assert_eq!(payload.correlation_id, Some("trace-789".to_string()));
    }
}
