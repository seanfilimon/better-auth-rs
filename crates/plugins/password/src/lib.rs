//! # Better Auth Password Plugin
//!
//! This plugin provides email/password authentication for Better Auth.
//! It handles password hashing, verification, and password reset flows.

use async_trait::async_trait;
use better_auth_core::context::{AuthContext, SignInCredentials, SignUpData};
use better_auth_core::error::{AuthError, AuthResult};
use better_auth_core::schema::{Field, FieldType, ModelDefinition, SchemaBuilder};
use better_auth_core::traits::{AuthPlugin, ExtensionProvider, SchemaProvider};
use better_auth_core::types::User;
use better_auth_events_sdk::{EventDefinition, EventProvider};
use serde::{Deserialize, Serialize};

/// Password plugin configuration.
#[derive(Debug, Clone)]
pub struct PasswordConfig {
    /// Minimum password length.
    pub min_length: usize,
    /// Require uppercase letters.
    pub require_uppercase: bool,
    /// Require lowercase letters.
    pub require_lowercase: bool,
    /// Require numbers.
    pub require_numbers: bool,
    /// Require special characters.
    pub require_special: bool,
    /// Password reset token expiration (in seconds).
    pub reset_token_expiry: u64,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: false,
            require_lowercase: false,
            require_numbers: false,
            require_special: false,
            reset_token_expiry: 3600, // 1 hour
        }
    }
}

impl PasswordConfig {
    /// Creates a new password config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets minimum password length.
    pub fn min_length(mut self, len: usize) -> Self {
        self.min_length = len;
        self
    }

    /// Requires uppercase letters.
    pub fn require_uppercase(mut self) -> Self {
        self.require_uppercase = true;
        self
    }

    /// Requires lowercase letters.
    pub fn require_lowercase(mut self) -> Self {
        self.require_lowercase = true;
        self
    }

    /// Requires numbers.
    pub fn require_numbers(mut self) -> Self {
        self.require_numbers = true;
        self
    }

    /// Requires special characters.
    pub fn require_special(mut self) -> Self {
        self.require_special = true;
        self
    }

    /// Validates a password against the configuration.
    pub fn validate(&self, password: &str) -> Result<(), String> {
        if password.len() < self.min_length {
            return Err(format!(
                "Password must be at least {} characters",
                self.min_length
            ));
        }

        if self.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err("Password must contain at least one uppercase letter".to_string());
        }

        if self.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err("Password must contain at least one lowercase letter".to_string());
        }

        if self.require_numbers && !password.chars().any(|c| c.is_numeric()) {
            return Err("Password must contain at least one number".to_string());
        }

        if self.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err("Password must contain at least one special character".to_string());
        }

        Ok(())
    }
}

/// User extension fields for password authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PasswordUserExt {
    /// Hashed password.
    pub password_hash: Option<String>,
}

impl ExtensionProvider for PasswordUserExt {
    fn extends() -> &'static str {
        "user"
    }

    fn fields() -> Vec<Field> {
        vec![Field::optional("password_hash", FieldType::Text).private()]
    }
}

/// Password reset token model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetToken {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl SchemaProvider for PasswordResetToken {
    fn schema() -> Vec<ModelDefinition> {
        vec![ModelDefinition::new("password_reset_token")
            .field(Field::primary_key("id"))
            .field(
                Field::new("user_id", FieldType::String(36))
                    .references("user.id")
                    .on_delete(better_auth_core::schema::ReferentialAction::Cascade),
            )
            .field(Field::new("token", FieldType::String(255)).unique())
            .field(Field::new("expires_at", FieldType::Timestamp))
            .field(Field::new("used", FieldType::Boolean).default("false"))
            .field(Field::new("created_at", FieldType::Timestamp))]
    }
}

/// Trait for password operations on users.
pub trait PasswordExt {
    /// Gets the password hash.
    fn password_hash(&self) -> Option<String>;

    /// Sets the password hash.
    fn set_password_hash(&mut self, hash: impl Into<String>);

    /// Checks if user has a password set.
    fn has_password(&self) -> bool;
}

impl PasswordExt for User {
    fn password_hash(&self) -> Option<String> {
        self.get_extension("password_hash")
    }

    fn set_password_hash(&mut self, hash: impl Into<String>) {
        self.set_extension("password_hash", hash.into());
    }

    fn has_password(&self) -> bool {
        self.password_hash().is_some()
    }
}

/// The password authentication plugin.
pub struct PasswordPlugin {
    config: PasswordConfig,
}

impl PasswordPlugin {
    /// Creates a new password plugin.
    pub fn new(config: PasswordConfig) -> Self {
        Self { config }
    }

    /// Gets the configuration.
    pub fn config(&self) -> &PasswordConfig {
        &self.config
    }

    /// Hashes a password.
    ///
    /// Note: In production, use a proper password hashing library like argon2 or bcrypt.
    /// This is a placeholder implementation.
    pub fn hash_password(&self, password: &str) -> String {
        // PLACEHOLDER: In production, use argon2 or bcrypt
        // This is NOT secure and is only for demonstration
        format!("hashed:{}", password)
    }

    /// Verifies a password against a hash.
    pub fn verify_password(&self, password: &str, hash: &str) -> bool {
        // PLACEHOLDER: In production, use proper verification
        hash == format!("hashed:{}", password)
    }

    /// Validates a password against the configuration.
    pub fn validate_password(&self, password: &str) -> AuthResult<()> {
        self.config.validate(password).map_err(|reason| {
            AuthError::WeakPassword { reason }
        })
    }
}

impl Default for PasswordPlugin {
    fn default() -> Self {
        Self::new(PasswordConfig::default())
    }
}

impl EventProvider for PasswordPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "password.changed",
                "Emitted when a user's password is changed",
                "password",
            ),
            EventDefinition::simple(
                "password.reset_requested",
                "Emitted when a password reset is requested",
                "password",
            ),
            EventDefinition::simple(
                "password.reset_completed",
                "Emitted when a password reset is completed",
                "password",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "password"
    }
}

#[async_trait]
impl AuthPlugin for PasswordPlugin {
    fn id(&self) -> &'static str {
        "password"
    }

    fn name(&self) -> &'static str {
        "Email/Password Authentication"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        // Add password reset token table
        for model in PasswordResetToken::schema() {
            builder.add_model_mut(model);
        }
    }

    async fn on_before_signup(
        &self,
        _ctx: &AuthContext,
        data: &mut SignUpData,
    ) -> AuthResult<()> {
        // Validate password if provided
        if let Some(password) = &data.password {
            self.validate_password(password)?;
        }
        Ok(())
    }

    async fn on_before_signin(
        &self,
        _ctx: &AuthContext,
        creds: &SignInCredentials,
    ) -> AuthResult<()> {
        // Basic validation
        if creds.email.is_empty() {
            return Err(AuthError::MissingField {
                field: "email".to_string(),
            });
        }
        if creds.password.is_empty() {
            return Err(AuthError::MissingField {
                field: "password".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        let config = PasswordConfig::new()
            .min_length(8)
            .require_uppercase()
            .require_numbers();

        assert!(config.validate("Short1").is_err());
        assert!(config.validate("longenough1").is_err()); // No uppercase
        assert!(config.validate("LongEnough").is_err()); // No number
        assert!(config.validate("LongEnough1").is_ok());
    }

    #[test]
    fn test_password_hashing() {
        let plugin = PasswordPlugin::default();
        let hash = plugin.hash_password("mypassword");
        assert!(plugin.verify_password("mypassword", &hash));
        assert!(!plugin.verify_password("wrongpassword", &hash));
    }
}
