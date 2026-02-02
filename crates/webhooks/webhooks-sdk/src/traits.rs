//! Traits for plugin webhook integration.

use std::sync::Arc;

use better_auth_events::Event;
use better_auth_webhooks::{WebhookEndpoint, WebhookSystem};
use serde_json::Value;

use crate::builder::WebhookEndpointConfig;

/// Trait for plugins that provide webhook endpoints.
///
/// Implement this trait to declare webhook endpoints your plugin registers.
pub trait WebhookProvider {
    /// Returns the webhook endpoint configurations this plugin provides.
    fn webhook_endpoints() -> Vec<WebhookEndpointConfig>;

    /// Returns the plugin identifier for webhook sources.
    fn webhook_source() -> &'static str;
}

/// Trait for transforming event payloads before webhook delivery.
///
/// Implement this trait to customize how events are serialized for webhooks.
pub trait WebhookTransformer {
    /// Transforms an event payload for webhook delivery.
    ///
    /// Return `None` to use the default payload format.
    /// Return `Some(value)` to use a custom payload.
    fn transform_payload(&self, event: &Event) -> Option<Value> {
        None
    }

    /// Filters whether an event should be sent to webhooks.
    ///
    /// Return `true` to allow the event, `false` to skip it.
    fn should_send(&self, event: &Event) -> bool {
        true
    }

    /// Adds custom headers to webhook requests.
    fn custom_headers(&self, event: &Event) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }
}

/// Default transformer that passes events through unchanged.
pub struct DefaultTransformer;

impl WebhookTransformer for DefaultTransformer {
    // Uses all default implementations
}

/// Helper for registering plugin webhooks.
pub struct PluginWebhookRegistrar<Q, R>
where
    Q: better_auth_webhooks::WebhookQueue + 'static,
    R: better_auth_webhooks::RetryStrategy + 'static,
{
    system: Arc<WebhookSystem<Q, R>>,
}

impl<Q, R> PluginWebhookRegistrar<Q, R>
where
    Q: better_auth_webhooks::WebhookQueue + 'static,
    R: better_auth_webhooks::RetryStrategy + 'static,
{
    /// Creates a new registrar.
    pub fn new(system: Arc<WebhookSystem<Q, R>>) -> Self {
        Self { system }
    }

    /// Registers all endpoints from a webhook provider.
    pub async fn register_provider<P: WebhookProvider>(&self) {
        for config in P::webhook_endpoints() {
            let endpoint = config.into_endpoint();
            self.system.register_endpoint(endpoint).await;
        }
    }

    /// Registers a single endpoint.
    pub async fn register_endpoint(&self, endpoint: WebhookEndpoint) {
        self.system.register_endpoint(endpoint).await;
    }

    /// Registers an endpoint from a config.
    pub async fn register_config(&self, config: WebhookEndpointConfig) {
        self.system.register_endpoint(config.into_endpoint()).await;
    }

    /// Returns the webhook system.
    pub fn system(&self) -> &WebhookSystem<Q, R> {
        &self.system
    }
}

/// Trait for plugins that can receive webhooks.
pub trait WebhookReceivable {
    /// Handles an incoming webhook payload.
    fn handle_webhook(&self, payload: &better_auth_webhooks::WebhookPayload) -> better_auth_webhooks::WebhookResult<()>;
}
