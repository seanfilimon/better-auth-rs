use super::storage::{DLQStorage, DeadLetter, DLQQuery};
use crate::{EventBus, Event, EventResult, EventError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Dead Letter Queue for handling failed events
pub struct DeadLetterQueue {
    storage: Arc<dyn DLQStorage>,
    config: DLQConfig,
    bus: Option<Arc<EventBus>>,
}

/// Configuration for dead letter queue
#[derive(Debug, Clone)]
pub struct DLQConfig {
    /// Maximum retry attempts before giving up
    pub max_retries: u32,
    
    /// Whether to automatically retry failed events
    pub auto_retry: bool,
    
    /// Delay between retry attempts (in seconds)
    pub retry_delay_secs: u64,
}

impl Default for DLQConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            auto_retry: false,
            retry_delay_secs: 60,
        }
    }
}

/// Statistics about the dead letter queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLQStats {
    /// Total number of dead letters
    pub total: usize,
    
    /// Dead letters by handler
    pub by_handler: std::collections::HashMap<String, usize>,
    
    /// Dead letters by event type
    pub by_event_type: std::collections::HashMap<String, usize>,
    
    /// Average attempts per dead letter
    pub avg_attempts: f64,
}

impl DeadLetterQueue {
    /// Create a new dead letter queue
    pub fn new(storage: Arc<dyn DLQStorage>) -> Self {
        Self {
            storage,
            config: DLQConfig::default(),
            bus: None,
        }
    }

    /// Create with custom configuration
    pub fn with_config(storage: Arc<dyn DLQStorage>, config: DLQConfig) -> Self {
        Self {
            storage,
            config,
            bus: None,
        }
    }

    /// Attach an event bus for retrying events
    pub fn with_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.bus = Some(bus);
        self
    }

    /// Send an event to the dead letter queue
    pub async fn send(&self, dead_letter: DeadLetter) -> EventResult<()> {
        if dead_letter.attempts >= self.config.max_retries {
            tracing::error!(
                "Event {} exceeded max retries ({}), permanently failed",
                dead_letter.event.id,
                self.config.max_retries
            );
        }

        self.storage.store(&dead_letter).await?;
        
        tracing::warn!(
            "Event {} sent to DLQ after {} attempts: {}",
            dead_letter.event.id,
            dead_letter.attempts,
            dead_letter.error
        );

        Ok(())
    }

    /// Retry a dead letter by ID
    pub async fn retry(&self, id: &str) -> EventResult<()> {
        let dead_letter = self.storage.get(id).await?
            .ok_or_else(|| EventError::Internal(format!("Dead letter {} not found", id)))?;

        if dead_letter.attempts >= self.config.max_retries {
            return Err(EventError::Internal(format!(
                "Cannot retry: max retries ({}) exceeded",
                self.config.max_retries
            )));
        }

        let bus = self.bus.as_ref()
            .ok_or_else(|| EventError::Internal("No event bus attached for retry".into()))?;

        // Try to re-emit the event
        match bus.emit_checked(dead_letter.event.clone()).await {
            Ok(_) => {
                // Success! Remove from DLQ
                self.storage.delete(id).await?;
                tracing::info!("Successfully retried dead letter {}", id);
                Ok(())
            }
            Err(e) => {
                // Failed again, update attempts
                let updated = DeadLetter {
                    attempts: dead_letter.attempts + 1,
                    last_failed_at: Utc::now(),
                    error: e.to_string(),
                    ..dead_letter
                };
                self.storage.update(&updated).await?;
                Err(e)
            }
        }
    }

    /// List dead letters matching query
    pub async fn list(&self, query: DLQQuery) -> EventResult<Vec<DeadLetter>> {
        self.storage.list(&query).await
    }

    /// Delete a dead letter by ID
    pub async fn delete(&self, id: &str) -> EventResult<()> {
        self.storage.delete(id).await
    }

    /// Get statistics about the DLQ
    pub async fn stats(&self) -> EventResult<DLQStats> {
        let storage_stats = self.storage.stats().await?;
        
        // Calculate average attempts
        let all_letters = self.storage.list(&DLQQuery::default()).await?;
        let avg_attempts = if all_letters.is_empty() {
            0.0
        } else {
            all_letters.iter().map(|l| l.attempts as f64).sum::<f64>() / all_letters.len() as f64
        };

        Ok(DLQStats {
            total: storage_stats.total_dead_letters,
            by_handler: storage_stats.by_handler,
            by_event_type: storage_stats.by_event_type,
            avg_attempts,
        })
    }

    /// Retry all dead letters for a specific handler
    pub async fn retry_handler(&self, handler_id: &str) -> EventResult<usize> {
        let query = DLQQuery {
            handler_id: Some(handler_id.to_string()),
            ..Default::default()
        };

        let dead_letters = self.list(query).await?;
        let mut retry_count = 0;

        for letter in dead_letters {
            if self.retry(&letter.id).await.is_ok() {
                retry_count += 1;
            }
        }

        Ok(retry_count)
    }

    /// Purge old dead letters
    pub async fn purge_older_than(&self, cutoff: DateTime<Utc>) -> EventResult<usize> {
        let all_letters = self.storage.list(&DLQQuery::default()).await?;
        let mut purged = 0;

        for letter in all_letters {
            if letter.first_failed_at < cutoff {
                self.storage.delete(&letter.id).await?;
                purged += 1;
            }
        }

        tracing::info!("Purged {} old dead letters", purged);
        Ok(purged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventType, dlq::InMemoryDLQStorage};

    fn create_test_dead_letter() -> DeadLetter {
        DeadLetter {
            id: uuid::Uuid::new_v4().to_string(),
            event: Event::new(
                EventType::new("test", "event"),
                serde_json::json!({"test": "data"}),
            ),
            handler_id: "test-handler".to_string(),
            error: "Test error".to_string(),
            attempts: 1,
            first_failed_at: Utc::now(),
            last_failed_at: Utc::now(),
            stack_trace: None,
        }
    }

    #[tokio::test]
    async fn test_send_to_dlq() {
        let storage = Arc::new(InMemoryDLQStorage::new());
        let dlq = DeadLetterQueue::new(storage.clone());
        
        let dead_letter = create_test_dead_letter();
        let id = dead_letter.id.clone();
        
        dlq.send(dead_letter).await.unwrap();
        
        let retrieved = storage.get(&id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_dlq_stats() {
        let storage = Arc::new(InMemoryDLQStorage::new());
        let dlq = DeadLetterQueue::new(storage);
        
        dlq.send(create_test_dead_letter()).await.unwrap();
        dlq.send(create_test_dead_letter()).await.unwrap();
        
        let stats = dlq.stats().await.unwrap();
        assert_eq!(stats.total, 2);
        assert!(stats.avg_attempts > 0.0);
    }

    #[tokio::test]
    async fn test_retry_with_bus() {
        let storage = Arc::new(InMemoryDLQStorage::new());
        let bus = Arc::new(EventBus::new());
        let dlq = DeadLetterQueue::new(storage.clone()).with_bus(bus);
        
        let dead_letter = create_test_dead_letter();
        let id = dead_letter.id.clone();
        
        dlq.send(dead_letter).await.unwrap();
        
        // Retry should succeed (event bus doesn't fail by default)
        let result = dlq.retry(&id).await;
        assert!(result.is_ok());
        
        // Should be removed from DLQ
        let retrieved = storage.get(&id).await.unwrap();
        assert!(retrieved.is_none());
    }
}
