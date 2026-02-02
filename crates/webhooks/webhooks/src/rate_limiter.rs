//! Rate Limiter for Webhook Endpoints
//!
//! Provides token bucket rate limiting per endpoint:
//! - Configurable requests per second
//! - Burst capacity
//! - Concurrent request limits
//! - Automatic token refill

use crate::{WebhookError, WebhookResult};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter for webhook endpoints
pub struct WebhookRateLimiter {
    limiters: Arc<RwLock<HashMap<String, Arc<TokenBucket>>>>,
    default_limit: EndpointRateLimit,
}

/// Rate limit configuration for an endpoint
#[derive(Debug, Clone)]
pub struct EndpointRateLimit {
    /// Maximum requests per second
    pub requests_per_second: u32,
    
    /// Burst capacity (max tokens in bucket)
    pub burst: u32,
    
    /// Maximum concurrent requests
    pub concurrent_requests: u32,
}

impl Default for EndpointRateLimit {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst: 20,
            concurrent_requests: 5,
        }
    }
}

/// Token bucket for rate limiting
pub struct TokenBucket {
    /// Current number of tokens
    tokens: AtomicU32,
    
    /// Maximum tokens (burst capacity)
    capacity: u32,
    
    /// Tokens refilled per second
    refill_rate: u32,
    
    /// Last time tokens were refilled
    last_refill: Arc<RwLock<Instant>>,
    
    /// Current concurrent requests
    concurrent: AtomicU32,
    
    /// Max concurrent requests
    max_concurrent: u32,
}

/// Permission to make a rate-limited request
pub struct RateLimitPermit {
    endpoint_id: String,
    bucket: Arc<TokenBucket>,
}

impl WebhookRateLimiter {
    /// Create a new rate limiter with default limits
    pub fn new() -> Self {
        Self::with_default_limit(EndpointRateLimit::default())
    }

    /// Create a rate limiter with custom default limits
    pub fn with_default_limit(default_limit: EndpointRateLimit) -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            default_limit,
        }
    }

    /// Acquire a permit for an endpoint
    ///
    /// Returns error if rate limit is exceeded
    pub async fn acquire(&self, endpoint_id: &str) -> WebhookResult<RateLimitPermit> {
        let bucket = self.get_or_create_bucket(endpoint_id).await;
        
        // Refill tokens based on elapsed time
        bucket.refill().await;
        
        // Check concurrent requests
        let current_concurrent = bucket.concurrent.load(Ordering::Acquire);
        if current_concurrent >= bucket.max_concurrent {
            return Err(WebhookError::Internal(format!(
                "Concurrent request limit exceeded for endpoint {}",
                endpoint_id
            )));
        }
        
        // Try to consume a token
        loop {
            let current = bucket.tokens.load(Ordering::Acquire);
            if current == 0 {
                return Err(WebhookError::Internal(format!(
                    "Rate limit exceeded for endpoint {}",
                    endpoint_id
                )));
            }
            
            if bucket.tokens.compare_exchange(
                current,
                current - 1,
                Ordering::Release,
                Ordering::Acquire,
            ).is_ok() {
                break;
            }
        }
        
        // Increment concurrent counter
        bucket.concurrent.fetch_add(1, Ordering::Release);
        
        Ok(RateLimitPermit {
            endpoint_id: endpoint_id.to_string(),
            bucket,
        })
    }

    /// Set custom rate limit for an endpoint
    pub async fn set_limit(&self, endpoint_id: &str, limit: EndpointRateLimit) {
        let bucket = Arc::new(TokenBucket {
            tokens: AtomicU32::new(limit.burst),
            capacity: limit.burst,
            refill_rate: limit.requests_per_second,
            last_refill: Arc::new(RwLock::new(Instant::now())),
            concurrent: AtomicU32::new(0),
            max_concurrent: limit.concurrent_requests,
        });
        
        let mut limiters = self.limiters.write().await;
        limiters.insert(endpoint_id.to_string(), bucket);
    }

    /// Get current rate limit info for an endpoint
    pub async fn get_limit_info(&self, endpoint_id: &str) -> RateLimitInfo {
        if let Some(bucket) = self.limiters.read().await.get(endpoint_id) {
            RateLimitInfo {
                capacity: bucket.capacity,
                available_tokens: bucket.tokens.load(Ordering::Acquire),
                refill_rate: bucket.refill_rate,
                concurrent_requests: bucket.concurrent.load(Ordering::Acquire),
                max_concurrent: bucket.max_concurrent,
            }
        } else {
            RateLimitInfo {
                capacity: self.default_limit.burst,
                available_tokens: self.default_limit.burst,
                refill_rate: self.default_limit.requests_per_second,
                concurrent_requests: 0,
                max_concurrent: self.default_limit.concurrent_requests,
            }
        }
    }

    /// Get or create a token bucket for an endpoint
    async fn get_or_create_bucket(&self, endpoint_id: &str) -> Arc<TokenBucket> {
        // Try to get existing bucket
        {
            let limiters = self.limiters.read().await;
            if let Some(bucket) = limiters.get(endpoint_id) {
                return bucket.clone();
            }
        }
        
        // Create new bucket with default limits
        let bucket = Arc::new(TokenBucket {
            tokens: AtomicU32::new(self.default_limit.burst),
            capacity: self.default_limit.burst,
            refill_rate: self.default_limit.requests_per_second,
            last_refill: Arc::new(RwLock::new(Instant::now())),
            concurrent: AtomicU32::new(0),
            max_concurrent: self.default_limit.concurrent_requests,
        });
        
        let mut limiters = self.limiters.write().await;
        limiters.insert(endpoint_id.to_string(), bucket.clone());
        bucket
    }
}

