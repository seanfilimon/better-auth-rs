# Better Auth (Rust)

A comprehensive, modular authentication framework for Rust applications, inspired by Better Auth for Node.js. Better Auth provides a flexible, plugin-based architecture for handling authentication, authorization, sessions, and user management.

## 🚀 Features

- **Modular Plugin System**: Extend functionality with plugins for OAuth, 2FA, JWT, Passkeys, and more
- **Multiple Storage Adapters**: Support for PostgreSQL, MySQL, SQLite, and in-memory storage
- **Event-Driven Architecture**: React to authentication events with a powerful event bus
- **Webhook System**: Built-in webhook delivery with retry logic and signature verification
- **Type-Safe**: Full type safety with Rust's strong type system
- **Framework Agnostic**: Core is independent of web frameworks (Axum integration provided)
- **Production Ready**: Circuit breakers, rate limiting, dead letter queues, and comprehensive error handling

## 📦 Architecture

Better Auth follows a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────┐
│ Applications/Binaries                   │ (server, CLI tools)
├─────────────────────────────────────────┤
│ Integrations (Framework)                │ (Axum, Actix, etc.)
├─────────────────────────────────────────┤
│ Plugins (Features)                      │ (OAuth, 2FA, etc.)
├─────────────────────────────────────────┤
│ SDK Layers                              │ (events-sdk, webhooks-sdk)
├─────────────────────────────────────────┤
│ Core Systems                            │ (events, webhooks)
├─────────────────────────────────────────┤
│ Core (Types & Traits)                   │ (foundational)
├─────────────────────────────────────────┤
│ Storage Adapters                        │ (database abstraction)
└─────────────────────────────────────────┘
```

## 🏗️ Project Structure

```
better-auth/
├── crates/
│   ├── core/                    # Core types, traits, and schema system
│   │   ├── core/                # Foundational types and traits
│   │   ├── macros/              # Procedural macros
│   │   └── main/                # Main entry point
│   ├── adapters/                # Storage adapters
│   │   └── memory/              # In-memory adapter for testing
│   ├── events/                  # Event system
│   │   ├── events/              # Event bus implementation
│   │   └── events-sdk/          # SDK for plugin event integration
│   ├── webhooks/                # Webhook system
│   │   ├── webhooks/            # Webhook delivery implementation
│   │   └── webhooks-sdk/        # SDK for webhook consumers
│   ├── plugins/                 # Authentication plugins
│   │   ├── oauth/               # OAuth 2.0 provider support
│   │   ├── two-factor/          # Two-factor authentication (TOTP)
│   │   ├── jwt/                 # JWT token management
│   │   ├── password/            # Password hashing and validation
│   │   ├── email-otp/           # Email-based OTP
│   │   ├── magic-link/          # Magic link authentication
│   │   ├── phone-number/        # Phone number authentication
│   │   ├── anonymous/           # Anonymous user sessions
│   │   ├── passkey/             # WebAuthn/Passkey support
│   │   ├── api-key/             # API key authentication
│   │   ├── access/              # Role-based access control (RBAC)
│   │   ├── otp-utils/           # Shared OTP utilities
│   │   └── integrations/        # Framework integrations
│   │       └── axum/            # Axum web framework integration
│   └── infra/                   # Infrastructure
│       ├── server/              # Main server application
│       ├── admin/               # Admin dashboard API
│       └── docs/                # Documentation generator
├── migrations/                  # Database migrations
└── .cursor/                     # Cursor IDE rules and standards
```

## 🚦 Quick Start

### Installation

Add Better Auth to your `Cargo.toml`:

```toml
[dependencies]
better-auth = "0.1.0"
better-auth-oauth = "0.1.0"
better-auth-password = "0.1.0"
better-auth-axum = "0.1.0"
```

### Basic Example

```rust
use better_auth::{AuthContext, AuthPlugin};
use better_auth_password::PasswordPlugin;
use better_auth_adapters_memory::MemoryAdapter;
use better_auth_axum::AuthLayer;

