//! Route mounting for Better Auth routes.

use axum::routing::{get, post};
use axum::Router;
use better_auth_core::traits::StorageAdapter;
use std::sync::Arc;

/// Creates an Axum router with all Better Auth routes.
///
/// # Example
///
/// ```rust,ignore
/// let auth_router = auth_routes(adapter);
/// let app = Router::new()
///     .nest("/api/auth", auth_router);
/// ```
pub fn auth_routes<S>(adapter: Arc<dyn StorageAdapter>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        // Core auth routes
        .route("/signup", post(signup_handler))
        .route("/signin", post(signin_handler))
        .route("/signout", post(signout_handler))
        .route("/session", get(session_handler))
        .route("/user", get(user_handler))
        .with_state(AuthState { adapter })
}

/// Shared state for auth routes.
#[derive(Clone)]
struct AuthState {
    adapter: Arc<dyn StorageAdapter>,
}

// Handler implementations (placeholders)

async fn signup_handler() -> &'static str {
    // TODO: Implement signup
    "signup"
}

async fn signin_handler() -> &'static str {
    // TODO: Implement signin
    "signin"
}

async fn signout_handler() -> &'static str {
    // TODO: Implement signout
    "signout"
}

async fn session_handler() -> &'static str {
    // TODO: Implement session check
    "session"
}

async fn user_handler() -> &'static str {
    // TODO: Implement user info
    "user"
}
