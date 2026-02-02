//! Webhook storage trait for persistence.

use async_trait::async_trait;

use crate::delivery::{WebhookDelivery, WebhookJob};
use crate::endpoint::WebhookEndpoint;
use crate::error::WebhookResult;

/// Trait for webhook storage backends.
#[async_trait]
pub trait WebhookStorage: Send + Sync {
    // ==================== Endpoint Operations ====================

    /// Saves a webhook endpoint.
    async fn save_endpoint(&self, endpoint: &WebhookEndpoint) -> WebhookResult<()>;

    /// Gets an endpoint by ID.
    async fn get_endpoint(&self, id: &str) -> WebhookResult<Option<WebhookEndpoint>>;

    /// Lists all endpoints.
    async fn list_endpoints(&self) -> WebhookResult<Vec<WebhookEndpoint>>;

    /// Deletes an endpoint.
    async fn delete_endpoint(&self, id: &str) -> WebhookResult<()>;

    // ==================== Job Operations ====================

    /// Saves a webhook job.
    async fn save_job(&self, job: &WebhookJob) -> WebhookResult<()>;

    /// Gets a job by ID.
    async fn get_job(&self, id: &str) -> WebhookResult<Option<WebhookJob>>;

    /// Lists pending jobs.
    async fn list_pending_jobs(&self, limit: usize) -> WebhookResult<Vec<WebhookJob>>;

    /// Updates a job.
    async fn update_job(&self, job: &WebhookJob) -> WebhookResult<()>;

    /// Deletes a job.
    async fn delete_job(&self, id: &str) -> WebhookResult<()>;

    // ==================== Delivery Log Operations ====================

    /// Saves a delivery log entry.
    async fn save_delivery(&self, delivery: &WebhookDelivery) -> WebhookResult<()>;

    /// Gets deliveries for a job.
    async fn get_deliveries_for_job(&self, job_id: &str) -> WebhookResult<Vec<WebhookDelivery>>;

    /// Gets recent deliveries for an endpoint.
    async fn get_deliveries_for_endpoint(
        &self,
        endpoint_id: &str,
        limit: usize,
    ) -> WebhookResult<Vec<WebhookDelivery>>;

    /// Deletes old delivery logs.
    async fn cleanup_old_deliveries(&self, older_than_days: u32) -> WebhookResult<usize>;
}

/// In-memory webhook storage for testing.
pub struct InMemoryWebhookStorage {
    endpoints: tokio::sync::RwLock<std::collections::HashMap<String, WebhookEndpoint>>,
    jobs: tokio::sync::RwLock<std::collections::HashMap<String, WebhookJob>>,
    deliveries: tokio::sync::RwLock<Vec<WebhookDelivery>>,
}

impl InMemoryWebhookStorage {
    /// Creates a new in-memory storage.
    pub fn new() -> Self {
        Self {
            endpoints: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            jobs: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            deliveries: tokio::sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryWebhookStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookStorage for InMemoryWebhookStorage {
    async fn save_endpoint(&self, endpoint: &WebhookEndpoint) -> WebhookResult<()> {
        let mut endpoints = self.endpoints.write().await;
        endpoints.insert(endpoint.id.clone(), endpoint.clone());
        Ok(())
    }

    async fn get_endpoint(&self, id: &str) -> WebhookResult<Option<WebhookEndpoint>> {
        let endpoints = self.endpoints.read().await;
        Ok(endpoints.get(id).cloned())
    }

    async fn list_endpoints(&self) -> WebhookResult<Vec<WebhookEndpoint>> {
        let endpoints = self.endpoints.read().await;
        Ok(endpoints.values().cloned().collect())
    }

    async fn delete_endpoint(&self, id: &str) -> WebhookResult<()> {
        let mut endpoints = self.endpoints.write().await;
        endpoints.remove(id);
        Ok(())
    }

    async fn save_job(&self, job: &WebhookJob) -> WebhookResult<()> {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id.clone(), job.clone());
        Ok(())
    }

    async fn get_job(&self, id: &str) -> WebhookResult<Option<WebhookJob>> {
        let jobs = self.jobs.read().await;
        Ok(jobs.get(id).cloned())
    }

    async fn list_pending_jobs(&self, limit: usize) -> WebhookResult<Vec<WebhookJob>> {
        let jobs = self.jobs.read().await;
        let now = chrono::Utc::now();
        Ok(jobs
            .values()
            .filter(|j| {
                j.status == crate::delivery::WebhookJobStatus::Pending && j.next_attempt <= now
            })
            .take(limit)
            .cloned()
            .collect())
    }

    async fn update_job(&self, job: &WebhookJob) -> WebhookResult<()> {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id.clone(), job.clone());
        Ok(())
    }

    async fn delete_job(&self, id: &str) -> WebhookResult<()> {
        let mut jobs = self.jobs.write().await;
        jobs.remove(id);
        Ok(())
    }

    async fn save_delivery(&self, delivery: &WebhookDelivery) -> WebhookResult<()> {
        let mut deliveries = self.deliveries.write().await;
        deliveries.push(delivery.clone());
        Ok(())
    }

    async fn get_deliveries_for_job(&self, job_id: &str) -> WebhookResult<Vec<WebhookDelivery>> {
        let deliveries = self.deliveries.read().await;
        Ok(deliveries
            .iter()
            .filter(|d| d.job_id == job_id)
            .cloned()
            .collect())
    }

    async fn get_deliveries_for_endpoint(
        &self,
        endpoint_id: &str,
        limit: usize,
    ) -> WebhookResult<Vec<WebhookDelivery>> {
        let deliveries = self.deliveries.read().await;
        Ok(deliveries
            .iter()
            .filter(|d| d.endpoint_id == endpoint_id)
            .rev()
            .take(limit)
            .cloned()
            .collect())
    }

    async fn cleanup_old_deliveries(&self, older_than_days: u32) -> WebhookResult<usize> {
        let mut deliveries = self.deliveries.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);
        let before_len = deliveries.len();
        deliveries.retain(|d| d.created_at > cutoff);
        Ok(before_len - deliveries.len())
    }
}
