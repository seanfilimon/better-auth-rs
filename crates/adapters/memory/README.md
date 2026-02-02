# Better Auth Memory Adapter

In-memory storage adapter for Better Auth, ideal for testing and development.

## Overview

The Memory Adapter provides a simple, fast, in-memory implementation of the `StorageAdapter` trait. It's perfect for:

- **Testing**: Unit and integration tests
- **Development**: Quick local development without database setup
- **Prototyping**: Rapid application prototyping
- **Examples**: Documentation and example code

⚠️ **Warning**: Data is **not persistent** and will be lost when the application stops. Do not use in production.

## Features

- ✅ **Zero Configuration**: No database setup required
- ✅ **Fast**: All operations in memory
- ✅ **Thread-Safe**: Uses `Arc<RwLock<T>>` for concurrent access
- ✅ **Full Implementation**: Supports all `StorageAdapter` methods
- ✅ **Clean State**: Easy to reset between tests

## Quick Start

```rust
use better_auth_adapters_memory::MemoryAdapter;
use better_auth_core::AuthContext;

#[tokio::main]
async fn main() {
    // Create adapter
    let adapter = MemoryAdapter::new();
    
    // Use with AuthContext
    let ctx = AuthContext::builder()
        .adapter(adapter)
        .build();
    
    // Use as normal
    let user = ctx.create_user(signup_data).await?;
}
```

## Usage

### Basic Setup

```rust
use better_auth_adapters_memory::MemoryAdapter;

let adapter = MemoryAdapter::new();
```

### With Initial Data

```rust
use better_auth_adapters_memory::MemoryAdapter;
use better_auth_core::{User, Session};

let mut adapter = MemoryAdapter::new();

// Pre-populate with test data
let user = User {
    id: "user_1".to_string(),
    email: "test@example.com".to_string(),
    email_verified: true,
    name: Some("Test User".to_string()),
    image: None,
    created_at: Utc::now(),
    updated_at: Utc::now(),
};

adapter.create_user(user).await?;
```

### Testing

Perfect for unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use better_auth_adapters_memory::MemoryAdapter;
    
    #[tokio::test]
    async fn test_user_creation() {
        let adapter = MemoryAdapter::new();
        let ctx = AuthContext::builder()
            .adapter(adapter)
            .build();
        
        let user = ctx.create_user(SignUpData {
            email: "test@example.com".to_string(),
            password: Some("password123".to_string()),
            name: None,
        }).await.unwrap();
        
        assert_eq!(user.email, "test@example.com");
    }
    
    #[tokio::test]
    async fn test_session_management() {
        let adapter = MemoryAdapter::new();
        // Test session operations...
    }
}
```

### Reset Between Tests

```rust
use better_auth_adapters_memory::MemoryAdapter;

