# Better Auth Webhooks SDK

SDK for external services to consume and verify Better Auth webhooks.

## Overview

The Webhooks SDK provides tools for services that **receive** webhooks from Better Auth:

- **Signature Verification**: Validate webhook authenticity
- **Payload Parsing**: Type-safe webhook deserialization
- **Client Utilities**: Helper functions for webhook handling
- **Trait Interfaces**: Implement webhook consumers easily

## Purpose

This SDK is for **webhook consumers**, not webhook senders. It helps external services:

1. Verify webhook signatures
2. Parse webhook payloads
3. Handle webhook events
4. Implement retry logic

```
Better Auth → [sends webhook] → Your Service [uses SDK]
```

## Quick Start

### Verify Webhook Signature

```rust
use better_auth_webhooks_sdk::WebhookVerifier;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let verifier = WebhookVerifier::new("your_webhook_secret");
    
    // In your webhook handler
    async fn handle_webhook(
        headers: HeaderMap,
        body: Bytes,
    ) -> Result<(), Error> {
        // Verify signature
        if !verifier.verify(&headers, &body)? {
            return Err("Invalid signature");
        }
        
        // Parse payload
        let webhook: Webhook = serde_json::from_slice(&body)?;
        
        // Process webhook
        match webhook.event.as_str() {
            "user.created" => handle_user_created(webhook.data).await?,
            "user.updated" => handle_user_updated(webhook.data).await?,
            _ => println!("Unknown event: {}", webhook.event),
        }
        
        Ok(())
    }
    
    Ok(())
}
```

### Implement Webhook Consumer

```rust
use better_auth_webhooks_sdk::{WebhookConsumer, Webhook};
use async_trait::async_trait;

struct MyWebhookHandler;

#[async_trait]
impl WebhookConsumer for MyWebhookHandler {
    async fn handle_webhook(&self, webhook: Webhook) -> Result<(), Box<dyn std::error::Error>> {
        match webhook.event.as_str() {
            "user.created" => {
                println!("New user: {:?}", webhook.data);
                // Your business logic
            },
            "user.updated" => {
                println!("User updated: {:?}", webhook.data);
                // Your business logic
            },
            _ => {
                println!("Unhandled event: {}", webhook.event);
            }
        }
        Ok(())
    }
    
    async fn on_error(&self, webhook: Webhook, error: Box<dyn std::error::Error>) {
        eprintln!("Failed to process webhook {}: {}", webhook.id, error);
        // Log to monitoring system
    }
}
```

## Core Types

### Webhook

The webhook payload structure:

```rust
pub struct Webhook {
    pub id: String,
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub data: Value,
    pub version: String,
}
```

Example payload:

```json
{
  "id": "wh_1234567890",
  "event": "user.created",
  "timestamp": "2024-01-01T12:00:00Z",
  "data": {
    "user_id": "user_123",
    "email": "user@example.com",
    "name": "John Doe"
  },
  "version": "1.0"
}
```

### WebhookVerifier

Verify webhook signatures:

```rust
pub struct WebhookVerifier {
    secret: String,
    tolerance_seconds: i64,
}

impl WebhookVerifier {
    pub fn new(secret: impl Into<String>) -> Self;
    
    pub fn with_tolerance(secret: impl Into<String>, tolerance: i64) -> Self;
    
    pub fn verify(&self, headers: &HeaderMap, body: &[u8]) -> Result<bool>;
    
    pub fn verify_signature(
        &self,
        signature: &str,
        timestamp: i64,
        body: &[u8]
    ) -> Result<bool>;
}
```

### WebhookConsumer Trait

Implement to handle webhooks:

```rust
#[async_trait]
pub trait WebhookConsumer: Send + Sync {
    async fn handle_webhook(&self, webhook: Webhook) -> Result<(), Box<dyn std::error::Error>>;
    
    async fn on_error(&self, webhook: Webhook, error: Box<dyn std::error::Error>) {
        // Default: log error
    }
    
    fn supported_events(&self) -> Vec<String> {
        vec!["*".to_string()] // All events by default
    }
}
```

## Signature Verification

Better Auth signs webhooks with HMAC-SHA256:

### Signature Header Format

```
X-Webhook-Signature: t=1234567890,v1=<hmac_signature>
```

Where:
- `t` = Unix timestamp
- `v1` = HMAC-SHA256 signature

### Verification Process

```rust
use better_auth_webhooks_sdk::WebhookVerifier;

let verifier = WebhookVerifier::new("your_secret");

// Extract signature from header
let signature_header = headers
    .get("X-Webhook-Signature")
    .ok_or("Missing signature")?
    .to_str()?;

// Verify (checks timestamp tolerance automatically)
if !verifier.verify(headers, &body)? {
    return Err("Invalid signature");
}
```

### Timestamp Tolerance

Prevent replay attacks by rejecting old webhooks:

```rust
// Default tolerance: 5 minutes
let verifier = WebhookVerifier::new("secret");

// Custom tolerance: 1 minute
let verifier = WebhookVerifier::with_tolerance("secret", 60);
```

## Client Utilities

### WebhookClient

Test webhook delivery:

```rust
use better_auth_webhooks_sdk::WebhookClient;

let client = WebhookClient::new("https://your-api.com/webhooks", "secret");

// Send test webhook
client.send_test_webhook("user.created", json!({
    "user_id": "123",
    "email": "test@example.com"
})).await?;
```

### WebhookBuilder

Build webhook payloads:

