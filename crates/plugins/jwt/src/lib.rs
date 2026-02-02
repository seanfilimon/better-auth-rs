//! # Better Auth JWT Plugin
//!
//! This plugin provides JWT (JSON Web Token) authentication support for Better Auth.
//! It can be used alongside or instead of session-based authentication.
//!
//! ## Features
//!
//! - Access and refresh token generation
//! - Multiple signing algorithms (HS256, HS384, HS512, RS256, ES256)
//! - Token refresh and revocation
//! - Configurable token TTLs
//! - Session-linked JWTs for hybrid mode
//!
//! ## Example
//!
//! ```rust,ignore
//! use better_auth_plugin_jwt::{JwtPlugin, JwtConfig};
//! use chrono::Duration;
//!
//! let jwt = JwtPlugin::new(
//!     JwtConfig::new("your-secret-key")
//!         .access_token_ttl(Duration::hours(1))
//!         .refresh_token_ttl(Duration::days(30))
//!         .issuer("https://auth.example.com")
//! );
//! ```

pub mod claims;
pub mod token;

pub use claims::{AccessTokenClaims, IdTokenClaims, RefreshTokenClaims};
pub use token::{JwtCodec, JwtError, TokenGenerator, TokenPair};

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::AuthResult;
use better_auth_core::router::{Method, Request, RequestHandler, Response, Route, Router};
use better_auth_core::traits::AuthPlugin;
use better_auth_core::types::Session;
use better_auth_events_sdk::{EventDefinition, EventProvider};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// JWT plugin configuration.
#[derive(Clone)]
pub struct JwtConfig {
    /// Secret key for symmetric algorithms (HS256, etc.).
    pub secret: String,
    /// Access token time-to-live.
    pub access_token_ttl: Duration,
    /// Refresh token time-to-live.
    pub refresh_token_ttl: Duration,
    /// Token issuer (iss claim).
    pub issuer: Option<String>,
    /// Token audience (aud claim).
    pub audience: Option<String>,
    /// Whether to include user info in access tokens.
    pub include_user_info: bool,
    /// Whether to link JWTs to sessions (hybrid mode).
    pub link_to_session: bool,
}

impl JwtConfig {
    /// Creates a new JWT config with the given secret.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            access_token_ttl: Duration::hours(1),
            refresh_token_ttl: Duration::days(30),
            issuer: None,
            audience: None,
            include_user_info: false,
            link_to_session: false,
        }
    }

    /// Sets the access token TTL.
    pub fn access_token_ttl(mut self, ttl: Duration) -> Self {
        self.access_token_ttl = ttl;
        self
    }

    /// Sets the refresh token TTL.
    pub fn refresh_token_ttl(mut self, ttl: Duration) -> Self {
        self.refresh_token_ttl = ttl;
        self
    }

    /// Sets the token issuer.
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Sets the token audience.
    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Enables including user info in access tokens.
    pub fn include_user_info(mut self, include: bool) -> Self {
        self.include_user_info = include;
        self
    }

    /// Enables linking JWTs to sessions.
    pub fn link_to_session(mut self, link: bool) -> Self {
        self.link_to_session = link;
        self
    }
}

/// In-memory store for revoked tokens.
/// In production, this should be backed by Redis or a database.
#[derive(Debug, Default)]
pub struct TokenRevocationStore {
    /// Set of revoked token IDs (jti claims).
    revoked_tokens: RwLock<HashSet<String>>,
    /// Set of revoked token families (for refresh token rotation).
    revoked_families: RwLock<HashSet<String>>,
}

impl TokenRevocationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Revokes a token by its JTI.
    pub fn revoke_token(&self, jti: &str) {
        let mut tokens = self.revoked_tokens.write().unwrap();
        tokens.insert(jti.to_string());
    }

    /// Revokes all tokens in a family.
    pub fn revoke_family(&self, family_id: &str) {
        let mut families = self.revoked_families.write().unwrap();
        families.insert(family_id.to_string());
    }

    /// Checks if a token is revoked.
    pub fn is_token_revoked(&self, jti: &str) -> bool {
        let tokens = self.revoked_tokens.read().unwrap();
        tokens.contains(jti)
    }

    /// Checks if a token family is revoked.
    pub fn is_family_revoked(&self, family_id: &str) -> bool {
        let families = self.revoked_families.read().unwrap();
        families.contains(family_id)
    }
}

