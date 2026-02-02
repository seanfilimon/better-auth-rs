# Better Auth Webhooks

Production-ready webhook delivery system with retry logic, rate limiting, and circuit breakers.

## Overview

The webhooks system provides reliable HTTP callback delivery for Better Auth events, featuring:

- **Automatic Retries**: Exponential backoff for failed deliveries
- **Circuit Breaker**: Prevent overwhelming failing endpoints
- **Rate Limiting**: Control delivery rate per endpoint
- **Signature Verification**: HMAC-based request signing
- **Queue Management**: Persistent queue with priority support
- **Webhook Reception**: Verify and process incoming webhooks
- **Multiple Storage Backends**: Memory, PostgreSQL, Redis

## Key Features

- ✅ **Reliable Delivery**: Automatic retries with exponential backoff
- ✅ **Circuit Breaker**: Auto-disable failing endpoints
- ✅ **Rate Limiting**: Token bucket algorithm
- ✅ **HMAC Signatures**: Secure request verification
- ✅ **Dead Letter Queue**: Handle permanently failed deliveries
- ✅ **Event Filtering**: Subscribe to specific event types
- ✅ **Batch Delivery**: Send multiple events in one request
- ✅ **Webhook Reception**: Verify incoming webhook signatures

## Architecture

```
Event → WebhookSystem → Queue → Delivery Worker
                           ↓
                    [Rate Limiter]
                           ↓
                   [Circuit Breaker]
                           ↓
                      HTTP Client
                           ↓
                    Target Endpoint
```

## Quick Start

### Basic Webhook Delivery

```rust
use better_auth_webhooks::{WebhookSystem, WebhookEndpoint};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system = WebhookSystem::new(storage);
    
    // Register endpoint
    system.register_endpoint(WebhookEndpoint {
        id: "endpoint_1".to_string(),
        url: "https://api.example.com/webhooks".to_string(),
        secret: "webhook_secret".to_string(),
        events: vec!["user.created".to_string(), "user.updated".to_string()],
        enabled: true,
        ..Default::default()
    }).await?;
    
    // Send webhook
    system.send_webhook("user.created", json!({
        "user_id": "123",
        "email": "user@example.com"
    })).await?;
    
    Ok(())
}
```

### With Retry Configuration

```rust
use better_auth_webhooks::{WebhookSystem, RetryStrategy};

let system = WebhookSystem::builder()
    .retry_strategy(RetryStrategy {
        max_retries: 5,
        initial_delay_ms: 1000,
        max_delay_ms: 60000,
        backoff_multiplier: 2.0,
    })
    .build(storage);
```

### With Circuit Breaker

```rust
use better_auth_webhooks::{WebhookSystem, CircuitBreakerConfig};

let system = WebhookSystem::builder()
    .circuit_breaker(CircuitBreakerConfig {
        failure_threshold: 5,
        success_threshold: 2,
        timeout_seconds: 60,
        half_open_max_requests: 3,
    })
    .build(storage);
```

### With Rate Limiting

```rust
use better_auth_webhooks::{WebhookSystem, RateLimitConfig};

let system = WebhookSystem::builder()
    .rate_limit(RateLimitConfig {
        requests_per_second: 10,
        burst_size: 20,
    })
    .build(storage);
```

## Components

### WebhookSystem (`system.rs`)

Main orchestrator:

```rust
pub struct WebhookSystem {
    queue: Arc<WebhookQueue>,
    delivery: Arc<DeliveryEngine>,
    storage: Arc<dyn WebhookStorage>,
    circuit_breaker: Arc<CircuitBreaker>,
    rate_limiter: Arc<RateLimiter>,
}
```

### WebhookEndpoint (`endpoint.rs`)

Endpoint configuration:

```rust
pub struct WebhookEndpoint {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub enabled: bool,
    pub rate_limit: Option<RateLimitConfig>,
    pub headers: HashMap<String, String>,
    pub timeout_seconds: u64,
}
```

### WebhookQueue (`queue.rs`)

Persistent queue with priorities:

```rust
pub struct WebhookQueue {
    storage: Arc<dyn WebhookStorage>,
    max_retries: u32,
    workers: usize,
}

impl WebhookQueue {
    pub async fn enqueue(&self, webhook: WebhookDelivery) -> Result<()>;
    pub async fn dequeue(&self) -> Result<Option<WebhookDelivery>>;
    pub async fn requeue(&self, webhook: WebhookDelivery) -> Result<()>;
}
```

### DeliveryEngine (`delivery.rs`)

HTTP delivery with retries:

```rust
pub struct DeliveryEngine {
    client: reqwest::Client,
    retry_strategy: RetryStrategy,
    signature_generator: SignatureGenerator,
}

impl DeliveryEngine {
    pub async fn deliver(&self, webhook: &WebhookDelivery) -> Result<DeliveryResult>;
}
```

### CircuitBreaker (`circuit_breaker.rs`)

Prevent overwhelming failing services:

```rust
pub struct CircuitBreaker {
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
}

pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Blocking requests
    HalfOpen, // Testing recovery
}
```

### RateLimiter (`rate_limiter.rs`)

Token bucket rate limiting:

```rust
pub struct RateLimiter {
    tokens_per_second: f64,
    burst_size: u32,
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
}

impl RateLimiter {
    pub async fn check_rate_limit(&self, endpoint_id: &str) -> Result<bool>;
}
```

### SignatureGenerator (`signature.rs`)

HMAC-SHA256 signature generation:

```rust
pub struct SignatureGenerator;

impl SignatureGenerator {
    pub fn generate(&self, secret: &str, payload: &[u8], timestamp: i64) -> String;
    pub fn verify(&self, signature: &str, secret: &str, payload: &[u8], timestamp: i64) -> bool;
}
```

