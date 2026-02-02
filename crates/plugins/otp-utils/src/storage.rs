//! Token storage utilities.

use serde::{Deserialize, Serialize};

/// How tokens should be stored.
#[derive(Debug, Clone, Default)]
pub enum TokenStorageMode {
    /// Store tokens in plain text.
    #[default]
    Plain,
    /// Hash tokens before storage.
    Hashed,
    /// Encrypt tokens before storage.
    Encrypted,
    /// Custom storage with user-provided functions.
    Custom,
}

/// Configuration for token storage.
#[derive(Debug, Clone)]
pub struct TokenStorage {
    /// Storage mode.
    pub mode: TokenStorageMode,
    /// Custom hash function (for Custom mode).
    hash_fn: Option<fn(&str) -> String>,
    /// Custom verify function (for Custom mode).
    verify_fn: Option<fn(&str, &str) -> bool>,
}

impl Default for TokenStorage {
    fn default() -> Self {
        Self {
            mode: TokenStorageMode::Plain,
            hash_fn: None,
            verify_fn: None,
        }
    }
}

impl TokenStorage {
    /// Creates a plain text storage.
    pub fn plain() -> Self {
        Self {
            mode: TokenStorageMode::Plain,
            ..Default::default()
        }
    }

    /// Creates a hashed storage.
    pub fn hashed() -> Self {
        Self {
            mode: TokenStorageMode::Hashed,
            ..Default::default()
        }
    }

    /// Creates a custom storage with provided functions.
    pub fn custom(hash_fn: fn(&str) -> String, verify_fn: fn(&str, &str) -> bool) -> Self {
        Self {
            mode: TokenStorageMode::Custom,
            hash_fn: Some(hash_fn),
            verify_fn: Some(verify_fn),
        }
    }

    /// Prepares a token for storage.
    pub fn prepare_for_storage(&self, token: &str) -> String {
        match &self.mode {
            TokenStorageMode::Plain => token.to_string(),
            TokenStorageMode::Hashed => self.default_hash(token),
            TokenStorageMode::Encrypted => {
                // For now, just use hashing as a placeholder
                // In production, this would use actual encryption
                self.default_hash(token)
            }
            TokenStorageMode::Custom => {
                if let Some(hash_fn) = self.hash_fn {
                    hash_fn(token)
                } else {
                    token.to_string()
                }
            }
        }
    }

    /// Verifies a token against a stored value.
    pub fn verify(&self, provided: &str, stored: &str) -> bool {
        match &self.mode {
            TokenStorageMode::Plain => provided == stored,
            TokenStorageMode::Hashed => self.default_hash(provided) == stored,
            TokenStorageMode::Encrypted => self.default_hash(provided) == stored,
            TokenStorageMode::Custom => {
                if let Some(verify_fn) = self.verify_fn {
                    verify_fn(provided, stored)
                } else {
                    provided == stored
                }
            }
        }
    }

    /// Default hash function (placeholder - use proper hashing in production).
    fn default_hash(&self, value: &str) -> String {
        // PLACEHOLDER: In production, use a proper hashing algorithm like SHA-256 or Argon2
        // This is NOT secure and is only for demonstration
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Represents a stored token with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    /// The token value (may be hashed).
    pub value: String,
    /// Whether the token is hashed.
    pub is_hashed: bool,
    /// Optional prefix for display (e.g., first 4 characters).
    pub prefix: Option<String>,
}

impl StoredToken {
    /// Creates a new stored token.
    pub fn new(value: impl Into<String>, is_hashed: bool) -> Self {
        Self {
            value: value.into(),
            is_hashed,
            prefix: None,
        }
    }

    /// Creates a stored token with a prefix for display.
    pub fn with_prefix(value: impl Into<String>, is_hashed: bool, prefix: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            is_hashed,
            prefix: Some(prefix.into()),
        }
    }

    /// Extracts a prefix from a token for display purposes.
    pub fn extract_prefix(token: &str, length: usize) -> String {
        token.chars().take(length).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_storage() {
        let storage = TokenStorage::plain();
        let token = "my-secret-token";
        
        let stored = storage.prepare_for_storage(token);
        assert_eq!(stored, token);
        assert!(storage.verify(token, &stored));
    }

    #[test]
    fn test_hashed_storage() {
        let storage = TokenStorage::hashed();
        let token = "my-secret-token";
        
        let stored = storage.prepare_for_storage(token);
        assert_ne!(stored, token); // Should be hashed
        assert!(storage.verify(token, &stored));
        assert!(!storage.verify("wrong-token", &stored));
    }

    #[test]
    fn test_custom_storage() {
        fn custom_hash(s: &str) -> String {
            format!("custom:{}", s)
        }
        
        fn custom_verify(provided: &str, stored: &str) -> bool {
            stored == format!("custom:{}", provided)
        }
        
        let storage = TokenStorage::custom(custom_hash, custom_verify);
        let token = "my-token";
        
        let stored = storage.prepare_for_storage(token);
        assert_eq!(stored, "custom:my-token");
        assert!(storage.verify(token, &stored));
    }

    #[test]
    fn test_stored_token_prefix() {
        let prefix = StoredToken::extract_prefix("sk_live_abc123xyz", 7);
        assert_eq!(prefix, "sk_live");
    }
}
