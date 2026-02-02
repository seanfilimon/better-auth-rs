//! Request handlers for the Magic Link plugin.

use async_trait::async_trait;
use better_auth_core::router::{Request, RequestHandler, Response};
use serde::{Deserialize, Serialize};

/// Request body for sending a magic link.
#[derive(Debug, Deserialize)]
pub struct SignInMagicLinkRequest {
    /// Email address to send the magic link to.
    pub email: String,
    /// Optional user name (for new user registration).
    pub name: Option<String>,
    /// URL to redirect after verification.
    #[serde(rename = "callbackURL")]
    pub callback_url: Option<String>,
    /// URL to redirect for new users.
    #[serde(rename = "newUserCallbackURL")]
    pub new_user_callback_url: Option<String>,
    /// URL to redirect on error.
    #[serde(rename = "errorCallbackURL")]
    pub error_callback_url: Option<String>,
}

/// Response for sending a magic link.
#[derive(Debug, Serialize)]
pub struct SignInMagicLinkResponse {
    pub success: bool,
}

/// Handler for POST /sign-in/magic-link
pub struct SignInMagicLinkHandler;

#[async_trait]
impl RequestHandler for SignInMagicLinkHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<SignInMagicLinkRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        // Validate email
        if body.email.is_empty() || !body.email.contains('@') {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_EMAIL",
                    "message": "Invalid email address"
                }
            }));
        }

        // In a real implementation, this would:
        // 1. Generate token
        // 2. Store in database
        // 3. Build URL
        // 4. Call sendMagicLink callback
        
        Response::ok().json(SignInMagicLinkResponse { success: true })
    }
}

/// Query parameters for verifying a magic link.
#[derive(Debug, Deserialize)]
pub struct VerifyMagicLinkQuery {
    /// The magic link token.
    pub token: String,
    /// URL to redirect after verification.
    #[serde(rename = "callbackURL")]
    pub callback_url: Option<String>,
}

/// Handler for GET /magic-link/verify
pub struct VerifyMagicLinkHandler;

#[async_trait]
impl RequestHandler for VerifyMagicLinkHandler {
    async fn handle(&self, req: Request) -> Response {
        let token = req.query_param("token");
        let callback_url = req.query_param("callbackURL");
        
        let Some(token) = token else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "MISSING_TOKEN",
                    "message": "Token is required"
                }
            }));
        };

        if token.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_TOKEN",
                    "message": "Invalid token"
                }
            }));
        }

        // In a real implementation, this would:
        // 1. Look up token in database
        // 2. Verify it's not expired or used
        // 3. Find or create user
        // 4. Create session
        // 5. Mark token as used
        // 6. Redirect to callback URL or return session
        
        // If callback URL is provided, redirect
        if let Some(url) = callback_url {
            return Response::new(302)
                .header("Location", url.as_str());
        }

        // Otherwise return session data
        Response::ok().json(serde_json::json!({
            "user": {
                "id": "user_placeholder",
                "email": "user@example.com",
                "email_verified": true
            },
            "session": {
                "id": "session_placeholder",
                "token": "session_token_placeholder",
                "expires_at": "2024-01-01T00:00:00Z"
            }
        }))
    }
}
