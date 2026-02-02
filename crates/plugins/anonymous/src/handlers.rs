//! Request handlers for the Anonymous plugin.

use async_trait::async_trait;
use better_auth_core::router::{Request, RequestHandler, Response};
use serde::Serialize;

/// Response for anonymous sign-in.
#[derive(Debug, Serialize)]
pub struct SignInAnonymousResponse {
    pub user: AnonymousUserResponse,
    pub session: SessionResponse,
}

#[derive(Debug, Serialize)]
pub struct AnonymousUserResponse {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub is_anonymous: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub token: String,
    pub expires_at: String,
}

/// Handler for POST /sign-in/anonymous
pub struct SignInAnonymousHandler;

#[async_trait]
impl RequestHandler for SignInAnonymousHandler {
    async fn handle(&self, _req: Request) -> Response {
        // In a real implementation, this would:
        // 1. Generate a random email
        // 2. Create an anonymous user
        // 3. Create a session
        // 4. Return user and session
        
        let user_id = uuid::Uuid::new_v4().to_string();
        let session_id = uuid::Uuid::new_v4().to_string();
        let token = uuid::Uuid::new_v4().to_string();
        let email = format!("temp@{}.com", uuid::Uuid::new_v4());
        
        Response::ok().json(SignInAnonymousResponse {
            user: AnonymousUserResponse {
                id: user_id,
                email,
                name: None,
                is_anonymous: true,
            },
            session: SessionResponse {
                id: session_id,
                token,
                expires_at: "2024-01-01T00:00:00Z".to_string(),
            },
        })
    }
}

/// Handler for POST /delete-anonymous-user
pub struct DeleteAnonymousUserHandler;

#[async_trait]
impl RequestHandler for DeleteAnonymousUserHandler {
    async fn handle(&self, _req: Request) -> Response {
        // In a real implementation, this would:
        // 1. Get the current session
        // 2. Verify the user is anonymous
        // 3. Delete the user and session
        
        Response::ok().json(serde_json::json!({
            "success": true
        }))
    }
}
