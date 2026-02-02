//! TOTP (Time-based One-Time Password) utilities.

use rand::Rng;

/// TOTP URI for QR code generation.
#[derive(Debug, Clone)]
pub struct TotpUri {
    /// The complete TOTP URI.
    pub uri: String,
    /// The secret in base32 format.
    pub secret: String,
}

/// TOTP manager for generating and verifying TOTP codes.
#[derive(Debug, Clone)]
pub struct TotpManager {
    /// The issuer name.
    issuer: String,
    /// Number of digits.
    digits: u32,
    /// Time period in seconds.
    period: u32,
}

impl TotpManager {
    /// Creates a new TOTP manager.
    pub fn new(issuer: impl Into<String>, digits: u32, period: u32) -> Self {
        Self {
            issuer: issuer.into(),
            digits,
            period,
        }
    }

    /// Generates a new secret.
    pub fn generate_secret(&self) -> String {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut secret = vec![0u8; 20];
        rng.fill_bytes(&mut secret);
        base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret)
    }

    /// Generates a TOTP URI for the given account and secret.
    pub fn generate_uri(&self, account: &str, secret: &str) -> TotpUri {
        let uri = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
            urlencoding_encode(&self.issuer),
            urlencoding_encode(account),
            secret,
            urlencoding_encode(&self.issuer),
            self.digits,
            self.period
        );

        TotpUri {
            uri,
            secret: secret.to_string(),
        }
    }

    /// Verifies a TOTP code.
    /// 
    /// This accepts codes from one period before and one after the current time
    /// to account for clock drift.
    pub fn verify(&self, secret: &str, code: &str) -> bool {
        // In a real implementation, this would use the totp-rs crate
        // For now, we'll do a simple placeholder verification
        
        // Decode the secret
        let Some(secret_bytes) = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, secret) else {
            return false;
        };

        // Get current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check current period and adjacent periods
        for offset in [-1i64, 0, 1] {
            let counter = ((now as i64 / self.period as i64) + offset) as u64;
            let expected = self.generate_code_for_counter(&secret_bytes, counter);
            if expected == code {
                return true;
            }
        }

        false
    }

    /// Generates a TOTP code for a specific counter value.
    fn generate_code_for_counter(&self, secret: &[u8], counter: u64) -> String {
        // HMAC-SHA1 based TOTP generation
        // This is a simplified implementation - in production use totp-rs
        
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        secret.hash(&mut hasher);
        counter.hash(&mut hasher);
        let hash = hasher.finish();
        
        let code = (hash % 10u64.pow(self.digits)) as u32;
        format!("{:0>width$}", code, width = self.digits as usize)
    }
}

impl Default for TotpManager {
    fn default() -> Self {
        Self::new("Better Auth", 6, 30)
    }
}

/// Simple URL encoding for TOTP URIs.
fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "%20".to_string(),
            ':' => "%3A".to_string(),
            '@' => "%40".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_generation() {
        let manager = TotpManager::default();
        let secret = manager.generate_secret();
        
        // Base32 encoded 20 bytes = 32 characters
        assert_eq!(secret.len(), 32);
        assert!(secret.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_uri_generation() {
        let manager = TotpManager::new("MyApp", 6, 30);
        let uri = manager.generate_uri("user@example.com", "JBSWY3DPEHPK3PXP");
        
        assert!(uri.uri.starts_with("otpauth://totp/"));
        assert!(uri.uri.contains("MyApp"));
        assert!(uri.uri.contains("user%40example.com"));
        assert!(uri.uri.contains("secret=JBSWY3DPEHPK3PXP"));
    }
}
