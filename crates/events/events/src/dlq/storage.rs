use crate::{Event, EventResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Storage trait for dead letter queue
#[async_trait]
pub trait DLQStorage: Send + Sync {
    /// Store a dead letter
    async fn store(&self, dead_letter: &DeadLetter) -> EventResult<String>;
    
    /// Get a dead letter by ID
    async fn get(&self, id: &str) -> EventResult<Option<DeadLetter>>;
    
    /// List dead letters matching query
    async fn list(&self, query: &DLQQuery) -> EventResult<Vec<DeadLetter>>;
    
    /// Delete a dead letter
    async fn delete(&self, id: &str) -> EventResult<()>;
    
    /// Update a dead letter
    async fn update(&self, dead_letter: &DeadLetter) -> EventResult<()>;
    
    /// Get statistics
    async fn stats(&self) -> EventResult<DLQStorageStats>;
}

/// Query parameters for dead letter listing
#[derive(Debug, Clone, Default)]
pub struct DLQQuery {
    /// Filter by handler ID
    pub handler_id: Option<String>,
    
    /// Filter by event type
    pub event_type: Option<String>,
    
    /// Filter by minimum attempts
    pub min_attempts: Option<u32>,
    
    /// Maximum number of results
    pub limit: Option<usize>,
    
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Statistics from DLQ storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLQStorageStats {
    pub total_dead_letters: usize,
    pub by_handler: std::collections::HashMap<String, usize>,
    pub by_event_type: std::collections::HashMap<String, usize>,
}

/// Dead letter representing a failed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetter {
    /// Unique ID for this dead letter
    pub id: String,
    
    /// The event that failed
    pub event: Event,
    
    /// Handler that failed to process the event
    pub handler_id: String,
    
    /// Error message
    pub error: String,
    
    /// Number of retry attempts
    pub attempts: u32,
    
    /// When the event first failed
    pub first_failed_at: DateTime<Utc>,
    
    /// When the event last failed
    pub last_failed_at: DateTime<Utc>,
    
    /// Optional stack trace
    pub stack_trace: Option<String>,
}

/// In-memory DLQ storage implementation
pub struct InMemoryDLQStorage {
    letters: std::sync::Arc<tokio::sync::RwLock<Vec<DeadLetter>>>,
}

impl InMemoryDLQStorage {
    pub fn new() -> Self {
        Self {
            letters: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

impl Default for InMemoryDLQStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DLQStorage for InMemoryDLQStorage {
    async fn store(&self, dead_letter: &DeadLetter) -> EventResult<String> {
        let mut letters = self.letters.write().await;
        letters.push(dead_letter.clone());
        Ok(dead_letter.id.clone())
    }
    
    async fn get(&self, id: &str) -> EventResult<Option<DeadLetter>> {
        let letters = self.letters.read().await;
        Ok(letters.iter().find(|l| l.id == id).cloned())
    }
    
    async fn list(&self, query: &DLQQuery) -> EventResult<Vec<DeadLetter>> {
        let letters = self.letters.read().await;
        
        let mut filtered: Vec<DeadLetter> = letters
            .iter()
            .filter(|l| {
                if let Some(ref handler_id) = query.handler_id {
                    if &l.handler_id != handler_id {
                        return false;
                    }
                }
                
                if let Some(ref event_type) = query.event_type {
                    if &l.event.event_type.to_string() != event_type {
                        return false;
                    }
                }
                
                if let Some(min_attempts) = query.min_attempts {
                    if l.attempts < min_attempts {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Apply pagination
        if let Some(offset) = query.offset {
            filtered = filtered.into_iter().skip(offset).collect();
        }
        if let Some(limit) = query.limit {
            filtered.truncate(limit);
        }
        
        Ok(filtered)
    }
    
    async fn delete(&self, id: &str) -> EventResult<()> {
        let mut letters = self.letters.write().await;
        letters.retain(|l| l.id != id);
        Ok(())
    }
    
    async fn update(&self, dead_letter: &DeadLetter) -> EventResult<()> {
        let mut letters = self.letters.write().await;
        if let Some(existing) = letters.iter_mut().find(|l| l.id == dead_letter.id) {
            *existing = dead_letter.clone();
        }
        Ok(())
    }
    
    async fn stats(&self) -> EventResult<DLQStorageStats> {
        let letters = self.letters.read().await;
        
        let mut by_handler = std::collections::HashMap::new();
        let mut by_event_type = std::collections::HashMap::new();
        
        for letter in letters.iter() {
            *by_handler.entry(letter.handler_id.clone()).or_insert(0) += 1;
            *by_event_type.entry(letter.event.event_type.to_string()).or_insert(0) += 1;
        }
        
        Ok(DLQStorageStats {
            total_dead_letters: letters.len(),
            by_handler,
            by_event_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventType;

    fn create_test_dead_letter(id: &str, handler_id: &str) -> DeadLetter {
        DeadLetter {
            id: id.to_string(),
            event: Event::new(
                EventType::new("test", "event"),
                serde_json::json!({"test": "data"}),
            ),
            handler_id: handler_id.to_string(),
            error: "Test error".to_string(),
            attempts: 1,
            first_failed_at: Utc::now(),
            last_failed_at: Utc::now(),
            stack_trace: None,
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let storage = InMemoryDLQStorage::new();
        let dead_letter = create_test_dead_letter("id-1", "handler-1");
        
        storage.store(&dead_letter).await.unwrap();
        
        let retrieved = storage.get("id-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "id-1");
    }

    #[tokio::test]
    async fn test_list_with_filter() {
        let storage = InMemoryDLQStorage::new();
        
        storage.store(&create_test_dead_letter("id-1", "handler-1")).await.unwrap();
        storage.store(&create_test_dead_letter("id-2", "handler-2")).await.unwrap();
        storage.store(&create_test_dead_letter("id-3", "handler-1")).await.unwrap();
        
        let query = DLQQuery {
            handler_id: Some("handler-1".to_string()),
            ..Default::default()
        };
        
        let results = storage.list(&query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let storage = InMemoryDLQStorage::new();
        
        storage.store(&create_test_dead_letter("id-1", "handler-1")).await.unwrap();
        storage.store(&create_test_dead_letter("id-2", "handler-2")).await.unwrap();
        
        let stats = storage.stats().await.unwrap();
        assert_eq!(stats.total_dead_letters, 2);
        assert_eq!(stats.by_handler.len(), 2);
    }
}
