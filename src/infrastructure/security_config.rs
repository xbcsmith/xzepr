// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/security_config.rs

use serde::{de, Deserialize, Deserializer};
use std::{collections::HashMap, fmt};
use thiserror::Error;

/// Errors that can occur during security configuration validation.
#[derive(Error, Debug, PartialEq)]
pub enum SecurityConfigError {
    /// CORS allowed_origins list is empty.
    #[error("CORS allowed_origins cannot be empty in production")]
    EmptyCorsOrigins,
    /// Wildcard CORS origins are not permitted in production.
    #[error("Wildcard CORS origin is not allowed in production")]
    WildcardCorsOrigin,
    /// A CORS origin is not HTTPS and is therefore unsafe for production.
    #[error("CORS origin must use HTTPS in production: {origin}")]
    NonHttpsCorsOrigin {
        /// The offending origin.
        origin: String,
    },
    /// Rate limit for anonymous users is configured to zero.
    #[error("Rate limit for anonymous users cannot be 0")]
    ZeroAnonymousRateLimit,
    /// Rate limit for authenticated users is configured to zero.
    #[error("Rate limit for authenticated users cannot be 0")]
    ZeroAuthenticatedRateLimit,
    /// Rate limit for admin users is configured to zero.
    #[error("Rate limit for admin users cannot be 0")]
    ZeroAdminRateLimit,
    /// Redis URL is required when use_redis is enabled but was not provided.
    #[error("Redis URL must be provided when use_redis is enabled")]
    MissingRedisUrl,
    /// Redis URL does not use a supported scheme.
    #[error("Redis URL must use redis:// or rediss://: {url}")]
    InvalidRedisUrl {
        /// The offending Redis URL.
        url: String,
    },
    /// Maximum request body size is zero.
    #[error("Maximum request body size cannot be 0")]
    ZeroMaxBodySize,
    /// Maximum string length is zero.
    #[error("Maximum string length cannot be 0")]
    ZeroMaxStringLength,
    /// Maximum array length is zero.
    #[error("Maximum array length cannot be 0")]
    ZeroMaxArrayLength,
    /// Strict validation is disabled in production.
    #[error("Strict request validation must be enabled in production")]
    StrictValidationDisabled,
    /// HSTS headers are disabled in production.
    #[error("HSTS must be enabled in production")]
    HstsDisabled,
    /// Monitoring log level is not one of the supported tracing levels.
    #[error("Invalid monitoring log level: {level}")]
    InvalidLogLevel {
        /// The configured log level.
        level: String,
    },
}

/// Security configuration for production deployments
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    /// CORS configuration
    #[serde(default)]
    pub cors: CorsSecurityConfig,
    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: RateLimitSecurityConfig,
    /// Input validation configuration
    #[serde(default)]
    pub validation: ValidationSecurityConfig,
    /// Security headers configuration
    #[serde(default)]
    pub headers: SecurityHeadersConfig,
    /// Monitoring configuration
    #[serde(default)]
    pub monitoring: MonitoringConfig,
}

/// CORS security configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorsSecurityConfig {
    /// List of allowed origins
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub allowed_origins: Vec<String>,
    /// Whether to allow credentials
    #[serde(default = "default_allow_credentials")]
    pub allow_credentials: bool,
    /// Maximum age for preflight cache in seconds
    #[serde(default = "default_max_age")]
    pub max_age_seconds: u64,
}

/// Rate limiting security configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitSecurityConfig {
    /// Requests per minute for anonymous users
    #[serde(default = "default_anonymous_rpm")]
    pub anonymous_rpm: u32,
    /// Requests per minute for authenticated users
    #[serde(default = "default_authenticated_rpm")]
    pub authenticated_rpm: u32,
    /// Requests per minute for admin users
    #[serde(default = "default_admin_rpm")]
    pub admin_rpm: u32,
    /// Per-endpoint rate limits
    #[serde(default)]
    pub per_endpoint: HashMap<String, u32>,
    /// Enable Redis backend for distributed rate limiting
    #[serde(default)]
    pub use_redis: bool,
    /// Redis connection URL
    pub redis_url: Option<String>,
}

