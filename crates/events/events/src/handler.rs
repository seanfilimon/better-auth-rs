//! Event handler trait and types.

use async_trait::async_trait;
use std::sync::Arc;

use crate::error::EventError;
use crate::event::Event;

/// Result of handling an event.
#[derive(Debug, Clone)]
pub struct HandlerResult {
    /// Handler identifier.
    pub handler_id: String,
    /// Whether the handler succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

impl HandlerResult {
    /// Creates a successful result.
    pub fn success(handler_id: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            handler_id: handler_id.into(),
            success: true,
            error: None,
            duration_ms,
        }
    }

    /// Creates a failed result.
    pub fn failure(handler_id: impl Into<String>, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            handler_id: handler_id.into(),
            success: false,
            error: Some(error.into()),
            duration_ms,
        }
    }
}

/// Trait for event handlers.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Returns a unique identifier for this handler.
    fn id(&self) -> &str {
        "anonymous"
    }

    /// Handles an event.
    async fn handle(&self, event: &Event) -> Result<(), EventError>;
}

/// A boxed event handler.
pub type BoxedHandler = Box<dyn EventHandler>;

/// Wrapper for function-based handlers.
pub struct FnHandler<F>
where
    F: Fn(&Event) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EventError>> + Send>>
        + Send
        + Sync,
{
    id: String,
    handler: F,
}

impl<F> FnHandler<F>
where
    F: Fn(&Event) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EventError>> + Send>>
        + Send
        + Sync,
{
    /// Creates a new function handler.
    pub fn new(id: impl Into<String>, handler: F) -> Self {
        Self {
            id: id.into(),
            handler,
        }
    }
}

#[async_trait]
impl<F> EventHandler for FnHandler<F>
where
    F: Fn(&Event) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EventError>> + Send>>
        + Send
        + Sync,
{
    fn id(&self) -> &str {
        &self.id
    }

    async fn handle(&self, event: &Event) -> Result<(), EventError> {
        (self.handler)(event).await
    }
}

/// Handler that wraps an Arc for shared ownership.
pub struct SharedHandler {
    inner: Arc<dyn EventHandler>,
}

impl SharedHandler {
    /// Creates a new shared handler.
    pub fn new(handler: Arc<dyn EventHandler>) -> Self {
        Self { inner: handler }
    }
}

#[async_trait]
impl EventHandler for SharedHandler {
    fn id(&self) -> &str {
        self.inner.id()
    }

    async fn handle(&self, event: &Event) -> Result<(), EventError> {
        self.inner.handle(event).await
    }
}

/// Handler that filters events before processing.
pub struct FilteredHandler<H: EventHandler> {
    inner: H,
    filter: Box<dyn Fn(&Event) -> bool + Send + Sync>,
}

impl<H: EventHandler> FilteredHandler<H> {
    /// Creates a new filtered handler.
    pub fn new(inner: H, filter: impl Fn(&Event) -> bool + Send + Sync + 'static) -> Self {
        Self {
            inner,
            filter: Box::new(filter),
        }
    }
}

#[async_trait]
impl<H: EventHandler> EventHandler for FilteredHandler<H> {
    fn id(&self) -> &str {
        self.inner.id()
    }

    async fn handle(&self, event: &Event) -> Result<(), EventError> {
        if (self.filter)(event) {
            self.inner.handle(event).await
        } else {
            Ok(())
        }
    }
}
