// src/infrastructure/security_config.rs

use serde::Deserialize;
use std::collections::HashMap;

/// Security configuration for production deployments
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    /// CORS configuration
    pub cors: CorsSecurityConfig,
    /// Rate limiting configuration
    pub rate_limit: RateLimitSecurityConfig,
    /// Input validation configuration
    pub validation: ValidationSecurityConfig,
    /// Security headers configuration
    pub headers: SecurityHeadersConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

/// CORS security configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CorsSecurityConfig {
    /// List of allowed origins
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

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors: CorsSecurityConfig::default(),
            rate_limit: RateLimitSecurityConfig::default(),
            validation: ValidationSecurityConfig::default(),
            headers: SecurityHeadersConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
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

    /// Validates the security configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate CORS
        if self.cors.allowed_origins.is_empty() {
            return Err("CORS allowed_origins cannot be empty in production".to_string());
        }

        // Check for wildcard in production
        if self.cors.allowed_origins.contains(&"*".to_string()) {
            tracing::warn!("Wildcard CORS origin detected - not recommended for production");
        }

        // Validate non-HTTPS origins in production
        for origin in &self.cors.allowed_origins {
            if origin != "*"
                && !origin.starts_with("https://")
                && !origin.starts_with("http://localhost")
            {
                tracing::warn!(
                    "Non-HTTPS origin detected: {} - not recommended for production",
                    origin
                );
            }
        }

        // Validate rate limits
        if self.rate_limit.anonymous_rpm == 0 {
            return Err("Rate limit for anonymous users cannot be 0".to_string());
        }

        // Validate Redis configuration if enabled
        if self.rate_limit.use_redis && self.rate_limit.redis_url.is_none() {
            return Err("Redis URL must be provided when use_redis is enabled".to_string());
        }

        Ok(())
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
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_redis_config() {
        let mut config = SecurityConfig::production();
        config.rate_limit.use_redis = true;
        config.rate_limit.redis_url = None;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_valid_config() {
        let mut config = SecurityConfig::production();
        config.cors.allowed_origins = vec!["https://app.example.com".to_string()];
        config.rate_limit.redis_url = Some("redis://localhost:6379".to_string());
        assert!(config.validate().is_ok());
    }
}
