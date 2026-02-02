//! Configuration for the Email OTP plugin.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// The purpose of an OTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtpPurpose {
    /// OTP for signing in.
    SignIn,
    /// OTP for email verification.
    EmailVerification,
    /// OTP for password reset.
    PasswordReset,
}

impl OtpPurpose {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            OtpPurpose::SignIn => "sign-in",
            OtpPurpose::EmailVerification => "email-verification",
            OtpPurpose::PasswordReset => "forget-password",
        }
    }

    /// Parses from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "sign-in" => Some(OtpPurpose::SignIn),
            "email-verification" => Some(OtpPurpose::EmailVerification),
            "forget-password" => Some(OtpPurpose::PasswordReset),
            _ => None,
        }
    }
}

/// Data passed to the sendVerificationOTP callback.
#[derive(Debug, Clone)]
pub struct EmailOtpData {
    /// The email address to send the OTP to.
    pub email: String,
    /// The OTP code.
    pub otp: String,
    /// The purpose of the OTP.
    pub otp_type: OtpPurpose,
}

impl EmailOtpData {
    /// Creates new OTP data.
    pub fn new(email: impl Into<String>, otp: impl Into<String>, otp_type: OtpPurpose) -> Self {
        Self {
            email: email.into(),
            otp: otp.into(),
            otp_type,
        }
    }
}

/// Type alias for the send OTP callback.
pub type SendOtpCallback = Arc<
    dyn Fn(EmailOtpData) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for custom OTP generator.
pub type OtpGeneratorFn = Arc<dyn Fn() -> String + Send + Sync>;

/// Configuration for the Email OTP plugin.
#[derive(Clone)]
pub struct EmailOtpConfig {
    /// Length of the OTP code. Default: 6.
    pub otp_length: usize,
    /// OTP expiration time in seconds. Default: 300 (5 minutes).
    pub expires_in: u64,
    /// Maximum verification attempts. Default: 3.
    pub allowed_attempts: u32,
    /// Whether to disable automatic sign-up for new users. Default: false.
    pub disable_sign_up: bool,
    /// Whether to send verification OTP on sign-up. Default: false.
    pub send_verification_on_sign_up: bool,
    /// Whether to override default email verification with OTP. Default: false.
    pub override_default_email_verification: bool,
    /// Callback to send the OTP.
    pub send_verification_otp: Option<SendOtpCallback>,
    /// Custom OTP generator function.
    pub generate_otp: Option<OtpGeneratorFn>,
    /// How to store OTPs: "plain", "hashed", or "encrypted".
    pub store_otp: OtpStorageMode,
}

/// How OTPs are stored in the database.
#[derive(Debug, Clone, Default)]
pub enum OtpStorageMode {
    /// Store OTPs in plain text.
    #[default]
    Plain,
    /// Hash OTPs before storage.
    Hashed,
    /// Encrypt OTPs before storage.
    Encrypted,
}

impl Default for EmailOtpConfig {
    fn default() -> Self {
        Self {
            otp_length: 6,
            expires_in: 300, // 5 minutes
            allowed_attempts: 3,
            disable_sign_up: false,
            send_verification_on_sign_up: false,
            override_default_email_verification: false,
            send_verification_otp: None,
            generate_otp: None,
            store_otp: OtpStorageMode::Plain,
        }
    }
}

impl EmailOtpConfig {
    /// Creates a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the OTP length.
    pub fn otp_length(mut self, length: usize) -> Self {
        self.otp_length = length;
        self
    }

    /// Sets the expiration time in seconds.
    pub fn expires_in(mut self, seconds: u64) -> Self {
        self.expires_in = seconds;
        self
    }

    /// Sets the maximum allowed attempts.
    pub fn allowed_attempts(mut self, attempts: u32) -> Self {
        self.allowed_attempts = attempts;
        self
    }

    /// Disables automatic sign-up.
    pub fn disable_sign_up(mut self) -> Self {
        self.disable_sign_up = true;
        self
    }

    /// Enables sending verification OTP on sign-up.
    pub fn send_verification_on_sign_up(mut self) -> Self {
        self.send_verification_on_sign_up = true;
        self
    }

    /// Overrides default email verification with OTP.
    pub fn override_default_email_verification(mut self) -> Self {
        self.override_default_email_verification = true;
        self
    }

    /// Sets the send OTP callback.
    pub fn send_verification_otp<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(EmailOtpData) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.send_verification_otp = Some(Arc::new(move |data| Box::pin(callback(data))));
        self
    }

    /// Sets a custom OTP generator.
    pub fn generate_otp_with<F>(mut self, generator: F) -> Self
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.generate_otp = Some(Arc::new(generator));
        self
    }

    /// Sets the OTP storage mode.
    pub fn store_otp(mut self, mode: OtpStorageMode) -> Self {
        self.store_otp = mode;
        self
    }
}

impl std::fmt::Debug for EmailOtpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailOtpConfig")
            .field("otp_length", &self.otp_length)
            .field("expires_in", &self.expires_in)
            .field("allowed_attempts", &self.allowed_attempts)
            .field("disable_sign_up", &self.disable_sign_up)
            .field("send_verification_on_sign_up", &self.send_verification_on_sign_up)
            .field("override_default_email_verification", &self.override_default_email_verification)
            .field("send_verification_otp", &self.send_verification_otp.is_some())
            .field("generate_otp", &self.generate_otp.is_some())
            .field("store_otp", &self.store_otp)
            .finish()
    }
}
