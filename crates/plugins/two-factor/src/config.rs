//! Configuration for the Two-Factor plugin.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Data passed to the sendOTP callback.
#[derive(Debug, Clone)]
pub struct TwoFactorOtpData {
    /// The user's email or phone.
    pub user_identifier: String,
    /// The OTP code.
    pub otp: String,
}

/// Type alias for the send OTP callback.
pub type SendOtpCallback = Arc<
    dyn Fn(TwoFactorOtpData) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

/// TOTP-specific options.
#[derive(Debug, Clone)]
pub struct TotpOptions {
    /// Number of digits in the TOTP code. Default: 6.
    pub digits: u32,
    /// Time period in seconds. Default: 30.
    pub period: u32,
}

impl Default for TotpOptions {
    fn default() -> Self {
        Self {
            digits: 6,
            period: 30,
        }
    }
}

/// OTP-specific options.
#[derive(Clone)]
pub struct OtpOptions {
    /// Callback to send OTP.
    pub send_otp: Option<SendOtpCallback>,
    /// OTP expiration in seconds. Default: 300 (5 minutes).
    pub period: u64,
    /// How to store OTP: "plain", "hashed", or "encrypted".
    pub store_otp: String,
}

impl Default for OtpOptions {
    fn default() -> Self {
        Self {
            send_otp: None,
            period: 300,
            store_otp: "plain".to_string(),
        }
    }
}

impl std::fmt::Debug for OtpOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OtpOptions")
            .field("send_otp", &self.send_otp.is_some())
            .field("period", &self.period)
            .field("store_otp", &self.store_otp)
            .finish()
    }
}

/// Backup code options.
#[derive(Clone)]
pub struct BackupCodeOptions {
    /// Number of backup codes to generate. Default: 10.
    pub amount: usize,
    /// Length of each backup code. Default: 10.
    pub length: usize,
    /// Custom backup code generator.
    pub custom_generator: Option<Arc<dyn Fn() -> Vec<String> + Send + Sync>>,
    /// How to store backup codes: "plain", "hashed", or "encrypted".
    pub store_backup_codes: String,
}

impl Default for BackupCodeOptions {
    fn default() -> Self {
        Self {
            amount: 10,
            length: 10,
            custom_generator: None,
            store_backup_codes: "plain".to_string(),
        }
    }
}

impl std::fmt::Debug for BackupCodeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackupCodeOptions")
            .field("amount", &self.amount)
            .field("length", &self.length)
            .field("custom_generator", &self.custom_generator.is_some())
            .field("store_backup_codes", &self.store_backup_codes)
            .finish()
    }
}

/// Configuration for the Two-Factor plugin.
#[derive(Clone)]
pub struct TwoFactorConfig {
    /// The issuer name for TOTP (displayed in authenticator apps).
    pub issuer: String,
    /// Name of the two-factor table. Default: "twoFactor".
    pub two_factor_table: String,
    /// Skip verification when enabling 2FA.
    pub skip_verification_on_enable: bool,
    /// TOTP options.
    pub totp_options: TotpOptions,
    /// OTP options.
    pub otp_options: OtpOptions,
    /// Backup code options.
    pub backup_code_options: BackupCodeOptions,
    /// Trusted device duration in days. Default: 30.
    pub trusted_device_days: u32,
}

impl Default for TwoFactorConfig {
    fn default() -> Self {
        Self {
            issuer: "Better Auth".to_string(),
            two_factor_table: "twoFactor".to_string(),
            skip_verification_on_enable: false,
            totp_options: TotpOptions::default(),
            otp_options: OtpOptions::default(),
            backup_code_options: BackupCodeOptions::default(),
            trusted_device_days: 30,
        }
    }
}

impl TwoFactorConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the issuer name.
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = issuer.into();
        self
    }

    /// Sets the two-factor table name.
    pub fn two_factor_table(mut self, name: impl Into<String>) -> Self {
        self.two_factor_table = name.into();
        self
    }

    /// Skips verification when enabling 2FA.
    pub fn skip_verification_on_enable(mut self) -> Self {
        self.skip_verification_on_enable = true;
        self
    }

    /// Sets TOTP options.
    pub fn totp_options(mut self, options: TotpOptions) -> Self {
        self.totp_options = options;
        self
    }

    /// Sets OTP options.
    pub fn otp_options(mut self, options: OtpOptions) -> Self {
        self.otp_options = options;
        self
    }

    /// Sets backup code options.
    pub fn backup_code_options(mut self, options: BackupCodeOptions) -> Self {
        self.backup_code_options = options;
        self
    }

    /// Sets trusted device duration in days.
    pub fn trusted_device_days(mut self, days: u32) -> Self {
        self.trusted_device_days = days;
        self
    }

    /// Sets the send OTP callback.
    pub fn send_otp<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(TwoFactorOtpData) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.otp_options.send_otp = Some(Arc::new(move |data| Box::pin(callback(data))));
        self
    }
}

impl std::fmt::Debug for TwoFactorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TwoFactorConfig")
            .field("issuer", &self.issuer)
            .field("two_factor_table", &self.two_factor_table)
            .field("skip_verification_on_enable", &self.skip_verification_on_enable)
            .field("totp_options", &self.totp_options)
            .field("trusted_device_days", &self.trusted_device_days)
            .finish()
    }
}
