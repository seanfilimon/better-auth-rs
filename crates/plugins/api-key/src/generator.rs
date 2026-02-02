//! API key generation utilities.

use rand::Rng;

/// API key generator.
#[derive(Debug, Clone)]
pub struct ApiKeyGenerator {
    /// Key length (not including prefix).
    length: usize,
    /// Optional prefix.
    prefix: Option<String>,
}

impl ApiKeyGenerator {
    /// Creates a new API key generator.
    pub fn new(length: usize, prefix: Option<String>) -> Self {
        Self { length, prefix }
    }

    /// Generates a new API key.
    pub fn generate(&self) -> String {
        let mut rng = rand::thread_rng();
        // Use alphanumeric without ambiguous characters
        let charset: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
        
        let key: String = (0..self.length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        if let Some(ref prefix) = self.prefix {
            format!("{}{}", prefix, key)
        } else {
            key
        }
    }

    /// Generates a key with a custom prefix.
    pub fn generate_with_prefix(&self, prefix: &str) -> String {
        let mut rng = rand::thread_rng();
        let charset: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
        
        let key: String = (0..self.length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        format!("{}{}", prefix, key)
    }

    /// Extracts the starting characters from a key.
    pub fn extract_start(key: &str, length: usize) -> String {
        key.chars().take(length).collect()
    }

    /// Hashes an API key for storage.
    /// 
    /// Note: In production, use a proper hashing algorithm like SHA-256 or Argon2.
    pub fn hash_key(key: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Verifies a key against a hash.
    pub fn verify_key(key: &str, hash: &str) -> bool {
        Self::hash_key(key) == hash
    }
}

impl Default for ApiKeyGenerator {
    fn default() -> Self {
        Self::new(64, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let generator = ApiKeyGenerator::new(32, None);
        let key = generator.generate();
        
        assert_eq!(key.len(), 32);
        assert!(key.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_key_with_prefix() {
        let generator = ApiKeyGenerator::new(32, Some("sk_live_".to_string()));
        let key = generator.generate();
        
        assert!(key.starts_with("sk_live_"));
        assert_eq!(key.len(), 32 + 8); // key + prefix
    }

    #[test]
    fn test_extract_start() {
        let start = ApiKeyGenerator::extract_start("sk_live_abc123xyz", 8);
        assert_eq!(start, "sk_live_");
    }

    #[test]
    fn test_key_hashing() {
        let key = "my_secret_key";
        let hash = ApiKeyGenerator::hash_key(key);
        
        assert!(ApiKeyGenerator::verify_key(key, &hash));
        assert!(!ApiKeyGenerator::verify_key("wrong_key", &hash));
    }

    #[test]
    fn test_uniqueness() {
        let generator = ApiKeyGenerator::default();
        let keys: Vec<String> = (0..100).map(|_| generator.generate()).collect();
        
        let unique: std::collections::HashSet<_> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len());
    }
}
