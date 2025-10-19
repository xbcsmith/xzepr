// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/infrastructure/config.rs

use serde::Deserialize;

use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub tls: TlsConfig,
    pub kafka: KafkaConfig,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_https: bool,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    // Legacy fields (deprecated)
    #[deprecated(note = "Use jwt.secret_key instead")]
    pub jwt_secret: Option<String>,
    #[deprecated(note = "Use jwt.access_token_expiration_seconds instead")]
    pub jwt_expiration_hours: Option<i64>,

    // JWT configuration
    pub jwt: JwtAuthConfig,

    // Authentication providers
    pub enable_local_auth: bool,
    pub enable_oidc: bool,
    pub keycloak: Option<KeycloakConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtAuthConfig {
    /// Access token expiration in seconds (default: 900 = 15 minutes)
    #[serde(default = "default_access_token_expiration")]
    pub access_token_expiration_seconds: i64,

    /// Refresh token expiration in seconds (default: 604800 = 7 days)
    #[serde(default = "default_refresh_token_expiration")]
    pub refresh_token_expiration_seconds: i64,

    /// Token issuer
    #[serde(default = "default_issuer")]
    pub issuer: String,

    /// Token audience
    #[serde(default = "default_audience")]
    pub audience: String,

    /// Algorithm: "RS256" or "HS256"
    #[serde(default = "default_algorithm")]
    pub algorithm: String,

    /// Private key path for RS256 (PEM format)
    pub private_key_path: Option<String>,

    /// Public key path for RS256 (PEM format)
    pub public_key_path: Option<String>,

    /// Secret key for HS256 (not recommended for production)
    pub secret_key: Option<String>,

    /// Enable token rotation on refresh
    #[serde(default = "default_enable_rotation")]
    pub enable_token_rotation: bool,

    /// Clock skew tolerance in seconds
    #[serde(default = "default_leeway")]
    pub leeway_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct KeycloakConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

#[derive(Debug, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            // Start with default values
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8443)?
            .set_default("server.enable_https", true)?
            .set_default("auth.enable_local_auth", true)?
            .set_default("auth.enable_oidc", false)?
            .set_default("auth.jwt.access_token_expiration_seconds", 900)?
            .set_default("auth.jwt.refresh_token_expiration_seconds", 604800)?
            .set_default("auth.jwt.issuer", "xzepr")?
            .set_default("auth.jwt.audience", "xzepr-api")?
            .set_default("auth.jwt.algorithm", "RS256")?
            .set_default("auth.jwt.enable_token_rotation", true)?
            .set_default("auth.jwt.leeway_seconds", 60)?
            .set_default(
                "database.url",
                "postgres://xzepr:password@localhost:5432/xzepr",
            )?
            .set_default("kafka.brokers", "localhost:9092")?;

        // Add configuration file if it exists
        builder = builder.add_source(File::with_name("config/default").required(false));

        // Add environment-specific config
        let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".into());
        builder = builder.add_source(File::with_name(&format!("config/{}", env)).required(false));

        // Override with environment variables
        builder = builder.add_source(Environment::with_prefix("XZEPR").separator("__"));

        builder.build()?.try_deserialize()
    }
}

// Default value functions for JWT config
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

fn default_algorithm() -> String {
    "RS256".to_string()
}

fn default_enable_rotation() -> bool {
    true
}

fn default_leeway() -> u64 {
    60 // 1 minute
}
