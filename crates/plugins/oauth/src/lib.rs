//! # Better Auth OAuth Plugin
//!
//! This plugin provides OAuth 2.0 authentication support for Better Auth.
//! It supports multiple providers (Google, GitHub, Discord, etc.) and handles the
//! complete OAuth flow including account linking.
//!
//! ## Features
//!
//! - Multiple OAuth providers (Google, GitHub, Discord)
//! - CSRF protection via state parameter
//! - Account linking and unlinking
//! - Configurable token response strategy (cookie, JWT, or both)
//! - Generic provider builder for custom OAuth2 providers
//!
//! ## Example
//!
//! ```rust,ignore
//! use better_auth_plugin_oauth::{OAuthConfig, OAuthPlugin, GoogleProvider, GitHubProvider};
//!
//! let oauth = OAuthPlugin::new(
//!     OAuthConfig::new()
//!         .callback_base("https://myapp.com/api/auth")
//!         .provider(GoogleProvider::new("client_id", "client_secret"))
//!         .provider(GitHubProvider::new("client_id", "client_secret"))
//! );
//! ```

mod provider;
mod routes;

pub use provider::{
    DiscordProvider, GenericOAuthProvider, GenericOAuthProviderBuilder, GitHubProvider,
    GoogleProvider, OAuthError, OAuthProvider, OAuthUserInfo, TokenSet,
};
pub use routes::{OAuthStateStore, TokenResponseStrategy};

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::AuthResult;
use better_auth_core::router::Router;
use better_auth_core::schema::SchemaBuilder;
use better_auth_core::traits::AuthPlugin;
use better_auth_core::types::Session;
use better_auth_events_sdk::{EventDefinition, EventProvider};
use std::collections::HashMap;
use std::sync::Arc;

/// OAuth plugin configuration.
#[derive(Clone)]
pub struct OAuthConfig {
    /// Registered OAuth providers.
    pub providers: HashMap<String, Arc<dyn OAuthProvider>>,
    /// Callback URL base (e.g., "https://myapp.com/api/auth").
    pub callback_base: String,
    /// Whether to allow account linking.
    pub allow_linking: bool,
    /// Whether to auto-create users on first OAuth login.
    pub auto_create_user: bool,
    /// Token response strategy.
    pub token_response: TokenResponseStrategy,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            callback_base: "/api/auth".to_string(),
            allow_linking: true,
            auto_create_user: true,
            token_response: TokenResponseStrategy::default(),
        }
    }
}

impl OAuthConfig {
    /// Creates a new OAuth config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the callback base URL.
    pub fn callback_base(mut self, base: impl Into<String>) -> Self {
        self.callback_base = base.into();
        self
    }

    /// Adds a provider.
    pub fn provider(mut self, provider: impl OAuthProvider + 'static) -> Self {
        self.providers
            .insert(provider.name().to_string(), Arc::new(provider));
        self
    }

    /// Sets whether to allow account linking.
    pub fn allow_linking(mut self, allow: bool) -> Self {
        self.allow_linking = allow;
        self
    }

    /// Sets whether to auto-create users.
    pub fn auto_create_user(mut self, auto: bool) -> Self {
        self.auto_create_user = auto;
        self
    }

    /// Sets the token response strategy.
    pub fn token_response(mut self, strategy: TokenResponseStrategy) -> Self {
        self.token_response = strategy;
        self
    }
}

/// The OAuth authentication plugin.
pub struct OAuthPlugin {
    config: Arc<OAuthConfig>,
    state_store: Arc<OAuthStateStore>,
}

impl OAuthPlugin {
    /// Creates a new OAuth plugin with the given configuration.
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            config: Arc::new(config),
            state_store: Arc::new(OAuthStateStore::new()),
        }
    }

    /// Creates a new OAuth plugin with default configuration.
    pub fn default_config() -> Self {
        Self::new(OAuthConfig::default())
    }

    /// Gets a provider by name.
    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn OAuthProvider>> {
        self.config.providers.get(name)
    }

    /// Returns all registered provider names.
    pub fn provider_names(&self) -> Vec<&str> {
        self.config.providers.keys().map(|s| s.as_str()).collect()
    }

    /// Returns a reference to the state store.
    pub fn state_store(&self) -> &Arc<OAuthStateStore> {
        &self.state_store
    }

    /// Cleans up expired OAuth states.
    pub fn cleanup_expired_states(&self) {
        self.state_store.cleanup_expired();
    }
}

