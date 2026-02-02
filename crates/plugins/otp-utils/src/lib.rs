//! # Better Auth OTP Utilities
//!
//! Shared utilities for OTP (One-Time Password) functionality across Better Auth plugins.
//! This crate provides:
//! - OTP generation (numeric and alphanumeric)
//! - Rate limiting logic
//! - Attempt tracking
//! - Expiration handling
//! - Token storage patterns

mod generator;
mod rate_limit;
mod storage;
mod verification;

pub use generator::{OtpGenerator, OtpConfig, OtpType};
pub use rate_limit::{RateLimiter, RateLimitConfig, RateLimitResult};
pub use storage::{TokenStorage, TokenStorageMode, StoredToken};
pub use verification::{VerificationResult, VerificationError, AttemptTracker};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Represents a verification code/token with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCode {
    /// The unique identifier for this verification.
    pub id: String,
    /// The identifier (email, phone, etc.) this code is for.
    pub identifier: String,
    /// The actual code/token value.
    pub code: String,
    /// The type of verification (sign-in, email-verification, password-reset, etc.).
    pub verification_type: String,
    /// When this code expires.
    pub expires_at: DateTime<Utc>,
    /// Number of verification attempts made.
    pub attempts: u32,
    /// Maximum allowed attempts.
    pub max_attempts: u32,
    /// Whether this code has been used.
    pub used: bool,
    /// When this code was created.
    pub created_at: DateTime<Utc>,
}

impl VerificationCode {
    /// Creates a new verification code.
    pub fn new(
        identifier: impl Into<String>,
        code: impl Into<String>,
        verification_type: impl Into<String>,
        expires_in: Duration,
        max_attempts: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            identifier: identifier.into(),
            code: code.into(),
            verification_type: verification_type.into(),
            expires_at: now + expires_in,
            attempts: 0,
            max_attempts,
            used: false,
            created_at: now,
        }
    }

    /// Checks if the code has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Checks if max attempts have been exceeded.
    pub fn is_max_attempts_exceeded(&self) -> bool {
        self.attempts >= self.max_attempts
    }

    /// Checks if the code is still valid (not expired, not used, attempts not exceeded).
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.used && !self.is_max_attempts_exceeded()
    }

    /// Increments the attempt counter.
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }

    /// Marks the code as used.
    pub fn mark_used(&mut self) {
        self.used = true;
    }

    /// Verifies the provided code against this verification code.
    pub fn verify(&mut self, provided_code: &str) -> VerificationResult {
        if self.used {
            return VerificationResult::AlreadyUsed;
        }

        if self.is_expired() {
            return VerificationResult::Expired;
        }

        self.increment_attempts();

        if self.is_max_attempts_exceeded() {
            return VerificationResult::TooManyAttempts;
        }

        if self.code == provided_code {
            self.mark_used();
            VerificationResult::Valid
        } else {
            VerificationResult::Invalid
        }
    }
}

/// Common verification types used across plugins.
pub mod verification_types {
    pub const SIGN_IN: &str = "sign-in";
    pub const EMAIL_VERIFICATION: &str = "email-verification";
    pub const PASSWORD_RESET: &str = "forget-password";
    pub const PHONE_VERIFICATION: &str = "phone-verification";
    pub const TWO_FACTOR: &str = "two-factor";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_code_creation() {
        let code = VerificationCode::new(
            "test@example.com",
            "123456",
            "sign-in",
            Duration::minutes(5),
            3,
        );

        assert!(!code.is_expired());
        assert!(!code.used);
        assert_eq!(code.attempts, 0);
        assert!(code.is_valid());
    }

    #[test]
    fn test_verification_code_verify() {
        let mut code = VerificationCode::new(
            "test@example.com",
            "123456",
            "sign-in",
            Duration::minutes(5),
            3,
        );

        // Wrong code
        assert_eq!(code.verify("000000"), VerificationResult::Invalid);
        assert_eq!(code.attempts, 1);

        // Correct code
        assert_eq!(code.verify("123456"), VerificationResult::Valid);
        assert!(code.used);

        // Already used
        assert_eq!(code.verify("123456"), VerificationResult::AlreadyUsed);
    }

    #[test]
    fn test_verification_code_max_attempts() {
        let mut code = VerificationCode::new(
            "test@example.com",
            "123456",
            "sign-in",
            Duration::minutes(5),
            2,
        );

        assert_eq!(code.verify("000000"), VerificationResult::Invalid);
        assert_eq!(code.verify("000000"), VerificationResult::TooManyAttempts);
        // Even correct code should fail after max attempts
        assert_eq!(code.verify("123456"), VerificationResult::TooManyAttempts);
    }
}
