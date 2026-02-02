//! JWT token encoding and decoding.

use crate::claims::{AccessTokenClaims, RefreshTokenClaims};
use chrono::Duration;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{de::DeserializeOwned, Serialize};

/// Error type for JWT operations.
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("Token encoding failed: {0}")]
    EncodingFailed(String),

    #[error("Token decoding failed: {0}")]
    DecodingFailed(String),

    #[error("Token expired")]
    Expired,

    #[error("Invalid token")]
    Invalid,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Missing required claim: {0}")]
    MissingClaim(String),

    #[error("Token revoked")]
    Revoked,
}

impl From<jsonwebtoken::errors::Error> for JwtError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => JwtError::Expired,
            ErrorKind::InvalidSignature => JwtError::InvalidSignature,
            ErrorKind::InvalidToken => JwtError::Invalid,
            _ => JwtError::DecodingFailed(err.to_string()),
        }
    }
}

/// JWT token encoder/decoder.
#[derive(Clone)]
pub struct JwtCodec {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    validation: Validation,
}

impl JwtCodec {
    /// Creates a new JWT codec with a symmetric secret (HMAC).
    pub fn new_symmetric(secret: &[u8], algorithm: Algorithm) -> Self {
        let mut validation = Validation::new(algorithm);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            algorithm,
            validation,
        }
    }

    /// Creates a new JWT codec with HS256 algorithm.
    pub fn hs256(secret: &str) -> Self {
        Self::new_symmetric(secret.as_bytes(), Algorithm::HS256)
    }

    /// Creates a new JWT codec with HS384 algorithm.
    pub fn hs384(secret: &str) -> Self {
        Self::new_symmetric(secret.as_bytes(), Algorithm::HS384)
    }

    /// Creates a new JWT codec with HS512 algorithm.
    pub fn hs512(secret: &str) -> Self {
        Self::new_symmetric(secret.as_bytes(), Algorithm::HS512)
    }

    /// Creates a new JWT codec with RSA keys (RS256).
    pub fn rs256(private_key_pem: &[u8], public_key_pem: &[u8]) -> Result<Self, JwtError> {
        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem)
            .map_err(|e| JwtError::EncodingFailed(e.to_string()))?;
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem)
            .map_err(|e| JwtError::DecodingFailed(e.to_string()))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Ok(Self {
            encoding_key,
            decoding_key,
            algorithm: Algorithm::RS256,
            validation,
        })
    }

    /// Creates a new JWT codec with EC keys (ES256).
    pub fn es256(private_key_pem: &[u8], public_key_pem: &[u8]) -> Result<Self, JwtError> {
        let encoding_key = EncodingKey::from_ec_pem(private_key_pem)
            .map_err(|e| JwtError::EncodingFailed(e.to_string()))?;
        let decoding_key = DecodingKey::from_ec_pem(public_key_pem)
            .map_err(|e| JwtError::DecodingFailed(e.to_string()))?;

        let mut validation = Validation::new(Algorithm::ES256);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Ok(Self {
            encoding_key,
            decoding_key,
            algorithm: Algorithm::ES256,
            validation,
        })
    }

    /// Sets the expected issuer for validation.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.validation.set_issuer(&[issuer.into()]);
        self
    }

    /// Sets the expected audience for validation.
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.validation.set_audience(&[audience.into()]);
        self
    }

    /// Disables expiration validation (use with caution).
    pub fn without_exp_validation(mut self) -> Self {
        self.validation.validate_exp = false;
        self
    }

    /// Encodes claims into a JWT token.
    pub fn encode<T: Serialize>(&self, claims: &T) -> Result<String, JwtError> {
        let header = Header::new(self.algorithm);
        encode(&header, claims, &self.encoding_key)
            .map_err(|e| JwtError::EncodingFailed(e.to_string()))
    }

    /// Decodes a JWT token into claims.
    pub fn decode<T: DeserializeOwned>(&self, token: &str) -> Result<TokenData<T>, JwtError> {
        decode::<T>(token, &self.decoding_key, &self.validation).map_err(JwtError::from)
    }

    /// Decodes a JWT token without validating the signature (for inspection only).
    pub fn decode_unsafe<T: DeserializeOwned>(token: &str) -> Result<TokenData<T>, JwtError> {
        let mut validation = Validation::default();
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_nbf = false;

        decode::<T>(token, &DecodingKey::from_secret(&[]), &validation).map_err(JwtError::from)
    }

    /// Returns the algorithm used by this codec.
    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }
}

/// Token pair containing access and refresh tokens.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenPair {
    /// The access token.
    pub access_token: String,
    /// The refresh token.
    pub refresh_token: String,
    /// Access token type (always "Bearer").
    pub token_type: String,
    /// Access token expiration in seconds.
    pub expires_in: u64,
    /// Refresh token expiration in seconds.
    pub refresh_expires_in: u64,
}

impl TokenPair {
    /// Creates a new token pair.
    pub fn new(
        access_token: String,
        refresh_token: String,
        access_ttl: Duration,
        refresh_ttl: Duration,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: access_ttl.num_seconds() as u64,
            refresh_expires_in: refresh_ttl.num_seconds() as u64,
        }
    }
}

