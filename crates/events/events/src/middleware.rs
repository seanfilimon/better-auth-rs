//! Event middleware for processing events before/after emission.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::error::EventError;
use crate::event::Event;
use crate::handler::HandlerResult;

/// Trait for event middleware.
#[async_trait]
pub trait EventMiddleware: Send + Sync {
    /// Called before an event is emitted.
    /// Can modify the event or reject it by returning an error.
    async fn before_emit(&self, event: &mut Event) -> Result<(), EventError>;

    /// Called after an event has been processed by all handlers.
    async fn after_emit(&self, event: &Event, results: &[HandlerResult]);
}

/// Chain of middleware to process events.
pub struct MiddlewareChain {
    middleware: Vec<Arc<dyn EventMiddleware>>,
}

impl MiddlewareChain {
    /// Creates a new empty middleware chain.
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    /// Adds middleware to the chain.
    pub fn add(&mut self, middleware: impl EventMiddleware + 'static) {
        self.middleware.push(Arc::new(middleware));
    }

    /// Runs all before_emit middleware.
    pub async fn before_emit(&self, event: &mut Event) -> Result<(), EventError> {
        for m in &self.middleware {
            m.before_emit(event).await?;
        }
        Ok(())
    }

    /// Runs all after_emit middleware.
    pub async fn after_emit(&self, event: &Event, results: &[HandlerResult]) {
        for m in &self.middleware {
            m.after_emit(event, results).await;
        }
    }

    /// Returns the number of middleware in the chain.
    pub fn len(&self) -> usize {
        self.middleware.len()
    }

    /// Checks if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.middleware.is_empty()
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware that logs events.
pub struct LoggingMiddleware {
    /// Log level for events.
    pub level: LogLevel,
}

/// Log level for the logging middleware.
#[derive(Debug, Clone, Copy, Default)]
pub enum LogLevel {
    /// Trace level (most verbose).
    Trace,
    /// Debug level.
    Debug,
    /// Info level (default).
    #[default]
    Info,
    /// Warn level.
    Warn,
}

impl LoggingMiddleware {
    /// Creates a new logging middleware with default settings.
    pub fn new() -> Self {
        Self {
            level: LogLevel::Info,
        }
    }

