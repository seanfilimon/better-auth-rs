//! Event registry for discovery and validation.

use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::EventType;

/// Definition of an event type for the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDefinition {
    /// The event type.
    pub event_type: EventType,
    /// JSON Schema for the payload (optional).
    pub payload_schema: Option<Value>,
    /// Human-readable description.
    pub description: String,
    /// Source plugin/system that defines this event.
    pub source: String,
    /// Whether this event is deprecated.
    pub deprecated: bool,
    /// Deprecation message if deprecated.
    pub deprecation_message: Option<String>,
}

impl EventDefinition {
    /// Creates a new event definition.
    pub fn new(
        event_type: EventType,
        description: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            event_type,
            payload_schema: None,
            description: description.into(),
            source: source.into(),
            deprecated: false,
            deprecation_message: None,
        }
    }

    /// Creates a simple event definition from a type string.
    pub fn simple(
        event_type: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::new(EventType::from_string(event_type), description, source)
    }

    /// Sets the payload schema.
    pub fn with_schema(mut self, schema: Value) -> Self {
        self.payload_schema = Some(schema);
        self
    }

    /// Marks the event as deprecated.
    pub fn deprecated(mut self, message: impl Into<String>) -> Self {
        self.deprecated = true;
        self.deprecation_message = Some(message.into());
        self
    }

    /// Returns the full event type string.
    pub fn type_string(&self) -> String {
        self.event_type.to_string()
    }

    /// Returns the simple event type string.
    pub fn simple_type_string(&self) -> String {
        self.event_type.simple_string()
    }
}

/// Registry for event definitions.
pub struct EventRegistry {
    /// Registered event definitions.
    definitions: RwLock<HashMap<String, EventDefinition>>,
}

impl EventRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            definitions: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a registry with standard auth events pre-registered.
    pub fn with_standard_events() -> Self {
        let registry = Self::new();

        // Register standard auth events
        let standard_events = vec![
            EventDefinition::simple("user.created", "Emitted when a new user is created", "core"),
            EventDefinition::simple("user.updated", "Emitted when a user is updated", "core"),
            EventDefinition::simple("user.deleted", "Emitted when a user is deleted", "core"),
            EventDefinition::simple(
                "session.created",
                "Emitted when a new session is created",
                "core",
            ),
            EventDefinition::simple(
                "session.destroyed",
                "Emitted when a session is destroyed",
                "core",
            ),
            EventDefinition::simple(
                "signin.success",
                "Emitted on successful authentication",
                "core",
            ),
            EventDefinition::simple(
                "signin.failed",
                "Emitted on failed authentication attempt",
                "core",
            ),
            EventDefinition::simple("signup.success", "Emitted on successful signup", "core"),
            EventDefinition::simple(
                "email.verified",
                "Emitted when email is verified",
                "core",
            ),
            EventDefinition::simple(
                "password.changed",
                "Emitted when password is changed",
                "core",
            ),
            EventDefinition::simple(
                "password.reset_requested",
                "Emitted when password reset is requested",
                "core",
            ),
        ];

        for def in standard_events {
            registry.register(def);
        }

        registry
    }

    /// Registers an event definition.
    pub fn register(&self, definition: EventDefinition) {
        let mut defs = self.definitions.write().unwrap();
        defs.insert(definition.simple_type_string(), definition);
    }

    /// Registers multiple event definitions.
    pub fn register_all(&self, definitions: impl IntoIterator<Item = EventDefinition>) {
        let mut defs = self.definitions.write().unwrap();
        for def in definitions {
            defs.insert(def.simple_type_string(), def);
        }
    }

    /// Gets an event definition by type string.
    pub fn get(&self, event_type: &str) -> Option<EventDefinition> {
        let defs = self.definitions.read().unwrap();
        defs.get(event_type).cloned()
    }

    /// Checks if an event type is registered.
    pub fn is_registered(&self, event_type: &str) -> bool {
        let defs = self.definitions.read().unwrap();
        defs.contains_key(event_type)
    }

    /// Returns all registered event definitions.
    pub fn all(&self) -> Vec<EventDefinition> {
        let defs = self.definitions.read().unwrap();
        defs.values().cloned().collect()
    }

    /// Returns event definitions from a specific source.
    pub fn by_source(&self, source: &str) -> Vec<EventDefinition> {
        let defs = self.definitions.read().unwrap();
        defs.values()
            .filter(|d| d.source == source)
            .cloned()
            .collect()
    }

    /// Returns event definitions matching a namespace.
    pub fn by_namespace(&self, namespace: &str) -> Vec<EventDefinition> {
        let defs = self.definitions.read().unwrap();
        defs.values()
            .filter(|d| d.event_type.namespace == namespace)
            .cloned()
            .collect()
    }

    /// Returns deprecated event definitions.
    pub fn deprecated(&self) -> Vec<EventDefinition> {
        let defs = self.definitions.read().unwrap();
        defs.values().filter(|d| d.deprecated).cloned().collect()
    }

    /// Unregisters an event definition.
    pub fn unregister(&self, event_type: &str) -> Option<EventDefinition> {
        let mut defs = self.definitions.write().unwrap();
        defs.remove(event_type)
    }

    /// Clears all registered events.
    pub fn clear(&self) {
        let mut defs = self.definitions.write().unwrap();
        defs.clear();
    }

    /// Returns the number of registered events.
    pub fn len(&self) -> usize {
        let defs = self.definitions.read().unwrap();
        defs.len()
    }

    /// Checks if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_registration() {
        let registry = EventRegistry::new();

        registry.register(EventDefinition::simple(
            "user.created",
            "User created event",
            "test",
        ));

        assert!(registry.is_registered("user.created"));
        assert!(!registry.is_registered("user.deleted"));

        let def = registry.get("user.created").unwrap();
        assert_eq!(def.description, "User created event");
        assert_eq!(def.source, "test");
    }

    #[test]
    fn test_standard_events() {
        let registry = EventRegistry::with_standard_events();

        assert!(registry.is_registered("user.created"));
        assert!(registry.is_registered("session.created"));
        assert!(registry.is_registered("signin.success"));
    }

    #[test]
    fn test_by_namespace() {
        let registry = EventRegistry::with_standard_events();

        let user_events = registry.by_namespace("user");
        assert_eq!(user_events.len(), 3); // created, updated, deleted

        let session_events = registry.by_namespace("session");
        assert_eq!(session_events.len(), 2); // created, destroyed
    }
}
