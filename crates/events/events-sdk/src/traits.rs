//! Traits for plugin event integration.

use std::sync::Arc;

use better_auth_events::{Event, EventBus, EventDefinition, EventHandler, EventError};

/// Trait for plugins that emit events.
///
/// Implement this trait to declare which events your plugin emits.
/// This enables event discovery and documentation generation.
pub trait EventProvider {
    /// Returns the event definitions this plugin emits.
    ///
    /// These definitions are registered with the event registry
    /// for discovery and validation.
    fn provided_events() -> Vec<EventDefinition>;

    /// Returns the plugin identifier used as the event source.
    fn event_source() -> &'static str;
}

/// Trait for plugins that subscribe to events.
///
/// Implement this trait to declare which events your plugin listens to.
pub trait EventSubscriber {
    /// Returns the event patterns this plugin subscribes to.
    ///
    /// Patterns can be:
    /// - Exact match: "user.created"
    /// - Namespace wildcard: "user.*"
    /// - All events: "*"
    fn subscribed_events() -> Vec<String>;

    /// Creates event handlers for the subscribed events.
    ///
    /// Override this to provide custom handlers. The default implementation
    /// returns an empty vector.
    fn create_handlers(&self) -> Vec<(String, Box<dyn EventHandler>)> {
        Vec::new()
    }
}

/// Trait for plugins that can emit events.
///
/// This trait provides a convenient interface for plugins to emit events
/// with proper source metadata.
pub trait PluginEventEmitter: EventProvider + Sync {
    /// Returns a reference to the event bus.
    fn event_bus(&self) -> &EventBus;

    /// Emits an event with the plugin's source metadata.
    fn emit_event(&self, event: Event) -> impl std::future::Future<Output = ()> + Send {
        let event = event.with_source(Self::event_source());
        async move {
            self.event_bus().emit(event).await;
        }
    }

    /// Emits a simple event with type string and payload.
    fn emit_simple(
        &self,
        event_type: impl Into<String>,
        payload: impl serde::Serialize,
    ) -> impl std::future::Future<Output = ()> + Send {
        let event = Event::simple(event_type, payload).with_source(Self::event_source());
        async move {
            self.event_bus().emit(event).await;
        }
    }

    /// Emits an event and waits for all handlers to complete.
    fn emit_sync(
        &self,
        event: Event,
    ) -> impl std::future::Future<Output = Vec<better_auth_events::HandlerResult>> + Send {
        let event = event.with_source(Self::event_source());
        async move { self.event_bus().emit_sync(event).await }
    }

    /// Emits an event and returns an error if any handler fails.
    fn emit_checked(
        &self,
        event: Event,
    ) -> impl std::future::Future<Output = Result<(), EventError>> + Send {
        let event = event.with_source(Self::event_source());
        async move { self.event_bus().emit_checked(event).await }
    }
}

/// Helper struct for registering plugin events and handlers.
pub struct PluginEventRegistrar {
    bus: Arc<EventBus>,
    registry: Arc<better_auth_events::EventRegistry>,
}

impl PluginEventRegistrar {
    /// Creates a new registrar.
    pub fn new(bus: Arc<EventBus>, registry: Arc<better_auth_events::EventRegistry>) -> Self {
        Self { bus, registry }
    }

    /// Registers a plugin's provided events.
    pub fn register_provider<P: EventProvider>(&self) {
        for def in P::provided_events() {
            self.registry.register(def);
        }
    }

    /// Registers a plugin's event subscriptions.
    pub async fn register_subscriber<S: EventSubscriber>(&self, subscriber: &S) {
        for (pattern, handler) in subscriber.create_handlers() {
            self.bus.on(&pattern, HandlerWrapper(handler)).await;
        }
    }

    /// Returns the event bus.
    pub fn bus(&self) -> &EventBus {
        &self.bus
    }

    /// Returns the event registry.
    pub fn registry(&self) -> &better_auth_events::EventRegistry {
        &self.registry
    }
}

/// Wrapper to make Box<dyn EventHandler> implement EventHandler.
struct HandlerWrapper(Box<dyn EventHandler>);

#[async_trait::async_trait]
impl EventHandler for HandlerWrapper {
    fn id(&self) -> &str {
        self.0.id()
    }

    async fn handle(&self, event: &Event) -> Result<(), EventError> {
        self.0.handle(event).await
    }
}

/// Macro helper for implementing EventProvider.
///
/// This is used by the `#[derive(EventPayload)]` macro to generate
/// event definitions automatically.
pub struct EventProviderHelper;

impl EventProviderHelper {
    /// Creates an event definition from components.
    pub fn create_definition(
        namespace: &str,
        name: &str,
        description: &str,
        source: &str,
    ) -> EventDefinition {
        EventDefinition::simple(format!("{}.{}", namespace, name), description, source)
    }
}
