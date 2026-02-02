//! Event Schema Registry and Validation
//!
//! Provides schema management for events:
//! - Schema registration and versioning
//! - JSON Schema validation
//! - Schema evolution and migration
//! - Backward compatibility checking

mod registry;
mod validator;
mod migration;

pub use registry::{EventSchemaRegistry, EventSchema, SchemaRegistryConfig};
pub use validator::{SchemaValidator, JsonSchemaValidator, ValidationResult, ValidationError};
pub use migration::{SchemaMigration, MigrationPath, MigrationStrategy};
