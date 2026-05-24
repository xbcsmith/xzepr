// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Secrets management utilities.
//!
//! This module provides wrapper types that prevent accidental exposure of sensitive
//! values in log output, debug traces, or error messages. Sensitive configuration
//! values such as passwords, API keys, and client secrets should be wrapped with
//! [`RedactedSecret`] so that they cannot leak through standard formatting.
//!
//! # Example
//!
//! ```
//! use xzepr::infrastructure::RedactedSecret;
//!
//! let secret = RedactedSecret::new("my-api-key".to_string());
//!
//! // Debug and Display always output [REDACTED]
//! assert_eq!(format!("{:?}", secret), "[REDACTED]");
//! assert_eq!(format!("{}", secret), "[REDACTED]");
//!
//! // Deref provides transparent read access to the wrapped value
//! assert_eq!(*secret, "my-api-key");
//! ```

use std::fmt;
use std::ops::Deref;

/// A wrapper type that prevents accidental exposure of sensitive values.
///
/// `RedactedSecret<T>` wraps any `T: Clone` and overrides `fmt::Debug` and
/// `fmt::Display` to always output `[REDACTED]` rather than the actual value.
/// This ensures that sensitive configuration—such as database passwords, JWT
/// signing keys, or OAuth client secrets—cannot be exposed by logging or
/// error-reporting infrastructure.
///
/// The wrapped value is accessible via `Deref` or by consuming the wrapper with
/// [`RedactedSecret::into_inner`].
///
/// # Note on zeroing memory
///
/// For `String` values that must be cleared from memory on drop, callers should
/// call [`into_inner`](RedactedSecret::into_inner) and then apply `zeroize::Zeroize`
/// directly. Generic zeroing cannot be implemented here without requiring
/// `T: zeroize::Zeroize`.
///
/// # Examples
///
/// ```
/// use xzepr::infrastructure::RedactedSecret;
///
/// let secret = RedactedSecret::new("db-password".to_string());
/// assert_eq!(format!("{:?}", secret), "[REDACTED]");
/// assert_eq!(*secret, "db-password");
/// ```
pub struct RedactedSecret<T: Clone> {
    inner: T,
}

impl<T: Clone> RedactedSecret<T> {
    /// Creates a new `RedactedSecret` wrapping the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The sensitive value to wrap.
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::infrastructure::RedactedSecret;
    ///
    /// let secret = RedactedSecret::new("api-key-value".to_string());
    /// assert_eq!(format!("{}", secret), "[REDACTED]");
    /// ```
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    /// Consumes the wrapper and returns the inner value.
    ///
    /// This is the explicit, intentional way to obtain ownership of the wrapped
    /// value. Callers must handle the returned secret with care.
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::infrastructure::RedactedSecret;
    ///
    /// let secret = RedactedSecret::new("api-key-value".to_string());
    /// let value = secret.into_inner();
    /// assert_eq!(value, "api-key-value");
    /// ```
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Clone> fmt::Debug for RedactedSecret<T> {
    /// Always outputs `[REDACTED]` regardless of the wrapped value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl<T: Clone> fmt::Display for RedactedSecret<T> {
    /// Always outputs `[REDACTED]` regardless of the wrapped value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl<T: Clone> Deref for RedactedSecret<T> {
    type Target = T;

    /// Provides transparent read access to the wrapped value.
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Clone> Clone for RedactedSecret<T> {
    /// Clones the wrapper. The clone also redacts its value in all output.
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<'de, T: Clone + serde::Deserialize<'de>> serde::Deserialize<'de> for RedactedSecret<T> {
    /// Deserializes the inner value and wraps it in `RedactedSecret`.
    ///
    /// The deserialized value is immediately hidden from any subsequent
    /// debug or display formatting.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self {
            inner: T::deserialize(deserializer)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redacted_secret_debug_hides_value() {
        let secret = RedactedSecret::new("super-secret-value");
        assert_eq!(
            format!("{:?}", secret),
            "[REDACTED]",
            "Debug output must always be [REDACTED]"
        );
    }

    #[test]
    fn test_redacted_secret_display_hides_value() {
        let secret = RedactedSecret::new("my-password");
        assert_eq!(
            format!("{}", secret),
            "[REDACTED]",
            "Display output must always be [REDACTED]"
        );
    }

    #[test]
    fn test_redacted_secret_deref_exposes_value() {
        let secret = RedactedSecret::new("inner-value".to_string());
        assert_eq!(
            *secret, "inner-value",
            "Deref must return the original inner value"
        );
    }

    #[test]
    fn test_redacted_secret_into_inner() {
        let secret = RedactedSecret::new("original-value".to_string());
        let value = secret.into_inner();
        assert_eq!(
            value, "original-value",
            "into_inner must return the original value"
        );
    }

    #[test]
    fn test_redacted_secret_clone() {
        let secret = RedactedSecret::new("cloneable-value".to_string());
        let cloned = secret.clone();
        // The clone must also redact output
        assert_eq!(
            format!("{:?}", cloned),
            "[REDACTED]",
            "Clone must also produce a redacted wrapper"
        );
        // But the inner value is preserved
        assert_eq!(
            *cloned, "cloneable-value",
            "Clone must preserve the inner value"
        );
    }
}
