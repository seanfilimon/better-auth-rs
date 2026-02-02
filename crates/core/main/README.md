# Better Auth Main

The main entry point crate that re-exports all Better Auth functionality with convenient APIs.

## Overview

`better-auth-main` (published as `better-auth`) is the primary crate users depend on. It:

- Re-exports core functionality from `better-auth-core`
- Provides convenient prelude imports
- Includes example applications
- Offers high-level builder patterns
- Feature-gates optional plugins and adapters

## Purpose

This crate acts as a "facade" that simplifies the import story for users:

```rust
// Instead of:
use better_auth_core::{AuthContext, User, Session};
use better_auth_events::{Event, EventBus};
use better_auth_plugin_oauth::OAuthPlugin;

// Users can write:
use better_auth::prelude::*;
```

## Features

All plugins and adapters are feature-gated:

```toml
[dependencies]
better-auth = { version = "0.1.0", features = ["oauth", "two-factor", "jwt"] }
```

### Available Features

**Core** (always enabled):
- `core` - Core types and traits
- `events` - Event system
- `webhooks` - Webhook system

**Authentication Plugins**:
- `oauth` - OAuth 2.0 providers
- `password` - Password authentication
- `jwt` - JWT tokens
- `two-factor` - TOTP 2FA
- `magic-link` - Magic link auth
- `email-otp` - Email OTP
- `phone-number` - SMS auth
- `anonymous` - Anonymous sessions
- `passkey` - WebAuthn/FIDO2
- `api-key` - API key authentication

**Access Control**:
- `access` - RBAC and permissions

**Adapters**:
- `memory` - In-memory storage (testing)
- `postgres` - PostgreSQL adapter (coming soon)
- `mysql` - MySQL adapter (coming soon)
- `sqlite` - SQLite adapter (coming soon)

**Integrations**:
- `axum` - Axum framework integration
- `actix` - Actix-web integration (coming soon)

**Utilities**:
- `macros` - Procedural macros
- `cli` - CLI tools (coming soon)

**Meta Features**:
- `full` - Enable all features
- `all-plugins` - Enable all auth plugins
- `all-adapters` - Enable all storage adapters

## Usage

### Basic Setup

```rust
use better_auth::prelude::*;

#[tokio::main]
async fn main() {
    let auth = AuthContext::builder()
        .adapter(MemoryAdapter::new())
        .build();
    
    // Use auth context
}
```

### With Plugins

```rust
use better_auth::prelude::*;
use better_auth::plugins::{PasswordPlugin, OAuthPlugin};

#[tokio::main]
async fn main() {
    let auth = AuthContext::builder()
        .adapter(MemoryAdapter::new())
        .plugin(PasswordPlugin::new())
        .plugin(OAuthPlugin::builder()
            .provider("google", google_config)
            .build())
        .build();
}
```

### With Framework Integration

```rust
use better_auth::prelude::*;
use better_auth::integrations::axum::AuthLayer;
use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let auth = AuthContext::builder()
        .adapter(my_adapter)
        .build();
    
    let app = Router::new()
        .route("/", get(|| async { "Hello" }))
        .layer(AuthLayer::new(auth));
    
    // Serve app
}
```

## Prelude

The prelude includes commonly used items:

```rust
use better_auth::prelude::*;

// Includes:
// - AuthContext, AuthPlugin, StorageAdapter
// - User, Session, Account
// - AuthResult, AuthError
// - Event, EventBus, EventHandler
// - SchemaBuilder, ModelDefinition
// - And more...
```

## Examples

See `examples/` directory:

- `basic_app.rs` - Minimal authentication setup
- `with_plugins.rs` - Using multiple plugins (planned)
- `custom_adapter.rs` - Custom storage adapter (planned)
- `axum_integration.rs` - Full Axum example (planned)

Run examples:

```bash
cargo run --example basic_app
```

## Project Structure

```
crates/core/main/
├── src/
│   ├── lib.rs        # Re-exports and feature gates
│   └── prelude.rs    # Prelude module
├── examples/
│   └── basic_app.rs  # Example applications
└── Cargo.toml        # Feature definitions
```

## Feature Flags in Cargo.toml

```toml
[features]
default = ["memory"]

# Core (always available)
core = []
events = []
webhooks = []

# Plugins
oauth = ["dep:better-auth-plugin-oauth"]
password = ["dep:better-auth-plugin-password"]
jwt = ["dep:better-auth-plugin-jwt"]
two-factor = ["dep:better-auth-plugin-two-factor"]
# ... more plugins

# Adapters
memory = ["dep:better-auth-adapter-memory"]
postgres = ["dep:better-auth-adapter-postgres"]

# Integrations
axum = ["dep:better-auth-integration-axum"]

# Meta
full = ["all-plugins", "all-adapters"]
all-plugins = ["oauth", "password", "jwt", "two-factor", ...]
all-adapters = ["postgres", "mysql", "sqlite"]
```

## Testing

Run tests:

```bash
cargo test -p better-auth-main
```

Test with all features:

```bash
cargo test -p better-auth-main --all-features
```

## Design Principles

1. **Convenient Imports**: Single `use better_auth::prelude::*` should cover 90% of use cases
2. **Feature Gated**: Keep compile times fast with optional features
3. **No Implementation**: This crate only re-exports, no implementation code
4. **Stable API**: This is the public API surface, keep it stable
5. **Good Examples**: Provide comprehensive examples for common scenarios

## Migration from Core

If you're currently using `better-auth-core` directly:

```rust
// Old
use better_auth_core::{AuthContext, User};
use better_auth_plugin_oauth::OAuthPlugin;

// New
use better_auth::prelude::*;
use better_auth::plugins::OAuthPlugin;
```

## Contributing

When adding new crates to the workspace:

1. Add feature flag to `Cargo.toml`
2. Add re-export to `lib.rs`
3. Update prelude if it's commonly used
4. Add example if it's a major feature
5. Update this README

## See Also

- [Core Documentation](../core/README.md)
- [Plugin Development](../../../.cursor/rules/plugin-development.mdc)
- [Architecture Guide](../../../.cursor/rules/crate-structure.mdc)
