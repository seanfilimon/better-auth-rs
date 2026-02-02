//! Webhook endpoint configuration builder.

use std::collections::{HashMap, HashSet};

use better_auth_webhooks::{EventFilter, WebhookEndpoint, WebhookMetadata};

/// Configuration for a webhook endpoint.
#[derive(Debug, Clone)]
pub struct WebhookEndpointConfig {
    url: String,
    secret: Option<String>,
    events: EventFilter,
    description: Option<String>,
    headers: HashMap<String, String>,
    timeout_ms: u64,
    enabled: bool,
}

impl WebhookEndpointConfig {
    /// Creates a new endpoint configuration.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            secret: None,
            events: EventFilter::All,
            description: None,
            headers: HashMap::new(),
            timeout_ms: 30000,
            enabled: true,
        }
    }

    /// Sets the secret for signing.
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Sets the event filter.
    pub fn with_events(mut self, events: EventFilter) -> Self {
        self.events = events;
        self
    }

    /// Subscribes to specific events.
    pub fn events(mut self, events: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let event_set: HashSet<String> = events.into_iter().map(|e| e.into()).collect();
        self.events = EventFilter::Specific(event_set);
        self
    }

    /// Subscribes to events matching patterns.
    pub fn patterns(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let pattern_list: Vec<String> = patterns.into_iter().map(|p| p.into()).collect();
        self.events = EventFilter::Pattern(pattern_list);
        self
    }

    /// Subscribes to all events.
    pub fn all_events(mut self) -> Self {
        self.events = EventFilter::All;
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Adds a custom header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Sets the timeout in milliseconds.
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = timeout;
        self
    }

    /// Disables the endpoint.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Converts to a WebhookEndpoint.
    pub fn into_endpoint(self) -> WebhookEndpoint {
        let secret = self.secret.unwrap_or_else(|| {
            // Generate a random secret if not provided
            uuid::Uuid::new_v4().to_string()
        });

        let mut endpoint = WebhookEndpoint::new(self.url, secret)
            .with_events(self.events)
            .timeout_ms(self.timeout_ms);

        if let Some(desc) = self.description {
            endpoint = endpoint.description(desc);
        }

        for (key, value) in self.headers {
            endpoint = endpoint.header(key, value);
        }

        if !self.enabled {
            endpoint = endpoint.disabled();
        }

        endpoint
    }
}

/// Builder for webhook system configuration.
pub struct WebhookConfigBuilder {
    max_retries: u32,
    log_deliveries: bool,
    worker_count: usize,
    poll_interval_ms: u64,
    endpoints: Vec<WebhookEndpointConfig>,
}

impl WebhookConfigBuilder {
    /// Creates a new configuration builder.
    pub fn new() -> Self {
        Self {
            max_retries: 5,
            log_deliveries: true,
            worker_count: 4,
            poll_interval_ms: 1000,
            endpoints: Vec::new(),
        }
    }

    /// Sets the maximum retries.
    pub fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Sets whether to log deliveries.
    pub fn log_deliveries(mut self, log: bool) -> Self {
        self.log_deliveries = log;
        self
    }

    /// Sets the worker count.
    pub fn worker_count(mut self, count: usize) -> Self {
        self.worker_count = count;
        self
    }

    /// Sets the poll interval.
    pub fn poll_interval_ms(mut self, ms: u64) -> Self {
        self.poll_interval_ms = ms;
        self
    }

    /// Adds an endpoint configuration.
    pub fn endpoint(mut self, config: WebhookEndpointConfig) -> Self {
        self.endpoints.push(config);
        self
    }

    /// Builds the webhook configuration.
    pub fn build(self) -> better_auth_webhooks::WebhookConfig {
        better_auth_webhooks::WebhookConfig::new()
            .max_retries(self.max_retries)
            .log_deliveries(self.log_deliveries)
            .worker_count(self.worker_count)
            .poll_interval_ms(self.poll_interval_ms)
    }

    /// Returns the endpoint configurations.
    pub fn endpoints(&self) -> &[WebhookEndpointConfig] {
        &self.endpoints
    }

    /// Takes the endpoint configurations.
    pub fn take_endpoints(self) -> Vec<WebhookEndpointConfig> {
        self.endpoints
    }
}

impl Default for WebhookConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_config() {
        let config = WebhookEndpointConfig::new("https://example.com/webhook")
            .secret("my-secret")
            .patterns(["user.*", "session.*"])
            .description("Test webhook")
            .header("X-Custom", "value")
            .timeout_ms(5000);

        let endpoint = config.into_endpoint();
        assert_eq!(endpoint.url, "https://example.com/webhook");
        assert_eq!(endpoint.secret, "my-secret");
        assert!(endpoint.metadata.description.is_some());
        assert_eq!(endpoint.metadata.timeout_ms, 5000);
    }

    #[test]
    fn test_config_builder() {
        let builder = WebhookConfigBuilder::new()
            .max_retries(3)
            .worker_count(2)
            .endpoint(WebhookEndpointConfig::new("https://example.com"));

        let config = builder.build();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.worker_count, 2);
    }
}
