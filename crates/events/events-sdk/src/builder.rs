//! Event builder utilities for plugins.

use better_auth_events::{Event, EventType, EventMetadata};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// Builder for creating events with plugin-specific defaults.
pub struct PluginEventBuilder {
    source: String,
    default_namespace: Option<String>,
    default_tags: HashMap<String, String>,
    correlation_id: Option<String>,
}

impl PluginEventBuilder {
    /// Creates a new plugin event builder.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            default_namespace: None,
            default_tags: HashMap::new(),
            correlation_id: None,
        }
    }

    /// Sets the default namespace for events.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.default_namespace = Some(namespace.into());
        self
    }

    /// Adds a default tag to all events.
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_tags.insert(key.into(), value.into());
        self
    }

    /// Sets the correlation ID for event tracing.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Builds an event with the configured defaults.
    pub fn build(&self, event_type: EventType, payload: impl Serialize) -> Event {
        let mut event = Event::new(event_type, payload).with_source(&self.source);

        // Apply default tags
        for (key, value) in &self.default_tags {
            event = event.with_tag(key, value);
        }

        // Apply correlation ID if set
        if let Some(ref correlation_id) = self.correlation_id {
            event = event.with_correlation_id(correlation_id);
        }

        event
    }

    /// Builds a simple event using the default namespace.
    pub fn build_simple(&self, name: impl Into<String>, payload: impl Serialize) -> Event {
        let name = name.into();
        let namespace = self
            .default_namespace
            .clone()
            .unwrap_or_else(|| self.source.clone());

        self.build(EventType::new(namespace, name), payload)
    }

    /// Creates a child builder with a specific correlation ID.
    pub fn child(&self, correlation_id: impl Into<String>) -> Self {
        Self {
            source: self.source.clone(),
            default_namespace: self.default_namespace.clone(),
            default_tags: self.default_tags.clone(),
            correlation_id: Some(correlation_id.into()),
        }
    }
}

/// Builder for constructing event payloads.
pub struct EventPayloadBuilder {
    data: HashMap<String, Value>,
}

impl EventPayloadBuilder {
    /// Creates a new payload builder.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Adds a field to the payload.
    pub fn field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.data.insert(
            key.into(),
            serde_json::to_value(value).unwrap_or(Value::Null),
        );
        self
    }

    /// Adds a string field.
    pub fn string(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.field(key, value.into())
    }

    /// Adds an integer field.
    pub fn int(self, key: impl Into<String>, value: i64) -> Self {
        self.field(key, value)
    }

    /// Adds a boolean field.
    pub fn bool(self, key: impl Into<String>, value: bool) -> Self {
        self.field(key, value)
    }

    /// Adds a nested object field.
    pub fn object(self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.field(key, value)
    }

    /// Adds an optional field (only if Some).
    pub fn optional<T: Serialize>(self, key: impl Into<String>, value: Option<T>) -> Self {
        if let Some(v) = value {
            self.field(key, v)
        } else {
            self
        }
    }

    /// Builds the payload as a JSON Value.
    pub fn build(self) -> Value {
        serde_json::to_value(self.data).unwrap_or(Value::Null)
    }

    /// Builds the payload and creates an event.
    pub fn into_event(self, event_type: EventType) -> Event {
        Event::new(event_type, self.build())
    }

    /// Builds the payload and creates a simple event.
    pub fn into_simple_event(self, event_type: impl Into<String>) -> Event {
        Event::simple(event_type, self.build())
    }
}

impl Default for EventPayloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can be converted to event payloads.
pub trait IntoEventPayload {
    /// Converts this type into a JSON Value payload.
    fn into_payload(self) -> Value;
}

impl<T: Serialize> IntoEventPayload for T {
    fn into_payload(self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_event_builder() {
        let builder = PluginEventBuilder::new("test-plugin")
            .with_namespace("test")
            .with_tag("env", "test");

        let event = builder.build_simple("action", serde_json::json!({"key": "value"}));

        assert_eq!(event.event_type.namespace, "test");
        assert_eq!(event.event_type.name, "action");
        assert_eq!(event.metadata.source, "test-plugin");
        assert_eq!(event.metadata.tags.get("env"), Some(&"test".to_string()));
    }

    #[test]
    fn test_payload_builder() {
        let payload = EventPayloadBuilder::new()
            .string("user_id", "123")
            .string("email", "test@example.com")
            .bool("verified", true)
            .optional("name", Some("Test User"))
            .optional::<String>("nickname", None)
            .build();

        assert_eq!(payload["user_id"], "123");
        assert_eq!(payload["email"], "test@example.com");
        assert_eq!(payload["verified"], true);
        assert_eq!(payload["name"], "Test User");
        assert!(payload.get("nickname").is_none());
    }
}
