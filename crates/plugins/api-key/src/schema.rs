//! Schema definitions for the API Key plugin.

use better_auth_core::schema::{Field, FieldType, IndexDefinition, ModelDefinition, ReferentialAction};
use better_auth_core::traits::SchemaProvider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an API key in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique identifier.
    pub id: String,
    /// Optional name for the API key.
    pub name: Option<String>,
    /// Starting characters for display.
    pub start: Option<String>,
    /// API key prefix.
    pub prefix: Option<String>,
    /// The hashed API key.
    pub key: String,
    /// The user ID.
    pub user_id: String,
    /// Refill interval in milliseconds.
    pub refill_interval: Option<i64>,
    /// Amount to refill.
    pub refill_amount: Option<i32>,
    /// Last refill timestamp.
    pub last_refill_at: Option<DateTime<Utc>>,
    /// Whether the key is enabled.
    pub enabled: bool,
    /// Whether rate limiting is enabled.
    pub rate_limit_enabled: bool,
    /// Rate limit time window in milliseconds.
    pub rate_limit_time_window: Option<i64>,
    /// Maximum requests per time window.
    pub rate_limit_max: Option<i32>,
    /// Current request count in the window.
    pub request_count: i32,
    /// Remaining requests (for quota-based limiting).
    pub remaining: Option<i32>,
    /// Last request timestamp.
    pub last_request: Option<DateTime<Utc>>,
    /// Expiration timestamp.
    pub expires_at: Option<DateTime<Utc>>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Permissions (JSON).
    pub permissions: Option<String>,
    /// Metadata (JSON).
    pub metadata: Option<String>,
}

impl ApiKey {
    /// Creates a new API key.
    pub fn new(user_id: impl Into<String>, key: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            start: None,
            prefix: None,
            key: key.into(),
            user_id: user_id.into(),
            refill_interval: None,
            refill_amount: None,
            last_refill_at: None,
            enabled: true,
            rate_limit_enabled: true,
            rate_limit_time_window: None,
            rate_limit_max: None,
            request_count: 0,
            remaining: None,
            last_request: None,
            expires_at: None,
            created_at: now,
            updated_at: now,
            permissions: None,
            metadata: None,
        }
    }

    /// Sets the name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets the starting characters.
    pub fn with_start(mut self, start: impl Into<String>) -> Self {
        self.start = Some(start.into());
        self
    }

    /// Sets the expiration.
    pub fn with_expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Sets the remaining count.
    pub fn with_remaining(mut self, remaining: i32) -> Self {
        self.remaining = Some(remaining);
        self
    }

    /// Sets rate limit configuration.
    pub fn with_rate_limit(mut self, time_window: i64, max_requests: i32) -> Self {
        self.rate_limit_enabled = true;
        self.rate_limit_time_window = Some(time_window);
        self.rate_limit_max = Some(max_requests);
        self
    }

    /// Sets permissions.
    pub fn with_permissions(mut self, permissions: HashMap<String, Vec<String>>) -> Self {
        self.permissions = Some(serde_json::to_string(&permissions).unwrap_or_default());
        self
    }

    /// Sets metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(serde_json::to_string(&metadata).unwrap_or_default());
        self
    }

    /// Gets permissions as a HashMap.
    pub fn get_permissions(&self) -> HashMap<String, Vec<String>> {
        self.permissions
            .as_ref()
            .and_then(|p| serde_json::from_str(p).ok())
            .unwrap_or_default()
    }

    /// Gets metadata as a JSON value.
    pub fn get_metadata(&self) -> serde_json::Value {
        self.metadata
            .as_ref()
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or(serde_json::Value::Null)
    }

    /// Checks if the key has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| Utc::now() > e).unwrap_or(false)
    }

    /// Checks if the key is valid (enabled and not expired).
    pub fn is_valid(&self) -> bool {
        self.enabled && !self.is_expired()
    }

    /// Checks if the key has the required permissions.
    pub fn has_permissions(&self, required: &HashMap<String, Vec<String>>) -> bool {
        let permissions = self.get_permissions();
        
        for (resource, actions) in required {
            let Some(allowed_actions) = permissions.get(resource) else {
                return false;
            };
            
            for action in actions {
                if !allowed_actions.contains(action) {
                    return false;
                }
            }
        }
        
        true
    }
}

/// Schema provider for API keys.
pub struct ApiKeySchema;

impl SchemaProvider for ApiKeySchema {
    fn schema() -> Vec<ModelDefinition> {
        vec![
            ModelDefinition::new("api_key")
                .field(Field::primary_key("id"))
                .field(Field::optional("name", FieldType::String(255)))
                .field(Field::optional("start", FieldType::String(20)))
                .field(Field::optional("prefix", FieldType::String(50)))
                .field(Field::new("key", FieldType::String(255)))
                .field(
                    Field::new("user_id", FieldType::String(36))
                        .references("user.id")
                        .on_delete(ReferentialAction::Cascade),
                )
                .field(Field::optional("refill_interval", FieldType::BigInt))
                .field(Field::optional("refill_amount", FieldType::Integer))
                .field(Field::optional("last_refill_at", FieldType::Timestamp))
                .field(Field::new("enabled", FieldType::Boolean).default("true"))
                .field(Field::new("rate_limit_enabled", FieldType::Boolean).default("true"))
                .field(Field::optional("rate_limit_time_window", FieldType::BigInt))
                .field(Field::optional("rate_limit_max", FieldType::Integer))
                .field(Field::new("request_count", FieldType::Integer).default("0"))
                .field(Field::optional("remaining", FieldType::Integer))
                .field(Field::optional("last_request", FieldType::Timestamp))
                .field(Field::optional("expires_at", FieldType::Timestamp))
                .field(Field::new("created_at", FieldType::Timestamp))
                .field(Field::new("updated_at", FieldType::Timestamp))
                .field(Field::optional("permissions", FieldType::Text))
                .field(Field::optional("metadata", FieldType::Json))
                .index(IndexDefinition::new(
                    "idx_api_key_user",
                    vec!["user_id".to_string()],
                ))
                .index(IndexDefinition::new(
                    "idx_api_key_key",
                    vec!["key".to_string()],
                )),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_creation() {
        let key = ApiKey::new("user_123", "hashed_key")
            .with_name("My API Key")
            .with_prefix("sk_live_");

        assert_eq!(key.user_id, "user_123");
        assert_eq!(key.name, Some("My API Key".to_string()));
        assert!(key.enabled);
        assert!(key.is_valid());
    }

    #[test]
    fn test_permissions() {
        let mut permissions = HashMap::new();
        permissions.insert("files".to_string(), vec!["read".to_string(), "write".to_string()]);
        
        let key = ApiKey::new("user_123", "key")
            .with_permissions(permissions.clone());

        assert!(key.has_permissions(&permissions));
        
        let mut required = HashMap::new();
        required.insert("files".to_string(), vec!["delete".to_string()]);
        assert!(!key.has_permissions(&required));
    }

    #[test]
    fn test_schema_definition() {
        let schema = ApiKeySchema::schema();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].name, "api_key");
    }
}