#[tokio::test]
async fn test_with_clean_state() {
    let adapter = MemoryAdapter::new();
    
    // Test 1
    adapter.create_user(user1).await?;
    
    // Reset for clean state
    adapter.clear().await;
    
    // Test 2 with empty state
    let users = adapter.list_users().await?;
    assert_eq!(users.len(), 0);
}
```

## Implementation Details

### Internal Storage

Uses `Arc<RwLock<HashMap<K, V>>>` for thread-safe storage:

```rust
pub struct MemoryAdapter {
    users: Arc<RwLock<HashMap<String, User>>>,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    accounts: Arc<RwLock<HashMap<String, Account>>>,
    verification_tokens: Arc<RwLock<HashMap<String, VerificationToken>>>,
}
```

### Concurrency

- **Read Operations**: Multiple concurrent reads
- **Write Operations**: Exclusive write access
- **No Deadlocks**: Simple lock ordering

### Performance

- **Get by ID**: O(1)
- **List All**: O(n)
- **Filter/Search**: O(n) with iteration
- **Update**: O(1)
- **Delete**: O(1)

## API Reference

Implements all `StorageAdapter` methods:

### User Operations

```rust
async fn create_user(&self, user: User) -> AuthResult<User>;
async fn get_user(&self, id: &str) -> AuthResult<Option<User>>;
async fn get_user_by_email(&self, email: &str) -> AuthResult<Option<User>>;
async fn update_user(&self, id: &str, updates: UserUpdates) -> AuthResult<User>;
async fn delete_user(&self, id: &str) -> AuthResult<()>;
async fn list_users(&self) -> AuthResult<Vec<User>>;
```

### Session Operations

```rust
async fn create_session(&self, session: Session) -> AuthResult<Session>;
async fn get_session(&self, token: &str) -> AuthResult<Option<Session>>;
async fn update_session(&self, token: &str, updates: SessionUpdates) -> AuthResult<Session>;
async fn delete_session(&self, token: &str) -> AuthResult<()>;
async fn delete_sessions_for_user(&self, user_id: &str) -> AuthResult<()>;
```

### Account Operations

```rust
async fn create_account(&self, account: Account) -> AuthResult<Account>;
async fn get_account(&self, provider: &str, provider_account_id: &str) -> AuthResult<Option<Account>>;
async fn list_accounts_for_user(&self, user_id: &str) -> AuthResult<Vec<Account>>;
async fn delete_account(&self, id: &str) -> AuthResult<()>;
```

### Verification Token Operations

```rust
async fn create_verification_token(&self, token: VerificationToken) -> AuthResult<VerificationToken>;
async fn get_verification_token(&self, identifier: &str, token: &str) -> AuthResult<Option<VerificationToken>>;
async fn delete_verification_token(&self, identifier: &str, token: &str) -> AuthResult<()>;
```

## Utility Methods

Additional methods for testing:

```rust
impl MemoryAdapter {
    /// Create a new empty adapter
    pub fn new() -> Self;
    
    /// Clear all data (useful for test cleanup)
    pub async fn clear(&self);
    
    /// Get count of users
    pub async fn user_count(&self) -> usize;
    
    /// Get count of sessions
    pub async fn session_count(&self) -> usize;
    
    /// Get count of accounts
    pub async fn account_count(&self) -> usize;
}
```

## Limitations

1. **No Persistence**: Data lost on restart
2. **No Transactions**: No ACID guarantees
3. **Memory Only**: Limited by RAM
4. **No Indexing**: Linear search for queries
5. **Single Process**: Cannot share between processes

## When to Use

✅ **Good For**:
- Unit tests
- Integration tests
- Local development
- Examples and demos
- Prototyping
- CI/CD testing

❌ **Not For**:
- Production environments
- Data that needs persistence
- High-performance requirements
- Multi-process applications
- Large datasets

## Migration to Production Adapter

When ready for production, switch to a persistent adapter:

```rust
// Development
#[cfg(debug_assertions)]
let adapter = MemoryAdapter::new();

// Production
#[cfg(not(debug_assertions))]
let adapter = PostgresAdapter::new(pool).await?;

let ctx = AuthContext::builder()
    .adapter(adapter)
    .build();
```

## Testing Example

Complete test example:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use better_auth_adapters_memory::MemoryAdapter;
    use better_auth_core::{AuthContext, SignUpData};
    
    async fn setup() -> AuthContext {
        let adapter = MemoryAdapter::new();
        AuthContext::builder()
            .adapter(adapter)
            .build()
    }
    
    #[tokio::test]
    async fn test_full_auth_flow() {
        let ctx = setup().await;
        
        // Sign up
        let user = ctx.create_user(SignUpData {
            email: "test@example.com".to_string(),
            password: Some("password123".to_string()),
            name: Some("Test User".to_string()),
        }).await.unwrap();
        
        // Sign in
        let session = ctx.sign_in(SignInCredentials {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        }).await.unwrap();
        
        // Verify session
        let verified = ctx.verify_session(&session.token).await.unwrap();
        assert!(verified.is_some());
        
        // Sign out
        ctx.sign_out(&session.token).await.unwrap();
        
        // Verify session deleted
        let verified = ctx.verify_session(&session.token).await.unwrap();
        assert!(verified.is_none());
    }
}
```

## Dependencies

Minimal dependencies:

- `better-auth-core` - Core types and traits
- `tokio` - Async runtime
- `async-trait` - Async trait support

## See Also

- [Storage Adapter Trait](../../core/core/README.md#storage-adapter)
- [PostgreSQL Adapter](../postgres/README.md) (coming soon)
- [Testing Guide](../../../.cursor/rules/testing-standards.mdc)
