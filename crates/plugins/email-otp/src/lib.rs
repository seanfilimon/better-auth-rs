//! # Better Auth Email OTP Plugin
//!
//! This plugin allows users to sign in, verify their email, or reset their password
//! using a one-time password (OTP) sent to their email address.

mod config;
mod schema;
mod handlers;

pub use config::{EmailOtpConfig, EmailOtpData, OtpPurpose};
pub use schema::{EmailOtp, EmailOtpSchema};

use async_trait::async_trait;
use better_auth_core::context::{AuthContext, SignInCredentials, SignUpData};
use better_auth_core::error::{AuthError, AuthResult};
use better_auth_core::router::{Method, Route, Router};
use better_auth_core::schema::SchemaBuilder;
use better_auth_core::traits::{AuthPlugin, SchemaProvider};
use better_auth_core::types::{Session, User};
use better_auth_events_sdk::{EventDefinition, EventProvider};
use better_auth_otp_utils::{OtpGenerator, OtpConfig, VerificationCode};
use chrono::Duration;
use std::sync::Arc;

/// The Email OTP authentication plugin.
pub struct EmailOtpPlugin {
    config: EmailOtpConfig,
}

impl EmailOtpPlugin {
    /// Creates a new Email OTP plugin with the given configuration.
    pub fn new(config: EmailOtpConfig) -> Self {
        Self { config }
    }

    /// Gets the plugin configuration.
    pub fn config(&self) -> &EmailOtpConfig {
        &self.config
    }

    /// Generates a new OTP code.
    pub fn generate_otp(&self) -> String {
        let generator = OtpGenerator::new(OtpConfig::numeric(self.config.otp_length));
        generator.generate()
    }

    /// Creates a verification code for the given email and purpose.
    pub fn create_verification_code(&self, email: &str, purpose: OtpPurpose) -> VerificationCode {
        let otp = self.generate_otp();
        VerificationCode::new(
            email,
            otp,
            purpose.as_str(),
            Duration::seconds(self.config.expires_in as i64),
            self.config.allowed_attempts,
        )
    }
}

impl Default for EmailOtpPlugin {
    fn default() -> Self {
        Self::new(EmailOtpConfig::default())
    }
}

impl EventProvider for EmailOtpPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "email_otp.sent",
                "Emitted when an OTP is sent to an email",
                "email_otp",
            ),
            EventDefinition::simple(
                "email_otp.verified",
                "Emitted when an OTP is successfully verified",
                "email_otp",
            ),
            EventDefinition::simple(
                "email_otp.failed",
                "Emitted when OTP verification fails",
                "email_otp",
            ),
            EventDefinition::simple(
                "email_otp.sign_in",
                "Emitted when a user signs in via email OTP",
                "email_otp",
            ),
            EventDefinition::simple(
                "email_otp.password_reset",
                "Emitted when a password is reset via email OTP",
                "email_otp",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "email_otp"
    }
}

#[async_trait]
impl AuthPlugin for EmailOtpPlugin {
    fn id(&self) -> &'static str {
        "email_otp"
    }

    fn name(&self) -> &'static str {
        "Email OTP Authentication"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        for model in EmailOtpSchema::schema() {
            builder.add_model_mut(model);
        }
    }

    fn register_routes(&self, router: &mut Router) {
        // POST /email-otp/send-verification-otp
        router.route(
            Route::new(
                Method::POST,
                "/email-otp/send-verification-otp",
                handlers::SendVerificationOtpHandler,
            )
            .summary("Send verification OTP to email")
            .description("Sends a one-time password to the specified email address for sign-in, email verification, or password reset.")
            .tag("email-otp"),
        );

        // POST /email-otp/check-verification-otp
        router.route(
            Route::new(
                Method::POST,
                "/email-otp/check-verification-otp",
                handlers::CheckVerificationOtpHandler,
            )
            .summary("Check if OTP is valid")
            .description("Checks if the provided OTP is valid without consuming it.")
            .tag("email-otp"),
        );

        // POST /sign-in/email-otp
        router.route(
            Route::new(
                Method::POST,
                "/sign-in/email-otp",
                handlers::SignInEmailOtpHandler,
            )
            .summary("Sign in with email OTP")
            .description("Signs in a user using their email and OTP. Creates a new user if they don't exist (unless disabled).")
            .tag("email-otp"),
        );

        // POST /email-otp/verify-email
        router.route(
            Route::new(
                Method::POST,
                "/email-otp/verify-email",
                handlers::VerifyEmailHandler,
            )
            .summary("Verify email address")
            .description("Verifies the user's email address using an OTP.")
            .tag("email-otp"),
        );

        // POST /email-otp/request-password-reset
        router.route(
            Route::new(
                Method::POST,
                "/email-otp/request-password-reset",
                handlers::RequestPasswordResetHandler,
            )
            .summary("Request password reset")
            .description("Sends a password reset OTP to the user's email.")
            .tag("email-otp"),
        );

        // POST /email-otp/reset-password
        router.route(
            Route::new(
                Method::POST,
                "/email-otp/reset-password",
                handlers::ResetPasswordHandler,
            )
            .summary("Reset password with OTP")
            .description("Resets the user's password using an OTP.")
            .tag("email-otp"),
        );
    }

    async fn on_before_signup(
        &self,
        _ctx: &AuthContext,
        _data: &mut SignUpData,
    ) -> AuthResult<()> {
        Ok(())
    }

    async fn on_after_signup(&self, _ctx: &AuthContext, user: &User) -> AuthResult<()> {
        // Optionally send verification OTP on signup
        if self.config.send_verification_on_sign_up {
            // This would trigger the sendVerificationOTP callback
            // Implementation depends on how callbacks are handled
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = EmailOtpPlugin::default();
        assert_eq!(plugin.id(), "email_otp");
        assert_eq!(plugin.config().otp_length, 6);
    }

    #[test]
    fn test_otp_generation() {
        let plugin = EmailOtpPlugin::default();
        let otp = plugin.generate_otp();
        assert_eq!(otp.len(), 6);
        assert!(otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_verification_code_creation() {
        let plugin = EmailOtpPlugin::default();
        let code = plugin.create_verification_code("test@example.com", OtpPurpose::SignIn);
        
        assert_eq!(code.identifier, "test@example.com");
        assert_eq!(code.verification_type, "sign-in");
        assert!(!code.is_expired());
    }
}
