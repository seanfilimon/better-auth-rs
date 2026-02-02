//! OTP generation utilities.

use rand::Rng;

/// Configuration for OTP generation.
#[derive(Debug, Clone)]
pub struct OtpConfig {
    /// Length of the OTP code.
    pub length: usize,
    /// Type of OTP to generate.
    pub otp_type: OtpType,
}

impl Default for OtpConfig {
    fn default() -> Self {
        Self {
            length: 6,
            otp_type: OtpType::Numeric,
        }
    }
}

impl OtpConfig {
    /// Creates a new OTP config with the specified length.
    pub fn new(length: usize) -> Self {
        Self {
            length,
            ..Default::default()
        }
    }

    /// Sets the OTP type.
    pub fn with_type(mut self, otp_type: OtpType) -> Self {
        self.otp_type = otp_type;
        self
    }

    /// Creates a numeric OTP config.
    pub fn numeric(length: usize) -> Self {
        Self {
            length,
            otp_type: OtpType::Numeric,
        }
    }

    /// Creates an alphanumeric OTP config.
    pub fn alphanumeric(length: usize) -> Self {
        Self {
            length,
            otp_type: OtpType::Alphanumeric,
        }
    }

    /// Creates an alphabetic OTP config.
    pub fn alphabetic(length: usize) -> Self {
        Self {
            length,
            otp_type: OtpType::Alphabetic,
        }
    }
}

/// Type of OTP to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtpType {
    /// Numeric only (0-9).
    Numeric,
    /// Alphanumeric (a-z, A-Z, 0-9).
    Alphanumeric,
    /// Alphabetic only (a-z, A-Z).
    Alphabetic,
    /// Alphanumeric without ambiguous characters (0, O, l, 1, I).
    AlphanumericUnambiguous,
}

impl OtpType {
    /// Returns the character set for this OTP type.
    pub fn charset(&self) -> &'static [u8] {
        match self {
            OtpType::Numeric => b"0123456789",
            OtpType::Alphanumeric => b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            OtpType::Alphabetic => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            OtpType::AlphanumericUnambiguous => b"23456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz",
        }
    }
}

/// OTP generator.
#[derive(Debug, Clone)]
pub struct OtpGenerator {
    config: OtpConfig,
}

impl OtpGenerator {
    /// Creates a new OTP generator with the given config.
    pub fn new(config: OtpConfig) -> Self {
        Self { config }
    }

    /// Creates a new OTP generator with default config (6-digit numeric).
    pub fn default_numeric() -> Self {
        Self::new(OtpConfig::numeric(6))
    }

    /// Creates a new OTP generator for alphanumeric codes.
    pub fn default_alphanumeric() -> Self {
        Self::new(OtpConfig::alphanumeric(32))
    }

    /// Generates a new OTP code.
    pub fn generate(&self) -> String {
        let mut rng = rand::thread_rng();
        let charset = self.config.otp_type.charset();
        
        (0..self.config.length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect()
    }

    /// Generates a cryptographically secure token (for magic links, etc.).
    pub fn generate_secure_token(length: usize) -> String {
        let mut rng = rand::thread_rng();
        let charset = OtpType::AlphanumericUnambiguous.charset();
        
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect()
    }

    /// Generates a UUID-based token.
    pub fn generate_uuid_token() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

impl Default for OtpGenerator {
    fn default() -> Self {
        Self::default_numeric()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_otp() {
        let generator = OtpGenerator::new(OtpConfig::numeric(6));
        let otp = generator.generate();
        
        assert_eq!(otp.len(), 6);
        assert!(otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_alphanumeric_otp() {
        let generator = OtpGenerator::new(OtpConfig::alphanumeric(10));
        let otp = generator.generate();
        
        assert_eq!(otp.len(), 10);
        assert!(otp.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_secure_token() {
        let token = OtpGenerator::generate_secure_token(32);
        
        assert_eq!(token.len(), 32);
        // Should not contain ambiguous characters
        assert!(!token.contains('0'));
        assert!(!token.contains('O'));
        assert!(!token.contains('l'));
        assert!(!token.contains('1'));
        assert!(!token.contains('I'));
    }

    #[test]
    fn test_uniqueness() {
        let generator = OtpGenerator::default_numeric();
        let codes: Vec<String> = (0..100).map(|_| generator.generate()).collect();
        
        // Check that we get some variety (not all the same)
        let unique_count = codes.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(unique_count > 90); // Should be mostly unique
    }
}
