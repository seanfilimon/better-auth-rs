//! Event types and structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// An event that can be emitted and handled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for this event instance.
    pub id: String,
    /// The event type (namespace + name + version).
    pub event_type: EventType,
    /// The event payload.
    pub payload: Value,
    /// Event metadata.
    pub metadata: EventMetadata,
    /// Timestamp when the event was created.
    pub timestamp: DateTime<Utc>,
    /// Optional correlation ID for tracing related events.
    pub correlation_id: Option<String>,
    /// Optional causation ID linking to the event that caused this one.
    pub causation_id: Option<String>,
}

impl Event {
    /// Creates a new event with the given type and payload.
    pub fn new(event_type: EventType, payload: impl Serialize) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            payload: serde_json::to_value(payload).unwrap_or(Value::Null),
            metadata: EventMetadata::default(),
            timestamp: Utc::now(),
            correlation_id: None,
            causation_id: None,
        }
    }

    /// Creates a new event from a simple type string (e.g., "user.created").
    pub fn simple(event_type: impl Into<String>, payload: impl Serialize) -> Self {
        Self::new(EventType::from_string(event_type), payload)
    }

    /// Sets the correlation ID for tracing.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Sets the causation ID linking to a parent event.
    pub fn with_causation_id(mut self, id: impl Into<String>) -> Self {
        self.causation_id = Some(id.into());
        self
    }

    /// Sets the source in metadata.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.metadata.source = source.into();
        self
    }

    /// Adds a tag to the event metadata.
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.tags.insert(key.into(), value.into());
        self
    }

    /// Returns the full event type string (e.g., "user.created.v1").
    pub fn type_string(&self) -> String {
        self.event_type.to_string()
    }

    /// Returns the simple type string without version (e.g., "user.created").
    pub fn simple_type_string(&self) -> String {
        self.event_type.simple_string()
    }

    /// Deserializes the payload to a specific type.
    pub fn payload_as<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        serde_json::from_value(self.payload.clone()).ok()
    }

    /// Creates a child event caused by this event.
    pub fn child(&self, event_type: EventType, payload: impl Serialize) -> Self {
        Self::new(event_type, payload)
            .with_causation_id(&self.id)
            .with_correlation_id(
                self.correlation_id
                    .clone()
                    .unwrap_or_else(|| self.id.clone()),
            )
    }
}

/// Event type identifier with namespace, name, and version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventType {
    /// Namespace (e.g., "auth", "user", "session").
    pub namespace: String,
    /// Event name (e.g., "created", "updated", "deleted").
    pub name: String,
    /// Schema version for this event type.
    pub version: u32,
}

impl EventType {
    /// Creates a new event type.
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version: 1,
        }
    }

    /// Creates an event type with a specific version.
    pub fn versioned(namespace: impl Into<String>, name: impl Into<String>, version: u32) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version,
        }
    }

    /// Parses an event type from a string like "user.created" or "user.created.v2".
    pub fn from_string(s: impl Into<String>) -> Self {
        let s = s.into();
        let parts: Vec<&str> = s.split('.').collect();

        match parts.len() {
            0 => Self::new("unknown", "unknown"),
            1 => Self::new("unknown", parts[0]),
            2 => Self::new(parts[0], parts[1]),
            _ => {
                // Check if last part is a version (e.g., "v2")
                if let Some(version_str) = parts.last() {
                    if version_str.starts_with('v') {
                        if let Ok(v) = version_str[1..].parse::<u32>() {
                            let name = parts[1..parts.len() - 1].join(".");
                            return Self::versioned(parts[0], name, v);
                        }
                    }
                }
                Self::new(parts[0], parts[1..].join("."))
            }
        }
    }

    /// Returns the full string representation (e.g., "user.created.v1").
    pub fn to_string(&self) -> String {
        format!("{}.{}.v{}", self.namespace, self.name, self.version)
    }

    /// Returns the simple string without version (e.g., "user.created").
    pub fn simple_string(&self) -> String {
        format!("{}.{}", self.namespace, self.name)
    }

    /// Checks if this event type matches a pattern (supports wildcards).
    pub fn matches(&self, pattern: &str) -> bool {
        let simple = self.simple_string();

        if pattern == "*" {
            return true;
        }

        if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            return simple.starts_with(prefix);
        }

        simple == pattern || self.to_string() == pattern
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.v{}", self.namespace, self.name, self.version)
    }
}

/// Metadata associated with an event.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventMetadata {
    /// Source plugin/system that emitted the event.
    pub source: String,
    /// Schema version string.
    pub schema_version: String,
    /// Custom tags for filtering and routing.
    pub tags: HashMap<String, String>,
}

impl EventMetadata {
    /// Creates new metadata with a source.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            schema_version: "1.0".to_string(),
            tags: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_parsing() {
        let et = EventType::from_string("user.created");
        assert_eq!(et.namespace, "user");
        assert_eq!(et.name, "created");
        assert_eq!(et.version, 1);

        let et = EventType::from_string("user.created.v2");
        assert_eq!(et.namespace, "user");
        assert_eq!(et.name, "created");
        assert_eq!(et.version, 2);

        let et = EventType::from_string("auth.password.reset.v3");
        assert_eq!(et.namespace, "auth");
        assert_eq!(et.name, "password.reset");
        assert_eq!(et.version, 3);
    }

    #[test]
    fn test_event_type_matching() {
        let et = EventType::new("user", "created");

        assert!(et.matches("user.created"));
        assert!(et.matches("user.*"));
        assert!(et.matches("*"));
        assert!(!et.matches("session.created"));
        assert!(!et.matches("session.*"));
    }

    #[test]
    fn test_event_creation() {
        let event = Event::simple("user.created", serde_json::json!({"user_id": "123"}));

        assert_eq!(event.event_type.namespace, "user");
        assert_eq!(event.event_type.name, "created");
        assert!(!event.id.is_empty());
    }

    #[test]
    fn test_event_chaining() {
        let parent = Event::simple("user.created", serde_json::json!({"user_id": "123"}))
            .with_correlation_id("trace-123");

        let child = parent.child(
            EventType::new("email", "sent"),
            serde_json::json!({"email": "test@example.com"}),
        );

        assert_eq!(child.causation_id, Some(parent.id.clone()));
        assert_eq!(child.correlation_id, Some("trace-123".to_string()));
    }
}