### WebhookReceiver (`receiver.rs`)

Verify incoming webhooks:

```rust
pub struct WebhookReceiver {
    secret: String,
}

impl WebhookReceiver {
    pub fn verify_signature(&self, headers: &HeaderMap, body: &[u8]) -> Result<bool>;
    pub fn parse_webhook(&self, body: &[u8]) -> Result<IncomingWebhook>;
}
```

## Retry Strategy

Exponential backoff with jitter:

```rust
pub struct RetryStrategy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl RetryStrategy {
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay_ms as f64 
            * self.backoff_multiplier.powi(attempt as i32);
        let delay = delay.min(self.max_delay_ms as f64);
        
        // Add jitter (±10%)
        let jitter = (rand::random::<f64>() - 0.5) * 0.2 * delay;
        Duration::from_millis((delay + jitter) as u64)
    }
}
```

## Webhook Payload

Standard webhook payload format:

```json
{
  "id": "webhook_123",
  "event": "user.created",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "user_id": "user_123",
    "email": "user@example.com"
  },
  "version": "1.0"
}
```

## Signature Verification

### Sending Webhooks

```rust
// Signature header format:
// X-Webhook-Signature: t=1234567890,v1=<hmac_signature>

let timestamp = Utc::now().timestamp();
let payload = serde_json::to_vec(&webhook)?;
let signature = signature_gen.generate(&secret, &payload, timestamp);

headers.insert(
    "X-Webhook-Signature",
    format!("t={},v1={}", timestamp, signature)
);
```

### Receiving Webhooks

```rust
use better_auth_webhooks::WebhookReceiver;

let receiver = WebhookReceiver::new("your_secret");

// Verify signature from headers
if !receiver.verify_signature(&headers, &body)? {
    return Err("Invalid signature");
}

// Parse webhook
let webhook = receiver.parse_webhook(&body)?;
```

## Storage Adapters

### In-Memory Storage

```rust
use better_auth_webhooks::storage::MemoryStorage;

let storage = MemoryStorage::new();
```

### PostgreSQL Storage

```rust
use better_auth_webhooks::storage::PostgresStorage;

let storage = PostgresStorage::new(pool).await?;
```

### Custom Storage

```rust
use async_trait::async_trait;
use better_auth_webhooks::storage::WebhookStorage;

struct MyStorage;

#[async_trait]
impl WebhookStorage for MyStorage {
    async fn save_endpoint(&self, endpoint: &WebhookEndpoint) -> Result<()> {
        // Implementation
    }
    
    async fn get_endpoint(&self, id: &str) -> Result<Option<WebhookEndpoint>> {
        // Implementation
    }
    
    // ... other methods
}
```

## Event Filtering

Subscribe to specific events:

```rust
// Subscribe to all user events
endpoint.events = vec!["user.*".to_string()];

// Subscribe to specific events
endpoint.events = vec![
    "user.created".to_string(),
    "user.updated".to_string(),
];

// Subscribe to all events
endpoint.events = vec!["*".to_string()];
```

## Error Handling

```rust
pub enum WebhookError {
    DeliveryFailed(String),
    RateLimitExceeded,
    CircuitBreakerOpen,
    InvalidSignature,
    EndpointNotFound(String),
    StorageError(String),
    SerializationError(String),
}
```

## Monitoring

Track webhook metrics:

```rust
let metrics = system.get_metrics().await?;

println!("Total deliveries: {}", metrics.total_deliveries);
println!("Successful: {}", metrics.successful_deliveries);
println!("Failed: {}", metrics.failed_deliveries);
println!("Average latency: {}ms", metrics.avg_latency_ms);
```

## Testing

```bash
cargo test -p better-auth-webhooks
```

With integration tests:

```bash
cargo test -p better-auth-webhooks --features integration-tests
```

## Database Schema

```sql
-- Webhook endpoints
CREATE TABLE webhook_endpoints (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    secret TEXT NOT NULL,
    events JSONB NOT NULL,
    enabled BOOLEAN DEFAULT true,
    rate_limit_rps INTEGER,
    headers JSONB,
    timeout_seconds INTEGER DEFAULT 30,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Webhook queue
CREATE TABLE webhook_queue (
    id TEXT PRIMARY KEY,
    endpoint_id TEXT NOT NULL REFERENCES webhook_endpoints(id),
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    attempts INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    next_retry_at TIMESTAMPTZ,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_webhook_queue_status ON webhook_queue(status);
CREATE INDEX idx_webhook_queue_next_retry ON webhook_queue(next_retry_at);
```

## Configuration

```rust
let system = WebhookSystem::builder()
    .retry_strategy(RetryStrategy {
        max_retries: 5,
        initial_delay_ms: 1000,
        max_delay_ms: 300000,
        backoff_multiplier: 2.0,
    })
    .circuit_breaker(CircuitBreakerConfig {
        failure_threshold: 5,
        success_threshold: 2,
        timeout_seconds: 60,
        half_open_max_requests: 3,
    })
    .rate_limit(RateLimitConfig {
        requests_per_second: 10,
        burst_size: 20,
    })
    .timeout_seconds(30)
    .workers(4)
    .build(storage);
```

## Best Practices

1. **Always Use Signatures**: Verify webhook authenticity
2. **Handle Retries**: Implement idempotent handlers
3. **Monitor Failures**: Track failed deliveries
4. **Rate Limit**: Respect downstream service limits
5. **Use Circuit Breakers**: Prevent cascade failures
6. **Log Everything**: Comprehensive delivery logging

## See Also

- [Webhooks SDK](../webhooks-sdk/README.md) - SDK for webhook consumers
- [Events System](../../events/events/README.md) - Event bus integration
- [Core](../../core/core/README.md) - Core types
