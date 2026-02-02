//! Schema definitions for the Email OTP plugin.

use better_auth_core::schema::{Field, FieldType, ModelDefinition, ReferentialAction};
use better_auth_core::traits::SchemaProvider;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents an email OTP record in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailOtp {
    /// Unique identifier.
    pub id: String,
    /// The email address this OTP is for.
    pub email: String,
    /// The OTP code (may be hashed).
    pub otp: String,
    /// The type of OTP (sign-in, email-verification, forget-password).
    pub otp_type: String,
    /// When this OTP expires.
    pub expires_at: DateTime<Utc>,
    /// Number of verification attempts.
    pub attempts: i32,
    /// When this OTP was created.
    pub created_at: DateTime<Utc>,
}

impl EmailOtp {
    /// Creates a new email OTP record.
    pub fn new(
        email: impl Into<String>,
        otp: impl Into<String>,
        otp_type: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            email: email.into(),
            otp: otp.into(),
            otp_type: otp_type.into(),
            expires_at,
            attempts: 0,
            created_at: Utc::now(),
        }
    }

    /// Checks if the OTP has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Increments the attempt counter.
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }
}

/// Schema provider for email OTP.
pub struct EmailOtpSchema;

impl SchemaProvider for EmailOtpSchema {
    fn schema() -> Vec<ModelDefinition> {
        vec![
            ModelDefinition::new("email_otp")
                .field(Field::primary_key("id"))
                .field(Field::new("email", FieldType::String(255)))
                .field(Field::new("otp", FieldType::String(255)))
                .field(Field::new("otp_type", FieldType::String(50)))
                .field(Field::new("expires_at", FieldType::Timestamp))
                .field(Field::new("attempts", FieldType::Integer).default("0"))
                .field(Field::new("created_at", FieldType::Timestamp))
                .index(better_auth_core::schema::IndexDefinition::new(
                    "idx_email_otp_email_type",
                    vec!["email".to_string(), "otp_type".to_string()],
                ))
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_email_otp_creation() {
        let otp = EmailOtp::new(
            "test@example.com",
            "123456",
            "sign-in",
            Utc::now() + Duration::minutes(5),
        );

        assert_eq!(otp.email, "test@example.com");
        assert_eq!(otp.otp, "123456");
        assert_eq!(otp.otp_type, "sign-in");
        assert_eq!(otp.attempts, 0);
        assert!(!otp.is_expired());
    }

    #[test]
    fn test_schema_definition() {
        let schema = EmailOtpSchema::schema();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].name, "email_otp");
    }
}
