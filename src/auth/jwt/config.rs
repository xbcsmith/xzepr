// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! JWT Configuration
//!
//! This module defines the configuration structures for JWT authentication,
//! including token expiration times, issuer/audience settings, and algorithm
//! selection.

use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Errors produced when validating [`JwtConfig`].
///
/// These errors indicate misconfiguration at startup and are never returned in
/// per-request paths.
#[derive(Debug, thiserror::Error)]
pub enum JwtConfigError {
    /// Access token expiration is zero or negative.
    #[error("Access token expiration must be positive")]
    NonPositiveAccessExpiration,
    /// Refresh token expiration is zero or negative.
    #[error("Refresh token expiration must be positive")]
    NonPositiveRefreshExpiration,
    /// Access token expiration is not shorter than refresh token expiration.
    #[error("Access token expiration must be less than refresh token expiration")]
    AccessExpirationExceedsRefresh,
    /// JWT issuer is empty.
    #[error("JWT issuer cannot be empty")]
    EmptyIssuer,
    /// JWT audience is empty.
    #[error("JWT audience cannot be empty")]
    EmptyAudience,
    /// RS256 requires a private key path.
    #[error("Private key path is required for RS256")]
    MissingPrivateKey,
    /// RS256 requires a public key path.
    #[error("Public key path is required for RS256")]
    MissingPublicKey,
    /// HS256/HS512 require a secret.
    #[error("Secret key is required for HS256/HS512")]
    MissingSecret,
    /// HS256/HS512 secret key is shorter than the minimum required length.
    #[error("Secret key must be at least 32 characters")]
    SecretTooShort,
}

/// JWT configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Access token expiration duration (default: 15 minutes)
    #[serde(default = "default_access_token_expiration")]
    pub access_token_expiration_seconds: i64,

    /// Refresh token expiration duration (default: 7 days)
    #[serde(default = "default_refresh_token_expiration")]
    pub refresh_token_expiration_seconds: i64,

    /// Token issuer (identifies who issued the token)
    #[serde(default = "default_issuer")]
    pub issuer: String,

    /// Token audience (identifies who the token is intended for)
    #[serde(default = "default_audience")]
    pub audience: String,

    /// Algorithm to use for signing (RS256 or HS256)
    #[serde(default = "default_algorithm")]
    pub algorithm: Algorithm,

    /// Private key path for RS256 (PEM format)
    pub private_key_path: Option<String>,

    /// Public key path for RS256 (PEM format)
    pub public_key_path: Option<String>,

    /// Secret key for HS256 (not recommended for production)
    pub secret_key: Option<String>,

    /// Enable token rotation on refresh
    #[serde(default = "default_enable_rotation")]
    pub enable_token_rotation: bool,

    /// Clock skew tolerance in seconds (for exp/nbf validation)
    #[serde(default = "default_leeway")]
    pub leeway_seconds: u64,
}

/// JWT signing algorithm
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Algorithm {
    /// RSA with SHA-256 (recommended for production)
    RS256,
    /// HMAC with SHA-256 (simpler but less secure)
    HS256,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            access_token_expiration_seconds: default_access_token_expiration(),
            refresh_token_expiration_seconds: default_refresh_token_expiration(),
            issuer: default_issuer(),
            audience: default_audience(),
            algorithm: default_algorithm(),
            private_key_path: None,
            public_key_path: None,
            secret_key: None,
            enable_token_rotation: default_enable_rotation(),
            leeway_seconds: default_leeway(),
        }
    }
}

impl JwtConfig {
    /// Create a new JWT config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get access token expiration as Duration
    pub fn access_token_expiration(&self) -> Duration {
        Duration::seconds(self.access_token_expiration_seconds)
    }

    /// Get refresh token expiration as Duration
    pub fn refresh_token_expiration(&self) -> Duration {
        Duration::seconds(self.refresh_token_expiration_seconds)
    }

