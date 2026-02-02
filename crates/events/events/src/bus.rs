//! Event bus for pub/sub communication.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::error::{EventError, EventResult};
use crate::event::Event;
use crate::handler::{BoxedHandler, EventHandler, HandlerResult};
use crate::middleware::{EventMiddleware, MiddlewareChain};

/// The event bus for publishing and subscribing to events.
pub struct EventBus {
    /// Subscribers mapped by event type pattern.
    subscribers: RwLock<HashMap<String, Vec<Arc<BoxedHandler>>>>,
    /// Wildcard subscribers (receive all events).
    wildcard_subscribers: RwLock<Vec<Arc<BoxedHandler>>>,
    /// Event history (optional, for debugging).
    history: RwLock<Vec<Event>>,
    /// Maximum history size.
    max_history: usize,
    /// Middleware chain.
    middleware: RwLock<MiddlewareChain>,
    /// Whether to run handlers in parallel.
    parallel_handlers: bool,
}

impl EventBus {
    /// Creates a new event bus.
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
            wildcard_subscribers: RwLock::new(Vec::new()),
            history: RwLock::new(Vec::new()),
            max_history: 1000,
            middleware: RwLock::new(MiddlewareChain::new()),
            parallel_handlers: true,
        }
    }

    /// Creates an event bus with custom configuration.
    pub fn with_config(max_history: usize, parallel_handlers: bool) -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
            wildcard_subscribers: RwLock::new(Vec::new()),
            history: RwLock::new(Vec::new()),
            max_history,
            middleware: RwLock::new(MiddlewareChain::new()),
            parallel_handlers,
        }
    }

    /// Creates an event bus with custom history size.
    pub fn with_history_size(max_history: usize) -> Self {
        Self::with_config(max_history, true)
    }

    /// Adds middleware to the event bus.
    pub async fn add_middleware(&self, middleware: impl EventMiddleware + 'static) {
        let mut chain = self.middleware.write().await;
        chain.add(middleware);
    }

    /// Subscribes to a specific event type or pattern.
    ///
    /// Patterns support:
    /// - Exact match: "user.created"
    /// - Namespace wildcard: "user.*"
    /// - All events: "*"
    pub async fn on(&self, pattern: &str, handler: impl EventHandler + 'static) {
        if pattern == "*" {
            let mut subs = self.wildcard_subscribers.write().await;
            subs.push(Arc::new(Box::new(handler)));
        } else {
            let mut subs = self.subscribers.write().await;
            subs.entry(pattern.to_string())
                .or_default()
                .push(Arc::new(Box::new(handler)));
        }
    }

    /// Subscribes to all events.
    pub async fn on_all(&self, handler: impl EventHandler + 'static) {
        let mut subs = self.wildcard_subscribers.write().await;
        subs.push(Arc::new(Box::new(handler)));
    }

    /// Emits an event to all matching subscribers (fire and forget).
    pub async fn emit(&self, event: Event) {
        let mut event = event;

        // Run before_emit middleware
        {
            let middleware = self.middleware.read().await;
            if let Err(e) = middleware.before_emit(&mut event).await {
                tracing::error!("Middleware rejected event: {}", e);
                return;
            }
        }

        // Store in history
        self.store_in_history(event.clone()).await;

        // Collect matching handlers
        let handlers = self.collect_handlers(&event).await;

        if self.parallel_handlers {
            // Spawn handlers in parallel
            for handler in handlers {
                let event = event.clone();
                tokio::spawn(async move {
                    if let Err(e) = handler.handle(&event).await {
                        tracing::error!("Event handler '{}' error: {}", handler.id(), e);
                    }
                });
            }
        } else {
            // Run handlers sequentially
            for handler in handlers {
                if let Err(e) = handler.handle(&event).await {
                    tracing::error!("Event handler '{}' error: {}", handler.id(), e);
                }
            }
        }
    }

    /// Emits an event and waits for all handlers to complete.
    pub async fn emit_sync(&self, event: Event) -> Vec<HandlerResult> {
        let mut event = event;
        let mut results = Vec::new();

        // Run before_emit middleware
        {
            let middleware = self.middleware.read().await;
            if let Err(e) = middleware.before_emit(&mut event).await {
                tracing::error!("Middleware rejected event: {}", e);
                return results;
            }
        }

        // Store in history
        self.store_in_history(event.clone()).await;

        // Collect matching handlers
        let handlers = self.collect_handlers(&event).await;

        // Run all handlers and collect results
        for handler in handlers {
            let start = Instant::now();
            let result = handler.handle(&event).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            results.push(match result {
                Ok(()) => HandlerResult::success(handler.id(), duration_ms),
                Err(e) => HandlerResult::failure(handler.id(), e.to_string(), duration_ms),
            });
        }

        // Run after_emit middleware
        {
            let middleware = self.middleware.read().await;
            middleware.after_emit(&event, &results).await;
        }

        results
    }

    /// Emits an event and returns an error if any handler fails.
    pub async fn emit_checked(&self, event: Event) -> EventResult<()> {
        let results = self.emit_sync(event).await;

        for result in results {
            if !result.success {
                return Err(EventError::HandlerFailed(
                    result.error.unwrap_or_else(|| "Unknown error".to_string()),
                ));
            }
        }

        Ok(())
    }

    /// Gets recent events from history.
    pub async fn recent_events(&self, count: usize) -> Vec<Event> {
        let history = self.history.read().await;
        history.iter().rev().take(count).cloned().collect()
    }

    /// Gets events of a specific type from history.
    pub async fn events_of_type(&self, event_type: &str) -> Vec<Event> {
        let history = self.history.read().await;
        history
            .iter()
            .filter(|e| e.event_type.matches(event_type))
            .cloned()
            .collect()
    }

    /// Gets the number of subscribers for a pattern.
    pub async fn subscriber_count(&self, pattern: &str) -> usize {
        if pattern == "*" {
            let subs = self.wildcard_subscribers.read().await;
            subs.len()
        } else {
            let subs = self.subscribers.read().await;
            subs.get(pattern).map(|v| v.len()).unwrap_or(0)
        }
    }

    /// Clears all subscribers.
    pub async fn clear_subscribers(&self) {
        let mut subs = self.subscribers.write().await;
        subs.clear();
        let mut wildcards = self.wildcard_subscribers.write().await;
        wildcards.clear();
    }

    /// Clears event history.
    pub async fn clear_history(&self) {
        let mut history = self.history.write().await;
        history.clear();
    }

    // Internal helper to store event in history
    async fn store_in_history(&self, event: Event) {
        let mut history = self.history.write().await;
        history.push(event);
        if history.len() > self.max_history {
            history.remove(0);
        }
    }

    // Internal helper to collect matching handlers
    async fn collect_handlers(&self, event: &Event) -> Vec<Arc<BoxedHandler>> {
        let mut handlers = Vec::new();

        // Get specific subscribers
        let subs = self.subscribers.read().await;
        for (pattern, pattern_handlers) in subs.iter() {
            if event.event_type.matches(pattern) {
                handlers.extend(pattern_handlers.iter().cloned());
            }
        }

        // Get wildcard subscribers
        let wildcards = self.wildcard_subscribers.read().await;
        handlers.extend(wildcards.iter().cloned());

        handlers
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventType;

    struct TestHandler {
        id: String,
        received: Arc<RwLock<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl EventHandler for TestHandler {
        fn id(&self) -> &str {
            &self.id
        }

        async fn handle(&self, event: &Event) -> Result<(), EventError> {
            let mut received = self.received.write().await;
            received.push(event.simple_type_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_emission() {
        let bus = EventBus::new();
        let received = Arc::new(RwLock::new(Vec::new()));

        bus.on(
            "test.event",
            TestHandler {
                id: "test".to_string(),
                received: received.clone(),
            },
        )
        .await;

        bus.emit_sync(Event::new(EventType::new("test", "event"), "payload"))
            .await;

        let events = received.read().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "test.event");
    }

    #[tokio::test]
    async fn test_wildcard_subscription() {
        let bus = EventBus::new();
        let received = Arc::new(RwLock::new(Vec::new()));

        bus.on_all(TestHandler {
            id: "wildcard".to_string(),
            received: received.clone(),
        })
        .await;

        bus.emit_sync(Event::new(EventType::new("any", "event"), "payload"))
            .await;
        bus.emit_sync(Event::new(EventType::new("another", "event"), "payload"))
            .await;

        let events = received.read().await;
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_pattern_subscription() {
        let bus = EventBus::new();
        let received = Arc::new(RwLock::new(Vec::new()));

        bus.on(
            "user.*",
            TestHandler {
                id: "user-handler".to_string(),
                received: received.clone(),
            },
        )
        .await;

        bus.emit_sync(Event::new(EventType::new("user", "created"), "payload"))
            .await;
        bus.emit_sync(Event::new(EventType::new("user", "updated"), "payload"))
            .await;
        bus.emit_sync(Event::new(EventType::new("session", "created"), "payload"))
            .await;

        let events = received.read().await;
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_event_history() {
        let bus = EventBus::with_history_size(10);

        for i in 0..15 {
            bus.emit_sync(Event::new(
                EventType::new("test", format!("event{}", i)),
                "payload",
            ))
            .await;
        }

        let recent = bus.recent_events(5).await;
        assert_eq!(recent.len(), 5);

        let all = bus.recent_events(100).await;
        assert_eq!(all.len(), 10); // Max history size
    }
}
