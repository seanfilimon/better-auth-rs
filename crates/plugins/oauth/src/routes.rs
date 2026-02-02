//! OAuth route handlers.

use crate::{OAuthConfig, OAuthState};
use async_trait::async_trait;
use better_auth_core::router::{CookieOptions, Method, Request, RequestHandler, Response, Route};
use better_auth_core::types::{Session, User};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// OAuth State Store (In-Memory)
// ============================================================================

/// In-memory store for OAuth state during the authorization flow.
/// In production, this should be backed by Redis or a database.
#[derive(Debug, Default)]
pub struct OAuthStateStore {
    states: RwLock<HashMap<String, OAuthState>>,
}

impl OAuthStateStore {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Stores an OAuth state.
    pub fn store(&self, state: &OAuthState) {
        let mut states = self.states.write().unwrap();
        states.insert(state.state.clone(), state.clone());
    }

    /// Retrieves and removes an OAuth state.
    pub fn take(&self, state_key: &str) -> Option<OAuthState> {
        let mut states = self.states.write().unwrap();
        states.remove(state_key)
    }

    /// Cleans up expired states.
    pub fn cleanup_expired(&self) {
        let mut states = self.states.write().unwrap();
        states.retain(|_, state| !state.is_expired());
    }
}

// ============================================================================
// Token Response Strategy
// ============================================================================

/// Strategy for how to return authentication tokens after OAuth callback.
#[derive(Debug, Clone, Default)]
pub enum TokenResponseStrategy {
    /// Set a session cookie and redirect (default, traditional web apps).
    #[default]
    SessionCookie,
    /// Return JWT tokens in JSON response (SPAs, mobile apps).
    JwtResponse,
    /// Both: Set cookie AND return JSON (hybrid apps).
    Both,
}

// ============================================================================
// Route Handlers
// ============================================================================

/// Handler for GET /oauth/signin/:provider
/// Redirects the user to the OAuth provider's authorization page.
pub struct SignInHandler {
    pub config: Arc<OAuthConfig>,
    pub state_store: Arc<OAuthStateStore>,
}

#[async_trait]
impl RequestHandler for SignInHandler {
    async fn handle(&self, req: Request) -> Response {
        // Get provider name from path params
        let provider_name = match req.param("provider") {
            Some(name) => name.clone(),
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "missing_provider".to_string(),
                    message: "Provider name is required".to_string(),
                });
            }
        };

        // Get the provider
        let provider = match self.config.providers.get(&provider_name) {
            Some(p) => p,
            None => {
                return Response::not_found().json(ErrorResponse {
                    error: "provider_not_found".to_string(),
                    message: format!("Provider '{}' is not configured", provider_name),
                });
            }
        };

        // Parse query parameters
        let redirect_url = req.query_param("redirect_url").cloned();
        let scopes: Vec<String> = req
            .query_param("scopes")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        // Create OAuth state for CSRF protection
        let mut oauth_state = OAuthState::new(&provider_name);
        if let Some(url) = redirect_url {
            oauth_state = oauth_state.with_redirect(url);
        }

        // Store the state
        self.state_store.store(&oauth_state);

        // Build the callback URL
        let callback_url = format!(
            "{}/oauth/callback/{}",
            self.config.callback_base, provider_name
        );

        // Generate the authorization URL
        let auth_url = provider.auth_url(&oauth_state.state, &scopes, &callback_url);

        // Redirect to the provider
        Response::new(302)
            .header("Location", auth_url)
            .header("Cache-Control", "no-store")
    }
}

/// Handler for GET /oauth/callback/:provider
/// Handles the OAuth callback from the provider.
pub struct CallbackHandler {
    pub config: Arc<OAuthConfig>,
    pub state_store: Arc<OAuthStateStore>,
    pub token_strategy: TokenResponseStrategy,
}

#[derive(Debug, Serialize)]
struct CallbackSuccessResponse {
    user: UserResponse,
    session: SessionResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<u64>,
}

#[derive(Debug, Serialize)]
struct UserResponse {
    id: String,
    email: String,
    name: Option<String>,
    image: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    id: String,
    token: String,
    expires_at: String,
}

