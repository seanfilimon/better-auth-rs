//! # Better Auth Webhooks
//!
//! Core webhook system for Better Auth providing:
//! - Webhook endpoint management
//! - Event-driven webhook delivery
//! - Pluggable queue backends
//! - Retry strategies with exponential backoff
//! - HMAC signature verification
//!
//! ## Example
//!
//! ```rust,ignore
//! use better_auth_webhooks::{WebhookSystem, WebhookEndpoint, EventFilter};
//!
//! let system = WebhookSystem::new();
//!
//! // Register an endpoint
//! let endpoint = WebhookEndpoint::new("https://example.com/webhook", "secret123")
//!     .with_events(EventFilter::Pattern(vec!["user.*".to_string()]));
//! system.register_endpoint(endpoint).await;
//!
//! // Connect to event bus
//! system.connect_to_events(&event_bus).await;
//! ```

mod endpoint;
mod delivery;
mod queue;
mod signature;
mod retry;
mod receiver;
mod storage;
mod error;
mod system;
pub mod circuit_breaker;
pub mod rate_limiter;

pub use endpoint::{WebhookEndpoint, WebhookMetadata, EventFilter};
pub use delivery::{WebhookJob, WebhookJobStatus, WebhookDelivery, DeliveryEngine};
pub use queue::{WebhookQueue, InMemoryQueue, QueueError};
pub use signature::{WebhookSigner, SignatureVersion};
pub use retry::{RetryStrategy, ExponentialBackoff, LinearBackoff, FixedDelay};
pub use receiver::{WebhookReceiver, WebhookPayload};
pub use storage::WebhookStorage;
pub use error::{WebhookError, WebhookResult};
pub use system::{WebhookSystem, WebhookConfig};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use rate_limiter::{WebhookRateLimiter, EndpointRateLimit, RateLimitPermit, RateLimitInfo};
