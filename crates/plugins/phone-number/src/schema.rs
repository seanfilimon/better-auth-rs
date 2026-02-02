//! Schema definitions for the Phone Number plugin.

use better_auth_core::schema::{Field, FieldType, IndexDefinition, ModelDefinition};
use better_auth_core::traits::{ExtensionProvider, SchemaProvider};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User extension fields for phone number authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhoneNumberUserExt {
    /// The user's phone number.
    pub phone_number: Option<String>,
    /// Whether the phone number is verified.
    pub phone_number_verified: bool,
}

impl ExtensionProvider for PhoneNumberUserExt {
    fn extends() -> &'static str {
        "user"
    }

    fn fields() -> Vec<Field> {
        vec![
            Field::optional("phone_number", FieldType::String(20)),
            Field::new("phone_number_verified", FieldType::Boolean).default("false"),
        ]
    }
}

/// Represents a phone verification record in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneVerification {
    /// Unique identifier.
    pub id: String,
    /// The phone number this verification is for.
    pub phone_number: String,
    /// The OTP code.
    pub code: String,
    /// When this verification expires.
    pub expires_at: DateTime<Utc>,
    /// Number of verification attempts.
    pub attempts: i32,
    /// When this verification was created.
    pub created_at: DateTime<Utc>,
}

impl PhoneVerification {
    /// Creates a new phone verification record.
    pub fn new(
        phone_number: impl Into<String>,
        code: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            phone_number: phone_number.into(),
            code: code.into(),
            expires_at,
            attempts: 0,
            created_at: Utc::now(),
        }
    }

    /// Checks if the verification has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Increments the attempt counter.
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }
}

/// Schema provider for phone verification.
pub struct PhoneNumberSchema;

impl SchemaProvider for PhoneNumberSchema {
    fn schema() -> Vec<ModelDefinition> {
        vec![
            ModelDefinition::new("phone_verification")
                .field(Field::primary_key("id"))
                .field(Field::new("phone_number", FieldType::String(20)))
                .field(Field::new("code", FieldType::String(10)))
                .field(Field::new("expires_at", FieldType::Timestamp))
                .field(Field::new("attempts", FieldType::Integer).default("0"))
                .field(Field::new("created_at", FieldType::Timestamp))
                .index(IndexDefinition::new(
                    "idx_phone_verification_phone",
                    vec!["phone_number".to_string()],
                ))
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_phone_verification_creation() {
        let verification = PhoneVerification::new(
            "+1234567890",
            "123456",
            Utc::now() + Duration::minutes(5),
        );

        assert_eq!(verification.phone_number, "+1234567890");
        assert_eq!(verification.code, "123456");
        assert_eq!(verification.attempts, 0);
        assert!(!verification.is_expired());
    }

    #[test]
    fn test_schema_definition() {
        let schema = PhoneNumberSchema::schema();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].name, "phone_verification");
    }

    #[test]
    fn test_user_extension_fields() {
        let fields = PhoneNumberUserExt::fields();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "phone_number");
        assert_eq!(fields[1].name, "phone_number_verified");
    }
}
