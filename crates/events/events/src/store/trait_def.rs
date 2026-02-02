use crate::{Event, EventResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Unique identifier for an event in the store
pub type EventId = uuid::Uuid;

/// Version number for events in a stream
pub type StreamVersion = u32;

/// Trait for persistent event storage
///
/// Implementations provide durable storage for events with support for:
/// - Event streams with versioning
/// - Correlation and causation tracking
/// - Time-based querying
/// - Stream subscriptions
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append an event to the store
    ///
    /// # Arguments
    ///
    /// * `event` - The event to append
    ///
    /// # Returns
    ///
    /// The unique ID assigned to the stored event
    async fn append(&self, event: &Event) -> EventResult<EventId>;

    /// Append multiple events atomically
    ///
    /// All events must belong to the same stream for atomicity guarantees.
    async fn append_batch(&self, events: &[Event]) -> EventResult<Vec<EventId>>;

    /// Retrieve a single event by ID
    async fn get(&self, id: &EventId) -> EventResult<Option<StoredEvent>>;

    /// Get all events in a stream
    ///
    /// # Arguments
    ///
    /// * `stream_id` - The stream identifier
    /// * `from_version` - Optional starting version (inclusive)
    async fn get_stream(
        &self,
        stream_id: &str,
        from_version: Option<StreamVersion>,
    ) -> EventResult<Vec<StoredEvent>>;

    /// Get events by correlation ID
    ///
    /// Returns all events that share the same correlation ID,
    /// useful for tracing related events across streams.
    async fn get_by_correlation(&self, correlation_id: &str) -> EventResult<Vec<StoredEvent>>;

    /// Query events with filtering and pagination
    async fn query(&self, query: EventQuery) -> EventResult<EventStream>;

    /// Subscribe to new events in a stream
    ///
    /// Returns a receiver that will receive all new events appended to the stream.
    async fn subscribe_to_stream(
        &self,
        stream_id: &str,
    ) -> EventResult<EventStreamSubscription>;

    /// Get the current version of a stream
    async fn get_stream_version(&self, stream_id: &str) -> EventResult<Option<StreamVersion>>;

    /// Create a snapshot of a stream at a specific version
    async fn create_snapshot(
        &self,
        stream_id: &str,
        version: StreamVersion,
        state: serde_json::Value,
    ) -> EventResult<()>;

    /// Get the latest snapshot for a stream
    async fn get_latest_snapshot(
        &self,
        stream_id: &str,
    ) -> EventResult<Option<EventSnapshot>>;

    /// Delete events from a stream up to a specific version
    ///
    /// Used for stream truncation after creating snapshots.
    async fn truncate_stream(
        &self,
        stream_id: &str,
        before_version: StreamVersion,
    ) -> EventResult<()>;
}

/// An event with storage metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    /// Unique storage ID
    pub id: EventId,
    
    /// The actual event
    pub event: Event,
    
    /// Stream this event belongs to
    pub stream_id: String,
    
    /// Version within the stream
    pub version: StreamVersion,
    
    /// When the event was stored
    pub stored_at: DateTime<Utc>,
}

/// Query parameters for event retrieval
#[derive(Debug, Clone, Default)]
pub struct EventQuery {
    /// Filter by event types (namespace.action)
    pub event_types: Vec<String>,
    
    /// Filter by stream IDs
    pub stream_ids: Vec<String>,
    
    /// Filter events after this timestamp
    pub start_time: Option<DateTime<Utc>>,
    
    /// Filter events before this timestamp
    pub end_time: Option<DateTime<Utc>>,
    
    /// Maximum number of events to return
    pub limit: Option<usize>,
    
    /// Number of events to skip
    pub offset: Option<usize>,
    
    /// Sort order for results
    pub ordering: EventOrdering,
}

/// Sort order for event queries
#[derive(Debug, Clone, Copy, Default)]
pub enum EventOrdering {
    /// Oldest events first
    #[default]
    Ascending,
    
    /// Newest events first
    Descending,
}

/// Stream of events from a query
pub struct EventStream {
    events: Vec<StoredEvent>,
    cursor: usize,
}

impl EventStream {
    pub fn new(events: Vec<StoredEvent>) -> Self {
        Self { events, cursor: 0 }
    }

    pub fn next(&mut self) -> Option<&StoredEvent> {
        if self.cursor < self.events.len() {
            let event = &self.events[self.cursor];
            self.cursor += 1;
            Some(event)
        } else {
            None
        }
    }

    pub fn events(&self) -> &[StoredEvent] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Subscription to a stream of events
pub struct EventStreamSubscription {
    receiver: mpsc::UnboundedReceiver<StoredEvent>,
    stream_id: String,
}

impl EventStreamSubscription {
    pub fn new(stream_id: String) -> (Self, mpsc::UnboundedSender<StoredEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                receiver: rx,
                stream_id,
            },
            tx,
        )
    }

    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    pub async fn recv(&mut self) -> Option<StoredEvent> {
        self.receiver.recv().await
    }
}

/// A snapshot of stream state at a specific version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSnapshot {
    pub stream_id: String,
    pub version: StreamVersion,
    pub state: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_stream_iteration() {
        let events = vec![];
        let mut stream = EventStream::new(events);
        assert_eq!(stream.len(), 0);
        assert!(stream.is_empty());
        assert!(stream.next().is_none());
    }

    #[test]
    fn test_event_ordering() {
        let asc = EventOrdering::Ascending;
        let desc = EventOrdering::Descending;
        
        // Just verify they can be created and used
        match asc {
            EventOrdering::Ascending => {}
            EventOrdering::Descending => panic!("Wrong variant"),
        }
        
        match desc {
            EventOrdering::Descending => {}
            EventOrdering::Ascending => panic!("Wrong variant"),
        }
    }
}
