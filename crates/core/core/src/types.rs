//! Core data types for Better Auth.
//!
//! This module defines the canonical `User` and `Session` structs that form
//! the foundation of the authentication system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Represents an authenticated user in the system.
///
/// The `User` struct contains the base fields that are always present,
/// plus an `extensions` map that holds plugin-specific data. Plugins
/// provide trait-based accessors to interact with their extension data
/// in a type-safe manner.
///
/// # Example
///
/// ```rust
/// use better_auth_core::User;
///
/// let user = User::new("user_123".to_string(), "user@example.com".to_string());
/// assert!(!user.email_verified);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier for the user (typically a UUID or CUID)
    pub id: String,

    /// User's email address
    pub email: String,

    /// Whether the user's email has been verified
    #[serde(default)]
    pub email_verified: bool,

    /// Optional display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional profile image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Timestamp when the user was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the user was last updated
    pub updated_at: DateTime<Utc>,

    /// Extension data from plugins.
    ///
    /// This map holds arbitrary key-value pairs that plugins can use
    /// to store additional user data. Plugins provide typed accessors
    /// via traits to interact with this data safely.
    #[serde(default, flatten)]
    pub extensions: HashMap<String, Value>,
}

impl User {
    /// Creates a new user with the given ID and email.
    ///
    /// The user is created with `email_verified` set to `false` and
    /// timestamps set to the current time.
    pub fn new(id: String, email: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            email,
            email_verified: false,
            name: None,
            image: None,
            created_at: now,
            updated_at: now,
            extensions: HashMap::new(),
        }
    }

    /// Gets an extension value by key, deserializing it to the requested type.
    ///
    /// Returns `None` if the key doesn't exist or deserialization fails.
    pub fn get_extension<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.extensions
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Sets an extension value by key.
    ///
    /// The value is serialized to JSON before storage.
    pub fn set_extension<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.extensions.insert(key.to_string(), json_value);
            self.updated_at = Utc::now();
        }
    }

    /// Removes an extension value by key.
    ///
    /// Returns the removed value if it existed.
    pub fn remove_extension(&mut self, key: &str) -> Option<Value> {
        let result = self.extensions.remove(key);
        if result.is_some() {
            self.updated_at = Utc::now();
        }
        result
    }
}

impl Default for User {
    fn default() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string(), String::new())
    }
}

/// Represents an active session for a user.
///
/// Sessions track authenticated user sessions and can store
/// additional metadata like device information, IP addresses, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for the session
    pub id: String,

    /// The ID of the user this session belongs to
    pub user_id: String,

    /// The session token (used for authentication)
    pub token: String,

    /// When the session expires
    pub expires_at: DateTime<Utc>,

    /// Timestamp when the session was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the session was last updated
    pub updated_at: DateTime<Utc>,

    /// Optional IP address of the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,

    /// Optional user agent string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Extension data from plugins
    #[serde(default, flatten)]
    pub extensions: HashMap<String, Value>,
}

impl Session {
    /// Creates a new session for the given user.
    ///
    /// The session is created with a random token and default expiration
    /// of 7 days from now.
    pub fn new(user_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            token: uuid::Uuid::new_v4().to_string(),
            expires_at: now + chrono::Duration::days(7),
            created_at: now,
            updated_at: now,
            ip_address: None,
            user_agent: None,
            extensions: HashMap::new(),
        }
    }

    /// Creates a new session with a custom expiration duration.
    pub fn with_expiration(user_id: String, duration: chrono::Duration) -> Self {
        let mut session = Self::new(user_id);
        session.expires_at = session.created_at + duration;
        session
    }

    /// Checks if the session has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Refreshes the session, extending its expiration.
    pub fn refresh(&mut self, duration: chrono::Duration) {
        self.expires_at = Utc::now() + duration;
        self.updated_at = Utc::now();
    }

    /// Gets an extension value by key.
    pub fn get_extension<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.extensions
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Sets an extension value by key.
    pub fn set_extension<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.extensions.insert(key.to_string(), json_value);
            self.updated_at = Utc::now();
        }
    }
}

/// Represents an account linked to a user (e.g., OAuth provider).
///
/// This is used for social login and other external authentication methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique identifier for the account
    pub id: String,

    /// The ID of the user this account belongs to
    pub user_id: String,

    /// The provider name (e.g., "google", "github")
    pub provider: String,

    /// The provider's account ID
    pub provider_account_id: String,

    /// Optional access token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,

    /// Optional refresh token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// Optional token expiration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,

    /// Timestamp when the account was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the account was last updated
    pub updated_at: DateTime<Utc>,
}

impl Account {
    /// Creates a new account for the given user and provider.
    pub fn new(user_id: String, provider: String, provider_account_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            provider,
            provider_account_id,
            access_token: None,
            refresh_token: None,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("test_id".to_string(), "test@example.com".to_string());
        assert_eq!(user.id, "test_id");
        assert_eq!(user.email, "test@example.com");
        assert!(!user.email_verified);
    }

    #[test]
    fn test_user_extensions() {
        let mut user = User::new("test_id".to_string(), "test@example.com".to_string());
        user.set_extension("custom_field", "custom_value");
        assert_eq!(
            user.get_extension::<String>("custom_field"),
            Some("custom_value".to_string())
        );
    }

    #[test]
    fn test_session_expiration() {
        let session = Session::with_expiration(
            "user_id".to_string(),
            chrono::Duration::seconds(-1), // Already expired
        );
        assert!(session.is_expired());
    }
}