impl Default for WebhookRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenBucket {
    /// Refill tokens based on elapsed time
    async fn refill(&self) {
        let mut last_refill = self.last_refill.write().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);
        
        if elapsed >= Duration::from_secs(1) {
            let seconds_elapsed = elapsed.as_secs() as u32;
            let tokens_to_add = seconds_elapsed * self.refill_rate;
            
            if tokens_to_add > 0 {
                // Add tokens up to capacity
                loop {
                    let current = self.tokens.load(Ordering::Acquire);
                    let new_value = (current + tokens_to_add).min(self.capacity);
                    
                    if self.tokens.compare_exchange(
                        current,
                        new_value,
                        Ordering::Release,
                        Ordering::Acquire,
                    ).is_ok() {
                        break;
                    }
                }
                
                *last_refill = now;
            }
        }
    }
}

impl Drop for RateLimitPermit {
    fn drop(&mut self) {
        // Decrement concurrent counter when permit is dropped
        self.bucket.concurrent.fetch_sub(1, Ordering::Release);
    }
}

/// Information about current rate limits
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub capacity: u32,
    pub available_tokens: u32,
    pub refill_rate: u32,
    pub concurrent_requests: u32,
    pub max_concurrent: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = WebhookRateLimiter::new();
        
        // Should be able to acquire permits up to burst limit
        let permit1 = limiter.acquire("endpoint-1").await;
        assert!(permit1.is_ok());
        
        drop(permit1);
        
        let permit2 = limiter.acquire("endpoint-1").await;
        assert!(permit2.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_exhaustion() {
        let limit = EndpointRateLimit {
            requests_per_second: 1,
            burst: 2,
            concurrent_requests: 5,
        };
        
        let limiter = WebhookRateLimiter::with_default_limit(limit);
        
        // Consume all tokens
        let _permit1 = limiter.acquire("endpoint-1").await.unwrap();
        let _permit2 = limiter.acquire("endpoint-1").await.unwrap();
        
        // Next acquisition should fail
        let result = limiter.acquire("endpoint-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_limit() {
        let limit = EndpointRateLimit {
            requests_per_second: 10,
            burst: 100,
            concurrent_requests: 2,
        };
        
        let limiter = WebhookRateLimiter::with_default_limit(limit);
        
        let _permit1 = limiter.acquire("endpoint-1").await.unwrap();
        let _permit2 = limiter.acquire("endpoint-1").await.unwrap();
        
        // Third concurrent request should fail
        let result = limiter.acquire("endpoint-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_limit_info() {
        let limiter = WebhookRateLimiter::new();
        
        let info = limiter.get_limit_info("endpoint-1").await;
        assert_eq!(info.capacity, 20); // Default burst
        assert_eq!(info.refill_rate, 10); // Default RPS
    }
}
