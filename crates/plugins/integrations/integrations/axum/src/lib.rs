//! # Better Auth Axum Integration
//!
//! This crate provides Axum integration for Better Auth, including:
//! - Route mounting
//! - Authentication middleware
//! - Session extractors
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use axum::{Router, routing::get};
//! use better_auth_axum::{AuthLayer, AuthSession};
//!
//! async fn protected_handler(session: AuthSession) -> String {
//!     format!("Hello, {}!", session.user.email)
//! }
//!
//! let app = Router::new()
//!     .route("/protected", get(protected_handler))
//!     .layer(AuthLayer::new(auth));
//! ```

mod extractor;
mod layer;
mod routes;

pub use extractor::{AuthSession, OptionalAuthSession};
pub use layer::AuthLayer;
pub use routes::auth_routes;

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use better_auth_core::error::AuthError;
use better_auth_core::router::{Request as AuthRequest, Response as AuthResponse, Method as AuthMethod};
use std::collections::HashMap;

/// Converts an Axum request to a Better Auth request.
pub fn to_auth_request(
    method: axum::http::Method,
    uri: &axum::http::Uri,
    headers: &HeaderMap,
    body: Option<serde_json::Value>,
) -> AuthRequest {
    let auth_method = match method {
        axum::http::Method::GET => AuthMethod::GET,
        axum::http::Method::POST => AuthMethod::POST,
        axum::http::Method::PUT => AuthMethod::PUT,
        axum::http::Method::PATCH => AuthMethod::PATCH,
        axum::http::Method::DELETE => AuthMethod::DELETE,
        axum::http::Method::OPTIONS => AuthMethod::OPTIONS,
        axum::http::Method::HEAD => AuthMethod::HEAD,
        _ => AuthMethod::GET,
    };

    let mut auth_headers = HashMap::new();
    for (key, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            auth_headers.insert(key.to_string(), v.to_string());
        }
    }

    // Parse query parameters
    let query: HashMap<String, String> = uri
        .query()
        .map(|q| {
            q.split('&')
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    Some((parts.next()?.to_string(), parts.next()?.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    AuthRequest {
        method: auth_method,
        path: uri.path().to_string(),
        params: HashMap::new(),
        query,
        headers: auth_headers,
        body,
        ip: None,
    }
}

/// Converts a Better Auth response to an Axum response.
pub fn to_axum_response(auth_response: AuthResponse) -> Response {
    let status = StatusCode::from_u16(auth_response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut response = if let Some(body) = auth_response.body {
        axum::Json(body).into_response()
    } else {
        status.into_response()
    };

    *response.status_mut() = status;

    // Add headers
    for (key, value) in auth_response.headers {
        if let (Ok(name), Ok(val)) = (
            axum::http::header::HeaderName::try_from(key),
            axum::http::header::HeaderValue::try_from(value),
        ) {
            response.headers_mut().insert(name, val);
        }
    }

    response
}

/// Wrapper for AuthError that implements IntoResponse.
pub struct AuthErrorResponse(pub AuthError);

impl IntoResponse for AuthErrorResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::json!({
            "error": self.0.to_string(),
            "code": self.0.status_code()
        });

        (status, axum::Json(body)).into_response()
    }
}

impl From<AuthError> for AuthErrorResponse {
    fn from(err: AuthError) -> Self {
        AuthErrorResponse(err)
    }
}
