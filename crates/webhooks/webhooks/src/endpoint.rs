//! Webhook endpoint configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Webhook endpoint configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    /// Unique identifier.
    pub id: String,
    /// Target URL.
    pub url: String,
    /// Secret for signing payloads.
    pub secret: String,
    /// Event filter.
    pub events: EventFilter,
    /// Whether this endpoint is enabled.
    pub enabled: bool,
    /// Endpoint metadata.
    pub metadata: WebhookMetadata,
}

impl WebhookEndpoint {
    /// Creates a new webhook endpoint.
    pub fn new(url: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            url: url.into(),
            secret: secret.into(),
            events: EventFilter::All,
            enabled: true,
            metadata: WebhookMetadata::default(),
        }
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
        self.metadata.description = Some(desc.into());
        self
    }

    /// Adds a custom header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.headers.insert(key.into(), value.into());
        self
    }

    /// Sets the timeout in milliseconds.
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.metadata.timeout_ms = timeout;
        self
    }

    /// Disables the endpoint.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Checks if this endpoint should receive an event.
    pub fn should_receive(&self, event_type: &str) -> bool {
        if !self.enabled {
            return false;
        }

        self.events.matches(event_type)
    }
}

/// Filter for which events a webhook receives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventFilter {
    /// Receive all events.
    All,
    /// Receive specific events.
    Specific(HashSet<String>),
    /// Receive events matching patterns (supports wildcards like "user.*").
    Pattern(Vec<String>),
}

impl EventFilter {
    /// Checks if an event type matches this filter.
    pub fn matches(&self, event_type: &str) -> bool {
        match self {
            EventFilter::All => true,
            EventFilter::Specific(events) => events.contains(event_type),
            EventFilter::Pattern(patterns) => {
                for pattern in patterns {
                    if pattern == "*" {
                        return true;
                    }
                    if pattern.ends_with(".*") {
                        let prefix = &pattern[..pattern.len() - 2];
                        if event_type.starts_with(prefix) {
                            return true;
                        }
                    } else if pattern == event_type {
                        return true;
                    }
                }
                false
            }
        }
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        EventFilter::All
    }
}

/// Metadata associated with a webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookMetadata {
    /// Human-readable description.
    pub description: Option<String>,
    /// Custom headers to include in requests.
    pub headers: HashMap<String, String>,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// When the endpoint was created.
    pub created_at: DateTime<Utc>,
    /// When the endpoint was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Default for WebhookMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            description: None,
            headers: HashMap::new(),
            timeout_ms: 30000, // 30 seconds
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_filter_all() {
        let filter = EventFilter::All;
        assert!(filter.matches("user.created"));
        assert!(filter.matches("anything"));
    }

    #[test]
    fn test_event_filter_specific() {
        let filter = EventFilter::Specific(
            vec!["user.created", "user.deleted"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
        assert!(filter.matches("user.created"));
        assert!(filter.matches("user.deleted"));
        assert!(!filter.matches("user.updated"));
        assert!(!filter.matches("session.created"));
    }

    #[test]
    fn test_event_filter_pattern() {
        let filter = EventFilter::Pattern(vec!["user.*".to_string(), "session.created".to_string()]);
        assert!(filter.matches("user.created"));
        assert!(filter.matches("user.deleted"));
        assert!(filter.matches("user.anything"));
        assert!(filter.matches("session.created"));
        assert!(!filter.matches("session.destroyed"));
    }

    #[test]
    fn test_endpoint_should_receive() {
        let endpoint = WebhookEndpoint::new("https://example.com", "secret")
            .patterns(["user.*", "session.created"]);

        assert!(endpoint.should_receive("user.created"));
        assert!(endpoint.should_receive("session.created"));
        assert!(!endpoint.should_receive("session.destroyed"));

        let disabled = endpoint.clone().disabled();
        assert!(!disabled.should_receive("user.created"));
    }
}
