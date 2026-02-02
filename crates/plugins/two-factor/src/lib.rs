//! # Better Auth Two-Factor Plugin
//!
//! This plugin adds two-factor authentication (2FA) support to Better Auth.
//! It supports TOTP (Time-based One-Time Password), OTP via email/SMS,
//! backup codes, and trusted devices.

mod config;
mod schema;
mod handlers;
mod totp;
mod backup;

pub use config::{TwoFactorConfig, TotpOptions, OtpOptions, BackupCodeOptions};
pub use schema::{TwoFactorData, TrustedDevice, TwoFactorSchema, TwoFactorUserExt as TwoFactorUserExtSchema};
pub use totp::{TotpManager, TotpUri};
pub use backup::BackupCodeManager;

use async_trait::async_trait;
use better_auth_core::context::AuthContext;
use better_auth_core::error::AuthResult;
use better_auth_core::router::{Method, Route, Router};
use better_auth_core::schema::{Field, FieldType, SchemaBuilder};
use better_auth_core::traits::{AuthPlugin, ExtensionProvider, SchemaProvider};
use better_auth_core::types::{Session, User};
use better_auth_events_sdk::{EventDefinition, EventProvider};
use serde::{Deserialize, Serialize};

/// Trait for accessing TwoFactor fields on User.
pub trait TwoFactorUserExt {
    /// Returns whether 2FA is enabled for this user.
    fn two_factor_enabled(&self) -> bool;
    /// Sets whether 2FA is enabled for this user.
    fn set_two_factor_enabled(&mut self, enabled: bool);
    /// Returns the 2FA secret if set.
    fn two_factor_secret(&self) -> Option<String>;
    /// Sets the 2FA secret.
    fn set_two_factor_secret(&mut self, secret: Option<String>);
}

impl TwoFactorUserExt for User {
    fn two_factor_enabled(&self) -> bool {
        self.get_extension("two_factor_enabled").unwrap_or(false)
    }

    fn set_two_factor_enabled(&mut self, enabled: bool) {
        self.set_extension("two_factor_enabled", enabled);
    }

    fn two_factor_secret(&self) -> Option<String> {
        self.get_extension("two_factor_secret")
    }

    fn set_two_factor_secret(&mut self, secret: Option<String>) {
        if let Some(s) = secret {
            self.set_extension("two_factor_secret", s);
        } else {
            self.remove_extension("two_factor_secret");
        }
    }
}

/// Two-factor authentication plugin.
pub struct TwoFactorPlugin {
    config: TwoFactorConfig,
    totp_manager: TotpManager,
    backup_manager: BackupCodeManager,
}

impl TwoFactorPlugin {
    /// Creates a new TwoFactor plugin with the given configuration.
    pub fn new(config: TwoFactorConfig) -> Self {
        let totp_manager = TotpManager::new(
            config.issuer.clone(),
            config.totp_options.digits,
            config.totp_options.period,
        );
        let backup_manager = BackupCodeManager::new(
            config.backup_code_options.amount,
            config.backup_code_options.length,
        );
        
        Self {
            config,
            totp_manager,
            backup_manager,
        }
    }

    /// Gets the plugin configuration.
    pub fn config(&self) -> &TwoFactorConfig {
        &self.config
    }

    /// Gets the TOTP manager.
    pub fn totp_manager(&self) -> &TotpManager {
        &self.totp_manager
    }

    /// Gets the backup code manager.
    pub fn backup_manager(&self) -> &BackupCodeManager {
        &self.backup_manager
    }
}

impl Default for TwoFactorPlugin {
    fn default() -> Self {
        Self::new(TwoFactorConfig::default())
    }
}

impl EventProvider for TwoFactorPlugin {
    fn provided_events() -> Vec<EventDefinition> {
        vec![
            EventDefinition::simple(
                "two_factor.enabled",
                "Emitted when 2FA is enabled for a user",
                "two_factor",
            ),
            EventDefinition::simple(
                "two_factor.disabled",
                "Emitted when 2FA is disabled for a user",
                "two_factor",
            ),
            EventDefinition::simple(
                "two_factor.totp_verified",
                "Emitted when TOTP verification succeeds",
                "two_factor",
            ),
            EventDefinition::simple(
                "two_factor.otp_sent",
                "Emitted when OTP is sent",
                "two_factor",
            ),
            EventDefinition::simple(
                "two_factor.otp_verified",
                "Emitted when OTP verification succeeds",
                "two_factor",
            ),
            EventDefinition::simple(
                "two_factor.backup_code_used",
                "Emitted when a backup code is used",
                "two_factor",
            ),
            EventDefinition::simple(
                "two_factor.backup_codes_generated",
                "Emitted when backup codes are generated",
                "two_factor",
            ),
            EventDefinition::simple(
                "two_factor.device_trusted",
                "Emitted when a device is marked as trusted",
                "two_factor",
            ),
        ]
    }

