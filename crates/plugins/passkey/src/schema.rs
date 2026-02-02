//! Schema definitions for the Passkey plugin.

use better_auth_core::schema::{Field, FieldType, IndexDefinition, ModelDefinition, ReferentialAction};
use better_auth_core::traits::SchemaProvider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a passkey credential in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passkey {
    /// Unique identifier.
    pub id: String,
    /// Optional name for the passkey.
    pub name: Option<String>,
    /// The public key.
    pub public_key: String,
    /// The user ID.
    pub user_id: String,
    /// The credential ID (base64 encoded).
    pub credential_id: String,
    /// The signature counter.
    pub counter: i64,
    /// Device type: "singleDevice" or "multiDevice".
    pub device_type: String,
    /// Whether the credential is backed up.
    pub backed_up: bool,
    /// Transports (JSON array).
    pub transports: Option<String>,
    /// When this passkey was created.
    pub created_at: DateTime<Utc>,
    /// Authenticator AAGUID.
    pub aaguid: Option<String>,
}

impl Passkey {
    /// Creates a new passkey.
    pub fn new(
        user_id: impl Into<String>,
        credential_id: impl Into<String>,
        public_key: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            public_key: public_key.into(),
            user_id: user_id.into(),
            credential_id: credential_id.into(),
            counter: 0,
            device_type: "singleDevice".to_string(),
            backed_up: false,
            transports: None,
            created_at: Utc::now(),
            aaguid: None,
        }
    }

    /// Sets the passkey name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the device type.
    pub fn with_device_type(mut self, device_type: impl Into<String>) -> Self {
        self.device_type = device_type.into();
        self
    }

    /// Sets the backed up flag.
    pub fn with_backed_up(mut self, backed_up: bool) -> Self {
        self.backed_up = backed_up;
        self
    }

    /// Sets the transports.
    pub fn with_transports(mut self, transports: Vec<String>) -> Self {
        self.transports = Some(serde_json::to_string(&transports).unwrap_or_default());
        self
    }

    /// Gets the transports as a vector.
    pub fn get_transports(&self) -> Vec<String> {
        self.transports
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default()
    }

    /// Increments the counter.
    pub fn increment_counter(&mut self, new_counter: i64) {
        if new_counter > self.counter {
            self.counter = new_counter;
        }
    }
}

/// Schema provider for passkeys.
pub struct PasskeySchema;

impl SchemaProvider for PasskeySchema {
    fn schema() -> Vec<ModelDefinition> {
        vec![
            ModelDefinition::new("passkey")
                .field(Field::primary_key("id"))
                .field(Field::optional("name", FieldType::String(255)))
                .field(Field::new("public_key", FieldType::Text))
                .field(
                    Field::new("user_id", FieldType::String(36))
                        .references("user.id")
                        .on_delete(ReferentialAction::Cascade),
                )
                .field(Field::new("credential_id", FieldType::String(512)).unique())
                .field(Field::new("counter", FieldType::BigInt).default("0"))
                .field(Field::new("device_type", FieldType::String(50)))
                .field(Field::new("backed_up", FieldType::Boolean).default("false"))
                .field(Field::optional("transports", FieldType::Text))
                .field(Field::new("created_at", FieldType::Timestamp))
                .field(Field::optional("aaguid", FieldType::String(36)))
                .index(IndexDefinition::new(
                    "idx_passkey_user",
                    vec!["user_id".to_string()],
                ))
                .index(IndexDefinition::unique(
                    "idx_passkey_credential",
                    vec!["credential_id".to_string()],
                )),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passkey_creation() {
        let passkey = Passkey::new("user_123", "cred_abc", "public_key_xyz")
            .with_name("My Passkey")
            .with_transports(vec!["usb".to_string(), "nfc".to_string()]);

        assert_eq!(passkey.user_id, "user_123");
        assert_eq!(passkey.credential_id, "cred_abc");
        assert_eq!(passkey.name, Some("My Passkey".to_string()));
        assert_eq!(passkey.get_transports(), vec!["usb", "nfc"]);
    }

    #[test]
    fn test_schema_definition() {
        let schema = PasskeySchema::schema();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].name, "passkey");
    }
}
