//! # Better Auth Passkey Plugin
//!
//! This plugin provides passkey (WebAuthn) authentication support.
//! Passkeys are a secure, passwordless authentication method using
//! cryptographic key pairs.

mod config;
mod schema;
mod handlers;
mod webauthn;

pub use config::PasskeyConfig;
pub use schema::{Passkey, PasskeySchema};
pub use webauthn::{WebAuthnChallenge, AuthenticatorSelection};

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::AuthResult;
use better_auth_core::router::{Method, Route, Router};
use better_auth_core::schema::SchemaBuilder;
use better_auth_core::traits::{AuthPlugin, SchemaProvider};
use better_auth_core::types::User;
use better_auth_events_sdk::{EventDefinition, EventProvider};

/// The Passkey authentication plugin.
pub struct PasskeyPlugin {
    config: PasskeyConfig,
}

impl PasskeyPlugin {
    /// Creates a new Passkey plugin with the given configuration.
    pub fn new(config: PasskeyConfig) -> Self {
        Self { config }
    }

    /// Gets the plugin configuration.
    pub fn config(&self) -> &PasskeyConfig {
        &self.config
    }
}

impl Default for PasskeyPlugin {
    fn default() -> Self {
        Self::new(PasskeyConfig::default())
    }
}

impl EventProvider for PasskeyPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "passkey.registered",
                "Emitted when a passkey is registered",
                "passkey",
            ),
            EventDefinition::simple(
                "passkey.authenticated",
                "Emitted when authentication with passkey succeeds",
                "passkey",
            ),
            EventDefinition::simple(
                "passkey.deleted",
                "Emitted when a passkey is deleted",
                "passkey",
            ),
            EventDefinition::simple(
                "passkey.updated",
                "Emitted when a passkey is updated",
                "passkey",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "passkey"
    }
}

#[async_trait]
impl AuthPlugin for PasskeyPlugin {
    fn id(&self) -> &'static str {
        "passkey"
    }

    fn name(&self) -> &'static str {
        "Passkey (WebAuthn) Authentication"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        for model in PasskeySchema::schema() {
            builder.add_model_mut(model);
        }
    }

    fn register_routes(&self, router: &mut Router) {
        // POST /passkey/add-passkey
        router.route(
            Route::new(Method::POST, "/passkey/add-passkey", handlers::AddPasskeyHandler)
                .summary("Register a passkey")
                .description("Registers a new passkey for the authenticated user.")
                .tag("passkey")
                .requires_auth(),
        );

        // POST /sign-in/passkey
        router.route(
            Route::new(Method::POST, "/sign-in/passkey", handlers::SignInPasskeyHandler)
                .summary("Sign in with passkey")
                .description("Authenticates a user using their passkey.")
                .tag("passkey"),
        );

        // GET /passkey/list-user-passkeys
        router.route(
            Route::new(Method::GET, "/passkey/list-user-passkeys", handlers::ListPasskeysHandler)
                .summary("List passkeys")
                .description("Lists all passkeys for the authenticated user.")
                .tag("passkey")
                .requires_auth(),
        );

        // POST /passkey/delete-passkey
        router.route(
            Route::new(Method::POST, "/passkey/delete-passkey", handlers::DeletePasskeyHandler)
                .summary("Delete a passkey")
                .description("Deletes a passkey by ID.")
                .tag("passkey")
                .requires_auth(),
        );

        // POST /passkey/update-passkey
        router.route(
            Route::new(Method::POST, "/passkey/update-passkey", handlers::UpdatePasskeyHandler)
                .summary("Update a passkey")
                .description("Updates a passkey's name.")
                .tag("passkey")
                .requires_auth(),
        );

        // POST /passkey/generate-registration-options
        router.route(
            Route::new(Method::POST, "/passkey/generate-registration-options", handlers::GenerateRegistrationOptionsHandler)
                .summary("Generate registration options")
                .description("Generates WebAuthn registration options for passkey creation.")
                .tag("passkey")
                .requires_auth(),
        );

        // POST /passkey/generate-authentication-options
        router.route(
            Route::new(Method::POST, "/passkey/generate-authentication-options", handlers::GenerateAuthenticationOptionsHandler)
                .summary("Generate authentication options")
                .description("Generates WebAuthn authentication options for passkey sign-in.")
                .tag("passkey"),
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
        let plugin = PasskeyPlugin::default();
        assert_eq!(plugin.id(), "passkey");
    }

    #[test]
    fn test_config() {
        let config = PasskeyConfig::new("example.com", "Example App", "https://example.com");
        let plugin = PasskeyPlugin::new(config);
        
        assert_eq!(plugin.config().rp_id, "example.com");
        assert_eq!(plugin.config().rp_name, "Example App");
    }
}
