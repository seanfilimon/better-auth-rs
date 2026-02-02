# Better Auth Events

A robust, production-grade event bus system for reactive authentication workflows.

## Overview

The events system provides a comprehensive event-driven architecture for Better Auth, enabling:

- **Event Bus**: Central event dispatcher with middleware support
- **Event Store**: Persistent storage for event history and replay
- **Dead Letter Queue (DLQ)**: Failed event handling with retry logic
- **Replay Engine**: Time-travel debugging and event sourcing
- **Schema Registry**: Versioned event definitions with validation
- **Multiple Backends**: In-memory, PostgreSQL, and Redis storage

## Key Features

- ✅ **Async/Await**: Full tokio async support
- ✅ **Type-Safe Events**: Strongly typed event definitions
- ✅ **Middleware Pipeline**: Transform, filter, and log events
- ✅ **Event Replay**: Replay historical events for debugging
- ✅ **Schema Validation**: Enforce event structure and versioning
- ✅ **DLQ Support**: Automatically handle failed events
- ✅ **Multi-Backend**: Pluggable storage adapters

## Architecture

```
EventEmitter → EventBus → Middleware → Handlers
                    ↓
                EventStore → [Memory|Postgres|Redis]
                    ↓
                   DLQ (for failed events)
```

## Quick Start

### Basic Usage

```rust
use better_auth_events::{EventBus, Event, EventHandler};
use async_trait::async_trait;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create event bus
    let bus = EventBus::new();
    
    // Register handler
    bus.on("user.created", |event: &Event| async {
        println!("User created: {:?}", event.data);
        Ok(())
    }).await;
    
    // Emit event
    bus.emit(Event::new("user.created")
        .with_data(json!({ "user_id": "123" })))
        .await?;
    
    Ok(())
}
```

### With Event Store

```rust
use better_auth_events::{EventBus, EventStore};
use better_auth_events::store::MemoryStore;

let store = MemoryStore::new();
let bus = EventBus::builder()
    .with_store(store)
    .build();

// Events are now persisted
bus.emit(Event::new("user.created")).await?;

// Query historical events
let events = store.query()
    .event_type("user.created")
    .since(start_time)
    .fetch()
    .await?;
```

### With Schema Validation

```rust
use better_auth_events::schema::{SchemaRegistry, EventSchema};

let mut registry = SchemaRegistry::new();

registry.register(EventSchema::builder()
    .name("user.created")
    .version("1.0.0")
    .field("user_id", FieldType::String, true)
    .field("email", FieldType::String, true)
    .build())?;

// Events are validated against schema
bus.emit(Event::new("user.created")
    .with_data(json!({
        "user_id": "123",
        "email": "user@example.com"
    })))
    .await?; // ✅ Valid

bus.emit(Event::new("user.created")
    .with_data(json!({ "user_id": "123" })))
    .await?; // ❌ ValidationError: missing required field 'email'
```

## Components

### Event Bus (`bus.rs`)

Central event dispatcher:

```rust
pub struct EventBus {
    handlers: Arc<RwLock<HashMap<String, Vec<Box<dyn EventHandler>>>>>,
    middleware: Vec<Box<dyn EventMiddleware>>,
    store: Option<Arc<dyn EventStore>>,
}
```

### Event (`event.rs`)

Core event structure:

```rust
pub struct Event {
    pub id: String,
    pub event_type: EventType,
    pub source: String,
    pub data: Value,
    pub metadata: EventMetadata,
    pub timestamp: DateTime<Utc>,
}
```

### Event Handler (`handler.rs`)

Async event handler trait:

```rust
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &Event) -> Result<(), EventError>;
}
```

### Event Store (`store/`)

Persistent event storage:

- `MemoryStore` - In-memory (testing)
- `PostgresStore` - PostgreSQL backend
- `RedisStore` - Redis backend

### Dead Letter Queue (`dlq/`)

Failed event management:

```rust
use better_auth_events::dlq::DLQHandler;

let dlq = DLQHandler::new(store);

// Failed events are automatically sent to DLQ
// Retry with exponential backoff
dlq.retry_failed().await?;
```

### Replay Engine (`replay/`)

Event replay for debugging:

```rust
use better_auth_events::replay::ReplayEngine;

let engine = ReplayEngine::new(store, bus);

// Replay events from specific time
engine.replay()
    .from(start_time)
    .to(end_time)
    .event_type("user.created")
    .execute()
    .await?;
```

### Schema Registry (`schema/`)

Event schema management:

```rust
use better_auth_events::schema::SchemaRegistry;

let mut registry = SchemaRegistry::new();
registry.register(my_schema)?;

// Validate events
registry.validate(&event)?;

// Generate migrations for schema changes
let migration = registry.generate_migration(old_schema, new_schema)?;
```

## Event Types

Built-in event types:

```rust
pub enum EventType {
    UserCreated,
    UserUpdated,
    UserDeleted,
    SessionCreated,
    SessionExpired,
    AccountLinked,
    AccountUnlinked,
    Custom(String),
}
```

## Middleware

Add middleware to transform/filter events:

```rust
use better_auth_events::EventMiddleware;

struct LoggingMiddleware;

#[async_trait]
impl EventMiddleware for LoggingMiddleware {
    async fn process(&self, event: Event) -> Result<Event, EventError> {
        println!("Event: {}", event.event_type);
        Ok(event)
    }
}

let bus = EventBus::builder()
    .with_middleware(LoggingMiddleware)
    .build();
```

## Storage Adapters

### Memory Store

For testing and development:

```rust
use better_auth_events::store::MemoryStore;

let store = MemoryStore::new();
```

### PostgreSQL Store

Production-ready persistence:

```rust
use better_auth_events::store::PostgresStore;

let store = PostgresStore::new("postgres://localhost/auth").await?;
```

### Redis Store

High-performance caching:

```rust
use better_auth_events::store::RedisStore;

let store = RedisStore::new("redis://localhost").await?;
```

## Error Handling

Comprehensive error types:

```rust
pub enum EventError {
    HandlerError(String),
    StoreError(String),
    ValidationError(String),
    SerializationError(String),
    NotFound(String),
}
```

## Testing

Run tests:

```bash
cargo test -p better-auth-events
```

With integration tests:

```bash
cargo test -p better-auth-events --features integration-tests
```

## Performance

- **Async**: Non-blocking event handling
- **Batching**: Bulk event emission
- **Buffering**: Configurable buffer sizes
- **Concurrency**: Parallel handler execution

## Configuration

```rust
let bus = EventBus::builder()
    .max_handlers_per_event(100)
    .buffer_size(1000)
    .enable_batching(true)
    .batch_size(50)
    .build();
```

## Migration SQL

Event store schema:

```sql
CREATE TABLE event_store (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    source TEXT NOT NULL,
    data JSONB NOT NULL,
    metadata JSONB,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_event_type ON event_store(event_type);
CREATE INDEX idx_timestamp ON event_store(timestamp);
CREATE INDEX idx_source ON event_store(source);
```

## Design Principles

1. **Reliability**: Events must not be lost
2. **Performance**: Minimal overhead, async by default
3. **Type Safety**: Strongly typed events
4. **Observability**: Full event history and replay
5. **Extensibility**: Pluggable stores and middleware

## See Also

- [Events SDK](../events-sdk/README.md) - SDK for plugin integration
- [Webhooks](../../webhooks/webhooks/README.md) - Webhook delivery system
- [Core](../../core/core/README.md) - Core types and traits
