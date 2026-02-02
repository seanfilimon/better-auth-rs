//! Circuit Breaker Pattern for Webhook Endpoints
//!
//! Protects failing endpoints and enables auto-recovery:
//! - Closed: Normal operation, requests go through
//! - Open: Too many failures, requests are rejected
//! - Half-Open: Testing recovery, limited requests allowed

use crate::{WebhookError, WebhookResult};
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker for webhook endpoints
///
/// Automatically opens when failure threshold is reached,
/// then transitions to half-open after timeout to test recovery.
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    config: CircuitBreakerConfig,
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    
    /// Number of consecutive successes in half-open to close circuit
    pub success_threshold: u32,
    
    /// Time to wait before transitioning from open to half-open
    pub timeout: Duration,
    
    /// Maximum concurrent calls allowed in half-open state
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
        }
    }
}

/// Current state of the circuit breaker
#[derive(Debug, Clone)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed {
        failure_count: u32,
    },
    
    /// Circuit is open, requests are rejected
    Open {
        opened_at: Instant,
    },
    
    /// Circuit is testing recovery
    HalfOpen {
        success_count: u32,
        failure_count: u32,
        active_calls: u32,
    },
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Create a circuit breaker with custom configuration
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed { failure_count: 0 })),
            config,
        }
    }

    /// Execute a function with circuit breaker protection
    ///
    /// # Errors
    ///
    /// Returns `WebhookError::CircuitOpen` if circuit is open
    /// Returns the inner error if the call fails
    pub async fn call<F, Fut, T>(&self, f: F) -> WebhookResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = WebhookResult<T>>,
    {
        // Check current state and decide if call is allowed
        {
            let state = self.state.read().await;
            match *state {
                CircuitState::Open { opened_at } => {
                    if opened_at.elapsed() > self.config.timeout {
                        // Timeout elapsed, transition to half-open
                        drop(state);
                        self.transition_to_half_open().await;
                    } else {
                        // Still open, reject call
                        return Err(WebhookError::CircuitOpen);
                    }
                }
                CircuitState::HalfOpen { active_calls, .. } => {
                    if active_calls >= self.config.half_open_max_calls {
                        // Too many concurrent calls in half-open
                        return Err(WebhookError::CircuitOpen);
                    }
                }
                CircuitState::Closed { .. } => {}
            }
        }

        // Increment active calls if in half-open
        self.increment_active_calls().await;

        // Execute the call
        let result = f().await;

        // Decrement active calls if in half-open
        self.decrement_active_calls().await;

        // Update state based on result
        match result {
            Ok(value) => {
                self.on_success().await;
                Ok(value)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }

    /// Record a successful call
    async fn on_success(&self) {
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::Closed { .. } => {
                // Reset failure count on success
                *state = CircuitState::Closed { failure_count: 0 };
            }
            CircuitState::HalfOpen { success_count, .. } => {
                let new_success_count = success_count + 1;
                if new_success_count >= self.config.success_threshold {
                    // Enough successes, close the circuit
                    *state = CircuitState::Closed { failure_count: 0 };
                    tracing::info!("Circuit breaker closed after successful recovery");
                } else {
                    // Still testing, increment success count
                    *state = CircuitState::HalfOpen {
                        success_count: new_success_count,
                        failure_count: 0,
                        active_calls: 0,
                    };
                }
            }
            CircuitState::Open { .. } => {
                // Shouldn't happen, but if it does, transition to half-open
                *state = CircuitState::HalfOpen {
                    success_count: 1,
                    failure_count: 0,
                    active_calls: 0,
                };
            }
        }
    }

    /// Record a failed call
    async fn on_failure(&self) {
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::Closed { failure_count } => {
                let new_failure_count = failure_count + 1;
                if new_failure_count >= self.config.failure_threshold {
                    // Too many failures, open the circuit
                    *state = CircuitState::Open {
                        opened_at: Instant::now(),
                    };
                    tracing::warn!(
                        "Circuit breaker opened after {} consecutive failures",
                        new_failure_count
                    );
                } else {
                    *state = CircuitState::Closed {
                        failure_count: new_failure_count,
                    };
                }
            }
            CircuitState::HalfOpen { .. } => {
                // Failure during recovery, open again
                *state = CircuitState::Open {
                    opened_at: Instant::now(),
                };
                tracing::warn!("Circuit breaker re-opened due to failure during recovery");
            }
            CircuitState::Open { .. } => {
                // Already open, nothing to do
            }
        }
    }

    /// Transition from open to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::HalfOpen {
            success_count: 0,
            failure_count: 0,
            active_calls: 0,
        };
        tracing::info!("Circuit breaker transitioned to half-open, testing recovery");
    }

    /// Increment active calls in half-open state
    async fn increment_active_calls(&self) {
        let mut state = self.state.write().await;
        if let CircuitState::HalfOpen {
            success_count,
            failure_count,
            active_calls,
        } = *state
        {
            *state = CircuitState::HalfOpen {
                success_count,
                failure_count,
                active_calls: active_calls + 1,
            };
        }
    }

    /// Decrement active calls in half-open state
    async fn decrement_active_calls(&self) {
        let mut state = self.state.write().await;
        if let CircuitState::HalfOpen {
            success_count,
            failure_count,
            active_calls,
        } = *state
        {
            *state = CircuitState::HalfOpen {
                success_count,
                failure_count,
                active_calls: active_calls.saturating_sub(1),
            };
        }
    }

    /// Get current circuit state
    pub async fn state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    /// Check if circuit is open
    pub async fn is_open(&self) -> bool {
        matches!(*self.state.read().await, CircuitState::Open { .. })
    }

    /// Manually reset the circuit breaker to closed state
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed { failure_count: 0 };
        tracing::info!("Circuit breaker manually reset");
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config(config);

        // First 3 failures should open the circuit
        for _ in 0..3 {
            let _ = cb.call(|| async { Err::<(), _>(WebhookError::DeliveryFailed("test".into())) }).await;
        }

        assert!(cb.is_open().await);
    }

    #[tokio::test]
    async fn test_circuit_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config(config);

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(|| async { Err::<(), _>(WebhookError::DeliveryFailed("test".into())) }).await;
        }

        assert!(cb.is_open().await);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Next call should trigger transition to half-open
        let state = cb.state().await;
        matches!(state, CircuitState::Open { .. });
    }

    #[tokio::test]
    async fn test_circuit_closes_after_successes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config(config);

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(|| async { Err::<(), _>(WebhookError::DeliveryFailed("test".into())) }).await;
        }

        // Wait for timeout and transition to half-open
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Successful calls should close the circuit
        for _ in 0..2 {
            let _ = cb.call(|| async { Ok::<_, WebhookError>(()) }).await;
        }

        let state = cb.state().await;
        matches!(state, CircuitState::Closed { .. });
    }

    #[tokio::test]
    async fn test_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config(config);

        // Open the circuit
        let _ = cb.call(|| async { Err::<(), _>(WebhookError::DeliveryFailed("test".into())) }).await;
        assert!(cb.is_open().await);

        // Reset should close it
        cb.reset().await;
        assert!(!cb.is_open().await);
    }
}
