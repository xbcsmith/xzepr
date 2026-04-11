// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Circuit breaker for OPA client with graceful degradation
//!
//! This module provides a circuit breaker pattern to handle OPA service failures
//! gracefully, allowing fallback to legacy RBAC when OPA is unavailable.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// Circuit breaker states
///
/// The circuit breaker transitions between three states based on failure patterns.
#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    /// Circuit is closed, requests are allowed through
    Closed {
        /// Number of consecutive failures
        consecutive_failures: u32,
    },

    /// Circuit is open, requests are rejected to allow recovery
    Open {
        /// When the circuit was opened
        opened_at: Instant,
    },

    /// Circuit is half-open, testing if service has recovered
    HalfOpen,
}

/// Circuit breaker for OPA client
///
/// Implements the circuit breaker pattern to prevent cascading failures when
/// OPA is unavailable. The circuit opens after a threshold of consecutive
/// failures and closes after a timeout period if a test request succeeds.
///
/// # Examples
///
/// ```
/// use xzepr::opa::circuit_breaker::CircuitBreaker;
/// use std::time::Duration;
///
/// # tokio_test::block_on(async {
/// let breaker = CircuitBreaker::new(3, Duration::from_secs(30));
///
/// // Simulate successful call
/// let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
/// assert!(result.is_ok());
/// # });
/// ```
pub struct CircuitBreaker {
    /// Current circuit state
    state: Arc<RwLock<CircuitState>>,

    /// Number of consecutive failures before opening circuit
    failure_threshold: u32,

    /// Duration to wait before attempting half-open state
    timeout_duration: Duration,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker
    ///
    /// # Arguments
    ///
    /// * `failure_threshold` - Number of consecutive failures before opening
    /// * `timeout_duration` - Duration to wait before testing recovery
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::circuit_breaker::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// let breaker = CircuitBreaker::new(5, Duration::from_secs(60));
    /// ```
    pub fn new(failure_threshold: u32, timeout_duration: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed {
                consecutive_failures: 0,
            })),
            failure_threshold,
            timeout_duration,
        }
    }

    /// Executes a function with circuit breaker protection
    ///
    /// # Arguments
    ///
    /// * `f` - Async function to execute
    ///
    /// # Returns
    ///
    /// Returns the function result if circuit is closed/half-open and call succeeds,
    /// or a circuit breaker error if circuit is open or call fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::circuit_breaker::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let breaker = CircuitBreaker::new(3, Duration::from_secs(30));
    ///
    /// let result = breaker.call(|| async {
    ///     Ok::<_, String>("success")
    /// }).await;
    ///
    /// assert!(result.is_ok());
    /// # });
    /// ```
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        // Check if we should attempt the call
        if !self.should_attempt().await {
            return Err(CircuitBreakerError::CircuitOpen);
        }

        // Execute the function
        match f().await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(CircuitBreakerError::CallFailed(e))
            }
        }
    }

    /// Checks if a call should be attempted based on circuit state
    ///
    /// # Returns
    ///
    /// `true` if the circuit is closed or half-open, `false` if open
    async fn should_attempt(&self) -> bool {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed { .. } => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open { opened_at } => {
                // Check if timeout has elapsed
                if opened_at.elapsed() >= self.timeout_duration {
                    // Transition to half-open
                    *state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Records a successful call
    ///
    /// Resets failure counter and closes the circuit if it was half-open.
    async fn record_success(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed {
            consecutive_failures: 0,
        };
    }

    /// Records a failed call
    ///
    /// Increments failure counter and opens circuit if threshold is exceeded.
    async fn record_failure(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed {
                consecutive_failures,
            } => {
                let new_failures = consecutive_failures + 1;
                if new_failures >= self.failure_threshold {
                    *state = CircuitState::Open {
                        opened_at: Instant::now(),
                    };
                } else {
                    *state = CircuitState::Closed {
                        consecutive_failures: new_failures,
                    };
                }
            }
            CircuitState::HalfOpen => {
                // Failed in half-open state, reopen circuit
                *state = CircuitState::Open {
                    opened_at: Instant::now(),
                };
            }
            CircuitState::Open { .. } => {
                // Already open, no action needed
            }
        }
    }

    /// Checks if the circuit is currently open
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::circuit_breaker::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let breaker = CircuitBreaker::new(3, Duration::from_secs(30));
    /// assert!(!breaker.is_open().await);
    /// # });
    /// ```
    pub async fn is_open(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, CircuitState::Open { .. })
    }

    /// Gets the current state of the circuit breaker
    ///
    /// # Returns
    ///
    /// A string representation of the current state
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::circuit_breaker::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let breaker = CircuitBreaker::new(3, Duration::from_secs(30));
    /// assert_eq!(breaker.state().await, "closed");
    /// # });
    /// ```
    pub async fn state(&self) -> String {
        let state = self.state.read().await;
        match *state {
            CircuitState::Closed { .. } => "closed".to_string(),
            CircuitState::Open { .. } => "open".to_string(),
            CircuitState::HalfOpen => "half_open".to_string(),
        }
    }

    /// Manually resets the circuit breaker to closed state
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::circuit_breaker::CircuitBreaker;
    /// use std::time::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let breaker = CircuitBreaker::new(3, Duration::from_secs(30));
    /// breaker.reset().await;
    /// # });
    /// ```
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed {
            consecutive_failures: 0,
        };
    }
}