/// Input validation security configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationSecurityConfig {
    /// Maximum request body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
    /// Maximum string length
    #[serde(default = "default_max_string_length")]
    pub max_string_length: usize,
    /// Maximum array length
    #[serde(default = "default_max_array_length")]
    pub max_array_length: usize,
    /// Enable strict validation mode
    #[serde(default = "default_strict_mode")]
    pub strict_mode: bool,
}

/// Security headers configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityHeadersConfig {
    /// Enable Content Security Policy
    #[serde(default = "default_true")]
    pub enable_csp: bool,
    /// CSP directives
    pub csp_directives: Option<String>,
    /// Enable HSTS
    #[serde(default = "default_true")]
    pub enable_hsts: bool,
    /// HSTS max age in seconds
    #[serde(default = "default_hsts_max_age")]
    pub hsts_max_age: u32,
    /// Include subdomains in HSTS
    #[serde(default = "default_true")]
    pub hsts_include_subdomains: bool,
    /// Enable HSTS preload
    #[serde(default)]
    pub hsts_preload: bool,
}

/// Monitoring configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
    /// Enable tracing
    #[serde(default = "default_true")]
    pub tracing_enabled: bool,
    /// Enable structured logging
    #[serde(default = "default_true")]
    pub structured_logging: bool,
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Enable JSON logs
    #[serde(default)]
    pub json_logs: bool,
    /// Jaeger endpoint for tracing
    pub jaeger_endpoint: Option<String>,
    /// Prometheus metrics port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
}

impl Default for CorsSecurityConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allow_credentials: true,
            max_age_seconds: 3600,
        }
    }
}

impl Default for RateLimitSecurityConfig {
    fn default() -> Self {
        Self {
            anonymous_rpm: 10,
            authenticated_rpm: 100,
            admin_rpm: 1000,
            per_endpoint: HashMap::new(),
            use_redis: false,
            redis_url: None,
        }
    }
}

impl Default for ValidationSecurityConfig {
    fn default() -> Self {
        Self {
            max_body_size: 1024 * 1024, // 1MB
            max_string_length: 10_000,
            max_array_length: 1_000,
            strict_mode: true,
        }
    }
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_csp: true,
            csp_directives: Some(
                "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https:; connect-src 'self'".to_string()
            ),
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            tracing_enabled: true,
            structured_logging: true,
            log_level: "info".to_string(),
            json_logs: false,
            jaeger_endpoint: None,
            metrics_port: 9090,
        }
    }
}

impl SecurityConfig {
    /// Creates a production security configuration
    pub fn production() -> Self {
        Self {
            cors: CorsSecurityConfig {
                allowed_origins: vec![],
                allow_credentials: true,
                max_age_seconds: 3600,
            },
            rate_limit: RateLimitSecurityConfig {
                anonymous_rpm: 10,
                authenticated_rpm: 100,
                admin_rpm: 1000,
                per_endpoint: [
                    ("/auth/login".to_string(), 5),
                    ("/auth/register".to_string(), 3),
                ]
                .into_iter()
                .collect(),
                use_redis: true,
                redis_url: None,
            },
            validation: ValidationSecurityConfig {
                max_body_size: 1024 * 1024, // 1MB
                max_string_length: 5_000,
                max_array_length: 500,
                strict_mode: true,
            },
            headers: SecurityHeadersConfig {
                enable_csp: true,
                csp_directives: Some(
                    "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https:; connect-src 'self'; frame-ancestors 'none'".to_string()
                ),
                enable_hsts: true,
                hsts_max_age: 63072000, // 2 years
                hsts_include_subdomains: true,
                hsts_preload: true,
            },
            monitoring: MonitoringConfig {
                metrics_enabled: true,
                tracing_enabled: true,
                structured_logging: true,
                log_level: "info".to_string(),
                json_logs: true,
                jaeger_endpoint: Some("http://jaeger:14268/api/traces".to_string()),
                metrics_port: 9090,
            },
        }
    }

