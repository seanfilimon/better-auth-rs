//! # Better Auth Core
//!
//! This crate provides the foundational types and traits for the Better Auth system.
//! It defines the core data structures (`User`, `Session`), error types, and the
//! trait interfaces that plugins and adapters must implement.

pub mod context;
pub mod error;
pub mod router;
pub mod schema;
pub mod traits;
pub mod types;

// Re-export commonly used items at the crate root
pub use error::{AuthError, AuthResult};
pub use schema::{
    core_schema, Field, FieldType, IndexDefinition, Migration, MigrationOp, MigrationRunner,
    ModelDefinition, ReferentialAction, SchemaBuilder, SchemaDefinition, SchemaDiff, SchemaDiffOp,
    SqlDialect,
};
pub use traits::{
    AuthExtension, AuthPlugin, ExtensionProvider, HookContext, SchemaProvider, StorageAdapter,
};
pub use types::{Account, Session, User};

// Re-export context types
pub use context::{AuthContext, RequestParts, SignInCredentials, SignUpData};

// Re-export event types from the events crate
pub use better_auth_events as events;
pub use better_auth_events::{
    auth_events, Event, EventBus, EventError, EventHandler, EventType, EventMetadata,
    EventRegistry, EventDefinition, EventMiddleware, EventEmitter,
};

// Re-export router types
pub use router::{CookieOptions, Method, Request, RequestHandler, Response, Route, Router};
