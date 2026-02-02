//! Request handlers for the Email OTP plugin.

use async_trait::async_trait;
use better_auth_core::router::{Request, RequestHandler, Response};
use serde::{Deserialize, Serialize};

/// Request body for sending verification OTP.
#[derive(Debug, Deserialize)]
pub struct SendVerificationOtpRequest {
    /// Email address to send OTP to.
    pub email: String,
    /// Type of OTP: "sign-in", "email-verification", or "forget-password".
    #[serde(rename = "type")]
    pub otp_type: String,
}

/// Response for sending verification OTP.
#[derive(Debug, Serialize)]
pub struct SendVerificationOtpResponse {
    pub success: bool,
}

/// Handler for POST /email-otp/send-verification-otp
pub struct SendVerificationOtpHandler;

#[async_trait]
impl RequestHandler for SendVerificationOtpHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<SendVerificationOtpRequest> = req.json();
        
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

        // Validate OTP type
        let valid_types = ["sign-in", "email-verification", "forget-password"];
        if !valid_types.contains(&body.otp_type.as_str()) {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_OTP_TYPE",
                    "message": "Invalid OTP type. Must be one of: sign-in, email-verification, forget-password"
                }
            }));
        }

        // In a real implementation, this would:
        // 1. Generate OTP
        // 2. Store in database
        // 3. Call sendVerificationOTP callback
        
        Response::ok().json(SendVerificationOtpResponse { success: true })
    }
}

/// Request body for checking verification OTP.
#[derive(Debug, Deserialize)]
pub struct CheckVerificationOtpRequest {
    pub email: String,
    #[serde(rename = "type")]
    pub otp_type: String,
    pub otp: String,
}

/// Response for checking verification OTP.
#[derive(Debug, Serialize)]
pub struct CheckVerificationOtpResponse {
    pub valid: bool,
}

/// Handler for POST /email-otp/check-verification-otp
pub struct CheckVerificationOtpHandler;

#[async_trait]
impl RequestHandler for CheckVerificationOtpHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<CheckVerificationOtpRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        // In a real implementation, this would check the OTP without consuming it
        
        Response::ok().json(CheckVerificationOtpResponse { valid: true })
    }
}

/// Request body for sign-in with email OTP.
#[derive(Debug, Deserialize)]
pub struct SignInEmailOtpRequest {
    pub email: String,
    pub otp: String,
}

/// Response for sign-in with email OTP.
#[derive(Debug, Serialize)]
pub struct SignInEmailOtpResponse {
    pub user: UserResponse,
    pub session: SessionResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub token: String,
    pub expires_at: String,
}

/// Handler for POST /sign-in/email-otp
pub struct SignInEmailOtpHandler;

#[async_trait]
impl RequestHandler for SignInEmailOtpHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<SignInEmailOtpRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        if body.email.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "MISSING_EMAIL",
                    "message": "Email is required"
                }
            }));
        }

        if body.otp.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "MISSING_OTP",
                    "message": "OTP is required"
                }
            }));
        }

        // In a real implementation, this would:
        // 1. Verify OTP
        // 2. Find or create user
        // 3. Create session
        // 4. Return user and session
        
        Response::ok().json(serde_json::json!({
            "user": {
                "id": "user_placeholder",
                "email": body.email,
                "email_verified": true,
                "name": null
            },
            "session": {
                "id": "session_placeholder",
                "token": "token_placeholder",
                "expires_at": "2024-01-01T00:00:00Z"
            }
        }))
    }
}

/// Request body for verifying email.
#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub email: String,
    pub otp: String,
}

/// Handler for POST /email-otp/verify-email
pub struct VerifyEmailHandler;

#[async_trait]
impl RequestHandler for VerifyEmailHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<VerifyEmailRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        // In a real implementation, this would verify the OTP and mark email as verified
        
        Response::ok().json(serde_json::json!({
            "success": true,
            "email_verified": true
        }))
    }
}

/// Request body for requesting password reset.
#[derive(Debug, Deserialize)]
pub struct RequestPasswordResetRequest {
    pub email: String,
}

/// Handler for POST /email-otp/request-password-reset
pub struct RequestPasswordResetHandler;

#[async_trait]
impl RequestHandler for RequestPasswordResetHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<RequestPasswordResetRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        // In a real implementation, this would send a password reset OTP
        
        Response::ok().json(serde_json::json!({
            "success": true
        }))
    }
}

/// Request body for resetting password.
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub email: String,
    pub otp: String,
    pub password: String,
}

/// Handler for POST /email-otp/reset-password
pub struct ResetPasswordHandler;

#[async_trait]
impl RequestHandler for ResetPasswordHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<ResetPasswordRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        if body.password.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "MISSING_PASSWORD",
                    "message": "New password is required"
                }
            }));
        }

        // In a real implementation, this would:
        // 1. Verify OTP
        // 2. Update user's password
        
        Response::ok().json(serde_json::json!({
            "success": true
        }))
    }
}
