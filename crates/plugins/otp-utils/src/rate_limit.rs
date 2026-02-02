//! Rate limiting utilities.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for rate limiting.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed within the time window.
    pub max_requests: u32,
    /// Time window duration.
    pub time_window: Duration,
    /// Whether rate limiting is enabled.
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 10,
            time_window: Duration::minutes(1),
            enabled: true,
        }
    }
}

impl RateLimitConfig {
    /// Creates a new rate limit config.
    pub fn new(max_requests: u32, time_window: Duration) -> Self {
        Self {
            max_requests,
            time_window,
            enabled: true,
        }
    }

    /// Disables rate limiting.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Creates a config for OTP sending (e.g., 3 per 5 minutes).
    pub fn for_otp_send() -> Self {
        Self::new(3, Duration::minutes(5))
    }

    /// Creates a config for OTP verification (e.g., 5 per minute).
    pub fn for_otp_verify() -> Self {
        Self::new(5, Duration::minutes(1))
    }

    /// Creates a config for API key usage.
    pub fn for_api_key(max_requests: u32, time_window_seconds: i64) -> Self {
        Self::new(max_requests, Duration::seconds(time_window_seconds))
    }
}

/// Result of a rate limit check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request is allowed.
    Allowed {
        /// Remaining requests in the current window.
        remaining: u32,
        /// When the current window resets.
        reset_at: DateTime<Utc>,
    },
    /// Request is rate limited.
    Limited {
        /// When the rate limit resets.
        reset_at: DateTime<Utc>,
        /// How long to wait before retrying (in milliseconds).
        retry_after_ms: i64,
    },
}

impl RateLimitResult {
    /// Returns true if the request is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed { .. })
    }

    /// Returns true if the request is rate limited.
    pub fn is_limited(&self) -> bool {
        matches!(self, RateLimitResult::Limited { .. })
    }
}

/// Tracks rate limit state for a single key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitState {
    /// Number of requests in the current window.
    pub request_count: u32,
    /// When the current window started.
    pub window_start: DateTime<Utc>,
    /// Last request timestamp.
    pub last_request: DateTime<Utc>,
}

impl RateLimitState {
    /// Creates a new rate limit state.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            request_count: 1,
            window_start: now,
            last_request: now,
        }
    }
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory rate limiter.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    states: HashMap<String, RateLimitState>,
}

impl RateLimiter {
    /// Creates a new rate limiter with the given config.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            states: HashMap::new(),
        }
    }

    /// Checks if a request is allowed for the given key.
    pub fn check(&mut self, key: &str) -> RateLimitResult {
        if !self.config.enabled {
            return RateLimitResult::Allowed {
                remaining: u32::MAX,
                reset_at: Utc::now() + Duration::days(365),
            };
        }

        let now = Utc::now();
        
        if let Some(state) = self.states.get_mut(key) {
            let window_end = state.window_start + self.config.time_window;
            
            // Check if window has expired
            if now > window_end {
                // Reset window
                state.request_count = 1;
                state.window_start = now;
                state.last_request = now;
                
                RateLimitResult::Allowed {
                    remaining: self.config.max_requests - 1,
                    reset_at: now + self.config.time_window,
                }
            } else if state.request_count >= self.config.max_requests {
                // Rate limited
                let retry_after = (window_end - now).num_milliseconds();
                RateLimitResult::Limited {
                    reset_at: window_end,
                    retry_after_ms: retry_after.max(0),
                }
            } else {
                // Increment counter
                state.request_count += 1;
                state.last_request = now;
                
                RateLimitResult::Allowed {
                    remaining: self.config.max_requests - state.request_count,
                    reset_at: window_end,
                }
            }
        } else {
            // First request for this key
            self.states.insert(key.to_string(), RateLimitState::new());
            
            RateLimitResult::Allowed {
                remaining: self.config.max_requests - 1,
                reset_at: now + self.config.time_window,
            }
        }
    }

    /// Resets the rate limit for a key.
    pub fn reset(&mut self, key: &str) {
        self.states.remove(key);
    }

    /// Cleans up expired entries.
    pub fn cleanup(&mut self) {
        let now = Utc::now();
        self.states.retain(|_, state| {
            state.window_start + self.config.time_window > now
        });
    }

    /// Gets the current state for a key.
    pub fn get_state(&self, key: &str) -> Option<&RateLimitState> {
        self.states.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_initial_requests() {
        let mut limiter = RateLimiter::new(RateLimitConfig::new(3, Duration::minutes(1)));
        
        assert!(limiter.check("user1").is_allowed());
        assert!(limiter.check("user1").is_allowed());
        assert!(limiter.check("user1").is_allowed());
    }

    #[test]
    fn test_rate_limiter_blocks_after_limit() {
        let mut limiter = RateLimiter::new(RateLimitConfig::new(2, Duration::minutes(1)));
        
        assert!(limiter.check("user1").is_allowed());
        assert!(limiter.check("user1").is_allowed());
        assert!(limiter.check("user1").is_limited());
    }

    #[test]
    fn test_rate_limiter_separate_keys() {
        let mut limiter = RateLimiter::new(RateLimitConfig::new(1, Duration::minutes(1)));
        
        assert!(limiter.check("user1").is_allowed());
        assert!(limiter.check("user1").is_limited());
        assert!(limiter.check("user2").is_allowed()); // Different key
    }

    #[test]
    fn test_rate_limiter_disabled() {
        let mut limiter = RateLimiter::new(RateLimitConfig::disabled());
        
        for _ in 0..100 {
            assert!(limiter.check("user1").is_allowed());
        }
    }
}
