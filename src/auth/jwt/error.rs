//! JWT error types
//!
//! This module defines error types for JWT authentication operations.

use thiserror::Error;

/// Errors that can occur during JWT operations
#[derive(Debug, Error)]
pub enum JwtError {
    /// Token has expired
    #[error("Token has expired")]
    Expired,

    /// Token is not yet valid (nbf claim)
    #[error("Token is not yet valid")]
    NotYetValid,

    /// Invalid token signature
    #[error("Invalid token signature")]
    InvalidSignature,

    /// Invalid token format
    #[error("Invalid token format: {0}")]
    InvalidFormat(String),

    /// Missing required claim
    #[error("Missing required claim: {0}")]
    MissingClaim(String),

    /// Invalid claim value
    #[error("Invalid claim value: {0}")]
    InvalidClaim(String),

    /// Token has been revoked
    #[error("Token has been revoked")]
    Revoked,

    /// Invalid issuer
    #[error("Invalid issuer: expected {expected}, got {actual}")]
    InvalidIssuer { expected: String, actual: String },

    /// Invalid audience
    #[error("Invalid audience: expected {expected}, got {actual}")]
    InvalidAudience { expected: String, actual: String },

    /// Key management error
    #[error("Key management error: {0}")]
    KeyError(String),

    /// Token encoding error
    #[error("Token encoding error: {0}")]
    EncodingError(String),

    /// Token decoding error
    #[error("Token decoding error: {0}")]
    DecodingError(String),

    /// Blacklist storage error
    #[error("Blacklist storage error: {0}")]
    BlacklistError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Internal error
    #[error("Internal JWT error: {0}")]
    Internal(String),
}

impl From<jsonwebtoken::errors::Error> for JwtError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => JwtError::Expired,
            ErrorKind::ImmatureSignature => JwtError::NotYetValid,
            ErrorKind::InvalidSignature => JwtError::InvalidSignature,
            ErrorKind::InvalidToken => JwtError::InvalidFormat(err.to_string()),
            ErrorKind::InvalidIssuer => JwtError::InvalidClaim("issuer".to_string()),
            ErrorKind::InvalidAudience => JwtError::InvalidClaim("audience".to_string()),
            _ => JwtError::DecodingError(err.to_string()),
        }
    }
}

/// Result type for JWT operations
pub type JwtResult<T> = Result<T, JwtError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = JwtError::Expired;
        assert_eq!(err.to_string(), "Token has expired");

        let err = JwtError::InvalidIssuer {
            expected: "xzepr".to_string(),
            actual: "other".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid issuer: expected xzepr, got other");
    }

    #[test]
    fn test_error_from_jsonwebtoken() {
        let jwt_err =
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::ExpiredSignature);
        let err: JwtError = jwt_err.into();
        matches!(err, JwtError::Expired);
    }
}
