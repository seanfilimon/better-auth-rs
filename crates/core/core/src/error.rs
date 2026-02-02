//! Error types for Better Auth.
//!
//! This module defines the `AuthError` enum which represents all possible
//! errors that can occur within the authentication system.

use thiserror::Error;

/// The main error type for Better Auth operations.
///
/// This enum covers all error cases that can occur during authentication,
/// session management, storage operations, and plugin execution.
#[derive(Debug, Error)]
pub enum AuthError {
    // ==================== Authentication Errors ====================
    /// The provided credentials are invalid.
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// The user was not found.
    #[error("User not found")]
    UserNotFound,

    /// The session was not found or has expired.
    #[error("Session not found or expired")]
    SessionNotFound,

    /// The session has expired.
    #[error("Session expired")]
    SessionExpired,

    /// The user's email has not been verified.
    #[error("Email not verified")]
    EmailNotVerified,

    /// The account is locked or disabled.
    #[error("Account locked")]
    AccountLocked,

    // ==================== Validation Errors ====================
    /// A required field is missing.
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    /// A field value is invalid.
    #[error("Invalid field value for '{field}': {reason}")]
    InvalidField { field: String, reason: String },

    /// The email format is invalid.
    #[error("Invalid email format")]
    InvalidEmail,

    /// The password does not meet requirements.
    #[error("Password does not meet requirements: {reason}")]
    WeakPassword { reason: String },

    // ==================== Storage Errors ====================
    /// A database operation failed.
    #[error("Database error: {message}")]
    DatabaseError { message: String },

    /// The requested record was not found.
    #[error("Record not found: {entity} with {key}={value}")]
    NotFound {
        entity: String,
        key: String,
        value: String,
    },

    /// A unique constraint was violated (e.g., duplicate email).
    #[error("Duplicate entry: {entity} with {field}={value} already exists")]
    DuplicateEntry {
        entity: String,
        field: String,
        value: String,
    },

    /// A schema migration failed.
    #[error("Migration error: {message}")]
    MigrationError { message: String },

    // ==================== Plugin Errors ====================
    /// A plugin operation failed.
    #[error("Plugin error in '{plugin}': {message}")]
    PluginError { plugin: String, message: String },

    /// A plugin is not configured or enabled.
    #[error("Plugin not enabled: {plugin}")]
    PluginNotEnabled { plugin: String },

    /// A plugin hook was rejected.
    #[error("Operation rejected by plugin '{plugin}': {reason}")]
    HookRejected { plugin: String, reason: String },

    // ==================== Token Errors ====================
    /// The token is invalid or malformed.
    #[error("Invalid token")]
    InvalidToken,

    /// The token has expired.
    #[error("Token expired")]
    TokenExpired,

    /// Token generation failed.
    #[error("Failed to generate token: {reason}")]
    TokenGenerationFailed { reason: String },

    // ==================== Rate Limiting ====================
    /// Too many requests have been made.
    #[error("Rate limit exceeded. Try again in {retry_after_seconds} seconds")]
    RateLimitExceeded { retry_after_seconds: u64 },

    // ==================== Configuration Errors ====================
    /// The configuration is invalid.
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    /// A required configuration value is missing.
    #[error("Missing configuration: {key}")]
    MissingConfiguration { key: String },

    // ==================== Internal Errors ====================
    /// An internal error occurred.
    #[error("Internal error: {message}")]
    InternalError { message: String },

    /// Serialization/deserialization failed.
    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    /// An unknown error occurred.
    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

impl AuthError {
    /// Creates a new database error.
    pub fn database(message: impl Into<String>) -> Self {
        Self::DatabaseError {
            message: message.into(),
        }
    }

    /// Creates a new not found error.
    pub fn not_found(entity: impl Into<String>, key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::NotFound {
            entity: entity.into(),
            key: key.into(),
            value: value.into(),
        }
    }

    /// Creates a new duplicate entry error.
    pub fn duplicate(entity: impl Into<String>, field: impl Into<String>, value: impl Into<String>) -> Self {
        Self::DuplicateEntry {
            entity: entity.into(),
            field: field.into(),
            value: value.into(),
        }
    }

    /// Creates a new plugin error.
    pub fn plugin(plugin: impl Into<String>, message: impl Into<String>) -> Self {
        Self::PluginError {
            plugin: plugin.into(),
            message: message.into(),
        }
    }

    /// Creates a new internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    /// Creates a new configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Returns true if this is a user-facing error (vs internal).
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidCredentials
                | Self::UserNotFound
                | Self::SessionNotFound
                | Self::SessionExpired
                | Self::EmailNotVerified
                | Self::AccountLocked
                | Self::MissingField { .. }
                | Self::InvalidField { .. }
                | Self::InvalidEmail
                | Self::WeakPassword { .. }
                | Self::InvalidToken
                | Self::TokenExpired
                | Self::RateLimitExceeded { .. }
        )
    }

    /// Returns an HTTP status code appropriate for this error.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::InvalidCredentials | Self::InvalidToken => 401,
            Self::AccountLocked | Self::EmailNotVerified => 403,
            Self::UserNotFound | Self::SessionNotFound | Self::NotFound { .. } => 404,
            Self::DuplicateEntry { .. } => 409,
            Self::MissingField { .. }
            | Self::InvalidField { .. }
            | Self::InvalidEmail
            | Self::WeakPassword { .. } => 422,
            Self::RateLimitExceeded { .. } => 429,
            _ => 500,
        }
    }
}

/// A Result type alias using AuthError.
pub type AuthResult<T> = Result<T, AuthError>;

impl From<serde_json::Error> for AuthError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AuthError::InvalidCredentials;
        assert_eq!(err.to_string(), "Invalid credentials");
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(AuthError::InvalidCredentials.status_code(), 401);
        assert_eq!(AuthError::UserNotFound.status_code(), 404);
        assert_eq!(AuthError::InvalidEmail.status_code(), 422);
    }

    #[test]
    fn test_is_user_error() {
        assert!(AuthError::InvalidCredentials.is_user_error());
        assert!(!AuthError::InternalError {
            message: "test".into()
        }
        .is_user_error());
    }
}