#[tokio::main]
async fn main() {
    // Create storage adapter
    let adapter = MemoryAdapter::new();
    
    // Build auth context with plugins
    let auth = AuthContext::builder()
        .adapter(adapter)
        .plugin(PasswordPlugin::new())
        .build();
    
    // Use with Axum
    let app = Router::new()
        .layer(AuthLayer::new(auth))
        .route("/api/auth/*", auth_routes());
    
    // Start server
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## 🔌 Available Plugins

### Authentication Methods
- **Password**: Traditional username/password authentication with bcrypt hashing
- **OAuth**: OAuth 2.0 support for Google, GitHub, Discord, and custom providers
- **Magic Link**: Passwordless authentication via email links
- **Email OTP**: One-time password via email
- **Phone Number**: SMS-based authentication
- **Passkey**: WebAuthn/FIDO2 passwordless authentication
- **Anonymous**: Temporary anonymous sessions
- **API Key**: API key generation and validation

### Security & Access Control
- **JWT**: JSON Web Token generation and validation
- **Two-Factor**: TOTP-based two-factor authentication with backup codes
- **Access**: Role-based access control (RBAC) with permissions

### Infrastructure
- **OTP Utils**: Shared utilities for OTP generation and rate limiting

## 🗄️ Storage Adapters

- **Memory**: In-memory storage for testing and development
- **PostgreSQL**: Production-ready PostgreSQL adapter (coming soon)
- **MySQL**: MySQL adapter (coming soon)
- **SQLite**: SQLite adapter (coming soon)

## 🎯 Core Features

### Event System
React to authentication events throughout your application:

```rust
// Listen to events
ctx.events().on("user.created", |event| async {
    println!("New user: {}", event.data);
    Ok(())
}).await?;

// Emit custom events
ctx.events().emit(Event::new("user.verified")).await?;
```

### Webhook Delivery
Send webhooks to external services with automatic retries:

```rust
// Register webhook endpoint
ctx.webhooks().register_endpoint(
    "https://api.example.com/webhooks",
    vec!["user.created", "user.updated"]
).await?;

// Webhooks are delivered automatically when events occur
```

### Schema Management
Define and migrate database schemas programmatically:

```rust
let schema = SchemaBuilder::new()
    .add_model(ModelDefinition {
        name: "custom_table",
        fields: vec![
            Field::new("id", FieldType::Text),
            Field::new("data", FieldType::Json),
        ],
    })
    .build();
```

## 🧪 Testing

Run all tests:

```bash
cargo test --workspace
```

Run tests for specific crate:

```bash
cargo test -p better-auth-core
cargo test -p better-auth-plugin-oauth
```

## 📚 Documentation

- **API Documentation**: Run `cargo doc --open` to generate and view docs
- **Architecture Guide**: See `.cursor/rules/crate-structure.mdc`
- **Plugin Development**: See `.cursor/rules/plugin-development.mdc`
- **Testing Standards**: See `.cursor/rules/testing-standards.mdc`

## 🏢 Deployment

### Docker

```bash
docker-compose up -d
```

### Kubernetes

```bash
kubectl apply -f crates/infra/deploy/k8s/
```

## 🤝 Contributing

Contributions are welcome! Please ensure:

1. All tests pass: `cargo test --workspace`
2. Code is formatted: `cargo fmt --all`
3. No clippy warnings: `cargo clippy --workspace -- -D warnings`
4. Follow the architectural guidelines in `.cursor/rules/`

## 📄 License

This project is licensed under the MIT License.

## 🙏 Acknowledgments

Inspired by [Better Auth](https://github.com/better-auth/better-auth) for Node.js.

## 🔗 Links

- **Repository**: https://github.com/better-auth/better-auth-rs
- **Documentation**: https://docs.better-auth.rs (coming soon)
- **Issues**: https://github.com/better-auth/better-auth-rs/issues
