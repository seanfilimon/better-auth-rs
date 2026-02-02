//! Authentication middleware layer for Axum.

use axum::body::Body;
use axum::http::{Request, Response};
use better_auth_core::traits::StorageAdapter;
use better_auth_core::types::{Session, User};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[cfg(feature = "jwt")]
use better_auth_plugin_jwt::{AccessTokenClaims, JwtCodec};

/// Configuration for the auth layer.
#[derive(Clone)]
pub struct AuthLayerConfig {
    /// Whether to validate JWTs (requires jwt feature).
    pub validate_jwt: bool,
    /// JWT secret for validation (required if validate_jwt is true).
    #[cfg(feature = "jwt")]
    pub jwt_secret: Option<String>,
    /// JWT issuer for validation.
    #[cfg(feature = "jwt")]
    pub jwt_issuer: Option<String>,
    /// JWT audience for validation.
    #[cfg(feature = "jwt")]
    pub jwt_audience: Option<String>,
}

impl Default for AuthLayerConfig {
    fn default() -> Self {
        Self {
            validate_jwt: false,
            #[cfg(feature = "jwt")]
            jwt_secret: None,
            #[cfg(feature = "jwt")]
            jwt_issuer: None,
            #[cfg(feature = "jwt")]
            jwt_audience: None,
        }
    }
}

impl AuthLayerConfig {
    /// Creates a new config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables JWT validation with the given secret.
    #[cfg(feature = "jwt")]
    pub fn with_jwt(mut self, secret: impl Into<String>) -> Self {
        self.validate_jwt = true;
        self.jwt_secret = Some(secret.into());
        self
    }

    /// Sets the expected JWT issuer.
    #[cfg(feature = "jwt")]
    pub fn jwt_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.jwt_issuer = Some(issuer.into());
        self
    }

    /// Sets the expected JWT audience.
    #[cfg(feature = "jwt")]
    pub fn jwt_audience(mut self, audience: impl Into<String>) -> Self {
        self.jwt_audience = Some(audience.into());
        self
    }
}

/// Layer that adds authentication to routes.
#[derive(Clone)]
pub struct AuthLayer {
    adapter: Arc<dyn StorageAdapter>,
    config: AuthLayerConfig,
    #[cfg(feature = "jwt")]
    jwt_codec: Option<JwtCodec>,
}

impl AuthLayer {
    /// Creates a new auth layer with the given storage adapter.
    pub fn new(adapter: Arc<dyn StorageAdapter>) -> Self {
        Self {
            adapter,
            config: AuthLayerConfig::default(),
            #[cfg(feature = "jwt")]
            jwt_codec: None,
        }
    }

    /// Creates a new auth layer with custom configuration.
    pub fn with_config(adapter: Arc<dyn StorageAdapter>, config: AuthLayerConfig) -> Self {
        #[cfg(feature = "jwt")]
        let jwt_codec = if config.validate_jwt {
            config.jwt_secret.as_ref().map(|secret| {
                let mut codec = JwtCodec::hs256(secret);
                if let Some(ref issuer) = config.jwt_issuer {
                    codec = codec.with_issuer(issuer);
                }
                if let Some(ref audience) = config.jwt_audience {
                    codec = codec.with_audience(audience);
                }
                codec
            })
        } else {
            None
        };

        Self {
            adapter,
            config,
            #[cfg(feature = "jwt")]
            jwt_codec,
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            adapter: self.adapter.clone(),
            #[cfg(feature = "jwt")]
            jwt_codec: self.jwt_codec.clone(),
            #[cfg(not(feature = "jwt"))]
            _config: self.config.clone(),
        }
    }
}

/// Middleware service that validates sessions and JWTs.
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    adapter: Arc<dyn StorageAdapter>,
    #[cfg(feature = "jwt")]
    jwt_codec: Option<JwtCodec>,
    #[cfg(not(feature = "jwt"))]
    _config: AuthLayerConfig,
}

/// Result of token extraction and validation.
#[derive(Debug)]
enum AuthResult {
    /// No token found.
    NoToken,
    /// Session token found.
    SessionToken(String),
    /// JWT found and validated.
    #[cfg(feature = "jwt")]
    JwtClaims(AccessTokenClaims),
    /// Invalid JWT.
    #[cfg(feature = "jwt")]
    InvalidJwt,
}