/// Token generator for creating access and refresh tokens.
#[derive(Clone)]
pub struct TokenGenerator {
    codec: JwtCodec,
    access_ttl: Duration,
    refresh_ttl: Duration,
    issuer: Option<String>,
    audience: Option<String>,
}

impl TokenGenerator {
    /// Creates a new token generator.
    pub fn new(codec: JwtCodec, access_ttl: Duration, refresh_ttl: Duration) -> Self {
        Self {
            codec,
            access_ttl,
            refresh_ttl,
            issuer: None,
            audience: None,
        }
    }

    /// Sets the issuer for generated tokens.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Sets the audience for generated tokens.
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Generates an access token for a user.
    pub fn generate_access_token(&self, user_id: &str) -> Result<String, JwtError> {
        let mut claims = AccessTokenClaims::new(user_id, self.access_ttl);

        if let Some(ref issuer) = self.issuer {
            claims = claims.with_issuer(issuer);
        }
        if let Some(ref audience) = self.audience {
            claims = claims.with_audience(audience);
        }

        self.codec.encode(&claims)
    }

    /// Generates a refresh token for a user.
    pub fn generate_refresh_token(&self, user_id: &str) -> Result<String, JwtError> {
        let mut claims = RefreshTokenClaims::new(user_id, self.refresh_ttl);

        if let Some(ref issuer) = self.issuer {
            claims = claims.with_issuer(issuer);
        }

        self.codec.encode(&claims)
    }

    /// Generates a token pair (access + refresh) for a user.
    pub fn generate_token_pair(&self, user_id: &str) -> Result<TokenPair, JwtError> {
        let access_token = self.generate_access_token(user_id)?;
        let refresh_token = self.generate_refresh_token(user_id)?;

        Ok(TokenPair::new(
            access_token,
            refresh_token,
            self.access_ttl,
            self.refresh_ttl,
        ))
    }

    /// Generates a token pair with a session ID.
    pub fn generate_token_pair_with_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<TokenPair, JwtError> {
        let mut access_claims = AccessTokenClaims::new(user_id, self.access_ttl)
            .with_session_id(session_id);
        let mut refresh_claims = RefreshTokenClaims::new(user_id, self.refresh_ttl)
            .with_session_id(session_id);

        if let Some(ref issuer) = self.issuer {
            access_claims = access_claims.with_issuer(issuer);
            refresh_claims = refresh_claims.with_issuer(issuer);
        }
        if let Some(ref audience) = self.audience {
            access_claims = access_claims.with_audience(audience);
        }

        let access_token = self.codec.encode(&access_claims)?;
        let refresh_token = self.codec.encode(&refresh_claims)?;

        Ok(TokenPair::new(
            access_token,
            refresh_token,
            self.access_ttl,
            self.refresh_ttl,
        ))
    }

    /// Validates and decodes an access token.
    pub fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims, JwtError> {
        let token_data = self.codec.decode::<AccessTokenClaims>(token)?;
        Ok(token_data.claims)
    }

    /// Validates and decodes a refresh token.
    pub fn validate_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims, JwtError> {
        let token_data = self.codec.decode::<RefreshTokenClaims>(token)?;
        Ok(token_data.claims)
    }

    /// Refreshes a token pair using a valid refresh token.
    pub fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPair, JwtError> {
        let claims = self.validate_refresh_token(refresh_token)?;
        
        // Generate new token pair, preserving session ID if present
        if let Some(session_id) = claims.session_id {
            self.generate_token_pair_with_session(&claims.sub, &session_id)
        } else {
            self.generate_token_pair(&claims.sub)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_codec_hs256() {
        let codec = JwtCodec::hs256("super-secret-key");
        
        let claims = AccessTokenClaims::new("user_123", Duration::hours(1));
        let token = codec.encode(&claims).unwrap();
        
        let decoded = codec.decode::<AccessTokenClaims>(&token).unwrap();
        assert_eq!(decoded.claims.sub, "user_123");
    }

    #[test]
    fn test_token_generator() {
        let codec = JwtCodec::hs256("super-secret-key");
        let generator = TokenGenerator::new(codec, Duration::hours(1), Duration::days(30))
            .with_issuer("https://auth.example.com");

        let pair = generator.generate_token_pair("user_123").unwrap();
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert_eq!(pair.token_type, "Bearer");
    }

    #[test]
    fn test_token_refresh() {
        let codec = JwtCodec::hs256("super-secret-key");
        let generator = TokenGenerator::new(codec, Duration::hours(1), Duration::days(30));

        let pair = generator.generate_token_pair("user_123").unwrap();
        let new_pair = generator.refresh_tokens(&pair.refresh_token).unwrap();
        
        assert_ne!(pair.access_token, new_pair.access_token);
    }

    #[test]
    fn test_expired_token() {
        let codec = JwtCodec::hs256("super-secret-key");
        
        // Create a token that's already expired
        let claims = AccessTokenClaims::new("user_123", Duration::seconds(-10));
        let token = codec.encode(&claims).unwrap();
        
        let result = codec.decode::<AccessTokenClaims>(&token);
        assert!(matches!(result, Err(JwtError::Expired)));
    }
}
