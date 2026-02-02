//! Webhook delivery job and engine.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use better_auth_events::Event;

use crate::endpoint::WebhookEndpoint;
use crate::error::{WebhookError, WebhookResult};
use crate::queue::{QueueError, WebhookQueue};
use crate::retry::RetryStrategy;
use crate::signature::WebhookSigner;

/// Webhook delivery job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookJob {
    /// Job ID.
    pub id: String,
    /// Endpoint ID.
    pub endpoint_id: String,
    /// Target URL.
    pub url: String,
    /// Payload to send.
    pub payload: Value,
    /// Secret for signing.
    pub secret: String,
    /// Number of attempts made.
    pub attempts: u32,
    /// Maximum attempts.
    pub max_attempts: u32,
    /// Next attempt time.
    pub next_attempt: DateTime<Utc>,
    /// Created at.
    pub created_at: DateTime<Utc>,
    /// Last error message.
    pub last_error: Option<String>,
    /// Status.
    pub status: WebhookJobStatus,
    /// Custom headers.
    pub headers: std::collections::HashMap<String, String>,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
}

/// Webhook job status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebhookJobStatus {
    /// Waiting to be processed.
    Pending,
    /// Currently being processed.
    Processing,
    /// Successfully delivered.
    Completed,
    /// Failed after all retries.
    Failed,
}

impl WebhookJob {
    /// Creates a new webhook job from an endpoint and event.
    pub fn new(endpoint: &WebhookEndpoint, event: &Event) -> Self {
        let payload = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "type": event.simple_type_string(),
            "data": event.payload,
            "timestamp": event.timestamp.to_rfc3339(),
            "correlation_id": event.correlation_id,
        });

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            endpoint_id: endpoint.id.clone(),
            url: endpoint.url.clone(),
            payload,
            secret: endpoint.secret.clone(),
            attempts: 0,
            max_attempts: 5,
            next_attempt: Utc::now(),
            created_at: Utc::now(),
            last_error: None,
            status: WebhookJobStatus::Pending,
            headers: endpoint.metadata.headers.clone(),
            timeout_ms: endpoint.metadata.timeout_ms,
        }
    }

    /// Sets the maximum attempts.
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Schedules the next retry attempt.
    pub fn schedule_retry(&mut self, strategy: &dyn RetryStrategy) {
        self.attempts += 1;

        if let Some(delay) = strategy.next_delay(self.attempts) {
            self.next_attempt = Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default();
            self.status = WebhookJobStatus::Pending;
        } else {
            self.status = WebhookJobStatus::Failed;
        }
    }

    /// Marks the job as completed.
    pub fn mark_completed(&mut self) {
        self.status = WebhookJobStatus::Completed;
    }

    /// Marks the job as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = WebhookJobStatus::Failed;
        self.last_error = Some(error.into());
    }

    /// Marks the job as processing.
    pub fn mark_processing(&mut self) {
        self.status = WebhookJobStatus::Processing;
    }

    /// Generates the signature for this job.
    pub fn generate_signature(&self) -> String {
        let signer = WebhookSigner::new(&self.secret);
        let timestamp = Utc::now().timestamp();
        let payload_bytes = self.payload.to_string();
        signer.sign_header(timestamp, payload_bytes.as_bytes())
    }

    /// Checks if the job can be retried.
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_attempts && self.status != WebhookJobStatus::Completed
    }
}

/// Webhook delivery log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    /// Delivery ID.
    pub id: String,
    /// Job ID.
    pub job_id: String,
    /// Endpoint ID.
    pub endpoint_id: String,
    /// Event type.
    pub event_type: String,
    /// HTTP status code (if received).
    pub status_code: Option<u16>,
    /// Response body (truncated).
    pub response_body: Option<String>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// When the delivery was attempted.
    pub created_at: DateTime<Utc>,
}