```rust
use better_auth_webhooks_sdk::WebhookBuilder;

let webhook = WebhookBuilder::new("user.created")
    .data(json!({
        "user_id": "123",
        "email": "user@example.com"
    }))
    .version("1.0")
    .build();
```

## Framework Integration

### Axum

```rust
use axum::{extract::State, http::HeaderMap, routing::post, Router, body::Bytes};
use better_auth_webhooks_sdk::WebhookVerifier;

async fn webhook_handler(
    State(verifier): State<WebhookVerifier>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<String, String> {
    // Verify signature
    if !verifier.verify(&headers, &body).map_err(|e| e.to_string())? {
        return Err("Invalid signature".to_string());
    }
    
    // Parse and handle webhook
    let webhook: Webhook = serde_json::from_slice(&body)
        .map_err(|e| e.to_string())?;
    
    // Process webhook
    handle_webhook(webhook).await?;
    
    Ok("OK".to_string())
}

#[tokio::main]
async fn main() {
    let verifier = WebhookVerifier::new("your_secret");
    
    let app = Router::new()
        .route("/webhooks", post(webhook_handler))
        .with_state(verifier);
    
    // Serve app...
}
```

### Actix-Web

```rust
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use better_auth_webhooks_sdk::WebhookVerifier;

async fn webhook_handler(
    verifier: web::Data<WebhookVerifier>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, actix_web::Error> {
    // Verify signature
    let headers = req.headers();
    if !verifier.verify(headers, &body)? {
        return Ok(HttpResponse::Unauthorized().finish());
    }
    
    // Parse and handle
    let webhook: Webhook = serde_json::from_slice(&body)?;
    handle_webhook(webhook).await?;
    
    Ok(HttpResponse::Ok().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let verifier = web::Data::new(WebhookVerifier::new("secret"));
    
    HttpServer::new(move || {
        App::new()
            .app_data(verifier.clone())
            .route("/webhooks", web::post().to(webhook_handler))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
```

## Retry Logic

Implement idempotent handlers:

```rust
use better_auth_webhooks_sdk::{WebhookConsumer, Webhook};

struct IdempotentHandler {
    processed_ids: Arc<RwLock<HashSet<String>>>,
}

#[async_trait]
impl WebhookConsumer for IdempotentHandler {
    async fn handle_webhook(&self, webhook: Webhook) -> Result<(), Box<dyn std::error::Error>> {
        // Check if already processed
        if self.processed_ids.read().await.contains(&webhook.id) {
            println!("Webhook {} already processed, skipping", webhook.id);
            return Ok(());
        }
        
        // Process webhook
        process_event(&webhook).await?;
        
        // Mark as processed
        self.processed_ids.write().await.insert(webhook.id.clone());
        
        Ok(())
    }
}
```

## Error Handling

```rust
pub enum WebhookError {
    InvalidSignature,
    ExpiredTimestamp,
    MalformedPayload(String),
    ProcessingError(String),
}
```

## Testing

### Mock Webhooks

```rust
use better_auth_webhooks_sdk::{Webhook, WebhookBuilder};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_webhook_handler() {
        let webhook = WebhookBuilder::new("user.created")
            .data(json!({"user_id": "123"}))
            .build();
        
        let handler = MyWebhookHandler;
        handler.handle_webhook(webhook).await.unwrap();
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_signature_verification() {
    let secret = "test_secret";
    let verifier = WebhookVerifier::new(secret);
    
    let body = br#"{"id":"wh_123","event":"test"}"#;
    let timestamp = Utc::now().timestamp();
    let signature = generate_signature(secret, body, timestamp);
    
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Webhook-Signature",
        format!("t={},v1={}", timestamp, signature).parse().unwrap()
    );
    
    assert!(verifier.verify(&headers, body).unwrap());
}
```

## Best Practices

1. **Always Verify Signatures**: Never trust unverified webhooks
2. **Use Timestamp Tolerance**: Prevent replay attacks
3. **Implement Idempotency**: Handle duplicate deliveries
4. **Return 200 Quickly**: Acknowledge receipt, process async
5. **Log Everything**: Track all webhook receipts
6. **Handle Failures Gracefully**: Expect retries from Better Auth

## Example: Complete Webhook Server

```rust
use axum::{extract::State, routing::post, Router};
use better_auth_webhooks_sdk::{WebhookVerifier, Webhook, WebhookConsumer};
use std::sync::Arc;

struct AppState {
    verifier: WebhookVerifier,
    consumer: Arc<dyn WebhookConsumer>,
}

async fn webhook_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<String, String> {
    // Verify
    if !state.verifier.verify(&headers, &body).map_err(|e| e.to_string())? {
        return Err("Invalid signature".to_string());
    }
    
    // Parse
    let webhook: Webhook = serde_json::from_slice(&body)
        .map_err(|e| format!("Parse error: {}", e))?;
    
    // Handle async (don't block response)
    let consumer = state.consumer.clone();
    tokio::spawn(async move {
        if let Err(e) = consumer.handle_webhook(webhook.clone()).await {
            consumer.on_error(webhook, e).await;
        }
    });
    
    Ok("OK".to_string())
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        verifier: WebhookVerifier::new("your_secret"),
        consumer: Arc::new(MyWebhookHandler),
    });
    
    let app = Router::new()
        .route("/webhooks", post(webhook_handler))
        .with_state(state);
    
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## See Also

- [Webhooks System](../webhooks/README.md) - Webhook delivery implementation
- [Events System](../../events/events/README.md) - Event bus
- [Better Auth Documentation](https://docs.better-auth.rs)
