# Better Auth Events SDK

SDK for plugins to define and interact with the Better Auth event system.

## Overview

The Events SDK provides a clean interface for plugins to:

- Define events declaratively
- Provide event metadata and documentation
- Subscribe to events without circular dependencies
- Build complex event definitions with type safety

## Purpose

This SDK separates the **interface** (used by plugins) from the **implementation** (events crate), preventing circular dependencies:

```
Plugin → Events SDK → Events Implementation
```

Without this separation, plugins would depend on the events crate, which would create tight coupling.

## Key Features

- **EventProvider Trait**: Plugins declare their events
- **EventDefinition Builder**: Fluent API for event definitions
- **Type Safety**: Compile-time event validation
- **Zero Runtime Cost**: Pure trait-based design
- **Documentation**: Built-in event documentation

## Quick Start

### Basic Event Provider

```rust
use better_auth_events_sdk::{EventProvider, EventDefinition};

pub struct MyPlugin;

impl EventProvider for MyPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "user.login",
                "Emitted when a user logs in",
                "my_plugin"
            ),
            EventDefinition::simple(
                "user.logout",
                "Emitted when a user logs out",
                "my_plugin"
            ),
        ]
    }
    
    fn event_source() -> &'static str {
        "my_plugin"
    }
}
```

### Complex Event Definitions

```rust
use better_auth_events_sdk::{EventDefinition, FieldDefinition, FieldType};

EventDefinition::builder()
    .name("user.profile_updated")
    .description("Emitted when user profile is modified")
    .source("profile_plugin")
    .version("1.0.0")
    .field(FieldDefinition {
        name: "user_id".to_string(),
        field_type: FieldType::String,
        required: true,
        description: Some("ID of the user".to_string()),
    })
    .field(FieldDefinition {
        name: "changes".to_string(),
        field_type: FieldType::Object,
        required: true,
        description: Some("Object containing changed fields".to_string()),
    })
    .field(FieldDefinition {
        name: "changed_by".to_string(),
        field_type: FieldType::String,
        required: false,
        description: Some("ID of user who made the change".to_string()),
    })
    .build()
```

### Event Metadata

```rust
EventDefinition::builder()
    .name("payment.processed")
    .description("Payment transaction completed")
    .source("payment_plugin")
    .category("transaction")
    .severity(EventSeverity::High)
    .requires_acknowledgment(true)
    .retention_days(90)
    .build()
```

## Core Types

### EventDefinition

Complete event specification:

```rust
pub struct EventDefinition {
    pub name: String,
    pub description: String,
    pub source: String,
    pub version: String,
    pub fields: Vec<FieldDefinition>,
    pub category: Option<String>,
    pub severity: EventSeverity,
    pub requires_acknowledgment: bool,
    pub retention_days: Option<u32>,
}
```

### FieldDefinition

Event field specification:

```rust
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub description: Option<String>,
    pub default_value: Option<Value>,
    pub validation: Option<FieldValidation>,
}
```

### FieldType

Supported field types:

```rust
pub enum FieldType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    DateTime,
    Uuid,
    Email,
    Url,
}
```

### EventSeverity

Event importance level:

```rust
pub enum EventSeverity {
    Low,       // Informational events
    Normal,    // Standard events
    High,      // Important events
    Critical,  // System-critical events
}
```

## EventProvider Trait

The core trait plugins implement:

```rust
pub trait EventProvider {
    /// Returns all events this plugin can emit
    fn provided_events() -> Vec<EventDefinition>;
    
    /// Source identifier for events
    fn event_source() -> &'static str;
    
    /// Optional: Event categories this plugin uses
    fn event_categories() -> Vec<String> {
        Vec::new()
    }
    
    /// Optional: Validate event before emission
    fn validate_event(&self, event: &Event) -> Result<(), ValidationError> {
        Ok(())
    }
}
```

## Builder Pattern

Fluent API for complex definitions:

