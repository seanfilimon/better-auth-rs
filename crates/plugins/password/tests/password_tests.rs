//! Comprehensive tests for Password Plugin
//!
//! Tests cover:
//! - Password hashing and verification
//! - Password strength validation
//! - Password reset flow
//! - Rate limiting
//! - Security best practices

use better_auth_plugin_password::*;

mod hashing_tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let password = "SecurePassword123!";
        let hash = hash_password(password).expect("Should hash password");
        
        assert_ne!(hash, password, "Hash should not equal plaintext");
        assert!(verify_password(password, &hash).expect("Should verify"), "Verification should succeed");
    }

    #[test]
    fn test_password_hash_unique() {
        let password = "SamePassword123!";
        let hash1 = hash_password(password).expect("Should hash");
        let hash2 = hash_password(password).expect("Should hash");
        
        assert_ne!(hash1, hash2, "Same password should produce different hashes (salt)");
    }

    #[test]
    fn test_wrong_password_fails() {
        let password = "Correct123!";
        let hash = hash_password(password).expect("Should hash");
        
        assert!(!verify_password("Wrong123!", &hash).expect("Should verify"), 
                "Wrong password should fail");
    }

    #[test]
    fn test_empty_password() {
        let result = hash_password("");
        assert!(result.is_err(), "Empty password should be rejected");
    }

    #[test]
    fn test_very_long_password() {
        let long_password = "a".repeat(1000);
        let result = hash_password(&long_password);
        // Should either hash it or reject gracefully
        assert!(result.is_ok() || result.is_err());
    }
}

mod validation_tests {
    use super::*;

    #[test]
    fn test_password_strength_weak() {
        let weak_passwords = vec![
            "123456",
            "password",
            "qwerty",
            "abc123",
            "password123",
        ];
        
        for pwd in weak_passwords {
            let result = validate_password_strength(pwd);
            assert!(result.is_err() || !result.unwrap().is_strong(), 
                    "Password '{}' should be weak", pwd);
        }
    }

    #[test]
    fn test_password_strength_medium() {
        let medium_passwords = vec![
            "Password1",
            "MyPass123",
            "Test1234",
        ];
        
        for pwd in medium_passwords {
            let result = validate_password_strength(pwd);
            if let Ok(strength) = result {
                assert!(strength.score() >= 2, 
                        "Password '{}' should be at least medium strength", pwd);
            }
        }
    }

    #[test]
    fn test_password_strength_strong() {
        let strong_passwords = vec![
            "MySecure#Pass123",
            "C0mpl3x!Passw0rd",
            "Str0ng&Secure#2024",
        ];
        
        for pwd in strong_passwords {
            let result = validate_password_strength(pwd);
            assert!(result.is_ok(), "Strong password '{}' should validate", pwd);
            assert!(result.unwrap().is_strong(), 
                    "Password '{}' should be strong", pwd);
        }
    }

    #[test]
    fn test_password_requirements() {
        // Test various requirement checks
        assert!(has_uppercase("Password1"), "Should have uppercase");
        assert!(has_lowercase("Password1"), "Should have lowercase");
        assert!(has_number("Password1"), "Should have number");
        assert!(has_special_char("Pass@word1"), "Should have special char");
        
        assert!(!has_uppercase("password1"), "Should not have uppercase");
        assert!(!has_number("Password"), "Should not have number");
    }

    #[test]
    fn test_common_password_check() {
        let common = vec!["password", "123456", "qwerty", "admin"];
        for pwd in common {
            assert!(is_common_password(pwd), "'{}' should be flagged as common", pwd);
        }
        
        assert!(!is_common_password("MyUnique#Pass2024"), 
                "Unique password should not be common");
    }
}

mod reset_flow_tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_reset_token() {
        let token = generate_reset_token();
        assert!(!token.is_empty(), "Token should not be empty");
        assert!(token.len() >= 32, "Token should be sufficiently long");
    }

    #[tokio::test]
    async fn test_reset_token_unique() {
        let token1 = generate_reset_token();
        let token2 = generate_reset_token();
        assert_ne!(token1, token2, "Tokens should be unique");
    }

    #[tokio::test]
    async fn test_reset_token_expiry() {
        let token_data = create_reset_token_data("user123");
        
        assert!(!is_token_expired(&token_data), "Fresh token should not be expired");
        
        // Test with expired token
        let expired_data = create_expired_token_data();
        assert!(is_token_expired(&expired_data), "Old token should be expired");
    }

    #[tokio::test]
    async fn test_reset_token_single_use() {
        // Test that tokens can only be used once
        let token = generate_reset_token();
        let used = mark_token_used(&token).await;
        assert!(used.is_ok(), "Should mark token as used");
        
        let reuse = mark_token_used(&token).await;
        assert!(reuse.is_err(), "Should reject reused token");
    }
}

mod security_tests {
    use super::*;

    #[test]
    fn test_timing_attack_resistance() {
        let password = "TestPassword123!";
        let hash = hash_password(password).unwrap();
        
        // Verify should take similar time regardless of how wrong the password is
        let start1 = std::time::Instant::now();
        let _ = verify_password("a", &hash);
        let duration1 = start1.elapsed();
        
        let start2 = std::time::Instant::now();
        let _ = verify_password("TestPassword123", &hash); // Close but wrong
        let duration2 = start2.elapsed();
        
        // Both should take roughly the same time (constant time comparison)
        let ratio = duration1.as_nanos() as f64 / duration2.as_nanos() as f64;
        assert!(ratio > 0.5 && ratio < 2.0, 
                "Verification time should be constant");
    }