#[async_trait]
impl RequestHandler for CallbackHandler {
    async fn handle(&self, req: Request) -> Response {
        // Get provider name from path params
        let provider_name = match req.param("provider") {
            Some(name) => name.clone(),
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "missing_provider".to_string(),
                    message: "Provider name is required".to_string(),
                });
            }
        };

        // Check for OAuth error from provider
        if let Some(error) = req.query_param("error") {
            let description = req
                .query_param("error_description")
                .cloned()
                .unwrap_or_else(|| "Unknown error".to_string());
            return Response::bad_request().json(ErrorResponse {
                error: error.clone(),
                message: description,
            });
        }

        // Get the authorization code
        let code = match req.query_param("code") {
            Some(c) => c.clone(),
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "missing_code".to_string(),
                    message: "Authorization code is required".to_string(),
                });
            }
        };

        // Validate state for CSRF protection
        let state_key = match req.query_param("state") {
            Some(s) => s.clone(),
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "missing_state".to_string(),
                    message: "State parameter is required".to_string(),
                });
            }
        };

        let oauth_state = match self.state_store.take(&state_key) {
            Some(s) => s,
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "invalid_state".to_string(),
                    message: "Invalid or expired state".to_string(),
                });
            }
        };

        // Verify state hasn't expired
        if oauth_state.is_expired() {
            return Response::bad_request().json(ErrorResponse {
                error: "expired_state".to_string(),
                message: "OAuth state has expired".to_string(),
            });
        }

        // Verify provider matches
        if oauth_state.provider != provider_name {
            return Response::bad_request().json(ErrorResponse {
                error: "provider_mismatch".to_string(),
                message: "Provider mismatch in OAuth state".to_string(),
            });
        }

        // Get the provider
        let provider = match self.config.providers.get(&provider_name) {
            Some(p) => p,
            None => {
                return Response::internal_error().json(ErrorResponse {
                    error: "provider_not_found".to_string(),
                    message: format!("Provider '{}' is not configured", provider_name),
                });
            }
        };

        // Build the callback URL (same as used in signin)
        let callback_url = format!(
            "{}/oauth/callback/{}",
            self.config.callback_base, provider_name
        );

        // Exchange the code for tokens
        let token_set = match provider.token_exchange(&code, &callback_url).await {
            Ok(tokens) => tokens,
            Err(e) => {
                return Response::internal_error().json(ErrorResponse {
                    error: "token_exchange_failed".to_string(),
                    message: e.to_string(),
                });
            }
        };

        // Get user info from the provider
        let user_info = match provider.get_user_info(&token_set.access_token).await {
            Ok(info) => info,
            Err(e) => {
                return Response::internal_error().json(ErrorResponse {
                    error: "user_info_failed".to_string(),
                    message: e.to_string(),
                });
            }
        };

        // At this point, we would:
        // 1. Check if an account exists for this provider + provider_account_id
        // 2. If yes, get the associated user and create a session
        // 3. If no, create a new user (if auto_create_user is true) and account
        //
        // For now, we'll create a mock user and session since we don't have
        // access to the storage adapter in this handler. In a real implementation,
        // the handler would receive the storage adapter via dependency injection.

        let user_id = uuid::Uuid::new_v4().to_string();
        let user = User::new(
            user_id.clone(),
            user_info.email.clone().unwrap_or_default(),
        );

        let session = Session::new(user_id);

        // Build response based on token strategy
        let user_response = UserResponse {
            id: user.id.clone(),
            email: user.email.clone(),
            name: user_info.name,
            image: user_info.picture,
        };

        let session_response = SessionResponse {
            id: session.id.clone(),
            token: session.token.clone(),
            expires_at: session.expires_at.to_rfc3339(),
        };

        match self.token_strategy {
            TokenResponseStrategy::SessionCookie => {
                // Set cookie and redirect
                let redirect_url = oauth_state
                    .redirect_url
                    .unwrap_or_else(|| "/".to_string());

                Response::new(302)
                    .header("Location", redirect_url)
                    .cookie(
                        "better_auth_session",
                        &session.token,
                        CookieOptions::secure(),
                    )
            }
            TokenResponseStrategy::JwtResponse => {
                // Return JSON response with tokens
                // In a real implementation, we'd generate JWTs here
                Response::ok().json(CallbackSuccessResponse {
                    user: user_response,
                    session: session_response,
                    access_token: Some(session.token.clone()),
                    refresh_token: None,
                    expires_in: Some(604800), // 7 days in seconds
                })
            }
            TokenResponseStrategy::Both => {
                // Set cookie AND return JSON
                let redirect_url = oauth_state.redirect_url.clone();

                let mut response = Response::ok()
                    .json(CallbackSuccessResponse {
                        user: user_response,
                        session: session_response,
                        access_token: Some(session.token.clone()),
                        refresh_token: None,
                        expires_in: Some(604800),
                    })
                    .cookie(
                        "better_auth_session",
                        &session.token,
                        CookieOptions::secure(),
                    );

                // If there's a redirect URL, include it in the response
                if let Some(url) = redirect_url {
                    response = response.header("X-Redirect-URL", url);
                }

                response
            }
        }
    }
}

/// Handler for POST /oauth/link/:provider
/// Links an OAuth account to an existing authenticated user.
pub struct LinkAccountHandler {
    pub config: Arc<OAuthConfig>,
    pub state_store: Arc<OAuthStateStore>,
}

