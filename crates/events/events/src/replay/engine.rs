use crate::{EventBus, EventStore, EventQuery, EventOrdering, EventResult, EventError};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Engine for replaying events from storage
pub struct ReplayEngine {
    store: Arc<dyn EventStore>,
    bus: Arc<EventBus>,
}

/// Configuration for event replay
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Start replaying from this timestamp
    pub from_timestamp: DateTime<Utc>,
    
    /// Optional end timestamp (replay until this time)
    pub to_timestamp: Option<DateTime<Utc>>,
    
    /// Filter to specific event types (namespace.action format)
    pub event_types: Option<Vec<String>>,
    
    /// Filter to specific streams
    pub stream_ids: Option<Vec<String>>,
    
    /// Speed at which to replay events
    pub speed: ReplaySpeed,
    
    /// Skip events that fail during replay
    pub skip_failed: bool,
    
    /// Maximum number of events to replay
    pub max_events: Option<usize>,
}

/// Speed control for event replay
#[derive(Debug, Clone)]
pub enum ReplaySpeed {
    /// Replay as fast as possible
    Fast,
    
    /// Preserve original timing between events
    RealTime,
    
    /// Custom speed multiplier (2.0 = 2x speed, 0.5 = half speed)
    Custom(f64),
}

/// Result of a replay operation
#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub stats: ReplayStats,
    pub errors: Vec<ReplayError>,
}

/// Statistics from replay operation
#[derive(Debug, Clone)]
pub struct ReplayStats {
    /// Total number of events replayed
    pub total_events: usize,
    
    /// Number of successfully replayed events
    pub successful: usize,
    
    /// Number of failed events
    pub failed: usize,
    
    /// Number of skipped events
    pub skipped: usize,
    
    /// Duration of replay operation
    pub duration: Duration,
    
    /// Time range covered by replay
    pub time_range: (DateTime<Utc>, DateTime<Utc>),
}

/// Error that occurred during replay
#[derive(Debug, Clone)]
pub struct ReplayError {
    pub event_id: uuid::Uuid,
    pub event_type: String,
    pub error: String,
    pub timestamp: DateTime<Utc>,
}

impl ReplayEngine {
    /// Create a new replay engine
    pub fn new(store: Arc<dyn EventStore>, bus: Arc<EventBus>) -> Self {
        Self { store, bus }
    }

    /// Replay events according to the configuration
    pub async fn replay(&self, config: ReplayConfig) -> EventResult<ReplayResult> {
        let start_time = std::time::Instant::now();
        
        // Build query from config
        let query = EventQuery {
            event_types: config.event_types.clone().unwrap_or_default(),
            stream_ids: config.stream_ids.clone().unwrap_or_default(),
            start_time: Some(config.from_timestamp),
            end_time: config.to_timestamp,
            limit: config.max_events,
            offset: None,
            ordering: EventOrdering::Ascending,
        };

        // Query events from store
        let mut event_stream = self.store.query(query).await?;
        
        let mut stats = ReplayStats {
            total_events: event_stream.len(),
            successful: 0,
            failed: 0,
            skipped: 0,
            duration: Duration::from_secs(0),
            time_range: (config.from_timestamp, config.from_timestamp),
        };
        
        let mut errors = Vec::new();
        let mut last_timestamp: Option<DateTime<Utc>> = None;

        tracing::info!(
            "Starting replay of {} events from {}",
            stats.total_events,
            config.from_timestamp
        );

        // Replay events
        while let Some(stored_event) = event_stream.next() {
            let event = &stored_event.event;
            
            // Handle timing between events
            if let Some(last_ts) = last_timestamp {
                match config.speed {
                    ReplaySpeed::Fast => {
                        // No delay
                    }
                    ReplaySpeed::RealTime => {
                        let delay = event.timestamp.signed_duration_since(last_ts);
                        if let Ok(duration) = delay.to_std() {
                            sleep(duration).await;
                        }
                    }
                    ReplaySpeed::Custom(multiplier) => {
                        let delay = event.timestamp.signed_duration_since(last_ts);
                        if let Ok(duration) = delay.to_std() {
                            let adjusted = duration.mul_f64(1.0 / multiplier);
                            sleep(adjusted).await;
                        }
                    }
                }
            }

            // Update time range
            if stats.successful == 0 {
                stats.time_range.0 = event.timestamp;
            }
            stats.time_range.1 = event.timestamp;

            // Emit event to bus
            match self.bus.emit_checked(event.clone()).await {
                Ok(_) => {
                    stats.successful += 1;
                    tracing::debug!(
                        "Replayed event {}/{}: {}",
                        stats.successful,
                        stats.total_events,
                        event.event_type
                    );
                }
                Err(e) => {
                    stats.failed += 1;
                    let error = ReplayError {
                        event_id: stored_event.id,
                        event_type: event.event_type.to_string(),
                        error: e.to_string(),
                        timestamp: event.timestamp,
                    };
                    errors.push(error.clone());
                    
                    tracing::warn!(
                        "Failed to replay event {}: {}",
                        event.event_type,
                        e
                    );

                    if !config.skip_failed {
                        return Err(EventError::Internal(format!(
                            "Replay failed at event {}: {}",
                            event.event_type, e
                        )));
                    }
                }
            }

            last_timestamp = Some(event.timestamp);
        }

        stats.duration = start_time.elapsed();

        tracing::info!(
            "Replay completed: {} successful, {} failed, {} skipped in {:?}",
            stats.successful,
            stats.failed,
            stats.skipped,
            stats.duration
        );

        Ok(ReplayResult { stats, errors })
    }

