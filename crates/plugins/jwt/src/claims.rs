//! JWT claims structures.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard JWT claims for access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject (user ID).
    pub sub: String,

    /// Issued at (Unix timestamp).
    pub iat: i64,

    /// Expiration time (Unix timestamp).
    pub exp: i64,

    /// Not before (Unix timestamp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,

    /// JWT ID (unique identifier for this token).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,

    /// Issuer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,

    /// Audience.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,

    /// Session ID (links JWT to a session for hybrid mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// User's email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// User's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Custom claims.
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl AccessTokenClaims {
    /// Creates new access token claims for a user.
    pub fn new(user_id: impl Into<String>, ttl: Duration) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.into(),
            iat: now.timestamp(),
            exp: (now + ttl).timestamp(),
            nbf: None,
            jti: Some(uuid::Uuid::new_v4().to_string()),
            iss: None,
            aud: None,
            session_id: None,
            email: None,
            name: None,
            custom: HashMap::new(),
        }
    }

    /// Sets the issuer.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.iss = Some(issuer.into());
        self
    }

    /// Sets the audience.
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.aud = Some(audience.into());
        self
    }

    /// Sets the session ID.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Sets the user's email.
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Sets the user's name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adds a custom claim.
    pub fn with_claim(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.custom.insert(key.into(), json_value);
        }
        self
    }

    /// Checks if the token has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Gets the expiration time as a DateTime.
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.exp, 0)
    }

    /// Gets the issued at time as a DateTime.
    pub fn issued_at(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.iat, 0)
    }
}

/// Claims for refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// Subject (user ID).
    pub sub: String,

    /// Issued at (Unix timestamp).
    pub iat: i64,

    /// Expiration time (Unix timestamp).
    pub exp: i64,

    /// JWT ID (unique identifier for this token).
    pub jti: String,

    /// Issuer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,

    /// Token family ID (for refresh token rotation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,

    /// Session ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl RefreshTokenClaims {
    /// Creates new refresh token claims for a user.
    pub fn new(user_id: impl Into<String>, ttl: Duration) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.into(),
            iat: now.timestamp(),
            exp: (now + ttl).timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            iss: None,
            family_id: Some(uuid::Uuid::new_v4().to_string()),
            session_id: None,
        }
    }

    /// Sets the issuer.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.iss = Some(issuer.into());
        self
    }

    /// Sets the session ID.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Sets the family ID (for token rotation).
    pub fn with_family_id(mut self, family_id: impl Into<String>) -> Self {
        self.family_id = Some(family_id.into());
        self
    }

    /// Checks if the token has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// ID token claims (OpenID Connect compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
    /// Subject (user ID).
    pub sub: String,

    /// Issued at (Unix timestamp).
    pub iat: i64,

    /// Expiration time (Unix timestamp).
    pub exp: i64,

    /// Issuer.
    pub iss: String,

    /// Audience.
    pub aud: String,

    /// Authentication time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_time: Option<i64>,

    /// Nonce (for OIDC flows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,

    /// User's email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Whether email is verified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,

    /// User's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// User's profile picture URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
}

impl IdTokenClaims {
    /// Creates new ID token claims.
    pub fn new(
        user_id: impl Into<String>,
        issuer: impl Into<String>,
        audience: impl Into<String>,
        ttl: Duration,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.into(),
            iat: now.timestamp(),
            exp: (now + ttl).timestamp(),
            iss: issuer.into(),
            aud: audience.into(),
            auth_time: Some(now.timestamp()),
            nonce: None,
            email: None,
            email_verified: None,
            name: None,
            picture: None,
        }
    }

    /// Sets the nonce.
    pub fn with_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    /// Sets the user's email.
    pub fn with_email(mut self, email: impl Into<String>, verified: bool) -> Self {
        self.email = Some(email.into());
        self.email_verified = Some(verified);
        self
    }

    /// Sets the user's name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the user's picture.
    pub fn with_picture(mut self, picture: impl Into<String>) -> Self {
        self.picture = Some(picture.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_token_claims() {
        let claims = AccessTokenClaims::new("user_123", Duration::hours(1))
            .with_issuer("https://auth.example.com")
            .with_email("user@example.com");

        assert_eq!(claims.sub, "user_123");
        assert!(claims.iss.is_some());
        assert!(claims.email.is_some());
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_refresh_token_claims() {
        let claims = RefreshTokenClaims::new("user_123", Duration::days(30))
            .with_session_id("session_456");

        assert_eq!(claims.sub, "user_123");
        assert!(claims.session_id.is_some());
        assert!(claims.family_id.is_some());
    }

    #[test]
    fn test_id_token_claims() {
        let claims = IdTokenClaims::new(
            "user_123",
            "https://auth.example.com",
            "my-app",
            Duration::hours(1),
        )
        .with_email("user@example.com", true)
        .with_name("John Doe");

        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.email_verified, Some(true));
    }
}
