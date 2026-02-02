//! Webhook queue abstraction.

use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::delivery::WebhookJob;

/// Error type for queue operations.
#[derive(Debug, Clone)]
pub struct QueueError(pub String);

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Queue error: {}", self.0)
    }
}

impl std::error::Error for QueueError {}

/// Trait for webhook job queues.
#[async_trait]
pub trait WebhookQueue: Send + Sync {
    /// Enqueues a job for delivery.
    async fn enqueue(&self, job: WebhookJob) -> Result<(), QueueError>;

    /// Dequeues the next job ready for delivery.
    async fn dequeue(&self) -> Result<Option<WebhookJob>, QueueError>;

    /// Marks a job as completed.
    async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError>;

    /// Marks a job as failed with an error message.
    async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError>;

    /// Schedules a job for retry.
    async fn schedule_retry(&self, job: WebhookJob) -> Result<(), QueueError>;

    /// Gets a job by ID.
    async fn get_job(&self, job_id: &str) -> Result<Option<WebhookJob>, QueueError>;

    /// Gets all pending jobs.
    async fn pending_jobs(&self) -> Result<Vec<WebhookJob>, QueueError>;

    /// Gets the queue length.
    async fn len(&self) -> Result<usize, QueueError>;

    /// Checks if the queue is empty.
    async fn is_empty(&self) -> Result<bool, QueueError> {
        Ok(self.len().await? == 0)
    }

    /// Clears all jobs from the queue.
    async fn clear(&self) -> Result<(), QueueError>;
}

/// In-memory webhook queue implementation.
pub struct InMemoryQueue {
    jobs: RwLock<VecDeque<WebhookJob>>,
    completed: RwLock<Vec<String>>,
    failed: RwLock<Vec<(String, String)>>,
}

impl InMemoryQueue {
    /// Creates a new in-memory queue.
    pub fn new() -> Self {
        Self {
            jobs: RwLock::new(VecDeque::new()),
            completed: RwLock::new(Vec::new()),
            failed: RwLock::new(Vec::new()),
        }
    }

    /// Creates a shared in-memory queue.
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl Default for InMemoryQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookQueue for InMemoryQueue {
    async fn enqueue(&self, job: WebhookJob) -> Result<(), QueueError> {
        let mut jobs = self.jobs.write().await;
        jobs.push_back(job);
        Ok(())
    }

    async fn dequeue(&self) -> Result<Option<WebhookJob>, QueueError> {
        let mut jobs = self.jobs.write().await;
        let now = chrono::Utc::now();

        // Find the first job that's ready
        let ready_idx = jobs.iter().position(|j| j.next_attempt <= now);

        if let Some(idx) = ready_idx {
            Ok(jobs.remove(idx))
        } else {
            Ok(None)
        }
    }

    async fn mark_complete(&self, job_id: &str) -> Result<(), QueueError> {
        let mut completed = self.completed.write().await;
        completed.push(job_id.to_string());
        Ok(())
    }

    async fn mark_failed(&self, job_id: &str, error: &str) -> Result<(), QueueError> {
        let mut failed = self.failed.write().await;
        failed.push((job_id.to_string(), error.to_string()));
        Ok(())
    }

    async fn schedule_retry(&self, job: WebhookJob) -> Result<(), QueueError> {
        let mut jobs = self.jobs.write().await;
        jobs.push_back(job);
        Ok(())
    }

    async fn get_job(&self, job_id: &str) -> Result<Option<WebhookJob>, QueueError> {
        let jobs = self.jobs.read().await;
        Ok(jobs.iter().find(|j| j.id == job_id).cloned())
    }

    async fn pending_jobs(&self) -> Result<Vec<WebhookJob>, QueueError> {
        let jobs = self.jobs.read().await;
        Ok(jobs.iter().cloned().collect())
    }

    async fn len(&self) -> Result<usize, QueueError> {
        let jobs = self.jobs.read().await;
        Ok(jobs.len())
    }

    async fn clear(&self) -> Result<(), QueueError> {
        let mut jobs = self.jobs.write().await;
        jobs.clear();
        let mut completed = self.completed.write().await;
        completed.clear();
        let mut failed = self.failed.write().await;
        failed.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delivery::WebhookJobStatus;
    use crate::endpoint::WebhookEndpoint;
    use better_auth_events::Event;

    #[tokio::test]
    async fn test_in_memory_queue() {
        let queue = InMemoryQueue::new();

        let endpoint = WebhookEndpoint::new("https://example.com", "secret");
        let event = Event::simple("test.event", serde_json::json!({}));
        let job = WebhookJob::new(&endpoint, &event);

        queue.enqueue(job.clone()).await.unwrap();
        assert_eq!(queue.len().await.unwrap(), 1);

        let dequeued = queue.dequeue().await.unwrap();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, job.id);

        assert_eq!(queue.len().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_queue_respects_next_attempt() {
        let queue = InMemoryQueue::new();

        let endpoint = WebhookEndpoint::new("https://example.com", "secret");
        let event = Event::simple("test.event", serde_json::json!({}));
        let mut job = WebhookJob::new(&endpoint, &event);

        // Set next_attempt to future
        job.next_attempt = chrono::Utc::now() + chrono::Duration::hours(1);

        queue.enqueue(job).await.unwrap();

        // Should not dequeue because next_attempt is in the future
        let dequeued = queue.dequeue().await.unwrap();
        assert!(dequeued.is_none());
    }
}
