# Better Auth Core

The foundational crate for the Better Auth authentication framework. This crate provides the core types, traits, error handling, and schema system that all other components depend on.

## Overview

`better-auth-core` is the base layer of Better Auth, containing:

- **Core Types**: `User`, `Session`, `Account` data structures
- **Trait Definitions**: `AuthPlugin`, `StorageAdapter`, `ExtensionProvider`
- **Error Handling**: Comprehensive `AuthError` type and `AuthResult` wrapper
- **Schema System**: Database schema definition and migration generation
- **Router Abstraction**: Framework-agnostic request/response types
- **Context**: Central `AuthContext` for managing authentication state

## Architecture Role

Core sits at the foundation of the Better Auth stack:

```
Applications → Integrations → Plugins → Core → Adapters
```

All other crates depend on core, but core depends on no other Better Auth crates (except events).

## Key Components

### Types (`types.rs`)

Core data structures for authentication:

```rust
use better_auth_core::{User, Session, Account};

let user = User {
    id: "user_123".to_string(),
    email: "user@example.com".to_string(),
    email_verified: true,
    name: Some("John Doe".to_string()),
    image: None,
    created_at: Utc::now(),
    updated_at: Utc::now(),
};
```

### Traits (`traits.rs`)

Extensibility through traits:

```rust
use better_auth_core::traits::{AuthPlugin, StorageAdapter};
use async_trait::async_trait;

#[async_trait]
impl StorageAdapter for MyAdapter {
    async fn get_user(&self, id: &str) -> AuthResult<Option<User>> {
        // Implementation
    }
    // ... other methods
}
```

### Schema System (`schema/`)

Define database schemas programmatically:

```rust
use better_auth_core::schema::{SchemaBuilder, ModelDefinition, Field, FieldType};

let schema = SchemaBuilder::new()
    .add_model(ModelDefinition {
        name: "users".to_string(),
        fields: vec![
            Field::new("id", FieldType::Text),
            Field::new("email", FieldType::Text),
        ],
    })
    .build();
```

### Context (`context/`)

Central authentication context:

```rust
use better_auth_core::context::AuthContext;

let ctx = AuthContext::builder()
    .adapter(my_adapter)
    .plugin(my_plugin)
    .build();

// Use context for operations
let user = ctx.get_user("user_123").await?;
```

### Router (`router/`)

Framework-agnostic routing:

```rust
use better_auth_core::router::{Router, Route, Method};

let mut router = Router::new();
router.add_route(Route {
    path: "/api/signin".to_string(),
    method: Method::Post,
    handler: Box::new(signin_handler),
});
```

## Error Handling

All operations return `AuthResult<T>`:

```rust
use better_auth_core::{AuthResult, AuthError};

fn validate_email(email: &str) -> AuthResult<()> {
    if !email.contains('@') {
        return Err(AuthError::ValidationError(
            "Invalid email format".to_string()
        ));
    }
    Ok(())
}
```

Error variants include:
- `DatabaseError`: Storage operation failures
- `ValidationError`: Input validation failures
- `NotFound`: Resource not found
- `Unauthorized`: Authentication failures
- `ConfigurationError`: Setup/config issues
- `PluginError`: Plugin-specific errors

## Dependencies

Minimal dependencies for maximum flexibility:

- `serde`: Serialization/deserialization
- `async-trait`: Async trait support
- `chrono`: Date/time handling
- `uuid`: UUID generation
- `thiserror`: Error type derivation
- `better-auth-events`: Event system integration

No web framework or database dependencies at this level.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
better-auth-core = "0.1.0"
```

Basic usage:

```rust
use better_auth_core::{AuthContext, User, AuthResult};

async fn example() -> AuthResult<()> {
    let ctx = AuthContext::builder()
        .adapter(my_adapter)
        .build();
    
    let user = ctx.create_user(SignUpData {
        email: "user@example.com".to_string(),
        password: Some("secure_password".to_string()),
        name: Some("John Doe".to_string()),
    }).await?;
    
    Ok(())
}
```

## Testing

Run tests:

```bash
cargo test -p better-auth-core
```

Integration tests are located in `tests/integration_tests.rs`.

## Design Principles

1. **No Plugin Dependencies**: Core never depends on plugins
2. **Trait-Based**: Extensibility through traits, not concrete types
3. **Minimal Dependencies**: Only essential dependencies
4. **Type Safety**: Leverage Rust's type system for correctness
5. **Framework Agnostic**: No web framework coupling

## See Also

- [Plugin Development Guide](../../../.cursor/rules/plugin-development.mdc)
- [Architecture Overview](../../../.cursor/rules/crate-structure.mdc)
- [better-auth-macros](../macros/) - Procedural macros
- [better-auth-main](../main/) - Main entry point
