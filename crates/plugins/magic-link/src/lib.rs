//! # Better Auth Magic Link Plugin
//!
//! This plugin allows users to authenticate using magic links sent to their email.
//! When a user enters their email, a link is sent to their email. When the user
//! clicks on the link, they are authenticated.

mod config;
mod schema;
mod handlers;

pub use config::{MagicLinkConfig, MagicLinkData};
pub use schema::{MagicLinkToken, MagicLinkSchema};

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::AuthResult;
use better_auth_core::router::{Method, Route, Router};
use better_auth_core::schema::SchemaBuilder;
use better_auth_core::traits::{AuthPlugin, SchemaProvider};
use better_auth_core::types::User;
use better_auth_events_sdk::{EventDefinition, EventProvider};
use better_auth_otp_utils::OtpGenerator;

/// The Magic Link authentication plugin.
pub struct MagicLinkPlugin {
    config: MagicLinkConfig,
}

impl MagicLinkPlugin {
    /// Creates a new Magic Link plugin with the given configuration.
    pub fn new(config: MagicLinkConfig) -> Self {
        Self { config }
    }

    /// Gets the plugin configuration.
    pub fn config(&self) -> &MagicLinkConfig {
        &self.config
    }

    /// Generates a new magic link token.
    pub fn generate_token(&self) -> String {
        if let Some(ref generator) = self.config.generate_token {
            generator()
        } else {
            OtpGenerator::generate_secure_token(64)
        }
    }

    /// Builds the magic link URL.
    pub fn build_url(&self, token: &str, callback_url: Option<&str>) -> String {
        let base = callback_url.unwrap_or("/");
        format!("/api/auth/magic-link/verify?token={}&callbackURL={}", token, base)
    }
}

impl Default for MagicLinkPlugin {
    fn default() -> Self {
        Self::new(MagicLinkConfig::default())
    }
}

impl EventProvider for MagicLinkPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "magic_link.sent",
                "Emitted when a magic link is sent",
                "magic_link",
            ),
            EventDefinition::simple(
                "magic_link.verified",
                "Emitted when a magic link is verified",
                "magic_link",
            ),
            EventDefinition::simple(
                "magic_link.expired",
                "Emitted when a magic link verification fails due to expiration",
                "magic_link",
            ),
            EventDefinition::simple(
                "magic_link.user_created",
                "Emitted when a new user is created via magic link",
                "magic_link",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "magic_link"
    }
}

#[async_trait]
impl AuthPlugin for MagicLinkPlugin {
    fn id(&self) -> &'static str {
        "magic_link"
    }

    fn name(&self) -> &'static str {
        "Magic Link Authentication"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        for model in MagicLinkSchema::schema() {
            builder.add_model_mut(model);
        }
    }

    fn register_routes(&self, router: &mut Router) {
        // POST /sign-in/magic-link
        router.route(
            Route::new(
                Method::POST,
                "/sign-in/magic-link",
                handlers::SignInMagicLinkHandler,
            )
            .summary("Send magic link")
            .description("Sends a magic link to the specified email address for authentication.")
            .tag("magic-link"),
        );

        // GET /magic-link/verify
        router.route(
            Route::new(
                Method::GET,
                "/magic-link/verify",
                handlers::VerifyMagicLinkHandler,
            )
            .summary("Verify magic link")
            .description("Verifies a magic link token and creates a session.")
            .tag("magic-link"),
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
        let plugin = MagicLinkPlugin::default();
        assert_eq!(plugin.id(), "magic_link");
    }

    #[test]
    fn test_token_generation() {
        let plugin = MagicLinkPlugin::default();
        let token = plugin.generate_token();
        assert_eq!(token.len(), 64);
    }

    #[test]
    fn test_url_building() {
        let plugin = MagicLinkPlugin::default();
        let url = plugin.build_url("abc123", Some("/dashboard"));
        assert!(url.contains("token=abc123"));
        assert!(url.contains("callbackURL=/dashboard"));
    }
}