    /// Creates a development security configuration
    pub fn development() -> Self {
        Self {
            cors: CorsSecurityConfig {
                allowed_origins: vec!["*".to_string()],
                allow_credentials: false,
                max_age_seconds: 3600,
            },
            rate_limit: RateLimitSecurityConfig {
                anonymous_rpm: 10000,
                authenticated_rpm: 10000,
                admin_rpm: 10000,
                per_endpoint: HashMap::new(),
                use_redis: false,
                redis_url: None,
            },
            validation: ValidationSecurityConfig {
                max_body_size: 10 * 1024 * 1024, // 10MB
                max_string_length: 100_000,
                max_array_length: 10_000,
                strict_mode: false,
            },
            headers: SecurityHeadersConfig {
                enable_csp: false,
                csp_directives: None,
                enable_hsts: false,
                hsts_max_age: 0,
                hsts_include_subdomains: false,
                hsts_preload: false,
            },
            monitoring: MonitoringConfig {
                metrics_enabled: true,
                tracing_enabled: true,
                structured_logging: true,
                log_level: "debug".to_string(),
                json_logs: false,
                jaeger_endpoint: None,
                metrics_port: 9090,
            },
        }
    }

    /// Validates the security configuration.
    ///
    /// # Errors
    ///
    /// Returns `SecurityConfigError::EmptyCorsOrigins` if CORS origins list is empty.
    /// Returns `SecurityConfigError::ZeroAnonymousRateLimit` if anonymous rate limit is 0.
    /// Returns `SecurityConfigError::MissingRedisUrl` if Redis is enabled but no URL provided.
    pub fn validate(&self) -> Result<(), SecurityConfigError> {
        self.validate_shared()?;

        if self.cors.allowed_origins.is_empty() {
            return Err(SecurityConfigError::EmptyCorsOrigins);
        }

        if self.rate_limit.use_redis {
            validate_redis_url(self.rate_limit.redis_url.as_deref())?;
        }

        Ok(())
    }

    /// Validates the security configuration for production use.
    ///
    /// # Errors
    ///
    /// Returns `SecurityConfigError` when any production-only invariant is
    /// violated, including wildcard CORS, non-HTTPS origins, disabled strict
    /// validation, disabled HSTS, or incomplete distributed rate limiting.
    pub fn validate_production(&self) -> Result<(), SecurityConfigError> {
        self.validate_shared()?;

        if self.cors.allowed_origins.is_empty() {
            return Err(SecurityConfigError::EmptyCorsOrigins);
        }

        if self.cors.allowed_origins.iter().any(|origin| origin == "*") {
            return Err(SecurityConfigError::WildcardCorsOrigin);
        }

        for origin in &self.cors.allowed_origins {
            if !origin.starts_with("https://") {
                return Err(SecurityConfigError::NonHttpsCorsOrigin {
                    origin: origin.clone(),
                });
            }
        }

        if self.rate_limit.use_redis {
            validate_redis_url(self.rate_limit.redis_url.as_deref())?;
        }

        if !self.validation.strict_mode {
            return Err(SecurityConfigError::StrictValidationDisabled);
        }

        if !self.headers.enable_hsts {
            return Err(SecurityConfigError::HstsDisabled);
        }

        validate_log_level(&self.monitoring.log_level)?;

        Ok(())
    }

    fn validate_shared(&self) -> Result<(), SecurityConfigError> {
        if self.rate_limit.anonymous_rpm == 0 {
            return Err(SecurityConfigError::ZeroAnonymousRateLimit);
        }
        if self.rate_limit.authenticated_rpm == 0 {
            return Err(SecurityConfigError::ZeroAuthenticatedRateLimit);
        }
        if self.rate_limit.admin_rpm == 0 {
            return Err(SecurityConfigError::ZeroAdminRateLimit);
        }
        if self.validation.max_body_size == 0 {
            return Err(SecurityConfigError::ZeroMaxBodySize);
        }
        if self.validation.max_string_length == 0 {
            return Err(SecurityConfigError::ZeroMaxStringLength);
        }
        if self.validation.max_array_length == 0 {
            return Err(SecurityConfigError::ZeroMaxArrayLength);
        }

        validate_log_level(&self.monitoring.log_level)?;

        Ok(())
    }
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVecVisitor;