impl Default for OAuthPlugin {
    fn default() -> Self {
        Self::default_config()
    }
}

impl EventProvider for OAuthPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "oauth.signin_started",
                "Emitted when OAuth sign-in flow starts",
                "oauth",
            ),
            EventDefinition::simple(
                "oauth.signin_completed",
                "Emitted when OAuth sign-in completes successfully",
                "oauth",
            ),
            EventDefinition::simple(
                "oauth.signin_failed",
                "Emitted when OAuth sign-in fails",
                "oauth",
            ),
            EventDefinition::simple(
                "oauth.account_linked",
                "Emitted when an OAuth account is linked to a user",
                "oauth",
            ),
            EventDefinition::simple(
                "oauth.account_unlinked",
                "Emitted when an OAuth account is unlinked from a user",
                "oauth",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "oauth"
    }
}

#[async_trait]
impl AuthPlugin for OAuthPlugin {
    fn id(&self) -> &'static str {
        "oauth"
    }

    fn name(&self) -> &'static str {
        "OAuth Authentication"
    }

    fn define_schema(&self, _builder: &mut SchemaBuilder) {
        // The account table is already in core, but we might add OAuth-specific fields
        // For now, we just ensure the account model has what we need
    }

    fn register_routes(&self, router: &mut Router) {
        let routes = routes::create_routes(
            self.config.clone(),
            self.state_store.clone(),
            self.config.token_response.clone(),
        );

        for route in routes {
            router.route(route);
        }
    }

    async fn on_after_signin(
        &self,
        _ctx: &AuthContext,
        _session: &mut Session,
    ) -> AuthResult<()> {
        // Could add OAuth-specific session data here
        Ok(())
    }
}

/// OAuth state stored during the flow.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OAuthState {
    /// Random state string for CSRF protection.
    pub state: String,
    /// The provider being used.
    pub provider: String,
    /// Optional redirect URL after auth.
    pub redirect_url: Option<String>,
    /// When this state expires.
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Whether this is a linking flow (vs. sign-in).
    #[serde(default)]
    pub is_linking: bool,
    /// User ID if this is a linking flow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl OAuthState {
    /// Creates a new OAuth state.
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            state: uuid::Uuid::new_v4().to_string(),
            provider: provider.into(),
            redirect_url: None,
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
            is_linking: false,
            user_id: None,
        }
    }

    /// Sets the redirect URL.
    pub fn with_redirect(mut self, url: impl Into<String>) -> Self {
        self.redirect_url = Some(url.into());
        self
    }

    /// Marks this as a linking flow.
    pub fn for_linking(mut self, user_id: impl Into<String>) -> Self {
        self.is_linking = true;
        self.user_id = Some(user_id.into());
        self
    }

    /// Checks if the state has expired.
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config_builder() {
        let config = OAuthConfig::new()
            .callback_base("https://example.com/api/auth")
            .allow_linking(false)
            .auto_create_user(false)
            .token_response(TokenResponseStrategy::JwtResponse);

        assert_eq!(config.callback_base, "https://example.com/api/auth");
        assert!(!config.allow_linking);
        assert!(!config.auto_create_user);
    }

    #[test]
    fn test_oauth_state() {
        let state = OAuthState::new("google")
            .with_redirect("https://example.com/dashboard");

        assert_eq!(state.provider, "google");
        assert_eq!(
            state.redirect_url,
            Some("https://example.com/dashboard".to_string())
        );
        assert!(!state.is_expired());
        assert!(!state.is_linking);
    }

    #[test]
    fn test_oauth_state_linking() {
        let state = OAuthState::new("github").for_linking("user_123");

        assert!(state.is_linking);
        assert_eq!(state.user_id, Some("user_123".to_string()));
    }

    #[test]
    fn test_state_store() {
        let store = OAuthStateStore::new();
        let state = OAuthState::new("google");
        let state_key = state.state.clone();

        store.store(&state);

        let retrieved = store.take(&state_key);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().provider, "google");

        // Should be removed after take
        let retrieved_again = store.take(&state_key);
        assert!(retrieved_again.is_none());
    }
}
