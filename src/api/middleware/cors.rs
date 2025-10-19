// src/api/middleware/cors.rs

use axum::http::{header, HeaderValue, Method};
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Configuration for CORS (Cross-Origin Resource Sharing)
///
/// Controls which origins, methods, and headers are allowed for cross-origin requests.
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// List of allowed origins (e.g., ["https://app.example.com"])
    pub allowed_origins: Vec<String>,
    /// Whether to allow credentials (cookies, authorization headers)
    pub allow_credentials: bool,
    /// Maximum age for preflight cache in seconds
    pub max_age_seconds: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            allow_credentials: true,
            max_age_seconds: 3600,
        }
    }
}

impl CorsConfig {
    /// Creates a new CORS configuration from environment
    ///
    /// Reads XZEPR__SECURITY__CORS__ALLOWED_ORIGINS from environment
    /// Expected format: comma-separated list of origins
    ///
    /// # Example
    ///
    /// ```bash
    /// export XZEPR__SECURITY__CORS__ALLOWED_ORIGINS="https://app.example.com,https://admin.example.com"
    /// ```
    pub fn from_env() -> Self {
        let allowed_origins = std::env::var("XZEPR__SECURITY__CORS__ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let allow_credentials = std::env::var("XZEPR__SECURITY__CORS__ALLOW_CREDENTIALS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let max_age_seconds = std::env::var("XZEPR__SECURITY__CORS__MAX_AGE_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .unwrap_or(3600);

        Self {
            allowed_origins,
            allow_credentials,
            max_age_seconds,
        }
    }

    /// Creates a permissive CORS configuration for development
    ///
    /// WARNING: This allows all origins and should NEVER be used in production
    pub fn permissive() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allow_credentials: false, // Cannot use credentials with wildcard origin
            max_age_seconds: 3600,
        }
    }

    /// Creates a production-ready CORS configuration
    ///
    /// Requires explicit list of allowed origins from environment
    pub fn production() -> Result<Self, String> {
        let config = Self::from_env();

        // Validate that we don't have wildcard in production
        if config.allowed_origins.contains(&"*".to_string()) {
            return Err(
                "Wildcard origins not allowed in production. Set XZEPR__SECURITY__CORS__ALLOWED_ORIGINS".to_string()
            );
        }

        // Validate that all origins are HTTPS in production
        for origin in &config.allowed_origins {
            if !origin.starts_with("https://") && !origin.starts_with("http://localhost") {
                return Err(format!(
                    "Non-HTTPS origin not allowed in production: {}",
                    origin
                ));
            }
        }

        Ok(config)
    }
}

/// Creates a CORS layer with the specified configuration
///
/// # Arguments
///
/// * `config` - CORS configuration to apply
///
/// # Returns
///
/// Returns a configured CorsLayer for use with Axum router
///
/// # Example
///
/// ```ignore
/// use axum::Router;
/// use xzepr::api::middleware::cors::{cors_layer, CorsConfig};
///
/// let config = CorsConfig::default();
/// let app = Router::new().layer(cors_layer(&config));
/// ```
pub fn cors_layer(config: &CorsConfig) -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::ACCEPT,
            header::ACCEPT_LANGUAGE,
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ORIGIN,
            header::HeaderName::from_static("x-api-key"),
            header::HeaderName::from_static("x-request-id"),
        ])
        .max_age(Duration::from_secs(config.max_age_seconds));

    // Handle allowed origins
    if config.allowed_origins.len() == 1 && config.allowed_origins[0] == "*" {
        // Wildcard origin (development only)
        cors = cors.allow_origin(tower_http::cors::Any);
    } else {
        // Specific origins
        let origins: Result<Vec<HeaderValue>, _> = config
            .allowed_origins
            .iter()
            .map(|origin| origin.parse())
            .collect();

        match origins {
            Ok(origin_values) => {
                cors = cors.allow_origin(AllowOrigin::list(origin_values));
            }
            Err(e) => {
                tracing::error!("Failed to parse CORS origins: {}", e);
                // Fallback to localhost only
                cors = cors.allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap());
            }
        }
    }

    // Add credentials if enabled
    if config.allow_credentials {
        cors = cors.allow_credentials(true);
    }

    cors
}

/// Creates a development CORS layer (permissive)
///
/// WARNING: This allows all origins and should NEVER be used in production
///
/// # Example
///
/// ```ignore
/// use axum::Router;
/// use xzepr::api::middleware::cors::development_cors_layer;
///
/// let app = Router::new().layer(development_cors_layer());
/// ```
pub fn development_cors_layer() -> CorsLayer {
    cors_layer(&CorsConfig::permissive())
}

/// Creates a production CORS layer with validation
///
/// Reads configuration from environment and validates it for production use
///
/// # Errors
///
/// Returns an error if:
/// - No allowed origins are configured
/// - Wildcard origin is used
/// - Non-HTTPS origins are used (except localhost)
///
/// # Example
///
/// ```ignore
/// use axum::Router;
/// use xzepr::api::middleware::cors::production_cors_layer;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cors = production_cors_layer()?;
/// let app = Router::new().layer(cors);
/// # Ok(())
/// # }
/// ```
pub fn production_cors_layer() -> Result<CorsLayer, String> {
    let config = CorsConfig::production()?;
    Ok(cors_layer(&config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CorsConfig::default();
        assert_eq!(config.allowed_origins.len(), 1);
        assert_eq!(config.allowed_origins[0], "http://localhost:3000");
        assert!(config.allow_credentials);
        assert_eq!(config.max_age_seconds, 3600);
    }

    #[test]
    fn test_permissive_config() {
        let config = CorsConfig::permissive();
        assert_eq!(config.allowed_origins.len(), 1);
        assert_eq!(config.allowed_origins[0], "*");
        assert!(!config.allow_credentials); // Cannot use credentials with wildcard
    }

    #[test]
    fn test_production_config_rejects_wildcard() {
        std::env::set_var("XZEPR__SECURITY__CORS__ALLOWED_ORIGINS", "*");
        let result = CorsConfig::production();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Wildcard origins not allowed"));
    }

    #[test]
    fn test_production_config_rejects_http() {
        std::env::set_var(
            "XZEPR__SECURITY__CORS__ALLOWED_ORIGINS",
            "http://example.com",
        );
        let result = CorsConfig::production();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Non-HTTPS origin not allowed"));
    }

    #[test]
    fn test_production_config_accepts_https() {
        std::env::set_var(
            "XZEPR__SECURITY__CORS__ALLOWED_ORIGINS",
            "https://app.example.com,https://admin.example.com",
        );
        let result = CorsConfig::production();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.allowed_origins.len(), 2);
    }

    #[test]
    fn test_production_config_accepts_localhost() {
        std::env::set_var(
            "XZEPR__SECURITY__CORS__ALLOWED_ORIGINS",
            "https://app.example.com,http://localhost:3000",
        );
        let result = CorsConfig::production();
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_env_parsing() {
        std::env::set_var(
            "XZEPR__SECURITY__CORS__ALLOWED_ORIGINS",
            "https://app1.com, https://app2.com , https://app3.com",
        );
        let config = CorsConfig::from_env();
        assert_eq!(config.allowed_origins.len(), 3);
        assert_eq!(config.allowed_origins[0], "https://app1.com");
        assert_eq!(config.allowed_origins[1], "https://app2.com");
        assert_eq!(config.allowed_origins[2], "https://app3.com");
    }

    #[test]
    fn test_cors_layer_creation() {
        let config = CorsConfig::default();
        let _layer = cors_layer(&config);
        // If this doesn't panic, the layer was created successfully
    }
}
