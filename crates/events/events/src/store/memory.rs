use super::trait_def::*;
use crate::{Event, EventResult, EventError};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// In-memory implementation of EventStore
///
/// Stores all events in memory. Useful for testing and development.
/// Data is lost when the process exits.
pub struct MemoryEventStore {
    events: Arc<RwLock<Vec<StoredEvent>>>,
    streams: Arc<RwLock<HashMap<String, StreamVersion>>>,
    snapshots: Arc<RwLock<HashMap<String, EventSnapshot>>>,
    subscriptions: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<StoredEvent>>>>>,
}

impl MemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            streams: Arc::new(RwLock::new(HashMap::new())),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn notify_subscribers(&self, stored_event: &StoredEvent) {
        let subs = self.subscriptions.read().await;
        if let Some(subscribers) = subs.get(&stored_event.stream_id) {
            for tx in subscribers {
                let _ = tx.send(stored_event.clone());
            }
        }
    }
}

impl Default for MemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventStore for MemoryEventStore {
    async fn append(&self, event: &Event) -> EventResult<EventId> {
        let id = EventId::new_v4();
        let stream_id = event.metadata.source.clone();
        
        // Get and increment stream version
        let mut streams = self.streams.write().await;
        let version = streams.entry(stream_id.clone())
            .and_modify(|v| *v += 1)
            .or_insert(1);
        let version = *version;
        drop(streams);

        let stored_event = StoredEvent {
            id,
            event: event.clone(),
            stream_id,
            version,
            stored_at: Utc::now(),
        };

        // Store event
        let mut events = self.events.write().await;
        events.push(stored_event.clone());
        drop(events);

        // Notify subscribers
        self.notify_subscribers(&stored_event).await;

        Ok(id)
    }

    async fn append_batch(&self, events: &[Event]) -> EventResult<Vec<EventId>> {
        if events.is_empty() {
            return Ok(Vec::new());
        }

        // Verify all events are from the same stream
        let first_stream = &events[0].metadata.source;
        if !events.iter().all(|e| &e.metadata.source == first_stream) {
            return Err(EventError::InvalidInput(
                "All events in batch must belong to the same stream".into()
            ));
        }

        let mut ids = Vec::with_capacity(events.len());
        for event in events {
            let id = self.append(event).await?;
            ids.push(id);
        }

        Ok(ids)
    }

    async fn get(&self, id: &EventId) -> EventResult<Option<StoredEvent>> {
        let events = self.events.read().await;
        Ok(events.iter().find(|e| &e.id == id).cloned())
    }

    async fn get_stream(
        &self,
        stream_id: &str,
        from_version: Option<StreamVersion>,
    ) -> EventResult<Vec<StoredEvent>> {
        let events = self.events.read().await;
        let from = from_version.unwrap_or(1);
        
        Ok(events
            .iter()
            .filter(|e| e.stream_id == stream_id && e.version >= from)
            .cloned()
            .collect())
    }

