//! Backup code management.

use rand::Rng;

/// Backup code manager.
#[derive(Debug, Clone)]
pub struct BackupCodeManager {
    /// Number of codes to generate.
    amount: usize,
    /// Length of each code.
    length: usize,
}

impl BackupCodeManager {
    /// Creates a new backup code manager.
    pub fn new(amount: usize, length: usize) -> Self {
        Self { amount, length }
    }

    /// Generates a set of backup codes.
    pub fn generate(&self) -> Vec<String> {
        let mut rng = rand::thread_rng();
        let charset: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // No ambiguous chars
        
        (0..self.amount)
            .map(|_| {
                (0..self.length)
                    .map(|_| {
                        let idx = rng.gen_range(0..charset.len());
                        charset[idx] as char
                    })
                    .collect()
            })
            .collect()
    }

    /// Verifies a backup code against a list of valid codes.
    /// Returns the index of the matched code if found.
    pub fn verify(&self, code: &str, valid_codes: &[String]) -> Option<usize> {
        let normalized = code.to_uppercase().replace(['-', ' '], "");
        valid_codes.iter().position(|c| c == &normalized)
    }

    /// Formats a backup code for display (e.g., "ABCD-EFGH-IJKL").
    pub fn format_for_display(code: &str) -> String {
        code.chars()
            .collect::<Vec<_>>()
            .chunks(4)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("-")
    }
}

impl Default for BackupCodeManager {
    fn default() -> Self {
        Self::new(10, 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_code_generation() {
        let manager = BackupCodeManager::new(10, 10);
        let codes = manager.generate();
        
        assert_eq!(codes.len(), 10);
        for code in &codes {
            assert_eq!(code.len(), 10);
            assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
        }
    }

    #[test]
    fn test_backup_code_verification() {
        let manager = BackupCodeManager::default();
        let codes = vec!["ABCDEFGHIJ".to_string(), "KLMNOPQRST".to_string()];
        
        assert_eq!(manager.verify("ABCDEFGHIJ", &codes), Some(0));
        assert_eq!(manager.verify("abcdefghij", &codes), Some(0)); // Case insensitive
        assert_eq!(manager.verify("ABCD-EFGH-IJ", &codes), Some(0)); // With dashes
        assert_eq!(manager.verify("INVALID", &codes), None);
    }

    #[test]
    fn test_format_for_display() {
        let formatted = BackupCodeManager::format_for_display("ABCDEFGHIJ");
        assert_eq!(formatted, "ABCD-EFGH-IJ");
    }

    #[test]
    fn test_uniqueness() {
        let manager = BackupCodeManager::new(100, 10);
        let codes = manager.generate();
        
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len()); // All codes should be unique
    }
}
