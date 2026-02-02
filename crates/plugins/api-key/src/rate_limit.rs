//! Rate limiting for API keys.

use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Rate limit state for an API key.
#[derive(Debug, Clone)]
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

/// Result of a rate limit check.
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request is allowed.
    Allowed {
        remaining: u32,
        reset_at: DateTime<Utc>,
    },
    /// Request is rate limited.
    Limited {
        reset_at: DateTime<Utc>,
        retry_after_ms: i64,
    },
}

impl RateLimitResult {
    /// Returns true if the request is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed { .. })
    }
}

/// API key rate limiter.
#[derive(Debug, Default)]
pub struct ApiKeyRateLimiter {
    states: HashMap<String, RateLimitState>,
}

impl ApiKeyRateLimiter {
    /// Creates a new rate limiter.
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// Checks if a request is allowed for the given API key.
    pub fn check(
        &mut self,
        key_id: &str,
        time_window_ms: i64,
        max_requests: u32,
        enabled: bool,
    ) -> RateLimitResult {
        if !enabled {
            return RateLimitResult::Allowed {
                remaining: u32::MAX,
                reset_at: Utc::now() + chrono::Duration::days(365),
            };
        }

        let now = Utc::now();
        let window_duration = chrono::Duration::milliseconds(time_window_ms);

        if let Some(state) = self.states.get_mut(key_id) {
            let window_end = state.window_start + window_duration;

            if now > window_end {
                // Reset window
                state.request_count = 1;
                state.window_start = now;
                state.last_request = now;

                RateLimitResult::Allowed {
                    remaining: max_requests - 1,
                    reset_at: now + window_duration,
                }
            } else if state.request_count >= max_requests {
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
                    remaining: max_requests - state.request_count,
                    reset_at: window_end,
                }
            }
        } else {
            // First request
            self.states.insert(key_id.to_string(), RateLimitState::new());

            RateLimitResult::Allowed {
                remaining: max_requests - 1,
                reset_at: now + window_duration,
            }
        }
    }

    /// Resets the rate limit for a key.
    pub fn reset(&mut self, key_id: &str) {
        self.states.remove(key_id);
    }

    /// Cleans up expired entries.
    pub fn cleanup(&mut self, max_window_ms: i64) {
        let now = Utc::now();
        let max_duration = chrono::Duration::milliseconds(max_window_ms);
        
        self.states.retain(|_, state| {
            state.window_start + max_duration > now
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_initial() {
        let mut limiter = ApiKeyRateLimiter::new();
        
        let result = limiter.check("key1", 60000, 10, true);
        assert!(result.is_allowed());
    }

    #[test]
    fn test_rate_limiter_blocks_after_limit() {
        let mut limiter = ApiKeyRateLimiter::new();
        
        for _ in 0..10 {
            limiter.check("key1", 60000, 10, true);
        }
        
        let result = limiter.check("key1", 60000, 10, true);
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_rate_limiter_disabled() {
        let mut limiter = ApiKeyRateLimiter::new();
        
        for _ in 0..100 {
            let result = limiter.check("key1", 60000, 10, false);
            assert!(result.is_allowed());
        }
    }
}
