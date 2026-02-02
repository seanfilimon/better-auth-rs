//! Configuration for the Phone Number plugin.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Data passed to the sendOTP callback.
#[derive(Debug, Clone)]
pub struct PhoneOtpData {
    /// The phone number to send the OTP to.
    pub phone_number: String,
    /// The OTP code.
    pub code: String,
}

impl PhoneOtpData {
    /// Creates new phone OTP data.
    pub fn new(phone_number: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            phone_number: phone_number.into(),
            code: code.into(),
        }
    }
}

/// Configuration for sign-up on verification.
#[derive(Clone)]
pub struct SignUpOnVerificationConfig {
    /// Function to generate a temporary email for the user.
    pub get_temp_email: Arc<dyn Fn(&str) -> String + Send + Sync>,
    /// Optional function to generate a temporary name for the user.
    pub get_temp_name: Option<Arc<dyn Fn(&str) -> String + Send + Sync>>,
}

impl SignUpOnVerificationConfig {
    /// Creates a new config with the given email generator.
    pub fn new<F>(get_temp_email: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        Self {
            get_temp_email: Arc::new(get_temp_email),
            get_temp_name: None,
        }
    }

    /// Sets the name generator.
    pub fn with_name_generator<F>(mut self, get_temp_name: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.get_temp_name = Some(Arc::new(get_temp_name));
        self
    }
}

impl std::fmt::Debug for SignUpOnVerificationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignUpOnVerificationConfig")
            .field("get_temp_email", &"<function>")
            .field("get_temp_name", &self.get_temp_name.is_some())
            .finish()
    }
}

/// Type alias for the send OTP callback.
pub type SendOtpCallback = Arc<
    dyn Fn(PhoneOtpData) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for custom OTP verification callback.
pub type VerifyOtpCallback = Arc<
    dyn Fn(String, String) -> Pin<Box<dyn Future<Output = bool> + Send>>
        + Send
        + Sync,
>;

/// Type alias for phone number validator.
pub type PhoneValidatorFn = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// Configuration for the Phone Number plugin.
#[derive(Clone)]
pub struct PhoneNumberConfig {
    /// Length of the OTP code. Default: 6.
    pub otp_length: usize,
    /// OTP expiration time in seconds. Default: 300 (5 minutes).
    pub expires_in: u64,
    /// Maximum verification attempts. Default: 3.
    pub allowed_attempts: u32,
    /// Callback to send the OTP.
    pub send_otp: Option<SendOtpCallback>,
    /// Optional custom OTP verification callback.
    pub verify_otp: Option<VerifyOtpCallback>,
    /// Optional phone number validator.
    pub phone_number_validator: Option<PhoneValidatorFn>,
    /// Configuration for sign-up on verification.
    pub sign_up_on_verification: Option<SignUpOnVerificationConfig>,
    /// Whether to require phone verification before sign-in. Default: false.
    pub require_verification: bool,
    /// Callback to send password reset OTP.
    pub send_password_reset_otp: Option<SendOtpCallback>,
    /// Callback after phone verification.
    pub callback_on_verification: Option<Arc<dyn Fn(&str, &str) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>>,
}

impl Default for PhoneNumberConfig {
    fn default() -> Self {
        Self {
            otp_length: 6,
            expires_in: 300, // 5 minutes
            allowed_attempts: 3,
            send_otp: None,
            verify_otp: None,
            phone_number_validator: None,
            sign_up_on_verification: None,
            require_verification: false,
            send_password_reset_otp: None,
            callback_on_verification: None,
        }
    }
}

impl PhoneNumberConfig {
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

    /// Sets the send OTP callback.
    pub fn send_otp<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(PhoneOtpData) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.send_otp = Some(Arc::new(move |data| Box::pin(callback(data))));
        self
    }

    /// Sets a custom OTP verification callback.
    pub fn verify_otp<F, Fut>(mut self, callback: F) -> Self
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        self.verify_otp = Some(Arc::new(move |phone, code| Box::pin(callback(phone, code))));
        self
    }

    /// Sets a phone number validator.
    pub fn phone_number_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.phone_number_validator = Some(Arc::new(validator));
        self
    }

    /// Enables sign-up on verification.
    pub fn sign_up_on_verification(mut self, config: SignUpOnVerificationConfig) -> Self {
        self.sign_up_on_verification = Some(config);
        self
    }

    /// Requires phone verification before sign-in.
    pub fn require_verification(mut self) -> Self {
        self.require_verification = true;
        self
    }

    /// Validates a phone number.
    pub fn validate_phone(&self, phone: &str) -> bool {
        if let Some(ref validator) = self.phone_number_validator {
            validator(phone)
        } else {
            // Basic validation: starts with + and has at least 10 digits
            phone.starts_with('+') && phone.len() >= 10
        }
    }
}

impl std::fmt::Debug for PhoneNumberConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhoneNumberConfig")
            .field("otp_length", &self.otp_length)
            .field("expires_in", &self.expires_in)
            .field("allowed_attempts", &self.allowed_attempts)
            .field("send_otp", &self.send_otp.is_some())
            .field("verify_otp", &self.verify_otp.is_some())
            .field("phone_number_validator", &self.phone_number_validator.is_some())
            .field("sign_up_on_verification", &self.sign_up_on_verification)
            .field("require_verification", &self.require_verification)
            .finish()
    }
}
