//! # Better Auth Events
//!
//! Core event system for Better Auth providing:
//! - Typed events with namespaces and versioning
//! - Pub/sub event bus with async handlers
//! - Middleware chain for event processing
//! - Event registry for discovery and validation
//!
//! ## Example
//!
//! ```rust,ignore
//! use better_auth_events::{Event, EventBus, EventType};
//!
//! let bus = EventBus::new();
//!
//! // Subscribe to events
//! bus.on("user.created", |event| async move {
//!     println!("User created: {:?}", event.payload);
//!     Ok(())
//! }).await;
//!
//! // Emit events
//! let event = Event::new(
//!     EventType::new("user", "created"),
//!     serde_json::json!({ "user_id": "123" })
//! );
//! bus.emit(event).await;
//! ```

mod event;
mod bus;
mod handler;
mod emitter;
mod registry;
mod middleware;
mod error;
pub mod store;
pub mod replay;
pub mod dlq;
pub mod schema;

pub use event::{Event, EventType, EventMetadata};
pub use bus::EventBus;
pub use handler::{EventHandler, BoxedHandler, HandlerResult};
pub use emitter::EventEmitter;
pub use registry::{EventRegistry, EventDefinition};
pub use middleware::{EventMiddleware, MiddlewareChain, LoggingMiddleware, MetricsMiddleware, ValidationMiddleware};
pub use error::{EventError, EventResult};
pub use store::{EventStore, StoredEvent, EventQuery, EventOrdering, EventStream, EventStreamSubscription, MemoryEventStore};
pub use replay::{ReplayEngine, ReplayConfig, ReplaySpeed, ReplayResult};
pub use dlq::{DeadLetterQueue, DeadLetter, DLQConfig, DLQStats, DLQStorage, InMemoryDLQStorage};
pub use schema::{EventSchemaRegistry, EventSchema, SchemaValidator, JsonSchemaValidator, ValidationResult};

/// Standard auth events namespace constants.
pub mod auth_events {
    /// Event emitted when a user is created.
    pub const USER_CREATED: &str = "user.created";
    /// Event emitted when a user is updated.
    pub const USER_UPDATED: &str = "user.updated";
    /// Event emitted when a user is deleted.
    pub const USER_DELETED: &str = "user.deleted";
    /// Event emitted when a session is created.
    pub const SESSION_CREATED: &str = "session.created";
    /// Event emitted when a session is destroyed.
    pub const SESSION_DESTROYED: &str = "session.destroyed";
    /// Event emitted on successful signin.
    pub const SIGNIN_SUCCESS: &str = "signin.success";
    /// Event emitted on failed signin.
    pub const SIGNIN_FAILED: &str = "signin.failed";
    /// Event emitted on signup.
    pub const SIGNUP_SUCCESS: &str = "signup.success";
    /// Event emitted when email is verified.
    pub const EMAIL_VERIFIED: &str = "email.verified";
    /// Event emitted when password is changed.
    pub const PASSWORD_CHANGED: &str = "password.changed";
    /// Event emitted when password reset is requested.
    pub const PASSWORD_RESET_REQUESTED: &str = "password.reset_requested";
}