/// Circuit breaker errors
///
/// Errors that can occur when using the circuit breaker.
#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request rejected
    #[error("Circuit breaker is open")]
    CircuitOpen,

    /// The protected call failed
    #[error("Call failed: {0}")]
    CallFailed(E),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_success() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert!(!breaker.is_open().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_failure() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        let result = breaker.call(|| async { Err::<String, _>("error") }).await;

        assert!(result.is_err());
        assert!(!breaker.is_open().await); // Not open yet
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_threshold() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        // Fail 3 times to exceed threshold
        for _ in 0..3 {
            let _ = breaker.call(|| async { Err::<String, _>("error") }).await;
        }

        assert!(breaker.is_open().await);

        // Next call should be rejected
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;

        assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen)));
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let breaker = CircuitBreaker::new(3, Duration::from_millis(100));

        // Fail 3 times to open circuit
        for _ in 0..3 {
            let _ = breaker.call(|| async { Err::<String, _>("error") }).await;
        }

        assert!(breaker.is_open().await);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Circuit should allow attempt (half-open)
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;

        assert!(result.is_ok());
        assert!(!breaker.is_open().await); // Should be closed now
    }

    #[tokio::test]
    async fn test_circuit_breaker_reopens_on_half_open_failure() {
        let breaker = CircuitBreaker::new(3, Duration::from_millis(100));

        // Fail 3 times to open circuit
        for _ in 0..3 {
            let _ = breaker.call(|| async { Err::<String, _>("error") }).await;
        }

        assert!(breaker.is_open().await);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Fail in half-open state
        let _ = breaker.call(|| async { Err::<String, _>("error") }).await;

        assert!(breaker.is_open().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        // Fail 3 times to open circuit
        for _ in 0..3 {
            let _ = breaker.call(|| async { Err::<String, _>("error") }).await;
        }

        assert!(breaker.is_open().await);

        // Reset circuit
        breaker.reset().await;
        assert!(!breaker.is_open().await);

        // Should allow calls now
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_state() {
        let breaker = CircuitBreaker::new(3, Duration::from_millis(100));

        assert_eq!(breaker.state().await, "closed");

        // Fail 3 times
        for _ in 0..3 {
            let _ = breaker.call(|| async { Err::<String, _>("error") }).await;
        }

        assert_eq!(breaker.state().await, "open");

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Trigger half-open check
        let _ = breaker.should_attempt().await;
        assert_eq!(breaker.state().await, "half_open");
    }

    #[tokio::test]
    async fn test_circuit_breaker_multiple_successes() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        // Multiple successful calls
        for _ in 0..10 {
            let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
            assert!(result.is_ok());
        }

        assert!(!breaker.is_open().await);
        assert_eq!(breaker.state().await, "closed");
    }

    #[tokio::test]
    async fn test_circuit_breaker_alternating_success_failure() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        // Fail twice
        for _ in 0..2 {
            let _ = breaker.call(|| async { Err::<String, _>("error") }).await;
        }

        // Success resets counter
        let result = breaker.call(|| async { Ok::<_, String>("success") }).await;
        assert!(result.is_ok());

        // Should not be open yet
        assert!(!breaker.is_open().await);
    }
}