```rust
use better_auth_events_sdk::EventDefinitionBuilder;

let event = EventDefinitionBuilder::new("order.created")
    .description("New order placed")
    .source("ecommerce")
    .version("2.0.0")
    .category("orders")
    .severity(EventSeverity::Normal)
    .field(FieldDefinition {
        name: "order_id".into(),
        field_type: FieldType::Uuid,
        required: true,
        description: Some("Unique order identifier".into()),
        ..Default::default()
    })
    .field(FieldDefinition {
        name: "total".into(),
        field_type: FieldType::Number,
        required: true,
        description: Some("Order total in cents".into()),
        validation: Some(FieldValidation::Range(0.0, 1_000_000.0)),
        ..Default::default()
    })
    .field(FieldDefinition {
        name: "items".into(),
        field_type: FieldType::Array,
        required: true,
        description: Some("Array of order items".into()),
        ..Default::default()
    })
    .retention_days(365)
    .requires_acknowledgment(false)
    .build();
```

## Validation

Add field validation rules:

```rust
pub enum FieldValidation {
    MinLength(usize),
    MaxLength(usize),
    Range(f64, f64),
    Regex(String),
    OneOf(Vec<String>),
    Custom(Box<dyn Fn(&Value) -> bool>),
}

// Example usage
FieldDefinition {
    name: "status".into(),
    field_type: FieldType::String,
    required: true,
    validation: Some(FieldValidation::OneOf(vec![
        "pending".into(),
        "processing".into(),
        "completed".into(),
        "failed".into(),
    ])),
    ..Default::default()
}
```

## Real-World Example

Complete plugin implementation:

```rust
use better_auth_events_sdk::{EventProvider, EventDefinition, FieldDefinition, FieldType, EventSeverity};

pub struct TwoFactorPlugin;

impl EventProvider for TwoFactorPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::builder()
                .name("two_factor.enabled")
                .description("User enabled two-factor authentication")
                .source("two_factor")
                .version("1.0.0")
                .category("security")
                .severity(EventSeverity::High)
                .field(FieldDefinition {
                    name: "user_id".into(),
                    field_type: FieldType::String,
                    required: true,
                    description: Some("ID of user enabling 2FA".into()),
                    ..Default::default()
                })
                .field(FieldDefinition {
                    name: "method".into(),
                    field_type: FieldType::String,
                    required: true,
                    description: Some("2FA method (totp, sms, etc)".into()),
                    validation: Some(FieldValidation::OneOf(vec![
                        "totp".into(),
                        "sms".into(),
                        "email".into(),
                    ])),
                    ..Default::default()
                })
                .retention_days(365)
                .requires_acknowledgment(true)
                .build(),
                
            EventDefinition::simple(
                "two_factor.verified",
                "User successfully verified 2FA code",
                "two_factor"
            ),
            
            EventDefinition::simple(
                "two_factor.failed",
                "User failed 2FA verification",
                "two_factor"
            ),
        ]
    }
    
    fn event_source() -> &'static str {
        "two_factor"
    }
    
    fn event_categories() -> Vec<String> {
        vec!["security".to_string(), "authentication".to_string()]
    }
}
```

## Usage in Plugins

Plugins use this SDK to declare events:

```rust
// In plugin's lib.rs
use better_auth_events_sdk::EventProvider;

impl EventProvider for MyPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        // Define events
    }
    
    fn event_source() -> &'static str {
        "my_plugin"
    }
}

// Events are automatically registered when plugin loads
```

## Design Principles

1. **Declarative**: Events defined through data, not code
2. **Type-Safe**: Strong typing for event structure
3. **Self-Documenting**: Descriptions and metadata built-in
4. **Versioned**: Support event schema evolution
5. **Validated**: Compile-time and runtime validation

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_definition() {
        let events = MyPlugin::provided_events();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].name, "user.login");
        assert_eq!(events[0].source, "my_plugin");
    }
}
```

## Dependencies

Minimal dependencies:

- `serde` - Serialization
- `serde_json` - JSON support
- `chrono` - DateTime types

## See Also

- [Events System](../events/README.md) - Event bus implementation
- [Plugin Development Guide](../../../.cursor/rules/plugin-development.mdc)
- [Core Traits](../../core/core/README.md)
