//! Request handlers for the Passkey plugin.

use async_trait::async_trait;
use better_auth_core::router::{Request, RequestHandler, Response};
use serde::{Deserialize, Serialize};

/// Request body for adding a passkey.
#[derive(Debug, Deserialize)]
pub struct AddPasskeyRequest {
    pub name: Option<String>,
    #[serde(rename = "authenticatorAttachment")]
    pub authenticator_attachment: Option<String>,
    /// The attestation response from the client.
    pub response: Option<serde_json::Value>,
}

/// Handler for POST /passkey/add-passkey
pub struct AddPasskeyHandler;

#[async_trait]
impl RequestHandler for AddPasskeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<AddPasskeyRequest> = req.json();
        
        // In a real implementation, this would:
        // 1. Verify the attestation response
        // 2. Extract the public key and credential ID
        // 3. Store the passkey in the database
        
        Response::ok().json(serde_json::json!({
            "id": "passkey_placeholder",
            "name": body.and_then(|b| b.name).unwrap_or_else(|| "My Passkey".to_string()),
            "credentialId": "credential_id_placeholder",
            "createdAt": "2024-01-01T00:00:00Z"
        }))
    }
}

/// Request body for signing in with passkey.
#[derive(Debug, Deserialize)]
pub struct SignInPasskeyRequest {
    #[serde(rename = "autoFill")]
    pub auto_fill: Option<bool>,
    /// The assertion response from the client.
    pub response: Option<serde_json::Value>,
}

/// Handler for POST /sign-in/passkey
pub struct SignInPasskeyHandler;

#[async_trait]
impl RequestHandler for SignInPasskeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let _body: Option<SignInPasskeyRequest> = req.json();
        
        // In a real implementation, this would:
        // 1. Verify the assertion response
        // 2. Find the user by credential ID
        // 3. Update the counter
        // 4. Create a session
        
        Response::ok().json(serde_json::json!({
            "user": {
                "id": "user_placeholder",
                "email": "user@example.com"
            },
            "session": {
                "id": "session_placeholder",
                "token": "token_placeholder"
            }
        }))
    }
}

/// Handler for GET /passkey/list-user-passkeys
pub struct ListPasskeysHandler;

#[async_trait]
impl RequestHandler for ListPasskeysHandler {
    async fn handle(&self, _req: Request) -> Response {
        // In a real implementation, this would fetch passkeys from the database
        
        Response::ok().json(serde_json::json!({
            "passkeys": [
                {
                    "id": "passkey_1",
                    "name": "MacBook Pro",
                    "createdAt": "2024-01-01T00:00:00Z",
                    "deviceType": "platform"
                }
            ]
        }))
    }
}

/// Request body for deleting a passkey.
#[derive(Debug, Deserialize)]
pub struct DeletePasskeyRequest {
    pub id: String,
}

/// Handler for POST /passkey/delete-passkey
pub struct DeletePasskeyHandler;

#[async_trait]
impl RequestHandler for DeletePasskeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<DeletePasskeyRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.id.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_ID", "message": "Passkey ID is required" }
            }));
        }

        Response::ok().json(serde_json::json!({ "success": true }))
    }
}

/// Request body for updating a passkey.
#[derive(Debug, Deserialize)]
pub struct UpdatePasskeyRequest {
    pub id: String,
    pub name: String,
}

/// Handler for POST /passkey/update-passkey
pub struct UpdatePasskeyHandler;

#[async_trait]
impl RequestHandler for UpdatePasskeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<UpdatePasskeyRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.id.is_empty() || body.name.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_FIELDS", "message": "ID and name are required" }
            }));
        }

        Response::ok().json(serde_json::json!({
            "id": body.id,
            "name": body.name
        }))
    }
}

/// Handler for POST /passkey/generate-registration-options
pub struct GenerateRegistrationOptionsHandler;

#[async_trait]
impl RequestHandler for GenerateRegistrationOptionsHandler {
    async fn handle(&self, _req: Request) -> Response {
        // In a real implementation, this would generate proper WebAuthn options
        
        Response::ok().json(serde_json::json!({
            "challenge": "random_challenge_base64",
            "rp": {
                "id": "localhost",
                "name": "Better Auth"
            },
            "user": {
                "id": "user_id_base64",
                "name": "user@example.com",
                "displayName": "User"
            },
            "pubKeyCredParams": [
                { "type": "public-key", "alg": -7 },
                { "type": "public-key", "alg": -257 }
            ],
            "timeout": 60000,
            "attestation": "none"
        }))
    }
}

/// Handler for POST /passkey/generate-authentication-options
pub struct GenerateAuthenticationOptionsHandler;

#[async_trait]
impl RequestHandler for GenerateAuthenticationOptionsHandler {
    async fn handle(&self, _req: Request) -> Response {
        // In a real implementation, this would generate proper WebAuthn options
        
        Response::ok().json(serde_json::json!({
            "challenge": "random_challenge_base64",
            "rpId": "localhost",
            "userVerification": "preferred",
            "timeout": 60000
        }))
    }
}
