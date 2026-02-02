//! Webhook client for receiving webhooks.

use better_auth_webhooks::{WebhookPayload, WebhookReceiver, WebhookResult};

/// Client for receiving and verifying webhooks.
pub struct WebhookClient {
    receiver: WebhookReceiver,
}

impl WebhookClient {
    /// Creates a new webhook client.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            receiver: WebhookReceiver::new(secret),
        }
    }

    /// Creates a client with custom tolerance.
    pub fn with_tolerance(secret: impl Into<String>, tolerance_secs: i64) -> Self {
        Self {
            receiver: WebhookReceiver::new(secret).with_tolerance(tolerance_secs),
        }
    }

    /// Verifies and parses a webhook.
    pub fn verify(&self, signature: &str, payload: &[u8]) -> WebhookResult<WebhookPayload> {
        self.receiver.verify(signature, payload)
    }

    /// Verifies a webhook signature only.
    pub fn verify_signature(&self, signature: &str, payload: &[u8]) -> WebhookResult<()> {
        self.receiver.verify_signature(signature, payload)
    }

    /// Returns the inner receiver.
    pub fn receiver(&self) -> &WebhookReceiver {
        &self.receiver
    }
}

/// Builder for webhook clients.
pub struct WebhookClientBuilder {
    secret: String,
    tolerance_secs: i64,
}

impl WebhookClientBuilder {
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

    /// Builds the client.
    pub fn build(self) -> WebhookClient {
        WebhookClient::with_tolerance(self.secret, self.tolerance_secs)
    }
}

/// Helper trait for extracting webhook data from HTTP requests.
pub trait WebhookExtractor {
    /// Extracts the signature header value.
    fn signature(&self) -> Option<&str>;

    /// Extracts the raw body bytes.
    fn body(&self) -> &[u8];
}

/// Convenience function for verifying webhooks from extractors.
pub fn verify_webhook<E: WebhookExtractor>(
    client: &WebhookClient,
    extractor: &E,
) -> WebhookResult<WebhookPayload> {
    let signature = extractor
        .signature()
        .ok_or(better_auth_webhooks::WebhookError::InvalidSignature)?;
    client.verify(signature, extractor.body())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = WebhookClientBuilder::new("test-secret")
            .tolerance(600)
            .build();

        // Client should be created successfully
        assert!(std::ptr::addr_of!(client.receiver).is_aligned());
    }
}