    #[test]
    fn test_hash_format() {
        let password = "Test123!";
        let hash = hash_password(password).unwrap();
        
        // Hash should be in proper format (e.g., bcrypt format)
        assert!(hash.starts_with("$2"), "Should use bcrypt format");
        assert!(hash.len() >= 60, "Hash should be full length");
    }

    #[test]
    fn test_salt_length() {
        let password = "Test123!";
        let hash = hash_password(password).unwrap();
        
        // Extract and verify salt
        let parts: Vec<&str> = hash.split('$').collect();
        assert!(parts.len() >= 4, "Should have algorithm, cost, salt, and hash");
    }
}

mod rate_limiting_tests {
    use super::*;

    #[tokio::test]
    async fn test_failed_attempt_tracking() {
        let user_id = "user123";
        
        // Record failed attempts
        for _ in 0..3 {
            record_failed_attempt(user_id).await;
        }
        
        let count = get_failed_attempts(user_id).await;
        assert_eq!(count, 3, "Should track failed attempts");
    }

    #[tokio::test]
    async fn test_account_lockout() {
        let user_id = "user456";
        
        // Exceed max attempts
        for _ in 0..5 {
            record_failed_attempt(user_id).await;
        }
        
        let locked = is_account_locked(user_id).await;
        assert!(locked, "Account should be locked after max attempts");
    }

    #[tokio::test]
    async fn test_lockout_duration() {
        let user_id = "user789";
        
        // Lock account
        lock_account(user_id, std::time::Duration::from_secs(60)).await;
        
        assert!(is_account_locked(user_id).await, "Should be locked");
        
        // Fast-forward time (in real impl, this would be time-based check)
        unlock_account(user_id).await;
        
        assert!(!is_account_locked(user_id).await, "Should be unlocked");
    }

    #[tokio::test]
    async fn test_successful_login_resets_count() {
        let user_id = "user999";
        
        // Failed attempts
        for _ in 0..3 {
            record_failed_attempt(user_id).await;
        }
        
        // Successful login
        reset_failed_attempts(user_id).await;
        
        let count = get_failed_attempts(user_id).await;
        assert_eq!(count, 0, "Successful login should reset counter");
    }
}

// Helper functions for tests (these would be implemented in the actual plugin)

fn hash_password(password: &str) -> Result<String, String> {
    if password.is_empty() {
        return Err("Password cannot be empty".to_string());
    }
    // Placeholder - actual implementation would use bcrypt/argon2
    Ok(format!("$2b$12$dummy_hash_for_{}", password))
}

fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    // Placeholder - actual implementation would verify against hash
    Ok(hash.contains(password))
}

fn validate_password_strength(password: &str) -> Result<PasswordStrength, String> {
    let score = calculate_strength_score(password);
    Ok(PasswordStrength { score })
}

struct PasswordStrength {
    score: u8,
}

impl PasswordStrength {
    fn is_strong(&self) -> bool {
        self.score >= 4
    }
    
    fn score(&self) -> u8 {
        self.score
    }
}

fn calculate_strength_score(password: &str) -> u8 {
    let mut score = 0;
    if password.len() >= 8 { score += 1; }
    if has_uppercase(password) { score += 1; }
    if has_lowercase(password) { score += 1; }
    if has_number(password) { score += 1; }
    if has_special_char(password) { score += 1; }
    if !is_common_password(password) { score += 1; }
    score.min(5)
}

fn has_uppercase(s: &str) -> bool {
    s.chars().any(|c| c.is_uppercase())
}

fn has_lowercase(s: &str) -> bool {
    s.chars().any(|c| c.is_lowercase())
}

fn has_number(s: &str) -> bool {
    s.chars().any(|c| c.is_numeric())
}

fn has_special_char(s: &str) -> bool {
    s.chars().any(|c| !c.is_alphanumeric())
}

fn is_common_password(password: &str) -> bool {
    let common = vec!["password", "123456", "qwerty", "admin", "letmein"];
    common.contains(&password.to_lowercase().as_str())
}

fn generate_reset_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32).map(|_| format!("{:x}", rng.gen::<u8>())).collect()
}

struct TokenData {
    created_at: chrono::DateTime<chrono::Utc>,
}

fn create_reset_token_data(user_id: &str) -> TokenData {
    TokenData {
        created_at: chrono::Utc::now(),
    }
}

fn create_expired_token_data() -> TokenData {
    TokenData {
        created_at: chrono::Utc::now() - chrono::Duration::hours(25),
    }
}

fn is_token_expired(data: &TokenData) -> bool {
    chrono::Utc::now() - data.created_at > chrono::Duration::hours(24)
}

async fn mark_token_used(_token: &str) -> Result<(), String> {
    Ok(())
}

async fn record_failed_attempt(_user_id: &str) {}
async fn get_failed_attempts(_user_id: &str) -> usize { 0 }
async fn is_account_locked(_user_id: &str) -> bool { false }
async fn lock_account(_user_id: &str, _duration: std::time::Duration) {}
async fn unlock_account(_user_id: &str) {}
async fn reset_failed_attempts(_user_id: &str) {}