/// The JWT authentication plugin.
pub struct JwtPlugin {
    config: JwtConfig,
    token_generator: TokenGenerator,
    revocation_store: Arc<TokenRevocationStore>,
}

impl JwtPlugin {
    /// Creates a new JWT plugin with the given configuration.
    pub fn new(config: JwtConfig) -> Self {
        let codec = JwtCodec::hs256(&config.secret);
        let mut generator =
            TokenGenerator::new(codec, config.access_token_ttl, config.refresh_token_ttl);

        if let Some(ref issuer) = config.issuer {
            generator = generator.with_issuer(issuer);
        }
        if let Some(ref audience) = config.audience {
            generator = generator.with_audience(audience);
        }

        Self {
            config,
            token_generator: generator,
            revocation_store: Arc::new(TokenRevocationStore::new()),
        }
    }

    /// Returns a reference to the token generator.
    pub fn token_generator(&self) -> &TokenGenerator {
        &self.token_generator
    }

    /// Returns a reference to the revocation store.
    pub fn revocation_store(&self) -> &Arc<TokenRevocationStore> {
        &self.revocation_store
    }

    /// Generates a token pair for a user.
    pub fn generate_tokens(&self, user_id: &str) -> Result<TokenPair, JwtError> {
        self.token_generator.generate_token_pair(user_id)
    }

    /// Generates a token pair linked to a session.
    pub fn generate_tokens_with_session(
        &self,
        user_id: &str,
        session_id: &str,
    ) -> Result<TokenPair, JwtError> {
        self.token_generator
            .generate_token_pair_with_session(user_id, session_id)
    }

    /// Validates an access token.
    pub fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims, JwtError> {
        let claims = self.token_generator.validate_access_token(token)?;

        // Check if token is revoked
        if let Some(ref jti) = claims.jti {
            if self.revocation_store.is_token_revoked(jti) {
                return Err(JwtError::Revoked);
            }
        }

        Ok(claims)
    }

    /// Validates a refresh token.
    pub fn validate_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims, JwtError> {
        let claims = self.token_generator.validate_refresh_token(token)?;

        // Check if token or family is revoked
        if self.revocation_store.is_token_revoked(&claims.jti) {
            return Err(JwtError::Revoked);
        }
        if let Some(ref family_id) = claims.family_id {
            if self.revocation_store.is_family_revoked(family_id) {
                return Err(JwtError::Revoked);
            }
        }

        Ok(claims)
    }

    /// Refreshes tokens using a refresh token.
    pub fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPair, JwtError> {
        // Validate the refresh token first
        let claims = self.validate_refresh_token(refresh_token)?;

        // Revoke the old refresh token (rotation)
        self.revocation_store.revoke_token(&claims.jti);

        // Generate new tokens
        if let Some(session_id) = claims.session_id {
            self.token_generator
                .generate_token_pair_with_session(&claims.sub, &session_id)
        } else {
            self.token_generator.generate_token_pair(&claims.sub)
        }
    }

    /// Revokes a token by its JTI.
    pub fn revoke_token(&self, jti: &str) {
        self.revocation_store.revoke_token(jti);
    }

    /// Revokes all tokens for a user by revoking the token family.
    pub fn revoke_all_tokens(&self, family_id: &str) {
        self.revocation_store.revoke_family(family_id);
    }
}

impl EventProvider for JwtPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "jwt.token_generated",
                "Emitted when JWT tokens are generated",
                "jwt",
            ),
            EventDefinition::simple(
                "jwt.token_refreshed",
                "Emitted when JWT tokens are refreshed",
                "jwt",
            ),
            EventDefinition::simple(
                "jwt.token_revoked",
                "Emitted when a JWT token is revoked",
                "jwt",
            ),
            EventDefinition::simple(
                "jwt.validation_failed",
                "Emitted when JWT validation fails",
                "jwt",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "jwt"
    }
}

