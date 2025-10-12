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
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub enable_local_auth: bool,
    pub enable_oidc: bool,
    pub keycloak: Option<KeycloakConfig>,
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
            .set_default("auth.jwt_expiration_hours", 24)?
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
