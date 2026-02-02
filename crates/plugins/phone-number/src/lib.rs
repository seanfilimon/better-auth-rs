//! # Better Auth Phone Number Plugin
//!
//! This plugin extends the authentication system by allowing users to sign in
//! and sign up using their phone number with OTP verification.

mod config;
mod schema;
mod handlers;

pub use config::{PhoneNumberConfig, PhoneOtpData, SignUpOnVerificationConfig};
pub use schema::{PhoneVerification, PhoneNumberSchema, PhoneNumberUserExt};

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::AuthResult;
use better_auth_core::router::{Method, Route, Router};
use better_auth_core::schema::{Field, FieldType, SchemaBuilder};
use better_auth_core::traits::{AuthPlugin, ExtensionProvider, SchemaProvider};
use better_auth_core::types::User;
use better_auth_events_sdk::{EventDefinition, EventProvider};
use better_auth_otp_utils::{OtpGenerator, OtpConfig};
use chrono::Duration;

/// Trait for phone number operations on users.
pub trait PhoneNumberExt {
    /// Gets the user's phone number.
    fn phone_number(&self) -> Option<String>;
    /// Sets the user's phone number.
    fn set_phone_number(&mut self, phone: impl Into<String>);
    /// Checks if the phone number is verified.
    fn phone_number_verified(&self) -> bool;
    /// Sets the phone number verification status.
    fn set_phone_number_verified(&mut self, verified: bool);
}

impl PhoneNumberExt for User {
    fn phone_number(&self) -> Option<String> {
        self.get_extension("phone_number")
    }

    fn set_phone_number(&mut self, phone: impl Into<String>) {
        self.set_extension("phone_number", phone.into());
    }

    fn phone_number_verified(&self) -> bool {
        self.get_extension("phone_number_verified").unwrap_or(false)
    }

    fn set_phone_number_verified(&mut self, verified: bool) {
        self.set_extension("phone_number_verified", verified);
    }
}

/// The Phone Number authentication plugin.
pub struct PhoneNumberPlugin {
    config: PhoneNumberConfig,
}

impl PhoneNumberPlugin {
    /// Creates a new Phone Number plugin with the given configuration.
    pub fn new(config: PhoneNumberConfig) -> Self {
        Self { config }
    }

    /// Gets the plugin configuration.
    pub fn config(&self) -> &PhoneNumberConfig {
        &self.config
    }

    /// Generates a new OTP code.
    pub fn generate_otp(&self) -> String {
        let generator = OtpGenerator::new(OtpConfig::numeric(self.config.otp_length));
        generator.generate()
    }
}

impl Default for PhoneNumberPlugin {
    fn default() -> Self {
        Self::new(PhoneNumberConfig::default())
    }
}

impl EventProvider for PhoneNumberPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "phone_number.otp_sent",
                "Emitted when an OTP is sent to a phone number",
                "phone_number",
            ),
            EventDefinition::simple(
                "phone_number.verified",
                "Emitted when a phone number is verified",
                "phone_number",
            ),
            EventDefinition::simple(
                "phone_number.sign_in",
                "Emitted when a user signs in with phone number",
                "phone_number",
            ),
            EventDefinition::simple(
                "phone_number.password_reset",
                "Emitted when a password is reset via phone",
                "phone_number",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "phone_number"
    }
}

#[async_trait]
impl AuthPlugin for PhoneNumberPlugin {
    fn id(&self) -> &'static str {
        "phone_number"
    }

    fn name(&self) -> &'static str {
        "Phone Number Authentication"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        // Add user extension fields
        for field in PhoneNumberUserExt::fields() {
            builder.add_field_mut("user", field);
        }
        
        // Add phone verification table
        for model in PhoneNumberSchema::schema() {
            builder.add_model_mut(model);
        }
    }

    fn register_routes(&self, router: &mut Router) {
        // POST /phone-number/send-otp
        router.route(
            Route::new(
                Method::POST,
                "/phone-number/send-otp",
                handlers::SendOtpHandler,
            )
            .summary("Send OTP to phone")
            .description("Sends a one-time password to the specified phone number.")
            .tag("phone-number"),
        );

        // POST /phone-number/verify
        router.route(
            Route::new(
                Method::POST,
                "/phone-number/verify",
                handlers::VerifyPhoneHandler,
            )
            .summary("Verify phone number")
            .description("Verifies a phone number using an OTP and optionally creates a session.")
            .tag("phone-number"),
        );

        // POST /sign-in/phone-number
        router.route(
            Route::new(
                Method::POST,
                "/sign-in/phone-number",
                handlers::SignInPhoneNumberHandler,
            )
            .summary("Sign in with phone number")
            .description("Signs in a user using their phone number and password.")
            .tag("phone-number"),
        );

        // POST /phone-number/request-password-reset
        router.route(
            Route::new(
                Method::POST,
                "/phone-number/request-password-reset",
                handlers::RequestPasswordResetHandler,
            )
            .summary("Request password reset")
            .description("Sends a password reset OTP to the user's phone number.")
            .tag("phone-number"),
        );

        // POST /phone-number/reset-password
        router.route(
            Route::new(
                Method::POST,
                "/phone-number/reset-password",
                handlers::ResetPasswordHandler,
            )
            .summary("Reset password")
            .description("Resets the user's password using an OTP.")
            .tag("phone-number"),
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
        let plugin = PhoneNumberPlugin::default();
        assert_eq!(plugin.id(), "phone_number");
        assert_eq!(plugin.config().otp_length, 6);
    }

    #[test]
    fn test_otp_generation() {
        let plugin = PhoneNumberPlugin::default();
        let otp = plugin.generate_otp();
        assert_eq!(otp.len(), 6);
        assert!(otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_user_extension() {
        let mut user = User::new("test_id".to_string(), "test@example.com".to_string());
        
        assert!(user.phone_number().is_none());
        assert!(!user.phone_number_verified());
        
        user.set_phone_number("+1234567890");
        user.set_phone_number_verified(true);
        
        assert_eq!(user.phone_number(), Some("+1234567890".to_string()));
        assert!(user.phone_number_verified());
    }
}
