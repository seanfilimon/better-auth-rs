//! Event emitter trait for plugins and systems.

use std::sync::Arc;

use crate::bus::EventBus;
use crate::event::{Event, EventType};

/// Trait for types that can emit events.
pub trait EventEmitter: Send + Sync {
    /// Returns a reference to the event bus.
    fn event_bus(&self) -> &EventBus;

    /// Emits an event asynchronously (fire and forget).
    fn emit(&self, event: Event) -> impl std::future::Future<Output = ()> + Send {
        async move {
            self.event_bus().emit(event).await;
        }
    }

    /// Emits an event and waits for all handlers.
    fn emit_sync(
        &self,
        event: Event,
    ) -> impl std::future::Future<Output = Vec<crate::handler::HandlerResult>> + Send {
        async move { self.event_bus().emit_sync(event).await }
    }

    /// Emits a simple event with type string and payload.
    fn emit_simple(
        &self,
        event_type: impl Into<String>,
        payload: impl serde::Serialize,
    ) -> impl std::future::Future<Output = ()> + Send {
        let event = Event::simple(event_type, payload);
        async move {
            self.event_bus().emit(event).await;
        }
    }
}

/// A shared event emitter that can be cloned.
#[derive(Clone)]
pub struct SharedEventEmitter {
    bus: Arc<EventBus>,
}

impl SharedEventEmitter {
    /// Creates a new shared event emitter.
    pub fn new(bus: Arc<EventBus>) -> Self {
        Self { bus }
    }

    /// Creates a new shared event emitter with a new event bus.
    pub fn with_new_bus() -> Self {
        Self {
            bus: Arc::new(EventBus::new()),
        }
    }

    /// Returns the inner event bus.
    pub fn bus(&self) -> &EventBus {
        &self.bus
    }

    /// Returns the Arc to the event bus.
    pub fn bus_arc(&self) -> Arc<EventBus> {
        self.bus.clone()
    }
}

impl EventEmitter for SharedEventEmitter {
    fn event_bus(&self) -> &EventBus {
        &self.bus
    }
}

/// Builder for creating events with a specific source.
pub struct EventBuilder {
    source: String,
    correlation_id: Option<String>,
}

impl EventBuilder {
    /// Creates a new event builder with a source.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            correlation_id: None,
        }
    }

    /// Sets the correlation ID for all events.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Builds an event with the configured source.
    pub fn build(&self, event_type: EventType, payload: impl serde::Serialize) -> Event {
        let mut event = Event::new(event_type, payload).with_source(&self.source);

        if let Some(ref correlation_id) = self.correlation_id {
            event = event.with_correlation_id(correlation_id);
        }

        event
    }

    /// Builds a simple event with the configured source.
    pub fn build_simple(
        &self,
        event_type: impl Into<String>,
        payload: impl serde::Serialize,
    ) -> Event {
        let mut event = Event::simple(event_type, payload).with_source(&self.source);

        if let Some(ref correlation_id) = self.correlation_id {
            event = event.with_correlation_id(correlation_id);
        }

        event
    }
}
