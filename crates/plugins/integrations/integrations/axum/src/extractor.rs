//! Session extractors for Axum handlers.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use better_auth_core::types::{Session, User};

/// Extractor for authenticated sessions.
///
/// This extractor will reject the request with 401 Unauthorized if
/// no valid session is found.
///
/// # Example
///
/// ```rust,ignore
/// async fn handler(session: AuthSession) -> String {
///     format!("Hello, {}!", session.user.email)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AuthSession {
    /// The authenticated user.
    pub user: User,
    /// The current session.
    pub session: Session,
}

/// Error returned when authentication fails.
#[derive(Debug)]
pub struct AuthSessionRejection {
    message: String,
}

impl IntoResponse for AuthSessionRejection {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": self.message,
            "code": 401
        });
        (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
    }
}

impl<S> FromRequestParts<S> for AuthSession
where
    S: Send + Sync,
{
    type Rejection = AuthSessionRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get session from extensions (set by middleware)
        let session = parts
            .extensions
            .get::<Session>()
            .cloned()
            .ok_or_else(|| AuthSessionRejection {
                message: "No session found".to_string(),
            })?;

        let user = parts
            .extensions
            .get::<User>()
            .cloned()
            .ok_or_else(|| AuthSessionRejection {
                message: "No user found".to_string(),
            })?;

        Ok(AuthSession { user, session })
    }
}

/// Extractor for optional authenticated sessions.
///
/// This extractor will return `None` if no valid session is found,
/// instead of rejecting the request.
///
/// # Example
///
/// ```rust,ignore
/// async fn handler(session: OptionalAuthSession) -> String {
///     match session.0 {
///         Some(s) => format!("Hello, {}!", s.user.email),
///         None => "Hello, guest!".to_string(),
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OptionalAuthSession(pub Option<AuthSession>);

impl<S> FromRequestParts<S> for OptionalAuthSession
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let session = parts.extensions.get::<Session>().cloned();
        let user = parts.extensions.get::<User>().cloned();

        match (session, user) {
            (Some(session), Some(user)) => Ok(OptionalAuthSession(Some(AuthSession { user, session }))),
            _ => Ok(OptionalAuthSession(None)),
        }
    }
}