impl WebhookDelivery {
    /// Creates a successful delivery record.
    pub fn success(job: &WebhookJob, status_code: u16, response_body: Option<String>, duration_ms: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            job_id: job.id.clone(),
            endpoint_id: job.endpoint_id.clone(),
            event_type: job.payload["type"].as_str().unwrap_or("unknown").to_string(),
            status_code: Some(status_code),
            response_body,
            error: None,
            duration_ms,
            created_at: Utc::now(),
        }
    }

    /// Creates a failed delivery record.
    pub fn failure(job: &WebhookJob, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            job_id: job.id.clone(),
            endpoint_id: job.endpoint_id.clone(),
            event_type: job.payload["type"].as_str().unwrap_or("unknown").to_string(),
            status_code: None,
            response_body: None,
            error: Some(error.into()),
            duration_ms,
            created_at: Utc::now(),
        }
    }
}

/// Webhook delivery engine.
pub struct DeliveryEngine<Q: WebhookQueue, R: RetryStrategy> {
    queue: Q,
    retry_strategy: R,
    #[cfg(feature = "http-client")]
    client: reqwest::Client,
}

impl<Q: WebhookQueue, R: RetryStrategy> DeliveryEngine<Q, R> {
    /// Creates a new delivery engine.
    pub fn new(queue: Q, retry_strategy: R) -> Self {
        Self {
            queue,
            retry_strategy,
            #[cfg(feature = "http-client")]
            client: reqwest::Client::new(),
        }
    }

    /// Enqueues a job for delivery.
    pub async fn enqueue(&self, job: WebhookJob) -> Result<(), QueueError> {
        self.queue.enqueue(job).await
    }

    /// Processes the next job in the queue.
    #[cfg(feature = "http-client")]
    pub async fn process_next(&self) -> WebhookResult<Option<WebhookDelivery>> {
        let job = match self.queue.dequeue().await {
            Ok(Some(job)) => job,
            Ok(None) => return Ok(None),
            Err(e) => return Err(WebhookError::QueueError(e.to_string())),
        };

        let delivery = self.deliver(&job).await;

        match &delivery {
            Ok(d) if d.error.is_none() => {
                self.queue
                    .mark_complete(&job.id)
                    .await
                    .map_err(|e| WebhookError::QueueError(e.to_string()))?;
            }
            _ => {
                let mut job = job;
                job.schedule_retry(&self.retry_strategy);

                if job.can_retry() {
                    self.queue
                        .schedule_retry(job)
                        .await
                        .map_err(|e| WebhookError::QueueError(e.to_string()))?;
                } else {
                    self.queue
                        .mark_failed(&job.id, job.last_error.as_deref().unwrap_or("Unknown error"))
                        .await
                        .map_err(|e| WebhookError::QueueError(e.to_string()))?;
                }
            }
        }

        delivery.map(Some)
    }

    /// Delivers a webhook job.
    #[cfg(feature = "http-client")]
    async fn deliver(&self, job: &WebhookJob) -> WebhookResult<WebhookDelivery> {
        let start = std::time::Instant::now();

        let signature = job.generate_signature();
        let payload = job.payload.to_string();

        let mut request = self
            .client
            .post(&job.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", signature)
            .timeout(std::time::Duration::from_millis(job.timeout_ms));

        // Add custom headers
        for (key, value) in &job.headers {
            request = request.header(key, value);
        }

        let response = request.body(payload).send().await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = resp.text().await.ok();

                if status >= 200 && status < 300 {
                    Ok(WebhookDelivery::success(job, status, body, duration_ms))
                } else {
                    Ok(WebhookDelivery::failure(
                        job,
                        format!("HTTP {}: {}", status, body.unwrap_or_default()),
                        duration_ms,
                    ))
                }
            }
            Err(e) => Ok(WebhookDelivery::failure(job, e.to_string(), duration_ms)),
        }
    }

    /// Gets the queue.
    pub fn queue(&self) -> &Q {
        &self.queue
    }

    /// Gets the retry strategy.
    pub fn retry_strategy(&self) -> &R {
        &self.retry_strategy
    }
}