    impl<'de> de::Visitor<'de> for StringOrVecVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a comma-separated string or a list of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect())
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                values.push(value);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_any(StringOrVecVisitor)
}

fn validate_redis_url(redis_url: Option<&str>) -> Result<(), SecurityConfigError> {
    let redis_url = redis_url
        .filter(|url| !url.trim().is_empty())
        .ok_or(SecurityConfigError::MissingRedisUrl)?;

    if redis_url.starts_with("redis://") || redis_url.starts_with("rediss://") {
        Ok(())
    } else {
        Err(SecurityConfigError::InvalidRedisUrl {
            url: redis_url.to_string(),
        })
    }
}

fn validate_log_level(level: &str) -> Result<(), SecurityConfigError> {
    match level {
        "trace" | "debug" | "info" | "warn" | "error" => Ok(()),
        _ => Err(SecurityConfigError::InvalidLogLevel {
            level: level.to_string(),
        }),
    }
}

// Default value functions
fn default_allow_credentials() -> bool {
    true
}

fn default_max_age() -> u64 {
    3600
}

fn default_anonymous_rpm() -> u32 {
    10
}

fn default_authenticated_rpm() -> u32 {
    100
}

fn default_admin_rpm() -> u32 {
    1000
}

fn default_max_body_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_max_string_length() -> usize {
    10_000
}

fn default_max_array_length() -> usize {
    1_000
}

fn default_strict_mode() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_hsts_max_age() -> u32 {
    31536000 // 1 year
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_metrics_port() -> u16 {
    9090
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_security_config() {
        let config = SecurityConfig::default();
        assert!(config.cors.allow_credentials);
        assert_eq!(config.rate_limit.anonymous_rpm, 10);
        assert!(config.monitoring.metrics_enabled);
    }

    #[test]
    fn test_production_config() {
        let config = SecurityConfig::production();
        assert!(config.headers.enable_hsts);
        assert_eq!(config.headers.hsts_max_age, 63072000);
        assert!(config.monitoring.json_logs);
        assert!(config.rate_limit.use_redis);
    }

    #[test]
    fn test_development_config() {
        let config = SecurityConfig::development();
        assert_eq!(config.cors.allowed_origins, vec!["*"]);
        assert!(!config.headers.enable_hsts);
        assert!(!config.monitoring.json_logs);
        assert!(!config.rate_limit.use_redis);
    }

    #[test]
    fn test_validate_empty_cors_origins() {
        let mut config = SecurityConfig::production();
        config.cors.allowed_origins = vec![];
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityConfigError::EmptyCorsOrigins
        ));
    }

    #[test]
    fn test_validate_redis_config() {
        let mut config = SecurityConfig::production();
        // production() has empty CORS origins; set them so we reach the Redis check
        config.cors.allowed_origins = vec!["https://app.example.com".to_string()];
        config.rate_limit.use_redis = true;
        config.rate_limit.redis_url = None;
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityConfigError::MissingRedisUrl
        ));
    }

    #[test]
    fn test_validate_valid_config() {
        let mut config = SecurityConfig::production();
        config.cors.allowed_origins = vec!["https://app.example.com".to_string()];
        config.rate_limit.redis_url = Some("redis://localhost:6379".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_anonymous_rate_limit() {
        let mut config = SecurityConfig::production();
        config.cors.allowed_origins = vec!["https://app.example.com".to_string()];
        config.rate_limit.redis_url = Some("redis://localhost:6379".to_string());
        config.rate_limit.anonymous_rpm = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityConfigError::ZeroAnonymousRateLimit
        ));
    }

    #[test]
    fn test_validate_invalid_security_config_fails_production() {
        // production() sets use_redis=true but no redis_url; after fixing CORS origins,
        // validation should fail with MissingRedisUrl
        let mut config = SecurityConfig::production();
        config.cors.allowed_origins = vec!["https://app.example.com".to_string()];
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityConfigError::MissingRedisUrl
        ));
    }
}