    fn event_source() -> &'static str {
        "two_factor"
    }
}

#[async_trait]
impl AuthPlugin for TwoFactorPlugin {
    fn id(&self) -> &'static str {
        "two_factor"
    }

    fn name(&self) -> &'static str {
        "Two-Factor Authentication"
    }

    fn define_schema(&self, builder: &mut SchemaBuilder) {
        // Add user extension field
        builder.add_field_mut("user", Field::new("two_factor_enabled", FieldType::Boolean).default("false"));
        
        // Add two_factor and trusted_device tables
        for model in TwoFactorSchema::schema() {
            builder.add_model_mut(model);
        }
    }

    fn register_routes(&self, router: &mut Router) {
        // POST /two-factor/enable
        router.route(
            Route::new(Method::POST, "/two-factor/enable", handlers::EnableHandler)
                .summary("Enable 2FA")
                .description("Enables two-factor authentication for the user. Returns TOTP URI and backup codes.")
                .tag("two-factor")
                .requires_auth(),
        );

        // POST /two-factor/disable
        router.route(
            Route::new(Method::POST, "/two-factor/disable", handlers::DisableHandler)
                .summary("Disable 2FA")
                .description("Disables two-factor authentication for the user.")
                .tag("two-factor")
                .requires_auth(),
        );

        // POST /two-factor/get-totp-uri
        router.route(
            Route::new(Method::POST, "/two-factor/get-totp-uri", handlers::GetTotpUriHandler)
                .summary("Get TOTP URI")
                .description("Gets the TOTP URI for generating a QR code.")
                .tag("two-factor")
                .requires_auth(),
        );

        // POST /two-factor/verify-totp
        router.route(
            Route::new(Method::POST, "/two-factor/verify-totp", handlers::VerifyTotpHandler)
                .summary("Verify TOTP")
                .description("Verifies a TOTP code.")
                .tag("two-factor"),
        );

        // POST /two-factor/send-otp
        router.route(
            Route::new(Method::POST, "/two-factor/send-otp", handlers::SendOtpHandler)
                .summary("Send OTP")
                .description("Sends an OTP to the user's email or phone.")
                .tag("two-factor"),
        );

        // POST /two-factor/verify-otp
        router.route(
            Route::new(Method::POST, "/two-factor/verify-otp", handlers::VerifyOtpHandler)
                .summary("Verify OTP")
                .description("Verifies an OTP code.")
                .tag("two-factor"),
        );

        // POST /two-factor/generate-backup-codes
        router.route(
            Route::new(Method::POST, "/two-factor/generate-backup-codes", handlers::GenerateBackupCodesHandler)
                .summary("Generate backup codes")
                .description("Generates new backup codes. Old codes are invalidated.")
                .tag("two-factor")
                .requires_auth(),
        );

        // POST /two-factor/verify-backup-code
        router.route(
            Route::new(Method::POST, "/two-factor/verify-backup-code", handlers::VerifyBackupCodeHandler)
                .summary("Verify backup code")
                .description("Verifies a backup code for account recovery.")
                .tag("two-factor"),
        );
    }

    async fn on_after_signin(
        &self,
        _ctx: &AuthContext,
        session: &mut Session,
    ) -> AuthResult<()> {
        // Check if user has 2FA enabled and set a flag on the session
        // indicating 2FA verification is required
        // This would be checked by the application to redirect to 2FA page
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = TwoFactorPlugin::default();
        assert_eq!(plugin.id(), "two_factor");
    }

    #[test]
    fn test_user_extension() {
        let mut user = User::new("test_id".to_string(), "test@example.com".to_string());
        
        assert!(!user.two_factor_enabled());
        
        user.set_two_factor_enabled(true);
        assert!(user.two_factor_enabled());
        
        user.set_two_factor_secret(Some("secret".to_string()));
        assert_eq!(user.two_factor_secret(), Some("secret".to_string()));
    }
}