#[async_trait]
impl RequestHandler for LinkAccountHandler {
    async fn handle(&self, req: Request) -> Response {
        // This endpoint requires authentication
        // In a real implementation, we'd check for a valid session

        let provider_name = match req.param("provider") {
            Some(name) => name.clone(),
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "missing_provider".to_string(),
                    message: "Provider name is required".to_string(),
                });
            }
        };

        // Check if provider exists
        if !self.config.providers.contains_key(&provider_name) {
            return Response::not_found().json(ErrorResponse {
                error: "provider_not_found".to_string(),
                message: format!("Provider '{}' is not configured", provider_name),
            });
        }

        // Check if linking is allowed
        if !self.config.allow_linking {
            return Response::forbidden().json(ErrorResponse {
                error: "linking_disabled".to_string(),
                message: "Account linking is disabled".to_string(),
            });
        }

        // Create OAuth state for the linking flow
        let oauth_state = OAuthState::new(&provider_name);
        self.state_store.store(&oauth_state);

        // Return the authorization URL for the client to redirect to
        let provider = self.config.providers.get(&provider_name).unwrap();
        let callback_url = format!(
            "{}/oauth/callback/{}",
            self.config.callback_base, provider_name
        );
        let auth_url = provider.auth_url(&oauth_state.state, &[], &callback_url);

        Response::ok().json(LinkResponse {
            auth_url,
            state: oauth_state.state,
        })
    }
}

#[derive(Debug, Serialize)]
struct LinkResponse {
    auth_url: String,
    state: String,
}

/// Handler for DELETE /oauth/unlink/:provider
/// Unlinks an OAuth account from the authenticated user.
pub struct UnlinkAccountHandler {
    pub config: Arc<OAuthConfig>,
}

#[async_trait]
impl RequestHandler for UnlinkAccountHandler {
    async fn handle(&self, req: Request) -> Response {
        // This endpoint requires authentication
        // In a real implementation, we'd check for a valid session and get the user

        let provider_name = match req.param("provider") {
            Some(name) => name.clone(),
            None => {
                return Response::bad_request().json(ErrorResponse {
                    error: "missing_provider".to_string(),
                    message: "Provider name is required".to_string(),
                });
            }
        };

        // Check if provider exists
        if !self.config.providers.contains_key(&provider_name) {
            return Response::not_found().json(ErrorResponse {
                error: "provider_not_found".to_string(),
                message: format!("Provider '{}' is not configured", provider_name),
            });
        }

        // Check if linking/unlinking is allowed
        if !self.config.allow_linking {
            return Response::forbidden().json(ErrorResponse {
                error: "linking_disabled".to_string(),
                message: "Account linking/unlinking is disabled".to_string(),
            });
        }

        // In a real implementation, we would:
        // 1. Get the authenticated user
        // 2. Find the account for this provider
        // 3. Ensure the user has at least one other auth method
        // 4. Delete the account

        Response::ok().json(UnlinkResponse {
            success: true,
            provider: provider_name,
        })
    }
}

#[derive(Debug, Serialize)]
struct UnlinkResponse {
    success: bool,
    provider: String,
}

/// Handler for GET /oauth/providers
/// Lists all configured OAuth providers.
pub struct ListProvidersHandler {
    pub config: Arc<OAuthConfig>,
}

#[async_trait]
impl RequestHandler for ListProvidersHandler {
    async fn handle(&self, _req: Request) -> Response {
        let providers: Vec<ProviderInfo> = self
            .config
            .providers
            .iter()
            .map(|(name, provider)| ProviderInfo {
                name: name.clone(),
                display_name: provider.display_name().to_string(),
            })
            .collect();

        Response::ok().json(ProvidersResponse { providers })
    }
}

#[derive(Debug, Serialize)]
struct ProviderInfo {
    name: String,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct ProvidersResponse {
    providers: Vec<ProviderInfo>,
}

// ============================================================================
// Error Response
// ============================================================================

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

// ============================================================================
// Route Registration
// ============================================================================

/// Creates all OAuth routes.
pub fn create_routes(
    config: Arc<OAuthConfig>,
    state_store: Arc<OAuthStateStore>,
    token_strategy: TokenResponseStrategy,
) -> Vec<Route> {
    vec![
        Route::new(
            Method::GET,
            "/oauth/signin/:provider",
            SignInHandler {
                config: config.clone(),
                state_store: state_store.clone(),
            },
        )
        .summary("Start OAuth sign-in")
        .description("Redirects to the OAuth provider's authorization page")
        .tag("oauth"),
        Route::new(
            Method::GET,
            "/oauth/callback/:provider",
            CallbackHandler {
                config: config.clone(),
                state_store: state_store.clone(),
                token_strategy,
            },
        )
        .summary("OAuth callback")
        .description("Handles the OAuth callback from the provider")
        .tag("oauth"),
        Route::new(
            Method::POST,
            "/oauth/link/:provider",
            LinkAccountHandler {
                config: config.clone(),
                state_store: state_store.clone(),
            },
        )
        .summary("Link OAuth account")
        .description("Links an OAuth account to the authenticated user")
        .tag("oauth")
        .requires_auth(),
        Route::new(
            Method::DELETE,
            "/oauth/unlink/:provider",
            UnlinkAccountHandler {
                config: config.clone(),
            },
        )
        .summary("Unlink OAuth account")
        .description("Unlinks an OAuth account from the authenticated user")
        .tag("oauth")
        .requires_auth(),
        Route::new(
            Method::GET,
            "/oauth/providers",
            ListProvidersHandler {
                config: config.clone(),
            },
        )
        .summary("List OAuth providers")
        .description("Lists all configured OAuth providers")
        .tag("oauth"),
    ]
}