    /// Creates a logging middleware with a specific log level.
    pub fn with_level(level: LogLevel) -> Self {
        Self { level }
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventMiddleware for LoggingMiddleware {
    async fn before_emit(&self, event: &mut Event) -> Result<(), EventError> {
        match self.level {
            LogLevel::Trace => {
                tracing::trace!(
                    event_id = %event.id,
                    event_type = %event.simple_type_string(),
                    source = %event.metadata.source,
                    "Emitting event"
                );
            }
            LogLevel::Debug => {
                tracing::debug!(
                    event_id = %event.id,
                    event_type = %event.simple_type_string(),
                    "Emitting event"
                );
            }
            LogLevel::Info => {
                tracing::info!(
                    event_type = %event.simple_type_string(),
                    "Emitting event"
                );
            }
            LogLevel::Warn => {
                tracing::warn!(
                    event_type = %event.simple_type_string(),
                    "Emitting event"
                );
            }
        }
        Ok(())
    }

    async fn after_emit(&self, event: &Event, results: &[HandlerResult]) {
        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.len() - success_count;

        match self.level {
            LogLevel::Trace | LogLevel::Debug => {
                tracing::debug!(
                    event_id = %event.id,
                    event_type = %event.simple_type_string(),
                    handlers = results.len(),
                    success = success_count,
                    failures = failure_count,
                    "Event processed"
                );
            }
            LogLevel::Info | LogLevel::Warn => {
                if failure_count > 0 {
                    tracing::warn!(
                        event_type = %event.simple_type_string(),
                        failures = failure_count,
                        "Event processed with failures"
                    );
                }
            }
        }
    }
}

/// Middleware that collects metrics about events.
pub struct MetricsMiddleware {
    /// Metrics collector (placeholder for actual metrics implementation).
    _start_times: std::sync::RwLock<std::collections::HashMap<String, Instant>>,
}

impl MetricsMiddleware {
    /// Creates a new metrics middleware.
    pub fn new() -> Self {
        Self {
            _start_times: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventMiddleware for MetricsMiddleware {
    async fn before_emit(&self, event: &mut Event) -> Result<(), EventError> {
        // Record start time
        let mut times = self._start_times.write().unwrap();
        times.insert(event.id.clone(), Instant::now());

        // In a real implementation, you would increment counters here
        // metrics::counter!("events_emitted_total", 1, "type" => event.simple_type_string());

        Ok(())
    }

    async fn after_emit(&self, event: &Event, results: &[HandlerResult]) {
        // Calculate duration
        let duration = {
            let times = self._start_times.read().unwrap();
            times.get(&event.id).map(|t| t.elapsed())
        };

        if let Some(duration) = duration {
            // In a real implementation, you would record histograms here
            // metrics::histogram!("event_processing_duration_ms", duration.as_millis() as f64);
            tracing::trace!(
                event_type = %event.simple_type_string(),
                duration_ms = duration.as_millis(),
                handlers = results.len(),
                "Event metrics recorded"
            );
        }

        // Clean up
        let mut times = self._start_times.write().unwrap();
        times.remove(&event.id);
    }
}

/// Middleware that validates events against the registry.
pub struct ValidationMiddleware {
    /// Whether to reject unknown events.
    pub reject_unknown: bool,
    /// Whether to warn about deprecated events.
    pub warn_deprecated: bool,
}

impl ValidationMiddleware {
    /// Creates a new validation middleware with default settings.
    pub fn new() -> Self {
        Self {
            reject_unknown: false,
            warn_deprecated: true,
        }
    }

    /// Creates a strict validation middleware that rejects unknown events.
    pub fn strict() -> Self {
        Self {
            reject_unknown: true,
            warn_deprecated: true,
        }
    }
}

impl Default for ValidationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventMiddleware for ValidationMiddleware {
    async fn before_emit(&self, event: &mut Event) -> Result<(), EventError> {
        // In a real implementation, this would check against the EventRegistry
        // For now, we just validate basic structure

        if event.event_type.namespace.is_empty() {
            return Err(EventError::ValidationError(
                "Event namespace cannot be empty".to_string(),
            ));
        }

        if event.event_type.name.is_empty() {
            return Err(EventError::ValidationError(
                "Event name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    async fn after_emit(&self, _event: &Event, _results: &[HandlerResult]) {
        // No-op for validation middleware
    }
}

/// Middleware that adds correlation IDs to events.
pub struct CorrelationMiddleware;

impl CorrelationMiddleware {
    /// Creates a new correlation middleware.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CorrelationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventMiddleware for CorrelationMiddleware {
    async fn before_emit(&self, event: &mut Event) -> Result<(), EventError> {
        // Ensure event has a correlation ID
        if event.correlation_id.is_none() {
            event.correlation_id = Some(event.id.clone());
        }
        Ok(())
    }

    async fn after_emit(&self, _event: &Event, _results: &[HandlerResult]) {
        // No-op
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventType;

    #[tokio::test]
    async fn test_middleware_chain() {
        let mut chain = MiddlewareChain::new();
        chain.add(LoggingMiddleware::new());
        chain.add(ValidationMiddleware::new());

        let mut event = Event::new(EventType::new("test", "event"), "payload");

        let result = chain.before_emit(&mut event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validation_middleware_rejects_empty_namespace() {
        let middleware = ValidationMiddleware::new();

        let mut event = Event::new(EventType::new("", "event"), "payload");

        let result = middleware.before_emit(&mut event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_correlation_middleware() {
        let middleware = CorrelationMiddleware::new();

        let mut event = Event::new(EventType::new("test", "event"), "payload");
        assert!(event.correlation_id.is_none());

        middleware.before_emit(&mut event).await.unwrap();
        assert!(event.correlation_id.is_some());
        assert_eq!(event.correlation_id, Some(event.id.clone()));
    }
}
