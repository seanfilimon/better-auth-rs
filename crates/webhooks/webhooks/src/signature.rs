//! HMAC signature generation and verification.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Signature version for webhook payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureVersion {
    /// Version 1: HMAC-SHA256
    V1,
}

impl Default for SignatureVersion {
    fn default() -> Self {
        SignatureVersion::V1
    }
}

/// Webhook signer for generating and verifying signatures.
pub struct WebhookSigner {
    secret: String,
    version: SignatureVersion,
}

impl WebhookSigner {
    /// Creates a new signer with the given secret.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            version: SignatureVersion::V1,
        }
    }

    /// Creates a signer with a specific version.
    pub fn with_version(secret: impl Into<String>, version: SignatureVersion) -> Self {
        Self {
            secret: secret.into(),
            version,
        }
    }

    /// Generates a signature for the given payload and timestamp.
    pub fn sign(&self, timestamp: i64, payload: &[u8]) -> String {
        match self.version {
            SignatureVersion::V1 => self.sign_v1(timestamp, payload),
        }
    }

    /// Generates a full signature header value.
    pub fn sign_header(&self, timestamp: i64, payload: &[u8]) -> String {
        let signature = self.sign(timestamp, payload);
        format!("t={},v1={}", timestamp, signature)
    }

    /// Verifies a signature against the payload.
    pub fn verify(&self, signature: &str, timestamp: i64, payload: &[u8]) -> bool {
        let expected = self.sign(timestamp, payload);
        constant_time_compare(&expected, signature)
    }

    /// Parses and verifies a signature header.
    pub fn verify_header(
        &self,
        header: &str,
        payload: &[u8],
        tolerance_secs: i64,
    ) -> Result<(), SignatureError> {
        let parts = parse_signature_header(header)?;

        let timestamp = parts
            .get("t")
            .and_then(|t| t.parse::<i64>().ok())
            .ok_or(SignatureError::InvalidFormat)?;

        // Check timestamp tolerance
        let now = chrono::Utc::now().timestamp();
        if (now - timestamp).abs() > tolerance_secs {
            return Err(SignatureError::Expired);
        }

        // Verify signature
        let signature = parts.get("v1").ok_or(SignatureError::InvalidFormat)?;
        if !self.verify(signature, timestamp, payload) {
            return Err(SignatureError::Invalid);
        }

        Ok(())
    }

    fn sign_v1(&self, timestamp: i64, payload: &[u8]) -> String {
        let mut mac =
            HmacSha256::new_from_slice(self.secret.as_bytes()).expect("HMAC can take key of any size");

        // Sign: timestamp.payload
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);

        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }
}

/// Signature verification errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    /// Invalid signature format.
    InvalidFormat,
    /// Signature is invalid.
    Invalid,
    /// Signature has expired.
    Expired,
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureError::InvalidFormat => write!(f, "Invalid signature format"),
            SignatureError::Invalid => write!(f, "Invalid signature"),
            SignatureError::Expired => write!(f, "Signature expired"),
        }
    }
}

impl std::error::Error for SignatureError {}

/// Parses a signature header into its components.
fn parse_signature_header(header: &str) -> Result<std::collections::HashMap<&str, &str>, SignatureError> {
    let mut parts = std::collections::HashMap::new();

    for part in header.split(',') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().ok_or(SignatureError::InvalidFormat)?;
        let value = kv.next().ok_or(SignatureError::InvalidFormat)?;
        parts.insert(key, value);
    }

    Ok(parts)
}

/// Constant-time string comparison to prevent timing attacks.
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let signer = WebhookSigner::new("test-secret");
        let payload = b"test payload";
        let timestamp = 1234567890;

        let signature = signer.sign(timestamp, payload);
        assert!(signer.verify(&signature, timestamp, payload));

        // Wrong payload should fail
        assert!(!signer.verify(&signature, timestamp, b"wrong payload"));

        // Wrong timestamp should fail
        assert!(!signer.verify(&signature, timestamp + 1, payload));
    }

    #[test]
    fn test_sign_header() {
        let signer = WebhookSigner::new("test-secret");
        let payload = b"test payload";
        let timestamp = 1234567890;

        let header = signer.sign_header(timestamp, payload);
        assert!(header.starts_with("t=1234567890,v1="));
    }

    #[test]
    fn test_verify_header() {
        let signer = WebhookSigner::new("test-secret");
        let payload = b"test payload";
        let timestamp = chrono::Utc::now().timestamp();

        let header = signer.sign_header(timestamp, payload);
        let result = signer.verify_header(&header, payload, 300);
        assert!(result.is_ok());
    }

    #[test]
    fn test_expired_signature() {
        let signer = WebhookSigner::new("test-secret");
        let payload = b"test payload";
        let old_timestamp = chrono::Utc::now().timestamp() - 600; // 10 minutes ago

        let header = signer.sign_header(old_timestamp, payload);
        let result = signer.verify_header(&header, payload, 300); // 5 minute tolerance
        assert_eq!(result, Err(SignatureError::Expired));
    }
}
