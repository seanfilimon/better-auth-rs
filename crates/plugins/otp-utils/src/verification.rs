//! Verification utilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Result of a verification attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// The code is valid.
    Valid,
    /// The code is invalid (wrong code).
    Invalid,
    /// The code has expired.
    Expired,
    /// The code has already been used.
    AlreadyUsed,
    /// Too many verification attempts.
    TooManyAttempts,
    /// No verification code found.
    NotFound,
}

impl VerificationResult {
    /// Returns true if verification was successful.
    pub fn is_valid(&self) -> bool {
        matches!(self, VerificationResult::Valid)
    }

    /// Returns an error message for failed verifications.
    pub fn error_message(&self) -> Option<&'static str> {
        match self {
            VerificationResult::Valid => None,
            VerificationResult::Invalid => Some("Invalid verification code"),
            VerificationResult::Expired => Some("Verification code has expired"),
            VerificationResult::AlreadyUsed => Some("Verification code has already been used"),
            VerificationResult::TooManyAttempts => Some("Too many verification attempts"),
            VerificationResult::NotFound => Some("No verification code found"),
        }
    }

    /// Returns an error code for API responses.
    pub fn error_code(&self) -> Option<&'static str> {
        match self {
            VerificationResult::Valid => None,
            VerificationResult::Invalid => Some("INVALID_CODE"),
            VerificationResult::Expired => Some("CODE_EXPIRED"),
            VerificationResult::AlreadyUsed => Some("CODE_ALREADY_USED"),
            VerificationResult::TooManyAttempts => Some("TOO_MANY_ATTEMPTS"),
            VerificationResult::NotFound => Some("CODE_NOT_FOUND"),
        }
    }
}

/// Error type for verification operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum VerificationError {
    #[error("Invalid verification code")]
    InvalidCode,
    
    #[error("Verification code has expired")]
    Expired,
    
    #[error("Verification code has already been used")]
    AlreadyUsed,
    
    #[error("Too many verification attempts")]
    TooManyAttempts,
    
    #[error("No verification code found for this identifier")]
    NotFound,
    
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl From<VerificationResult> for Result<(), VerificationError> {
    fn from(result: VerificationResult) -> Self {
        match result {
            VerificationResult::Valid => Ok(()),
            VerificationResult::Invalid => Err(VerificationError::InvalidCode),
            VerificationResult::Expired => Err(VerificationError::Expired),
            VerificationResult::AlreadyUsed => Err(VerificationError::AlreadyUsed),
            VerificationResult::TooManyAttempts => Err(VerificationError::TooManyAttempts),
            VerificationResult::NotFound => Err(VerificationError::NotFound),
        }
    }
}

/// Tracks verification attempts for an identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// Number of attempts made.
    pub attempts: u32,
    /// Maximum allowed attempts.
    pub max_attempts: u32,
    /// When the first attempt was made.
    pub first_attempt: DateTime<Utc>,
    /// When the last attempt was made.
    pub last_attempt: DateTime<Utc>,
}

impl AttemptRecord {
    /// Creates a new attempt record.
    pub fn new(max_attempts: u32) -> Self {
        let now = Utc::now();
        Self {
            attempts: 1,
            max_attempts,
            first_attempt: now,
            last_attempt: now,
        }
    }

    /// Increments the attempt counter.
    pub fn increment(&mut self) {
        self.attempts += 1;
        self.last_attempt = Utc::now();
    }

    /// Checks if max attempts have been exceeded.
    pub fn is_exceeded(&self) -> bool {
        self.attempts >= self.max_attempts
    }

    /// Returns remaining attempts.
    pub fn remaining(&self) -> u32 {
        self.max_attempts.saturating_sub(self.attempts)
    }
}

/// In-memory attempt tracker.
#[derive(Debug, Default)]
pub struct AttemptTracker {
    records: HashMap<String, AttemptRecord>,
    default_max_attempts: u32,
}

impl AttemptTracker {
    /// Creates a new attempt tracker.
    pub fn new(default_max_attempts: u32) -> Self {
        Self {
            records: HashMap::new(),
            default_max_attempts,
        }
    }

    /// Records an attempt for the given key.
    pub fn record_attempt(&mut self, key: &str) -> &AttemptRecord {
        self.records
            .entry(key.to_string())
            .and_modify(|r| r.increment())
            .or_insert_with(|| AttemptRecord::new(self.default_max_attempts))
    }

    /// Checks if attempts are exceeded for the given key.
    pub fn is_exceeded(&self, key: &str) -> bool {
        self.records
            .get(key)
            .map(|r| r.is_exceeded())
            .unwrap_or(false)
    }

    /// Gets the attempt record for a key.
    pub fn get(&self, key: &str) -> Option<&AttemptRecord> {
        self.records.get(key)
    }

    /// Resets attempts for a key.
    pub fn reset(&mut self, key: &str) {
        self.records.remove(key);
    }

    /// Clears all records.
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_messages() {
        assert!(VerificationResult::Valid.error_message().is_none());
        assert!(VerificationResult::Invalid.error_message().is_some());
        assert!(VerificationResult::Expired.error_message().is_some());
    }

    #[test]
    fn test_attempt_tracker() {
        let mut tracker = AttemptTracker::new(3);
        
        // First attempt
        let record = tracker.record_attempt("user1");
        assert_eq!(record.attempts, 1);
        assert!(!record.is_exceeded());
        
        // Second attempt
        tracker.record_attempt("user1");
        assert_eq!(tracker.get("user1").unwrap().attempts, 2);
        
        // Third attempt - should be exceeded
        tracker.record_attempt("user1");
        assert!(tracker.is_exceeded("user1"));
        
        // Reset
        tracker.reset("user1");
        assert!(!tracker.is_exceeded("user1"));
    }

    #[test]
    fn test_attempt_record_remaining() {
        let mut record = AttemptRecord::new(3);
        assert_eq!(record.remaining(), 2);
        
        record.increment();
        assert_eq!(record.remaining(), 1);
        
        record.increment();
        assert_eq!(record.remaining(), 0);
        assert!(record.is_exceeded());
    }
}
