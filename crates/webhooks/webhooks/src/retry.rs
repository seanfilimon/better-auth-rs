//! Retry strategies for webhook delivery.

use std::time::Duration;

/// Trait for retry strategies.
pub trait RetryStrategy: Send + Sync {
    /// Returns the delay before the next attempt, or None if max retries exceeded.
    fn next_delay(&self, attempt: u32) -> Option<Duration>;

    /// Returns the maximum number of attempts.
    fn max_attempts(&self) -> u32;

    /// Checks if another retry should be attempted.
    fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts()
    }
}

/// Exponential backoff retry strategy.
///
/// Delay increases exponentially: base * 2^attempt
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    /// Base delay.
    pub base: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
    /// Maximum number of attempts.
    pub max_attempts: u32,
    /// Jitter factor (0.0 to 1.0).
    pub jitter: f64,
}

impl ExponentialBackoff {
    /// Creates a new exponential backoff strategy.
    pub fn new() -> Self {
        Self {
            base: Duration::from_secs(1),
            max_delay: Duration::from_secs(3600), // 1 hour
            max_attempts: 5,
            jitter: 0.1,
        }
    }

    /// Sets the base delay.
    pub fn base(mut self, base: Duration) -> Self {
        self.base = base;
        self
    }

    /// Sets the maximum delay.
    pub fn max_delay(mut self, max: Duration) -> Self {
        self.max_delay = max;
        self
    }

    /// Sets the maximum attempts.
    pub fn max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Sets the jitter factor.
    pub fn jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryStrategy for ExponentialBackoff {
    fn next_delay(&self, attempt: u32) -> Option<Duration> {
        if attempt >= self.max_attempts {
            return None;
        }

        let multiplier = 2_u64.saturating_pow(attempt);
        let delay = self.base.saturating_mul(multiplier as u32);
        let delay = std::cmp::min(delay, self.max_delay);

        // Apply jitter
        if self.jitter > 0.0 {
            let jitter_range = (delay.as_millis() as f64 * self.jitter) as u64;
            let jitter_offset = (rand_simple() * jitter_range as f64) as u64;
            Some(delay + Duration::from_millis(jitter_offset))
        } else {
            Some(delay)
        }
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

/// Linear backoff retry strategy.
///
/// Delay increases linearly: base * attempt
#[derive(Debug, Clone)]
pub struct LinearBackoff {
    /// Base delay.
    pub base: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
    /// Maximum number of attempts.
    pub max_attempts: u32,
}

impl LinearBackoff {
    /// Creates a new linear backoff strategy.
    pub fn new() -> Self {
        Self {
            base: Duration::from_secs(5),
            max_delay: Duration::from_secs(300), // 5 minutes
            max_attempts: 5,
        }
    }

    /// Sets the base delay.
    pub fn base(mut self, base: Duration) -> Self {
        self.base = base;
        self
    }

    /// Sets the maximum delay.
    pub fn max_delay(mut self, max: Duration) -> Self {
        self.max_delay = max;
        self
    }

    /// Sets the maximum attempts.
    pub fn max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }
}

impl Default for LinearBackoff {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryStrategy for LinearBackoff {
    fn next_delay(&self, attempt: u32) -> Option<Duration> {
        if attempt >= self.max_attempts {
            return None;
        }

        let delay = self.base.saturating_mul(attempt + 1);
        Some(std::cmp::min(delay, self.max_delay))
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

/// Fixed delay retry strategy.
///
/// Always uses the same delay between attempts.
#[derive(Debug, Clone)]
pub struct FixedDelay {
    /// Fixed delay between attempts.
    pub delay: Duration,
    /// Maximum number of attempts.
    pub max_attempts: u32,
}

impl FixedDelay {
    /// Creates a new fixed delay strategy.
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            max_attempts: 3,
        }
    }

    /// Sets the maximum attempts.
    pub fn max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }
}

impl RetryStrategy for FixedDelay {
    fn next_delay(&self, attempt: u32) -> Option<Duration> {
        if attempt >= self.max_attempts {
            None
        } else {
            Some(self.delay)
        }
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

/// No retry strategy - fails immediately.
#[derive(Debug, Clone, Default)]
pub struct NoRetry;

impl RetryStrategy for NoRetry {
    fn next_delay(&self, _attempt: u32) -> Option<Duration> {
        None
    }

    fn max_attempts(&self) -> u32 {
        1
    }
}

/// Simple pseudo-random number generator for jitter.
fn rand_simple() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos as f64 / u32::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let strategy = ExponentialBackoff::new()
            .base(Duration::from_secs(1))
            .max_attempts(5)
            .jitter(0.0);

        assert_eq!(strategy.next_delay(0), Some(Duration::from_secs(1)));
        assert_eq!(strategy.next_delay(1), Some(Duration::from_secs(2)));
        assert_eq!(strategy.next_delay(2), Some(Duration::from_secs(4)));
        assert_eq!(strategy.next_delay(3), Some(Duration::from_secs(8)));
        assert_eq!(strategy.next_delay(4), Some(Duration::from_secs(16)));
        assert_eq!(strategy.next_delay(5), None);
    }

    #[test]
    fn test_linear_backoff() {
        let strategy = LinearBackoff::new()
            .base(Duration::from_secs(5))
            .max_attempts(3);

        assert_eq!(strategy.next_delay(0), Some(Duration::from_secs(5)));
        assert_eq!(strategy.next_delay(1), Some(Duration::from_secs(10)));
        assert_eq!(strategy.next_delay(2), Some(Duration::from_secs(15)));
        assert_eq!(strategy.next_delay(3), None);
    }

    #[test]
    fn test_fixed_delay() {
        let strategy = FixedDelay::new(Duration::from_secs(10)).max_attempts(3);

        assert_eq!(strategy.next_delay(0), Some(Duration::from_secs(10)));
        assert_eq!(strategy.next_delay(1), Some(Duration::from_secs(10)));
        assert_eq!(strategy.next_delay(2), Some(Duration::from_secs(10)));
        assert_eq!(strategy.next_delay(3), None);
    }

    #[test]
    fn test_no_retry() {
        let strategy = NoRetry;
        assert_eq!(strategy.next_delay(0), None);
        assert_eq!(strategy.max_attempts(), 1);
    }
}