    /// Validate the configuration.
    ///
    /// # Errors
    ///
    /// Returns a [`JwtConfigError`] variant describing the first constraint
    /// violation found.
    pub fn validate(&self) -> Result<(), JwtConfigError> {
        // Validate expiration times
        if self.access_token_expiration_seconds <= 0 {
            return Err(JwtConfigError::NonPositiveAccessExpiration);
        }

        if self.refresh_token_expiration_seconds <= 0 {
            return Err(JwtConfigError::NonPositiveRefreshExpiration);
        }

        if self.access_token_expiration_seconds >= self.refresh_token_expiration_seconds {
            return Err(JwtConfigError::AccessExpirationExceedsRefresh);
        }

        // Validate issuer and audience
        if self.issuer.is_empty() {
            return Err(JwtConfigError::EmptyIssuer);
        }

        if self.audience.is_empty() {
            return Err(JwtConfigError::EmptyAudience);
        }

        // Validate algorithm-specific requirements
        match self.algorithm {
            Algorithm::RS256 => {
                if self.private_key_path.is_none() {
                    return Err(JwtConfigError::MissingPrivateKey);
                }
                if self.public_key_path.is_none() {
                    return Err(JwtConfigError::MissingPublicKey);
                }
            }
            Algorithm::HS256 => {
                if self.secret_key.is_none() {
                    return Err(JwtConfigError::MissingSecret);
                }
                if let Some(ref secret) = self.secret_key {
                    if secret.len() < 32 {
                        return Err(JwtConfigError::SecretTooShort);
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a development configuration (HS256 with a simple secret)
    #[cfg(test)]
    pub fn development() -> Self {
        Self {
            access_token_expiration_seconds: 900,     // 15 minutes
            refresh_token_expiration_seconds: 604800, // 7 days
            issuer: "xzepr-dev".to_string(),
            audience: "xzepr-api-dev".to_string(),
            algorithm: Algorithm::HS256,
            private_key_path: None,
            public_key_path: None,
            secret_key: Some("dev-secret-key-min-32-characters-long".to_string()),
            enable_token_rotation: true,
            leeway_seconds: 60,
        }
    }

    /// Create a production configuration template (RS256)
    pub fn production_template() -> Self {
        Self {
            access_token_expiration_seconds: 900,     // 15 minutes
            refresh_token_expiration_seconds: 604800, // 7 days
            issuer: "xzepr".to_string(),
            audience: "xzepr-api".to_string(),
            algorithm: Algorithm::RS256,
            private_key_path: Some("/etc/xzepr/keys/private.pem".to_string()),
            public_key_path: Some("/etc/xzepr/keys/public.pem".to_string()),
            secret_key: None,
            enable_token_rotation: true,
            leeway_seconds: 60,
        }
    }
}

// Default value functions for serde
fn default_access_token_expiration() -> i64 {
    900 // 15 minutes
}

fn default_refresh_token_expiration() -> i64 {
    604800 // 7 days
}

fn default_issuer() -> String {
    "xzepr".to_string()
}

fn default_audience() -> String {
    "xzepr-api".to_string()
}

fn default_algorithm() -> Algorithm {
    Algorithm::RS256
}

fn default_enable_rotation() -> bool {
    true
}

fn default_leeway() -> u64 {
    60 // 1 minute clock skew tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = JwtConfig::default();
        assert_eq!(config.access_token_expiration_seconds, 900);
        assert_eq!(config.refresh_token_expiration_seconds, 604800);
        assert_eq!(config.issuer, "xzepr");
        assert_eq!(config.audience, "xzepr-api");
        assert_eq!(config.algorithm, Algorithm::RS256);
        assert!(config.enable_token_rotation);
        assert_eq!(config.leeway_seconds, 60);
    }

    #[test]
    fn test_development_config() {
        let config = JwtConfig::development();
        assert_eq!(config.algorithm, Algorithm::HS256);
        assert!(config.secret_key.is_some());
        assert_eq!(config.issuer, "xzepr-dev");
    }

    #[test]
    fn test_production_template() {
        let config = JwtConfig::production_template();
        assert_eq!(config.algorithm, Algorithm::RS256);
        assert!(config.private_key_path.is_some());
        assert!(config.public_key_path.is_some());
        assert!(config.secret_key.is_none());
    }

    #[test]
    fn test_validate_success() {
        let config = JwtConfig::development();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_negative_access_expiration() {
        let mut config = JwtConfig::development();
        config.access_token_expiration_seconds = -1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_access_longer_than_refresh() {
        let mut config = JwtConfig::development();
        config.access_token_expiration_seconds = 1000;
        config.refresh_token_expiration_seconds = 500;
        let result = config.validate();
        assert!(matches!(
            result,
            Err(JwtConfigError::AccessExpirationExceedsRefresh)
        ));
    }

    #[test]
    fn test_validate_empty_issuer() {
        let mut config = JwtConfig::development();
        config.issuer = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_rs256_missing_keys() {
        let config = JwtConfig {
            algorithm: Algorithm::RS256,
            private_key_path: None,
            public_key_path: None,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_hs256_missing_secret() {
        let config = JwtConfig {
            algorithm: Algorithm::HS256,
            secret_key: None,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_hs256_short_secret() {
        let config = JwtConfig {
            algorithm: Algorithm::HS256,
            secret_key: Some("short".to_string()),
            ..Default::default()
        };
        let result = config.validate();
        assert!(matches!(result, Err(JwtConfigError::SecretTooShort)));
    }

    #[test]
    fn test_jwt_config_validate_non_positive_access_expiration() {
        let mut config = JwtConfig::development();
        config.access_token_expiration_seconds = 0;
        assert!(matches!(
            config.validate(),
            Err(JwtConfigError::NonPositiveAccessExpiration)
        ));
    }

    #[test]
    fn test_jwt_config_validate_non_positive_refresh_expiration() {
        let mut config = JwtConfig::development();
        config.refresh_token_expiration_seconds = -1;
        assert!(matches!(
            config.validate(),
            Err(JwtConfigError::NonPositiveRefreshExpiration)
        ));
    }

    #[test]
    fn test_jwt_config_validate_access_exceeds_refresh() {
        let mut config = JwtConfig::development();
        config.access_token_expiration_seconds = 86400;
        config.refresh_token_expiration_seconds = 3600;
        assert!(matches!(
            config.validate(),
            Err(JwtConfigError::AccessExpirationExceedsRefresh)
        ));
    }

    #[test]
    fn test_jwt_config_validate_empty_issuer() {
        let mut config = JwtConfig::development();
        config.issuer = String::new();
        assert!(matches!(
            config.validate(),
            Err(JwtConfigError::EmptyIssuer)
        ));
    }

    #[test]
    fn test_jwt_config_validate_empty_audience() {
        let mut config = JwtConfig::development();
        config.audience = String::new();
        assert!(matches!(
            config.validate(),
            Err(JwtConfigError::EmptyAudience)
        ));
    }

    #[test]
    fn test_access_token_expiration_duration() {
        let config = JwtConfig::development();
        let duration = config.access_token_expiration();
        assert_eq!(duration.num_seconds(), 900);
    }

    #[test]
    fn test_refresh_token_expiration_duration() {
        let config = JwtConfig::development();
        let duration = config.refresh_token_expiration();
        assert_eq!(duration.num_seconds(), 604800);
    }

    #[test]
    fn test_serde_serialization() {
        let config = JwtConfig::development();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("xzepr-dev"));
        assert!(json.contains("HS256"));
    }

    #[test]
    fn test_serde_deserialization() {
        let json = r#"{
            "access_token_expiration_seconds": 600,
            "refresh_token_expiration_seconds": 86400,
            "issuer": "test",
            "audience": "test-api",
            "algorithm": "HS256",
            "secret_key": "test-secret-key-with-32-chars-min",
            "enable_token_rotation": false,
            "leeway_seconds": 30
        }"#;

        let config: JwtConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.access_token_expiration_seconds, 600);
        assert_eq!(config.issuer, "test");
        assert_eq!(config.algorithm, Algorithm::HS256);
        assert!(!config.enable_token_rotation);
    }
}
