//! # Better Auth API Key Plugin
//!
//! This plugin allows you to create and manage API keys for your application.
//! It provides a way to authenticate and authorize API requests by verifying API keys.
//!
//! ## Features
//!
//! - Create, manage, and verify API keys
//! - Built-in rate limiting
//! - Custom expiration times, remaining count, and refill systems
//! - Metadata for API keys
//! - Custom prefix
//! - Sessions from API keys
//! - Permissions support

mod config;
mod schema;
mod handlers;
mod generator;
mod rate_limit;

pub use config::{ApiKeyConfig, RateLimitConfig, StorageMode};
pub use schema::{ApiKey, ApiKeySchema};
pub use generator::ApiKeyGenerator;
pub use rate_limit::ApiKeyRateLimiter;

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::AuthResult;
use better_auth_core::router::{Method, Route, Router};
use better_auth_core::schema::SchemaBuilder;
use better_auth_core::traits::{AuthPlugin, SchemaProvider};
use better_auth_core::types::User;
use better_auth_events_sdk::{EventDefinition, EventProvider};

/// The API Key authentication plugin.
pub struct ApiKeyPlugin {
    config: ApiKeyConfig,
    generator: ApiKeyGenerator,
}

impl ApiKeyPlugin {
    /// Creates a new API Key plugin with the given configuration.
    pub fn new(config: ApiKeyConfig) -> Self {
        let generator = ApiKeyGenerator::new(config.default_key_length, config.default_prefix.clone());
        Self { config, generator }
    }

    /// Gets the plugin configuration.
    pub fn config(&self) -> &ApiKeyConfig {
        &self.config
    }

    /// Gets the API key generator.
    pub fn generator(&self) -> &ApiKeyGenerator {
        &self.generator
    }

    /// Generates a new API key.
    pub fn generate_key(&self) -> String {
        self.generator.generate()
    }
}

impl Default for ApiKeyPlugin {
    fn default() -> Self {
        Self::new(ApiKeyConfig::default())
    }
}

impl EventProvider for ApiKeyPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "api_key.created",
                "Emitted when an API key is created",
                "api_key",
            ),
            EventDefinition::simple(
                "api_key.verified",
                "Emitted when an API key is successfully verified",
                "api_key",
            ),
            EventDefinition::simple(
                "api_key.revoked",
                "Emitted when an API key is revoked",
                "api_key",
            ),
            EventDefinition::simple(
                "api_key.rate_limited",
                "Emitted when an API key is rate limited",
                "api_key",
            ),
            EventDefinition::simple(
                "api_key.expired",
                "Emitted when an expired API key is used",
                "api_key",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "api_key"
    }
}

#[async_trait]
impl AuthPlugin for ApiKeyPlugin {
    fn id(&self) -> &'static str {
        "api_key"
    }

    fn name(&self) -> &'static str {
        "API Key Authentication"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        for model in ApiKeySchema::schema() {
            builder.add_model_mut(model);
        }
    }

    fn register_routes(&self, router: &mut Router) {
        // POST /api-key/create
        router.route(
            Route::new(Method::POST, "/api-key/create", handlers::CreateApiKeyHandler)
                .summary("Create API key")
                .description("Creates a new API key for the authenticated user.")
                .tag("api-key")
                .requires_auth(),
        );

        // POST /api-key/verify
        router.route(
            Route::new(Method::POST, "/api-key/verify", handlers::VerifyApiKeyHandler)
                .summary("Verify API key")
                .description("Verifies an API key and optionally checks permissions.")
                .tag("api-key"),
        );

        // GET /api-key/get
        router.route(
            Route::new(Method::GET, "/api-key/get", handlers::GetApiKeyHandler)
                .summary("Get API key")
                .description("Gets details about an API key by ID.")
                .tag("api-key")
                .requires_auth(),
        );

        // POST /api-key/update
        router.route(
            Route::new(Method::POST, "/api-key/update", handlers::UpdateApiKeyHandler)
                .summary("Update API key")
                .description("Updates an API key's properties.")
                .tag("api-key")
                .requires_auth(),
        );

        // POST /api-key/delete
        router.route(
            Route::new(Method::POST, "/api-key/delete", handlers::DeleteApiKeyHandler)
                .summary("Delete API key")
                .description("Deletes an API key.")
                .tag("api-key")
                .requires_auth(),
        );

        // GET /api-key/list
        router.route(
            Route::new(Method::GET, "/api-key/list", handlers::ListApiKeysHandler)
                .summary("List API keys")
                .description("Lists all API keys for the authenticated user.")
                .tag("api-key")
                .requires_auth(),
        );

        // POST /api-key/delete-all-expired
        router.route(
            Route::new(Method::POST, "/api-key/delete-all-expired", handlers::DeleteExpiredKeysHandler)
                .summary("Delete expired API keys")
                .description("Deletes all expired API keys.")
                .tag("api-key"),
        );
    }

    async fn on_after_signup(&self, _ctx: &AuthContext, _user: &User) -> AuthResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = ApiKeyPlugin::default();
        assert_eq!(plugin.id(), "api_key");
    }

    #[test]
    fn test_key_generation() {
        let plugin = ApiKeyPlugin::default();
        let key = plugin.generate_key();
        assert_eq!(key.len(), 64);
    }

    #[test]
    fn test_key_with_prefix() {
        let config = ApiKeyConfig::new().default_prefix("sk_live_");
        let plugin = ApiKeyPlugin::new(config);
        let key = plugin.generate_key();
        assert!(key.starts_with("sk_live_"));
    }
}
