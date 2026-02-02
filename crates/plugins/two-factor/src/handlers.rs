//! Request handlers for the Two-Factor plugin.

use async_trait::async_trait;
use better_auth_core::router::{Request, RequestHandler, Response};
use serde::{Deserialize, Serialize};

/// Request body for enabling 2FA.
#[derive(Debug, Deserialize)]
pub struct EnableRequest {
    pub password: String,
    pub issuer: Option<String>,
}

/// Response for enabling 2FA.
#[derive(Debug, Serialize)]
pub struct EnableResponse {
    #[serde(rename = "totpURI")]
    pub totp_uri: String,
    #[serde(rename = "backupCodes")]
    pub backup_codes: Vec<String>,
}

/// Handler for POST /two-factor/enable
pub struct EnableHandler;

#[async_trait]
impl RequestHandler for EnableHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<EnableRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.password.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_PASSWORD", "message": "Password is required" }
            }));
        }

        // In a real implementation, this would:
        // 1. Verify password
        // 2. Generate TOTP secret
        // 3. Generate backup codes
        // 4. Store in database
        
        Response::ok().json(EnableResponse {
            totp_uri: "otpauth://totp/BetterAuth:user@example.com?secret=PLACEHOLDER".to_string(),
            backup_codes: vec![
                "ABCD-EFGH-IJ".to_string(),
                "KLMN-OPQR-ST".to_string(),
            ],
        })
    }
}

/// Request body for disabling 2FA.
#[derive(Debug, Deserialize)]
pub struct DisableRequest {
    pub password: String,
}

/// Handler for POST /two-factor/disable
pub struct DisableHandler;

#[async_trait]
impl RequestHandler for DisableHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<DisableRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.password.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_PASSWORD", "message": "Password is required" }
            }));
        }

        Response::ok().json(serde_json::json!({ "success": true }))
    }
}

/// Request body for getting TOTP URI.
#[derive(Debug, Deserialize)]
pub struct GetTotpUriRequest {
    pub password: String,
}

/// Handler for POST /two-factor/get-totp-uri
pub struct GetTotpUriHandler;

#[async_trait]
impl RequestHandler for GetTotpUriHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<GetTotpUriRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        Response::ok().json(serde_json::json!({
            "totpURI": "otpauth://totp/BetterAuth:user@example.com?secret=PLACEHOLDER"
        }))
    }
}

/// Request body for verifying TOTP.
#[derive(Debug, Deserialize)]
pub struct VerifyTotpRequest {
    pub code: String,
    #[serde(rename = "trustDevice")]
    pub trust_device: Option<bool>,
}

/// Handler for POST /two-factor/verify-totp
pub struct VerifyTotpHandler;

#[async_trait]
impl RequestHandler for VerifyTotpHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<VerifyTotpRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.code.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_CODE", "message": "Code is required" }
            }));
        }

        Response::ok().json(serde_json::json!({
            "success": true,
            "session": {
                "id": "session_placeholder",
                "token": "token_placeholder"
            }
        }))
    }
}

/// Handler for POST /two-factor/send-otp
pub struct SendOtpHandler;

#[async_trait]
impl RequestHandler for SendOtpHandler {
    async fn handle(&self, _req: Request) -> Response {
        Response::ok().json(serde_json::json!({ "success": true }))
    }
}

/// Request body for verifying OTP.
#[derive(Debug, Deserialize)]
pub struct VerifyOtpRequest {
    pub code: String,
    #[serde(rename = "trustDevice")]
    pub trust_device: Option<bool>,
}

/// Handler for POST /two-factor/verify-otp
pub struct VerifyOtpHandler;

#[async_trait]
impl RequestHandler for VerifyOtpHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<VerifyOtpRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.code.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_CODE", "message": "Code is required" }
            }));
        }

        Response::ok().json(serde_json::json!({
            "success": true,
            "session": {
                "id": "session_placeholder",
                "token": "token_placeholder"
            }
        }))
    }
}

/// Request body for generating backup codes.
#[derive(Debug, Deserialize)]
pub struct GenerateBackupCodesRequest {
    pub password: String,
}

/// Handler for POST /two-factor/generate-backup-codes
pub struct GenerateBackupCodesHandler;

#[async_trait]
impl RequestHandler for GenerateBackupCodesHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<GenerateBackupCodesRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.password.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_PASSWORD", "message": "Password is required" }
            }));
        }

        Response::ok().json(serde_json::json!({
            "backupCodes": [
                "ABCD-EFGH-IJ",
                "KLMN-OPQR-ST",
                "UVWX-YZ12-34"
            ]
        }))
    }
}

/// Request body for verifying backup code.
#[derive(Debug, Deserialize)]
pub struct VerifyBackupCodeRequest {
    pub code: String,
    #[serde(rename = "disableSession")]
    pub disable_session: Option<bool>,
    #[serde(rename = "trustDevice")]
    pub trust_device: Option<bool>,
}

/// Handler for POST /two-factor/verify-backup-code
pub struct VerifyBackupCodeHandler;

#[async_trait]
impl RequestHandler for VerifyBackupCodeHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<VerifyBackupCodeRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.code.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_CODE", "message": "Backup code is required" }
            }));
        }

        Response::ok().json(serde_json::json!({
            "success": true,
            "session": {
                "id": "session_placeholder",
                "token": "token_placeholder"
            }
        }))
    }
}
