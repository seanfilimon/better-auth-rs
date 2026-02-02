//! Request handlers for the API Key plugin.

use async_trait::async_trait;
use better_auth_core::router::{Request, RequestHandler, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request body for creating an API key.
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: Option<String>,
    #[serde(rename = "expiresIn")]
    pub expires_in: Option<u64>,
    pub prefix: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Handler for POST /api-key/create
pub struct CreateApiKeyHandler;

#[async_trait]
impl RequestHandler for CreateApiKeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<CreateApiKeyRequest> = req.json();
        
        Response::ok().json(serde_json::json!({
            "id": "key_placeholder",
            "key": "sk_live_placeholder_key",
            "name": body.and_then(|b| b.name),
            "createdAt": "2024-01-01T00:00:00Z"
        }))
    }
}

/// Request body for verifying an API key.
#[derive(Debug, Deserialize)]
pub struct VerifyApiKeyRequest {
    pub key: String,
    pub permissions: Option<HashMap<String, Vec<String>>>,
}

/// Handler for POST /api-key/verify
pub struct VerifyApiKeyHandler;

#[async_trait]
impl RequestHandler for VerifyApiKeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<VerifyApiKeyRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.key.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_KEY", "message": "API key is required" }
            }));
        }

        Response::ok().json(serde_json::json!({
            "valid": true,
            "error": null,
            "key": {
                "id": "key_placeholder",
                "userId": "user_placeholder"
            }
        }))
    }
}

/// Handler for GET /api-key/get
pub struct GetApiKeyHandler;

#[async_trait]
impl RequestHandler for GetApiKeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let id = req.query_param("id");
        
        let Some(id) = id else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_ID", "message": "API key ID is required" }
            }));
        };

        Response::ok().json(serde_json::json!({
            "id": id,
            "name": "My API Key",
            "start": "sk_live_",
            "userId": "user_placeholder",
            "createdAt": "2024-01-01T00:00:00Z"
        }))
    }
}

/// Request body for updating an API key.
#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyRequest {
    #[serde(rename = "keyId")]
    pub key_id: String,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

/// Handler for POST /api-key/update
pub struct UpdateApiKeyHandler;

#[async_trait]
impl RequestHandler for UpdateApiKeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<UpdateApiKeyRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.key_id.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_KEY_ID", "message": "Key ID is required" }
            }));
        }

        Response::ok().json(serde_json::json!({
            "id": body.key_id,
            "name": body.name,
            "updatedAt": "2024-01-01T00:00:00Z"
        }))
    }
}

/// Request body for deleting an API key.
#[derive(Debug, Deserialize)]
pub struct DeleteApiKeyRequest {
    #[serde(rename = "keyId")]
    pub key_id: String,
}

/// Handler for POST /api-key/delete
pub struct DeleteApiKeyHandler;

#[async_trait]
impl RequestHandler for DeleteApiKeyHandler {
    async fn handle(&self, req: Request) -> Response {
        let body: Option<DeleteApiKeyRequest> = req.json();
        
        let Some(body) = body else {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "INVALID_REQUEST", "message": "Invalid request body" }
            }));
        };

        if body.key_id.is_empty() {
            return Response::bad_request().json(serde_json::json!({
                "error": { "code": "MISSING_KEY_ID", "message": "Key ID is required" }
            }));
        }

        Response::ok().json(serde_json::json!({ "success": true }))
    }
}

/// Handler for GET /api-key/list
pub struct ListApiKeysHandler;

#[async_trait]
impl RequestHandler for ListApiKeysHandler {
    async fn handle(&self, _req: Request) -> Response {
        Response::ok().json(serde_json::json!({
            "keys": [
                {
                    "id": "key_1",
                    "name": "Production Key",
                    "start": "sk_live_",
                    "createdAt": "2024-01-01T00:00:00Z"
                }
            ]
        }))
    }
}

/// Handler for POST /api-key/delete-all-expired
pub struct DeleteExpiredKeysHandler;

#[async_trait]
impl RequestHandler for DeleteExpiredKeysHandler {
    async fn handle(&self, _req: Request) -> Response {
        Response::ok().json(serde_json::json!({
            "deleted": 0
        }))
    }
}
