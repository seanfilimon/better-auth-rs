//! Webhook system - main entry point.

use std::sync::Arc;
use tokio::sync::RwLock;

use better_auth_events::{Event, EventBus, EventError, EventHandler};

use crate::delivery::{DeliveryEngine, WebhookJob};
use crate::endpoint::WebhookEndpoint;
use crate::error::WebhookResult;
use crate::queue::{InMemoryQueue, WebhookQueue};
use crate::retry::{ExponentialBackoff, RetryStrategy};

/// Webhook system configuration.
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Whether to log deliveries.
    pub log_deliveries: bool,
    /// Worker count for processing.
    pub worker_count: usize,
    /// Poll interval in milliseconds.
    pub poll_interval_ms: u64,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            log_deliveries: true,
            worker_count: 4,
            poll_interval_ms: 1000,
        }
    }
}

impl WebhookConfig {
    /// Creates a new configuration.
    pub fn new() -> Self {
        Self::default()
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
}

/// The main webhook system.
pub struct WebhookSystem<Q: WebhookQueue = InMemoryQueue, R: RetryStrategy = ExponentialBackoff> {
    config: WebhookConfig,
    endpoints: RwLock<Vec<WebhookEndpoint>>,
    engine: Arc<DeliveryEngine<Q, R>>,
}

impl WebhookSystem<InMemoryQueue, ExponentialBackoff> {
    /// Creates a new webhook system with default queue and retry strategy.
    pub fn new() -> Self {
        Self::with_config(WebhookConfig::default())
    }

    /// Creates a new webhook system with custom configuration.
    pub fn with_config(config: WebhookConfig) -> Self {
        let queue = InMemoryQueue::new();
        let retry = ExponentialBackoff::new().max_attempts(config.max_retries);
        let engine = Arc::new(DeliveryEngine::new(queue, retry));

        Self {
            config,
            endpoints: RwLock::new(Vec::new()),
            engine,
        }
    }
}

impl<Q: WebhookQueue + 'static, R: RetryStrategy + 'static> WebhookSystem<Q, R> {
    /// Creates a webhook system with custom queue and retry strategy.
    pub fn with_queue_and_retry(config: WebhookConfig, queue: Q, retry: R) -> Self {
        let engine = Arc::new(DeliveryEngine::new(queue, retry));

        Self {
            config,
            endpoints: RwLock::new(Vec::new()),
            engine,
        }
    }

    /// Registers a webhook endpoint.
    pub async fn register_endpoint(&self, endpoint: WebhookEndpoint) {
        let mut endpoints = self.endpoints.write().await;
        endpoints.push(endpoint);
    }

    /// Unregisters a webhook endpoint by ID.
    pub async fn unregister_endpoint(&self, id: &str) -> Option<WebhookEndpoint> {
        let mut endpoints = self.endpoints.write().await;
        let idx = endpoints.iter().position(|e| e.id == id)?;
        Some(endpoints.remove(idx))
    }

    /// Gets all registered endpoints.
    pub async fn endpoints(&self) -> Vec<WebhookEndpoint> {
        let endpoints = self.endpoints.read().await;
        endpoints.clone()
    }

    /// Gets an endpoint by ID.
    pub async fn get_endpoint(&self, id: &str) -> Option<WebhookEndpoint> {
        let endpoints = self.endpoints.read().await;
        endpoints.iter().find(|e| e.id == id).cloned()
    }

    /// Queues webhooks for an event.
    pub async fn queue_event(&self, event: &Event) -> WebhookResult<usize> {
        let endpoints = self.endpoints.read().await;
        let event_type = event.simple_type_string();
        let mut queued = 0;

        for endpoint in endpoints.iter() {
            if endpoint.should_receive(&event_type) {
                let job = WebhookJob::new(endpoint, event)
                    .with_max_attempts(self.config.max_retries);
                self.engine.enqueue(job).await.map_err(|e| {
                    crate::error::WebhookError::QueueError(e.to_string())
                })?;
                queued += 1;
            }
        }

        Ok(queued)
    }

    /// Creates an event handler that queues webhooks.
    pub fn create_event_handler(self: Arc<Self>) -> WebhookEventHandler<Q, R> {
        WebhookEventHandler { system: self }
    }

    /// Connects to an event bus.
    pub async fn connect_to_events(self: Arc<Self>, bus: &EventBus) {
        let handler = self.create_event_handler();
        bus.on_all(handler).await;
    }

    /// Gets the delivery engine.
    pub fn engine(&self) -> &DeliveryEngine<Q, R> {
        &self.engine
    }

    /// Gets the configuration.
    pub fn config(&self) -> &WebhookConfig {
        &self.config
    }
}

impl Default for WebhookSystem<InMemoryQueue, ExponentialBackoff> {
    fn default() -> Self {
        Self::new()
    }
}

/// Event handler that queues webhooks.
pub struct WebhookEventHandler<Q: WebhookQueue, R: RetryStrategy> {
    system: Arc<WebhookSystem<Q, R>>,
}

#[async_trait::async_trait]
impl<Q: WebhookQueue + 'static, R: RetryStrategy + 'static> EventHandler for WebhookEventHandler<Q, R> {
    fn id(&self) -> &str {
        "webhook-system"
    }

    async fn handle(&self, event: &Event) -> Result<(), EventError> {
        self.system
            .queue_event(event)
            .await
            .map_err(|e| EventError::HandlerFailed(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::EventFilter;

    #[tokio::test]
    async fn test_webhook_system() {
        let system = WebhookSystem::new();

        let endpoint = WebhookEndpoint::new("https://example.com/webhook", "secret")
            .with_events(EventFilter::Pattern(vec!["user.*".to_string()]));

        system.register_endpoint(endpoint).await;

        let endpoints = system.endpoints().await;
        assert_eq!(endpoints.len(), 1);
    }

    #[tokio::test]
    async fn test_queue_event() {
        let system = WebhookSystem::new();

        let endpoint = WebhookEndpoint::new("https://example.com/webhook", "secret")
            .with_events(EventFilter::Pattern(vec!["user.*".to_string()]));

        system.register_endpoint(endpoint).await;

        let event = Event::simple("user.created", serde_json::json!({"user_id": "123"}));
        let queued = system.queue_event(&event).await.unwrap();

        assert_eq!(queued, 1);
    }

    #[tokio::test]
    async fn test_event_filtering() {
        let system = WebhookSystem::new();

        let endpoint = WebhookEndpoint::new("https://example.com/webhook", "secret")
            .with_events(EventFilter::Pattern(vec!["user.*".to_string()]));

        system.register_endpoint(endpoint).await;

        // Should be queued
        let event = Event::simple("user.created", serde_json::json!({}));
        let queued = system.queue_event(&event).await.unwrap();
        assert_eq!(queued, 1);

        // Should not be queued
        let event = Event::simple("session.created", serde_json::json!({}));
        let queued = system.queue_event(&event).await.unwrap();
        assert_eq!(queued, 0);
    }
}