#[async_trait]
impl AuthPlugin for JwtPlugin {
    fn id(&self) -> &'static str {
        "jwt"
    }

    fn name(&self) -> &'static str {
        "JWT Authentication"
    }

    fn register_routes(&self, router: &mut Router) {
        // POST /jwt/refresh - Refresh tokens
        router.route(
            Route::new(
                Method::POST,
                "/jwt/refresh",
                RefreshHandler {
                    plugin: self.token_generator.clone(),
                    revocation_store: self.revocation_store.clone(),
                },
            )
            .summary("Refresh JWT tokens")
            .description("Exchanges a refresh token for new access and refresh tokens")
            .tag("jwt"),
        );

        // POST /jwt/revoke - Revoke a token
        router.route(
            Route::new(
                Method::POST,
                "/jwt/revoke",
                RevokeHandler {
                    revocation_store: self.revocation_store.clone(),
                },
            )
            .summary("Revoke JWT token")
            .description("Revokes a JWT token or token family")
            .tag("jwt"),
        );
    }

    async fn on_after_signin(
        &self,
        _ctx: &AuthContext,
        session: &mut Session,
    ) -> AuthResult<()> {
        // If configured to link JWTs to sessions, generate tokens
        if self.config.link_to_session {
            if let Ok(pair) = self.generate_tokens_with_session(&session.user_id, &session.id) {
                session.set_extension("jwt_access_token", &pair.access_token);
                session.set_extension("jwt_refresh_token", &pair.refresh_token);
            }
        }
        Ok(())
    }
}

// ============================================================================
// Route Handlers
// ============================================================================

/// Handler for POST /jwt/refresh
struct RefreshHandler {
    plugin: TokenGenerator,
    revocation_store: Arc<TokenRevocationStore>,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl Clone for RefreshHandler {
    fn clone(&self) -> Self {
        Self {
            plugin: self.plugin.clone(),
            revocation_store: self.revocation_store.clone(),
        }
    }
}

#[async_trait]
impl RequestHandler for RefreshHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: RefreshRequest = match req.json() {
            Some(b) => b,
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "invalid_request".to_string(),
                    message: "Missing or invalid request body".to_string(),
                });
            }
        };

        // Validate the refresh token
        let claims = match self.plugin.validate_refresh_token(&body.refresh_token) {
            Ok(c) => c,
            Err(e) => {
                return Response::unauthorized().json(ErrorResponse {
                    error: "invalid_token".to_string(),
                    message: e.to_string(),
                });
            }
        };

        // Check if revoked
        if self.revocation_store.is_token_revoked(&claims.jti) {
            return Response::unauthorized().json(ErrorResponse {
                error: "token_revoked".to_string(),
                message: "Refresh token has been revoked".to_string(),
            });
        }
        if let Some(ref family_id) = claims.family_id {
            if self.revocation_store.is_family_revoked(family_id) {
                return Response::unauthorized().json(ErrorResponse {
                    error: "token_revoked".to_string(),
                    message: "Token family has been revoked".to_string(),
                });
            }
        }

        // Revoke the old refresh token (rotation)
        self.revocation_store.revoke_token(&claims.jti);

        // Generate new tokens
        let pair = if let Some(session_id) = claims.session_id {
            self.plugin
                .generate_token_pair_with_session(&claims.sub, &session_id)
        } else {
            self.plugin.generate_token_pair(&claims.sub)
        };

        match pair {
            Ok(tokens) => Response::ok().json(tokens),
            Err(e) => Response::internal_error().json(ErrorResponse {
                error: "token_generation_failed".to_string(),
                message: e.to_string(),
            }),
        }
    }
}

/// Handler for POST /jwt/revoke
struct RevokeHandler {
    revocation_store: Arc<TokenRevocationStore>,
}