impl<S> Service<Request<Body>> for AuthMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let adapter = self.adapter.clone();
        let mut inner = self.inner.clone();

        #[cfg(feature = "jwt")]
        let jwt_codec = self.jwt_codec.clone();

        Box::pin(async move {
            // Extract and validate token
            #[cfg(feature = "jwt")]
            let auth_result = extract_and_validate_token(&req, jwt_codec.as_ref());

            #[cfg(not(feature = "jwt"))]
            let auth_result = extract_token_simple(&req);

            match auth_result {
                AuthResult::SessionToken(token) => {
                    // Validate session token against database
                    if let Ok(Some(session)) = adapter.get_session_by_token(&token).await {
                        if !session.is_expired() {
                            if let Ok(Some(user)) = adapter.get_user_by_id(&session.user_id).await {
                                req.extensions_mut().insert(session);
                                req.extensions_mut().insert(user);
                            }
                        }
                    }
                }
                #[cfg(feature = "jwt")]
                AuthResult::JwtClaims(claims) => {
                    // JWT is valid, get user from database
                    if let Ok(Some(user)) = adapter.get_user_by_id(&claims.sub).await {
                        req.extensions_mut().insert(user.clone());

                        // If JWT has a session_id, try to get the session
                        if let Some(session_id) = &claims.session_id {
                            if let Ok(Some(session)) =
                                adapter.get_session_by_id(session_id).await
                            {
                                if !session.is_expired() {
                                    req.extensions_mut().insert(session);
                                }
                            }
                        } else {
                            // Create a synthetic session from JWT claims
                            let session = create_session_from_jwt(&claims, &user);
                            req.extensions_mut().insert(session);
                        }

                        // Also insert the JWT claims for handlers that need them
                        req.extensions_mut().insert(claims);
                    }
                }
                #[cfg(feature = "jwt")]
                AuthResult::InvalidJwt => {
                    // Invalid JWT - don't authenticate
                    // Could optionally return 401 here
                }
                AuthResult::NoToken => {
                    // No token - continue without authentication
                }
            }

            inner.call(req).await
        })
    }
}

/// Extracts token from request (simple version without JWT).
#[cfg(not(feature = "jwt"))]
fn extract_token_simple(req: &Request<Body>) -> AuthResult {
    if let Some(token) = extract_bearer_token(req) {
        return AuthResult::SessionToken(token);
    }

    if let Some(token) = extract_cookie_token(req) {
        return AuthResult::SessionToken(token);
    }

    AuthResult::NoToken
}

/// Extracts and validates token from request (with JWT support).
#[cfg(feature = "jwt")]
fn extract_and_validate_token(req: &Request<Body>, jwt_codec: Option<&JwtCodec>) -> AuthResult {
    // Try Authorization header first
    if let Some(token) = extract_bearer_token(req) {
        // Check if it looks like a JWT (has 3 parts separated by dots)
        if token.matches('.').count() == 2 {
            // Try to validate as JWT
            if let Some(codec) = jwt_codec {
                match codec.decode::<AccessTokenClaims>(&token) {
                    Ok(token_data) => {
                        return AuthResult::JwtClaims(token_data.claims);
                    }
                    Err(_) => {
                        // Invalid JWT
                        return AuthResult::InvalidJwt;
                    }
                }
            }
        }

        // Not a JWT or JWT validation disabled, treat as session token
        return AuthResult::SessionToken(token);
    }

    // Try cookie
    if let Some(token) = extract_cookie_token(req) {
        return AuthResult::SessionToken(token);
    }

    AuthResult::NoToken
}

/// Extracts bearer token from Authorization header.
fn extract_bearer_token(req: &Request<Body>) -> Option<String> {
    req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|v| v.starts_with("Bearer "))
        .map(|v| v[7..].to_string())
}

/// Extracts session token from cookie.
fn extract_cookie_token(req: &Request<Body>) -> Option<String> {
    req.headers()
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(|c| c.trim())
                .find(|c| c.starts_with("better_auth_session="))
                .map(|c| c[20..].to_string())
        })
}

/// Creates a synthetic session from JWT claims.
#[cfg(feature = "jwt")]
fn create_session_from_jwt(claims: &AccessTokenClaims, user: &User) -> Session {
    use chrono::{DateTime, Utc};

    let expires_at = claims
        .expires_at()
        .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1));

    Session {
        id: claims.jti.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        user_id: user.id.clone(),
        token: String::new(), // JWT doesn't have a separate token
        expires_at,
        created_at: claims.issued_at().unwrap_or_else(Utc::now),
        updated_at: Utc::now(),
        ip_address: None,
        user_agent: None,
        extensions: std::collections::HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token() {
        let req = Request::builder()
            .header("authorization", "Bearer test_token_123")
            .body(Body::empty())
            .unwrap();

        let token = extract_bearer_token(&req);
        assert_eq!(token, Some("test_token_123".to_string()));
    }

    #[test]
    fn test_extract_cookie_token() {
        let req = Request::builder()
            .header("cookie", "other=value; better_auth_session=session_token_456; another=test")
            .body(Body::empty())
            .unwrap();

        let token = extract_cookie_token(&req);
        assert_eq!(token, Some("session_token_456".to_string()));
    }

    #[test]
    fn test_no_token() {
        let req = Request::builder().body(Body::empty()).unwrap();

        assert!(extract_bearer_token(&req).is_none());
        assert!(extract_cookie_token(&req).is_none());
    }
}
