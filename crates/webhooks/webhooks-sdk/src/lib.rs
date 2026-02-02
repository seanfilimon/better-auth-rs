//! # Better Auth Webhooks SDK
//!
//! SDK for integrating plugins with the Better Auth webhook system.
//!
//! This crate provides traits and utilities for plugins to:
//! - Register webhook endpoints
//! - Transform event payloads for webhooks
//! - Receive and verify incoming webhooks
//!
//! ## Example
//!
//! ```rust,ignore
//! use better_auth_webhooks_sdk::{WebhookProvider, WebhookEndpointConfig};
//!
//! pub struct MyPlugin;
//!
//! impl WebhookProvider for MyPlugin {
//!     fn webhook_endpoints() -> Vec<WebhookEndpointConfig> {
//!         vec![
//!             WebhookEndpointConfig::new("https://example.com/webhook")
//!                 .events(["myplugin.*"]),
//!         ]
//!     }
//! }
//! ```

mod traits;
mod builder;
mod client;

pub use traits::{WebhookProvider, WebhookTransformer, PluginWebhookRegistrar};
pub use builder::{WebhookEndpointConfig, WebhookConfigBuilder};
pub use client::{WebhookClient, WebhookClientBuilder};

// Re-export core webhook types for convenience
pub use better_auth_webhooks::{
    WebhookEndpoint, WebhookMetadata, EventFilter,
    WebhookJob, WebhookJobStatus, WebhookDelivery,
    WebhookReceiver, WebhookPayload,
    WebhookError, WebhookResult,
    RetryStrategy, ExponentialBackoff, LinearBackoff, FixedDelay,
    WebhookSystem, WebhookConfig,
};
