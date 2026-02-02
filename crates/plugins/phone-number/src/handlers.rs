//! Request handlers for the Phone Number plugin.

use async_trait::async_trait;
use better_auth_core::router::{Request, RequestHandler, Response};
use serde::{Deserialize, Serialize};

/// Request body for sending OTP.
#[derive(Debug, Deserialize)]
pub struct SendOtpRequest {
    /// Phone number to send OTP to.
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,
}

/// Response for sending OTP.
#[derive(Debug, Serialize)]
pub struct SendOtpResponse {
    pub success: bool,
}

/// Handler for POST /phone-number/send-otp
pub struct SendOtpHandler;

#[async_trait]
impl RequestHandler for SendOtpHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<SendOtpRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        // Validate phone number
        if body.phone_number.is_empty() || !body.phone_number.starts_with('+') {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_PHONE_NUMBER",
                    "message": "Invalid phone number format"
                }
            }));
        }

        // In a real implementation, this would:
        // 1. Generate OTP
        // 2. Store in database
        // 3. Call sendOTP callback
        
        Response::ok().json(SendOtpResponse { success: true })
    }
}

/// Request body for verifying phone number.
#[derive(Debug, Deserialize)]
pub struct VerifyPhoneRequest {
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,
    pub code: String,
    #[serde(rename = "disableSession")]
    pub disable_session: Option<bool>,
    #[serde(rename = "updatePhoneNumber")]
    pub update_phone_number: Option<bool>,
}

/// Handler for POST /phone-number/verify
pub struct VerifyPhoneHandler;

#[async_trait]
impl RequestHandler for VerifyPhoneHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<VerifyPhoneRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        if body.code.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "MISSING_CODE",
                    "message": "OTP code is required"
                }
            }));
        }

        // In a real implementation, this would:
        // 1. Verify OTP
        // 2. Mark phone as verified
        // 3. Optionally create session
        
        Response::ok().json(serde_json::json!({
            "success": true,
            "phone_number_verified": true
        }))
    }
}

/// Request body for sign-in with phone number.
#[derive(Debug, Deserialize)]
pub struct SignInPhoneNumberRequest {
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,
    pub password: String,
    #[serde(rename = "rememberMe")]
    pub remember_me: Option<bool>,
}

/// Handler for POST /sign-in/phone-number
pub struct SignInPhoneNumberHandler;

#[async_trait]
impl RequestHandler for SignInPhoneNumberHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<SignInPhoneNumberRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "INVALID_REQUEST",
                    "message": "Invalid request body"
                }
            }));
        };

        if body.phone_number.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "MISSING_PHONE_NUMBER",
                    "message": "Phone number is required"
                }
            }));
        }

        if body.password.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": {
                    "code": "MISSING_PASSWORD",
                    "message": "Password is required"
                }
            }));
        }

        // In a real implementation, this would:
        // 1. Find user by phone number
        // 2. Verify password
        // 3. Create session
        
        Response::ok().json(serde_json::json!({
            "user": {
                "id": "user_placeholder",
                "email": "user@example.com",
                "phone_number": body.phone_number,
                "phone_number_verified": true
            },
            "session": {
                "id": "session_placeholder",
                "token": "token_placeholder",
                "expires_at": "2024-01-01T00:00:00Z"
            }
        }))
    }
}

/// Request body for requesting password reset.
#[derive(Debug, Deserialize)]
pub struct RequestPasswordResetRequest {
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,
}

/// Handler for POST /phone-number/request-password-reset
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
    pub otp: String,
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
}

/// Handler for POST /phone-number/reset-password
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

        if body.new_password.is_empty() {
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