#[derive(Debug, Deserialize)]
struct RevokeRequest {
    /// Token to revoke (access or refresh token).
    #[serde(default)]
    token: Option<String>,
    /// Token ID (jti) to revoke directly.
    #[serde(default)]
    jti: Option<String>,
    /// Token family ID to revoke all tokens in the family.
    #[serde(default)]
    family_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct RevokeResponse {
    success: bool,
}

impl Clone for RevokeHandler {
    fn clone(&self) -> Self {
        Self {
            revocation_store: self.revocation_store.clone(),
        }
    }
}

#[async_trait]
impl RequestHandler for RevokeHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: RevokeRequest = match req.json() {
            Some(b) => b,
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "invalid_request".to_string(),
                    message: "Missing or invalid request body".to_string(),
                });
            }
        };

        // Revoke by JTI
        if let Some(jti) = body.jti {
            self.revocation_store.revoke_token(&jti);
            return Response::ok().json(RevokeResponse { success: true });
        }

        // Revoke by family ID
        if let Some(family_id) = body.family_id {
            self.revocation_store.revoke_family(&family_id);
            return Response::ok().json(RevokeResponse { success: true });
        }

        // Revoke by token (decode to get JTI)
        if let Some(token) = body.token {
            // Try to decode as access token
            if let Ok(token_data) =
                JwtCodec::decode_unsafe::<AccessTokenClaims>(&token)
            {
                if let Some(jti) = token_data.claims.jti {
                    self.revocation_store.revoke_token(&jti);
                    return Response::ok().json(RevokeResponse { success: true });
                }
            }

            // Try to decode as refresh token
            if let Ok(token_data) =
                JwtCodec::decode_unsafe::<RefreshTokenClaims>(&token)
            {
                self.revocation_store.revoke_token(&token_data.claims.jti);
                return Response::ok().json(RevokeResponse { success: true });
            }

            return Response::bad_request().json(ErrorResponse {
                error: "invalid_token".to_string(),
                message: "Could not decode token".to_string(),
            });
        }

        Response::bad_request().json(ErrorResponse {
            error: "invalid_request".to_string(),
            message: "Must provide token, jti, or family_id".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_config_builder() {
        let config = JwtConfig::new("secret")
            .access_token_ttl(Duration::hours(2))
            .refresh_token_ttl(Duration::days(7))
            .issuer("https://auth.example.com")
            .audience("my-app")
            .include_user_info(true)
            .link_to_session(true);

        assert_eq!(config.access_token_ttl, Duration::hours(2));
        assert_eq!(config.refresh_token_ttl, Duration::days(7));
        assert!(config.include_user_info);
        assert!(config.link_to_session);
    }

    #[test]
    fn test_jwt_plugin_token_generation() {
        let plugin = JwtPlugin::new(JwtConfig::new("super-secret-key"));

        let pair = plugin.generate_tokens("user_123").unwrap();
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());

        // Validate the access token
        let claims = plugin.validate_access_token(&pair.access_token).unwrap();
        assert_eq!(claims.sub, "user_123");
    }

    #[test]
    fn test_token_revocation() {
        let plugin = JwtPlugin::new(JwtConfig::new("super-secret-key"));

        let pair = plugin.generate_tokens("user_123").unwrap();
        let claims = plugin.validate_access_token(&pair.access_token).unwrap();

        // Revoke the token
        if let Some(jti) = claims.jti {
            plugin.revoke_token(&jti);

            // Validation should now fail
            let result = plugin.validate_access_token(&pair.access_token);
            assert!(matches!(result, Err(JwtError::Revoked)));
        }
    }

    #[test]
    fn test_token_refresh() {
        let plugin = JwtPlugin::new(JwtConfig::new("super-secret-key"));

        let pair = plugin.generate_tokens("user_123").unwrap();
        let new_pair = plugin.refresh_tokens(&pair.refresh_token).unwrap();

        // New tokens should be different
        assert_ne!(pair.access_token, new_pair.access_token);
        assert_ne!(pair.refresh_token, new_pair.refresh_token);

        // Old refresh token should be revoked
        let result = plugin.refresh_tokens(&pair.refresh_token);
        assert!(matches!(result, Err(JwtError::Revoked)));
    }
}
