//! Schema definitions for the Two-Factor plugin.

use better_auth_core::schema::{Field, FieldType, IndexDefinition, ModelDefinition, ReferentialAction};
use better_auth_core::traits::{ExtensionProvider, SchemaProvider};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User extension fields for two-factor authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TwoFactorUserExt {
    /// Whether 2FA is enabled for this user.
    pub two_factor_enabled: bool,
}

impl ExtensionProvider for TwoFactorUserExt {
    fn extends() -> &'static str {
        "user"
    }

    fn fields() -> Vec<Field> {
        vec![
            Field::new("two_factor_enabled", FieldType::Boolean).default("false"),
        ]
    }
}

/// Represents two-factor data in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorData {
    /// Unique identifier.
    pub id: String,
    /// The user ID.
    pub user_id: String,
    /// The TOTP secret (encrypted).
    pub secret: String,
    /// Backup codes (encrypted, JSON array).
    pub backup_codes: String,
    /// When this was created.
    pub created_at: DateTime<Utc>,
}

impl TwoFactorData {
    /// Creates new two-factor data.
    pub fn new(user_id: impl Into<String>, secret: impl Into<String>, backup_codes: Vec<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            secret: secret.into(),
            backup_codes: serde_json::to_string(&backup_codes).unwrap_or_default(),
            created_at: Utc::now(),
        }
    }

    /// Gets the backup codes as a vector.
    pub fn get_backup_codes(&self) -> Vec<String> {
        serde_json::from_str(&self.backup_codes).unwrap_or_default()
    }

    /// Sets the backup codes.
    pub fn set_backup_codes(&mut self, codes: Vec<String>) {
        self.backup_codes = serde_json::to_string(&codes).unwrap_or_default();
    }
}

/// Represents a trusted device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    /// Unique identifier.
    pub id: String,
    /// The user ID.
    pub user_id: String,
    /// Hash of the device identifier.
    pub device_hash: String,
    /// User agent string.
    pub user_agent: Option<String>,
    /// IP address.
    pub ip_address: Option<String>,
    /// When this trust expires.
    pub expires_at: DateTime<Utc>,
    /// When this was created.
    pub created_at: DateTime<Utc>,
}

impl TrustedDevice {
    /// Creates a new trusted device.
    pub fn new(
        user_id: impl Into<String>,
        device_hash: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            device_hash: device_hash.into(),
            user_agent: None,
            ip_address: None,
            expires_at,
            created_at: Utc::now(),
        }
    }

    /// Checks if the device trust has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Schema provider for two-factor authentication.
pub struct TwoFactorSchema;

impl SchemaProvider for TwoFactorSchema {
    fn schema() -> Vec<ModelDefinition> {
        vec![
            // Two-factor data table
            ModelDefinition::new("two_factor")
                .field(Field::primary_key("id"))
                .field(
                    Field::new("user_id", FieldType::String(36))
                        .references("user.id")
                        .on_delete(ReferentialAction::Cascade)
                        .unique(),
                )
                .field(Field::new("secret", FieldType::Text).private())
                .field(Field::new("backup_codes", FieldType::Text).private())
                .field(Field::new("created_at", FieldType::Timestamp))
                .index(IndexDefinition::unique(
                    "idx_two_factor_user",
                    vec!["user_id".to_string()],
                )),
            
            // Trusted devices table
            ModelDefinition::new("trusted_device")
                .field(Field::primary_key("id"))
                .field(
                    Field::new("user_id", FieldType::String(36))
                        .references("user.id")
                        .on_delete(ReferentialAction::Cascade),
                )
                .field(Field::new("device_hash", FieldType::String(255)))
                .field(Field::optional("user_agent", FieldType::Text))
                .field(Field::optional("ip_address", FieldType::String(45)))
                .field(Field::new("expires_at", FieldType::Timestamp))
                .field(Field::new("created_at", FieldType::Timestamp))
                .index(IndexDefinition::new(
                    "idx_trusted_device_user",
                    vec!["user_id".to_string()],
                ))
                .index(IndexDefinition::unique(
                    "idx_trusted_device_hash",
                    vec!["user_id".to_string(), "device_hash".to_string()],
                )),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_factor_data_creation() {
        let data = TwoFactorData::new(
            "user_123",
            "secret_abc",
            vec!["code1".to_string(), "code2".to_string()],
        );

        assert_eq!(data.user_id, "user_123");
        assert_eq!(data.secret, "secret_abc");
        assert_eq!(data.get_backup_codes(), vec!["code1", "code2"]);
    }

    #[test]
    fn test_schema_definition() {
        let schema = TwoFactorSchema::schema();
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].name, "two_factor");
        assert_eq!(schema[1].name, "trusted_device");
    }
}