    /// Replay all events from a specific stream
    pub async fn replay_stream(
        &self,
        stream_id: &str,
        from_version: u32,
    ) -> EventResult<ReplayResult> {
        let start_time = std::time::Instant::now();
        
        // Get stream events
        let stored_events = self.store.get_stream(stream_id, Some(from_version)).await?;
        
        let mut stats = ReplayStats {
            total_events: stored_events.len(),
            successful: 0,
            failed: 0,
            skipped: 0,
            duration: Duration::from_secs(0),
            time_range: (Utc::now(), Utc::now()),
        };
        
        let mut errors = Vec::new();

        tracing::info!(
            "Replaying stream '{}' from version {} ({} events)",
            stream_id,
            from_version,
            stats.total_events
        );

        // Replay events in order
        for stored_event in stored_events {
            let event = &stored_event.event;
            
            // Update time range
            if stats.successful == 0 {
                stats.time_range.0 = event.timestamp;
            }
            stats.time_range.1 = event.timestamp;

            match self.bus.emit_checked(event.clone()).await {
                Ok(_) => {
                    stats.successful += 1;
                }
                Err(e) => {
                    stats.failed += 1;
                    errors.push(ReplayError {
                        event_id: stored_event.id,
                        event_type: event.event_type.to_string(),
                        error: e.to_string(),
                        timestamp: event.timestamp,
                    });
                }
            }
        }

        stats.duration = start_time.elapsed();

        tracing::info!(
            "Stream replay completed: {} successful, {} failed in {:?}",
            stats.successful,
            stats.failed,
            stats.duration
        );

        Ok(ReplayResult { stats, errors })
    }

    /// Replay events up to a specific point in time (time travel)
    pub async fn replay_until(
        &self,
        until: DateTime<Utc>,
        speed: ReplaySpeed,
    ) -> EventResult<ReplayResult> {
        let config = ReplayConfig {
            from_timestamp: DateTime::<Utc>::MIN_UTC,
            to_timestamp: Some(until),
            event_types: None,
            stream_ids: None,
            speed,
            skip_failed: false,
            max_events: None,
        };

        self.replay(config).await
    }
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            from_timestamp: Utc::now() - chrono::Duration::hours(24),
            to_timestamp: None,
            event_types: None,
            stream_ids: None,
            speed: ReplaySpeed::Fast,
            skip_failed: true,
            max_events: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, EventType, MemoryEventStore};

    #[tokio::test]
    async fn test_replay_fast() {
        let store = Arc::new(MemoryEventStore::new());
        let bus = Arc::new(EventBus::new());
        let engine = ReplayEngine::new(store.clone(), bus.clone());

        // Create and store test events
        let event1 = Event::new(
            EventType::new("test", "event1"),
            serde_json::json!({"data": "test1"}),
        );
        let event2 = Event::new(
            EventType::new("test", "event2"),
            serde_json::json!({"data": "test2"}),
        );

        store.append(&event1).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        store.append(&event2).await.unwrap();

        // Replay events
        let config = ReplayConfig {
            from_timestamp: Utc::now() - chrono::Duration::seconds(10),
            speed: ReplaySpeed::Fast,
            ..Default::default()
        };

        let result = engine.replay(config).await.unwrap();
        assert_eq!(result.stats.successful, 2);
        assert_eq!(result.stats.failed, 0);
    }

    #[tokio::test]
    async fn test_replay_stream() {
        let store = Arc::new(MemoryEventStore::new());
        let bus = Arc::new(EventBus::new());
        let engine = ReplayEngine::new(store.clone(), bus.clone());

        // Create stream events
        for i in 1..=3 {
            let mut event = Event::new(
                EventType::new("test", "stream_event"),
                serde_json::json!({"index": i}),
            );
            event.metadata.source = "test-stream".to_string();
            store.append(&event).await.unwrap();
        }

        // Replay stream from version 2
        let result = engine.replay_stream("test-stream", 2).await.unwrap();
        assert_eq!(result.stats.successful, 2); // versions 2 and 3
    }
}