    async fn get_by_correlation(&self, correlation_id: &str) -> EventResult<Vec<StoredEvent>> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| {
                e.event.correlation_id.as_deref() == Some(correlation_id)
            })
            .cloned()
            .collect())
    }

    async fn query(&self, query: EventQuery) -> EventResult<EventStream> {
        let events = self.events.read().await;
        
        let mut filtered: Vec<StoredEvent> = events
            .iter()
            .filter(|e| {
                // Filter by event types
                if !query.event_types.is_empty() {
                    let event_type_full = e.event.event_type.to_string();
                    let event_type_simple = e.event.event_type.simple_string();
                    if !query.event_types.iter().any(|et| {
                        et == &event_type_full || et == &event_type_simple
                    }) {
                        return false;
                    }
                }

                // Filter by stream IDs
                if !query.stream_ids.is_empty() {
                    if !query.stream_ids.contains(&e.stream_id) {
                        return false;
                    }
                }

                // Filter by start time
                if let Some(start) = query.start_time {
                    if e.event.timestamp < start {
                        return false;
                    }
                }

                // Filter by end time
                if let Some(end) = query.end_time {
                    if e.event.timestamp > end {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by ordering
        match query.ordering {
            EventOrdering::Ascending => {
                filtered.sort_by_key(|e| e.event.timestamp);
            }
            EventOrdering::Descending => {
                filtered.sort_by_key(|e| std::cmp::Reverse(e.event.timestamp));
            }
        }

        // Apply pagination
        if let Some(offset) = query.offset {
            filtered = filtered.into_iter().skip(offset).collect();
        }
        if let Some(limit) = query.limit {
            filtered.truncate(limit);
        }

        Ok(EventStream::new(filtered))
    }

    async fn subscribe_to_stream(
        &self,
        stream_id: &str,
    ) -> EventResult<EventStreamSubscription> {
        let (subscription, tx) = EventStreamSubscription::new(stream_id.to_string());
        
        let mut subs = self.subscriptions.write().await;
        subs.entry(stream_id.to_string())
            .or_insert_with(Vec::new)
            .push(tx);

        Ok(subscription)
    }

    async fn get_stream_version(&self, stream_id: &str) -> EventResult<Option<StreamVersion>> {
        let streams = self.streams.read().await;
        Ok(streams.get(stream_id).copied())
    }

    async fn create_snapshot(
        &self,
        stream_id: &str,
        version: StreamVersion,
        state: serde_json::Value,
    ) -> EventResult<()> {
        let snapshot = EventSnapshot {
            stream_id: stream_id.to_string(),
            version,
            state,
            created_at: Utc::now(),
        };

        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(stream_id.to_string(), snapshot);

        Ok(())
    }

    async fn get_latest_snapshot(
        &self,
        stream_id: &str,
    ) -> EventResult<Option<EventSnapshot>> {
        let snapshots = self.snapshots.read().await;
        Ok(snapshots.get(stream_id).cloned())
    }

    async fn truncate_stream(
        &self,
        stream_id: &str,
        before_version: StreamVersion,
    ) -> EventResult<()> {
        let mut events = self.events.write().await;
        events.retain(|e| {
            e.stream_id != stream_id || e.version >= before_version
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventType;

    fn create_test_event(source: &str) -> Event {
        let mut event = Event::new(
            EventType::new("test", "event"),
            serde_json::json!({"data": "test"}),
        );
        event.metadata.source = source.to_string();
        event
    }

    #[tokio::test]
    async fn test_append_and_retrieve() {
        let store = MemoryEventStore::new();
        let event = create_test_event("test-stream");
        
        let id = store.append(&event).await.unwrap();
        let retrieved = store.get(&id).await.unwrap();
        
        assert!(retrieved.is_some());
        let stored = retrieved.unwrap();
        assert_eq!(stored.id, id);
        assert_eq!(stored.stream_id, "test-stream");
        assert_eq!(stored.version, 1);
    }

    #[tokio::test]
    async fn test_stream_versioning() {
        let store = MemoryEventStore::new();
        
        let event1 = create_test_event("stream-1");
        let event2 = create_test_event("stream-1");
        let event3 = create_test_event("stream-1");
        
        store.append(&event1).await.unwrap();
        store.append(&event2).await.unwrap();
        store.append(&event3).await.unwrap();
        
        let stream_events = store.get_stream("stream-1", None).await.unwrap();
        assert_eq!(stream_events.len(), 3);
        assert_eq!(stream_events[0].version, 1);
        assert_eq!(stream_events[1].version, 2);
        assert_eq!(stream_events[2].version, 3);
    }

    #[tokio::test]
    async fn test_batch_append() {
        let store = MemoryEventStore::new();
        
        let events = vec![
            create_test_event("stream-1"),
            create_test_event("stream-1"),
            create_test_event("stream-1"),
        ];
        
        let ids = store.append_batch(&events).await.unwrap();
        assert_eq!(ids.len(), 3);
        
        let stream_events = store.get_stream("stream-1", None).await.unwrap();
        assert_eq!(stream_events.len(), 3);
    }

    #[tokio::test]
    async fn test_query_filtering() {
        let store = MemoryEventStore::new();
        
        let mut event1 = create_test_event("stream-1");
        event1.event_type = EventType::new("user", "created");
        
        let mut event2 = create_test_event("stream-2");
        event2.event_type = EventType::new("user", "updated");
        
        store.append(&event1).await.unwrap();
        store.append(&event2).await.unwrap();
        
        // Query by event type
        let query = EventQuery {
            event_types: vec!["user.created".to_string()],
            ..Default::default()
        };
        
        let stream = store.query(query).await.unwrap();
        assert_eq!(stream.len(), 1);
    }

    #[tokio::test]
    async fn test_snapshot() {
        let store = MemoryEventStore::new();
        
        let state = serde_json::json!({"counter": 42});
        store.create_snapshot("stream-1", 10, state.clone()).await.unwrap();
        
        let snapshot = store.get_latest_snapshot("stream-1").await.unwrap();
        assert!(snapshot.is_some());
        
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.stream_id, "stream-1");
        assert_eq!(snapshot.version, 10);
        assert_eq!(snapshot.state, state);
    }
}
