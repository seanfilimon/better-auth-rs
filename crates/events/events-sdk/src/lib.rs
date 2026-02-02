//! # Better Auth Events SDK
//!
//! SDK for integrating plugins with the Better Auth event system.
//!
//! This crate provides traits and utilities for plugins to:
//! - Declare events they emit
//! - Subscribe to events from other plugins
//! - Build and emit events with proper metadata
//!
//! ## Example
//!
//! ```rust,ignore
//! use better_auth_events_sdk::{EventProvider, EventSubscriber};
//!
//! pub struct MyPlugin;
//!
//! impl EventProvider for MyPlugin {
//!     fn provided_events() -> Vec<EventDefinition> {
//!         vec![
//!             EventDefinition::simple("myplugin.action", "Action performed", "myplugin"),
//!         ]
//!     }
//! }
//!
//! impl EventSubscriber for MyPlugin {
//!     fn subscribed_events() -> Vec<String> {
//!         vec!["user.created".to_string()]
//!     }
//! }
//! ```

mod traits;
mod builder;

pub use traits::{EventProvider, EventSubscriber, PluginEventEmitter};
pub use builder::{PluginEventBuilder, EventPayloadBuilder};

// Re-export core event types for convenience
pub use better_auth_events::{
    Event, EventType, EventMetadata, EventBus, EventHandler, EventError, EventResult,
    EventDefinition, EventRegistry, EventEmitter,
    auth_events,
};
