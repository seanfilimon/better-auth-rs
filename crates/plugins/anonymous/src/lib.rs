//! # Better Auth Anonymous Plugin
//!
//! This plugin allows users to have an authenticated experience without requiring
//! them to provide an email address, password, OAuth provider, or any other
//! Personally Identifiable Information (PII). Users can later link an authentication
//! method to their account when ready.

mod config;
mod schema;
mod handlers;

pub use config::AnonymousConfig;
pub use schema::AnonymousUserExt;

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::AuthResult;
use better_auth_core::router::{Method, Route, Router};
use better_auth_core::schema::{Field, FieldType, SchemaBuilder};
use better_auth_core::traits::{AuthPlugin, ExtensionProvider};
use better_auth_core::types::User;
use better_auth_events_sdk::{EventDefinition, EventProvider};

/// Trait for anonymous user operations.
pub trait AnonymousExt {
    /// Checks if the user is anonymous.
    fn is_anonymous(&self) -> bool;
    /// Sets the anonymous status.
    fn set_anonymous(&mut self, anonymous: bool);
}

impl AnonymousExt for User {
    fn is_anonymous(&self) -> bool {
        self.get_extension("is_anonymous").unwrap_or(false)
    }

    fn set_anonymous(&mut self, anonymous: bool) {
        self.set_extension("is_anonymous", anonymous);
    }
}

/// The Anonymous authentication plugin.
pub struct AnonymousPlugin {
    config: AnonymousConfig,
}

impl AnonymousPlugin {
    /// Creates a new Anonymous plugin with the given configuration.
    pub fn new(config: AnonymousConfig) -> Self {
        Self { config }
    }

    /// Gets the plugin configuration.
    pub fn config(&self) -> &AnonymousConfig {
        &self.config
    }

    /// Generates a random email for an anonymous user.
    pub fn generate_email(&self) -> String {
        if let Some(ref generator) = self.config.generate_random_email {
            generator()
        } else {
            let id = uuid::Uuid::new_v4();
            if let Some(ref domain) = self.config.email_domain_name {
                format!("temp-{}@{}", id, domain)
            } else {
                format!("temp@{}.com", id)
            }
        }
    }

    /// Generates a name for an anonymous user.
    pub fn generate_name(&self) -> Option<String> {
        self.config.generate_name.as_ref().map(|f| f())
    }
}

impl Default for AnonymousPlugin {
    fn default() -> Self {
        Self::new(AnonymousConfig::default())
    }
}

impl EventProvider for AnonymousPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "anonymous.sign_in",
                "Emitted when an anonymous user signs in",
                "anonymous",
            ),
            EventDefinition::simple(
                "anonymous.account_linked",
                "Emitted when an anonymous account is linked to a real account",
                "anonymous",
            ),
            EventDefinition::simple(
                "anonymous.deleted",
                "Emitted when an anonymous user is deleted",
                "anonymous",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "anonymous"
    }
}

#[async_trait]
impl AuthPlugin for AnonymousPlugin {
    fn id(&self) -> &'static str {
        "anonymous"
    }

    fn name(&self) -> &'static str {
        "Anonymous Authentication"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        // Add user extension field
        for field in AnonymousUserExt::fields() {
            builder.add_field_mut("user", field);
        }
    }

    fn register_routes(&self, router: &mut Router) {
        // POST /sign-in/anonymous
        router.route(
            Route::new(
                Method::POST,
                "/sign-in/anonymous",
                handlers::SignInAnonymousHandler,
            )
            .summary("Sign in anonymously")
            .description("Creates an anonymous user and session without requiring any credentials.")
            .tag("anonymous"),
        );

        // POST /delete-anonymous-user
        router.route(
            Route::new(
                Method::POST,
                "/delete-anonymous-user",
                handlers::DeleteAnonymousUserHandler,
            )
            .summary("Delete anonymous user")
            .description("Deletes the current anonymous user and their session.")
            .tag("anonymous")
            .requires_auth(),
        );
    }

    async fn on_before_signin(
        &self,
        _ctx: &AuthContext,
        _creds: &better_auth_core::context::SignInCredentials,
    ) -> AuthResult<()> {
        // When a user signs in with real credentials, check if they're anonymous
        // and trigger the onLinkAccount callback if configured
        Ok(())
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
        let plugin = AnonymousPlugin::default();
        assert_eq!(plugin.id(), "anonymous");
    }

    #[test]
    fn test_email_generation() {
        let plugin = AnonymousPlugin::default();
        let email = plugin.generate_email();
        assert!(email.contains("temp"));
        assert!(email.contains("@"));
    }

    #[test]
    fn test_email_generation_with_domain() {
        let config = AnonymousConfig::new().email_domain_name("example.com");
        let plugin = AnonymousPlugin::new(config);
        let email = plugin.generate_email();
        assert!(email.ends_with("@example.com"));
    }

    #[test]
    fn test_user_extension() {
        let mut user = User::new("test_id".to_string(), "test@example.com".to_string());
        
        assert!(!user.is_anonymous());
        
        user.set_anonymous(true);
        assert!(user.is_anonymous());
    }
}
