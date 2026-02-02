//! Schema definitions for the Magic Link plugin.

use better_auth_core::schema::{Field, FieldType, IndexDefinition, ModelDefinition};
use better_auth_core::traits::SchemaProvider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a magic link token in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicLinkToken {
    /// Unique identifier.
    pub id: String,
    /// The email address this token is for.
    pub email: String,
    /// The token value (may be hashed).
    pub token: String,
    /// When this token expires.
    pub expires_at: DateTime<Utc>,
    /// Whether this token has been used.
    pub used: bool,
    /// When this token was created.
    pub created_at: DateTime<Utc>,
}

impl MagicLinkToken {
    /// Creates a new magic link token.
    pub fn new(
        email: impl Into<String>,
        token: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            email: email.into(),
            token: token.into(),
            expires_at,
            used: false,
            created_at: Utc::now(),
        }
    }

    /// Checks if the token has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Checks if the token is valid (not expired and not used).
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.used
    }

    /// Marks the token as used.
    pub fn mark_used(&mut self) {
        self.used = true;
    }
}

/// Schema provider for magic link tokens.
pub struct MagicLinkSchema;

impl SchemaProvider for MagicLinkSchema {
    fn schema() -> Vec<ModelDefinition> {
        vec![
            ModelDefinition::new("magic_link_token")
                .field(Field::primary_key("id"))
                .field(Field::new("email", FieldType::String(255)))
                .field(Field::new("token", FieldType::String(255)).unique())
                .field(Field::new("expires_at", FieldType::Timestamp))
                .field(Field::new("used", FieldType::Boolean).default("false"))
                .field(Field::new("created_at", FieldType::Timestamp))
                .index(IndexDefinition::new(
                    "idx_magic_link_email",
                    vec!["email".to_string()],
                ))
                .index(IndexDefinition::unique(
                    "idx_magic_link_token",
                    vec!["token".to_string()],
                ))
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_magic_link_token_creation() {
        let token = MagicLinkToken::new(
            "test@example.com",
            "abc123",
            Utc::now() + Duration::minutes(5),
        );

        assert_eq!(token.email, "test@example.com");
        assert_eq!(token.token, "abc123");
        assert!(!token.used);
        assert!(token.is_valid());
    }

    #[test]
    fn test_schema_definition() {
        let schema = MagicLinkSchema::schema();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].name, "magic_link_token");
    }
}
