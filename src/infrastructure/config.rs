// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use serde::Deserialize;
use thiserror::Error;

use config::{Config, ConfigError, Environment, File};

use crate::infrastructure::messaging::config::KafkaAuthConfig;
use crate::infrastructure::security_config::{SecurityConfig, SecurityConfigError};
use crate::opa::types::OpaConfig;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub tls: TlsConfig,
    pub kafka: KafkaConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub graphql: GraphqlConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opa: Option<OpaConfig>,
}

/// Database connection configuration.
///
/// The `Debug` implementation redacts the password from the connection URL
/// so that it cannot be accidentally exposed via logging.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL.
    pub url: String,
    /// Maximum number of database connections.
    #[serde(default = "default_database_max_connections")]
    pub max_connections: u32,
    /// Minimum number of database connections.
    #[serde(default = "default_database_min_connections")]
    pub min_connections: u32,
    /// Connection timeout in seconds.
    #[serde(default = "default_database_connection_timeout_seconds")]
    pub connection_timeout_seconds: u64,
}

impl fmt::Debug for DatabaseConfig {
    /// Formats the config, masking the password component of the URL.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &mask_password(&self.url))
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .field(
                "connection_timeout_seconds",
                &self.connection_timeout_seconds,
            )
            .finish()
    }
}

#[derive(Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct KafkaConfig {
    pub brokers: String,
    #[serde(default = "default_kafka_topic")]
    pub default_topic: String,
    #[serde(default = "default_kafka_partitions")]
    pub default_topic_partitions: i32,
    #[serde(default = "default_kafka_replication_factor")]
    pub default_topic_replication_factor: i32,
    /// Optional authentication configuration
    /// Can be loaded from YAML config or environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<KafkaAuthConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_https: bool,
    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: u64,
}

/// Authentication configuration including JWT settings and provider options.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    // JWT configuration
    pub jwt: JwtAuthConfig,

    // Authentication providers
    pub enable_local_auth: bool,
    pub enable_oidc: bool,
    pub keycloak: Option<KeycloakConfig>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
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

impl fmt::Debug for JwtAuthConfig {
    /// Formats the config, showing `[REDACTED]` for `secret_key` if present.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secret_key_display: &str = if self.secret_key.is_some() {
            "[REDACTED]"
        } else {
            "None"
        };
        f.debug_struct("JwtAuthConfig")
            .field(
                "access_token_expiration_seconds",
                &self.access_token_expiration_seconds,
            )
            .field(
                "refresh_token_expiration_seconds",
                &self.refresh_token_expiration_seconds,
            )
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("algorithm", &self.algorithm)
            .field("private_key_path", &self.private_key_path)
            .field("public_key_path", &self.public_key_path)
            .field("secret_key", &secret_key_display)
            .field("enable_token_rotation", &self.enable_token_rotation)
            .field("leeway_seconds", &self.leeway_seconds)
            .finish()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeycloakConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    #[serde(default)]
    pub allowed_redirect_hosts: Vec<String>,
    #[serde(default = "default_oidc_session_ttl")]
    pub session_ttl_seconds: u64,
    #[serde(default = "default_oidc_max_sessions")]
    pub max_sessions_per_user: usize,
}

impl fmt::Debug for KeycloakConfig {
    /// Formats the config, showing `[REDACTED]` for `client_secret`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeycloakConfig")
            .field("issuer_url", &self.issuer_url)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("redirect_url", &self.redirect_url)
            .field("allowed_redirect_hosts", &self.allowed_redirect_hosts)
            .field("session_ttl_seconds", &self.session_ttl_seconds)
            .field("max_sessions_per_user", &self.max_sessions_per_user)
            .finish()
    }
}

/// OIDC session configuration.
///
/// Controls the lifetime and concurrency limits for OIDC-authenticated sessions.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OidcSessionConfig {
    /// Session time-to-live in seconds (default: 3600 = 1 hour).
    #[serde(default = "default_oidc_session_ttl")]
    pub session_ttl_seconds: u64,
    /// Maximum number of concurrent sessions per user (default: 10).
    #[serde(default = "default_oidc_max_sessions")]
    pub max_sessions_per_user: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

/// GraphQL security configuration loaded from runtime settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlConfig {
    /// Maximum GraphQL query complexity.
    #[serde(default = "default_graphql_max_complexity")]
    pub max_complexity: usize,
    /// Maximum GraphQL query depth.
    #[serde(default = "default_graphql_max_depth")]
    pub max_depth: usize,
    /// Whether GraphQL complexity and depth limits are enforced.
    #[serde(default = "default_graphql_enforce_complexity")]
    pub enforce_complexity: bool,
}

impl Default for GraphqlConfig {
    fn default() -> Self {
        Self {
            max_complexity: default_graphql_max_complexity(),
            max_depth: default_graphql_max_depth(),
            enforce_complexity: default_graphql_enforce_complexity(),
        }
    }
}

/// Errors produced by GraphQL runtime configuration validation.
#[derive(Error, Debug)]
pub enum GraphqlConfigError {
    /// Maximum query complexity is zero.
    #[error("GraphQL max_complexity cannot be 0")]
    ZeroMaxComplexity,
    /// Maximum query depth is zero.
    #[error("GraphQL max_depth cannot be 0")]
    ZeroMaxDepth,
    /// GraphQL complexity enforcement is disabled in production.
    #[error("GraphQL complexity enforcement must be enabled in production")]
    EnforcementDisabled,
}

impl GraphqlConfig {
    /// Validates GraphQL settings for production use.
    ///
    /// # Errors
    ///
    /// Returns `GraphqlConfigError` if complexity limits are zero or disabled.
    pub fn validate_production(&self) -> Result<(), GraphqlConfigError> {
        if self.max_complexity == 0 {
            return Err(GraphqlConfigError::ZeroMaxComplexity);
        }
        if self.max_depth == 0 {
            return Err(GraphqlConfigError::ZeroMaxDepth);
        }
        if !self.enforce_complexity {
            return Err(GraphqlConfigError::EnforcementDisabled);
        }
        Ok(())
    }
}

/// Errors produced when validating production runtime settings.
#[derive(Error, Debug)]
pub enum SettingsValidationError {
    /// HTTPS is disabled for the production server.
    #[error("HTTPS must be enabled in production")]
    HttpsRequired,
    /// The database URL contains an insecure default or placeholder password.
    #[error("Database URL contains an insecure default or placeholder password")]
    InsecureDatabaseUrl,
    /// The configured JWT algorithm is not supported.
    #[error("Unsupported JWT algorithm: {algorithm}")]
    UnsupportedJwtAlgorithm {
        /// The unsupported algorithm value.
        algorithm: String,
    },
    /// HS256 is not allowed for production signing.
    #[error("JWT algorithm HS256 is not allowed in production")]
    Hs256NotAllowed,
    /// RS256 is missing a private key path.
    #[error("RS256 requires auth.jwt.private_key_path in production")]
    MissingJwtPrivateKey,
    /// RS256 is missing a public key path.
    #[error("RS256 requires auth.jwt.public_key_path in production")]
    MissingJwtPublicKey,
    /// OIDC is enabled without a provider configuration.
    #[error("OIDC is enabled but auth.keycloak is not configured")]
    OidcMissingProvider,
    /// OIDC issuer URL does not use HTTPS.
    #[error("OIDC issuer URL must use HTTPS in production")]
    OidcIssuerNotHttps,
    /// OIDC redirect URL does not use HTTPS.
    #[error("OIDC redirect URL must use HTTPS in production")]
    OidcRedirectNotHttps,
    /// OIDC redirect host allowlist is empty.
    #[error("OIDC allowed_redirect_hosts cannot be empty in production")]
    OidcRedirectAllowlistEmpty,
    /// OIDC redirect host is not allowlisted.
    #[error("OIDC redirect host '{host}' is not in allowed_redirect_hosts")]
    OidcRedirectHostNotAllowed {
        /// The redirect URL host.
        host: String,
    },
    /// OIDC client ID is empty.
    #[error("OIDC client_id cannot be empty in production")]
    OidcClientIdEmpty,
    /// OIDC client secret is empty or placeholder text.
    #[error("OIDC client_secret cannot be empty or a placeholder in production")]
    OidcClientSecretInvalid,
    /// TLS certificate path is empty.
    #[error("TLS cert_path cannot be empty in production")]
    EmptyTlsCertPath,
    /// TLS private key path is empty.
    #[error("TLS key_path cannot be empty in production")]
    EmptyTlsKeyPath,
    /// Security configuration is invalid.
    #[error(transparent)]
    Security(#[from] SecurityConfigError),
    /// OPA configuration is invalid.
    #[error("OPA configuration error: {0}")]
    Opa(String),
    /// GraphQL configuration is invalid.
    #[error(transparent)]
    Graphql(#[from] GraphqlConfigError),
}

impl Settings {
    /// Loads configuration from files and environment variables.
    ///
    /// Configuration is layered in this order (later sources override earlier):
    /// 1. Built-in defaults
    /// 2. `config/default.yaml`
    /// 3. `config/{RUST_ENV}.yaml` (where `RUST_ENV` defaults to `"development"`)
    /// 4. Environment variables with prefix `XZEPR` and separator `__`
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if the configuration cannot be built or deserialized.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use xzepr::infrastructure::config::Settings;
    ///
    /// let settings = Settings::new().expect("configuration should load");
    /// println!("Listening on {}:{}", settings.server.host, settings.server.port);
    /// ```
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            // Start with default values
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8443)?
            .set_default("server.enable_https", true)?
            .set_default("server.request_timeout_seconds", 30)?
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
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 1)?
            .set_default("database.connection_timeout_seconds", 30)?
            .set_default("kafka.brokers", "localhost:9092")?
            .set_default("kafka.default_topic", "xzepr.dev.events")?
            .set_default("kafka.default_topic_partitions", 3)?
            .set_default("kafka.default_topic_replication_factor", 1)?
            .set_default("opa.enabled", false)?
            .set_default("opa.url", "http://localhost:8181")?
            .set_default("opa.timeout_seconds", 5)?
            .set_default("opa.policy_path", "/v1/data/xzepr/rbac/allow")?
            .set_default("opa.cache_ttl_seconds", 300)?
            .set_default("opa.fail_safe_mode", "fail_closed")?
            .set_default("graphql.max_complexity", 100)?
            .set_default("graphql.max_depth", 10)?
            .set_default("graphql.enforce_complexity", true)?;

        // Add configuration file if it exists
        builder = builder.add_source(File::with_name("config/default").required(false));

        // Add environment-specific config
        let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".into());
        builder = builder.add_source(File::with_name(&format!("config/{}", env)).required(false));

        // Override with environment variables
        builder = builder.add_source(Environment::with_prefix("XZEPR").separator("__"));

        builder.build()?.try_deserialize()
    }

    /// Validates that the configuration meets production security requirements.
    ///
    /// This method checks for common misconfigurations that would be unsafe in a
    /// production environment. Critical failures are returned as `Err`; advisories
    /// are emitted via the `tracing` infrastructure as warnings.
    ///
    /// # Checks performed
    ///
    /// - HTTPS must be enabled when `RUST_ENV=production`.
    /// - Database URL must not contain a default insecure password (`:password@`).
    /// - JWT algorithm should be RS256 in production; HS256 triggers a warning.
    /// - If OIDC is enabled, the Keycloak issuer URL must use `https://`.
    /// - If OIDC is enabled, the Keycloak redirect URL must use `https://`.
    ///
    /// # Errors
    ///
    /// Returns `SettingsValidationError` when a critical misconfiguration is
    /// detected.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use xzepr::infrastructure::config::Settings;
    ///
    /// let settings = Settings::new().expect("configuration should load");
    /// settings
    ///     .validate_production()
    ///     .expect("production checks should pass");
    /// ```
    pub fn validate_production(&self) -> Result<(), SettingsValidationError> {
        if !self.server.enable_https {
            return Err(SettingsValidationError::HttpsRequired);
        }

        if self.database.url.contains(":password@") || self.database.url.contains("CHANGE_ME") {
            return Err(SettingsValidationError::InsecureDatabaseUrl);
        }

        self.validate_production_jwt()?;
        self.validate_production_oidc()?;
        self.validate_production_tls()?;
        self.security.validate_production()?;
        self.graphql.validate_production()?;

        if let Some(opa) = &self.opa {
            opa.validate_production()
                .map_err(|e| SettingsValidationError::Opa(e.to_string()))?;
        }

        Ok(())
    }

    fn validate_production_jwt(&self) -> Result<(), SettingsValidationError> {
        match self.auth.jwt.algorithm.to_uppercase().as_str() {
            "RS256" => {
                if self
                    .auth
                    .jwt
                    .private_key_path
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    return Err(SettingsValidationError::MissingJwtPrivateKey);
                }
                if self
                    .auth
                    .jwt
                    .public_key_path
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    return Err(SettingsValidationError::MissingJwtPublicKey);
                }
            }
            "HS256" => return Err(SettingsValidationError::Hs256NotAllowed),
            algorithm => {
                return Err(SettingsValidationError::UnsupportedJwtAlgorithm {
                    algorithm: algorithm.to_string(),
                })
            }
        }

        Ok(())
    }

    fn validate_production_oidc(&self) -> Result<(), SettingsValidationError> {
        if !self.auth.enable_oidc {
            return Ok(());
        }

        let keycloak = self
            .auth
            .keycloak
            .as_ref()
            .ok_or(SettingsValidationError::OidcMissingProvider)?;

        if keycloak.client_id.trim().is_empty() {
            return Err(SettingsValidationError::OidcClientIdEmpty);
        }
        if keycloak.client_secret.trim().is_empty() || keycloak.client_secret.contains("CHANGE_ME")
        {
            return Err(SettingsValidationError::OidcClientSecretInvalid);
        }
        if !keycloak.issuer_url.starts_with("https://") {
            return Err(SettingsValidationError::OidcIssuerNotHttps);
        }
        if !keycloak.redirect_url.starts_with("https://") {
            return Err(SettingsValidationError::OidcRedirectNotHttps);
        }
        if keycloak.allowed_redirect_hosts.is_empty() {
            return Err(SettingsValidationError::OidcRedirectAllowlistEmpty);
        }

        let redirect_host = extract_host(&keycloak.redirect_url).ok_or_else(|| {
            SettingsValidationError::OidcRedirectHostNotAllowed {
                host: keycloak.redirect_url.clone(),
            }
        })?;

        if !keycloak
            .allowed_redirect_hosts
            .iter()
            .any(|allowed| allowed == &redirect_host)
        {
            return Err(SettingsValidationError::OidcRedirectHostNotAllowed {
                host: redirect_host,
            });
        }

        Ok(())
    }

    fn validate_production_tls(&self) -> Result<(), SettingsValidationError> {
        if self.tls.cert_path.trim().is_empty() {
            return Err(SettingsValidationError::EmptyTlsCertPath);
        }
        if self.tls.key_path.trim().is_empty() {
            return Err(SettingsValidationError::EmptyTlsKeyPath);
        }
        Ok(())
    }
}

fn extract_host(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let host = rest.split('/').next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

fn default_request_timeout_seconds() -> u64 {
    30
}

fn default_database_max_connections() -> u32 {
    10
}

fn default_database_min_connections() -> u32 {
    1
}

fn default_database_connection_timeout_seconds() -> u64 {
    30
}

fn default_graphql_max_complexity() -> usize {
    100
}

fn default_graphql_max_depth() -> usize {
    10
}

fn default_graphql_enforce_complexity() -> bool {
    true
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

// Default value functions for Kafka config
fn default_kafka_topic() -> String {
    "xzepr.dev.events".to_string()
}

fn default_kafka_partitions() -> i32 {
    3
}

fn default_kafka_replication_factor() -> i32 {
    1
}

// Default value functions for OIDC session config

/// Returns the default OIDC session TTL in seconds (1 hour).
fn default_oidc_session_ttl() -> u64 {
    3600
}

/// Returns the default maximum number of concurrent sessions per user.
fn default_oidc_max_sessions() -> usize {
    10
}

/// Masks the password component of a database connection URL for safe logging.
///
/// # Arguments
///
/// * `url` - The database connection URL string.
///
/// # Returns
///
/// A copy of the URL with the password replaced by `***`, or the original
/// URL unchanged if no password component is detected.
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(proto_end) = url.find("://") {
            let proto = &url[..proto_end + 3];
            let after_at = &url[at_pos..];
            if let Some(colon_pos) = url[proto_end + 3..at_pos].find(':') {
                let username = &url[proto_end + 3..proto_end + 3 + colon_pos];
                return format!("{}{}:***{}", proto, username, after_at);
            }
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::messaging::config::{
        KafkaAuthConfig, SaslConfig, SaslMechanism, SecurityProtocol, SslConfig,
    };
    use std::env;
    use std::sync::Mutex;

    // Mutex to prevent concurrent test execution that modifies environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn valid_production_security_config() -> SecurityConfig {
        let mut security = SecurityConfig::default();
        security.cors.allowed_origins = vec!["https://app.example.com".to_string()];
        security.rate_limit.use_redis = true;
        security.rate_limit.redis_url = Some("redis://redis:6379".to_string());
        security
    }

    fn valid_production_settings_yaml() -> &'static str {
        r#"
server:
  host: "0.0.0.0"
  port: 8443
  enable_https: true
  request_timeout_seconds: 30

database:
  url: "postgres://xzepr:secure-password@postgres:5432/xzepr"
  max_connections: 20
  min_connections: 5
  connection_timeout_seconds: 30

auth:
  enable_local_auth: true
  enable_oidc: true
  jwt:
    access_token_expiration_seconds: 900
    refresh_token_expiration_seconds: 604800
    issuer: "xzepr-production"
    audience: "xzepr-api"
    algorithm: "RS256"
    private_key_path: "/etc/xzepr/keys/jwt_rsa"
    public_key_path: "/etc/xzepr/keys/jwt_rsa.pub"
    secret_key: null
    enable_token_rotation: true
    leeway_seconds: 60
  keycloak:
    issuer_url: "https://keycloak.example.com/realms/xzepr"
    client_id: "xzepr-client"
    client_secret: "real-client-secret"
    redirect_url: "https://xzepr.example.com/auth/callback"
    allowed_redirect_hosts:
      - "xzepr.example.com"
    session_ttl_seconds: 3600
    max_sessions_per_user: 10

tls:
  cert_path: "/etc/xzepr/tls/cert.pem"
  key_path: "/etc/xzepr/tls/key.pem"

kafka:
  brokers: "redpanda-0:9092"
  default_topic: "xzepr.prod.events"
  default_topic_partitions: 3
  default_topic_replication_factor: 1

opa:
  enabled: true
  url: "https://opa.example.com:8181"
  timeout_seconds: 5
  policy_path: "/v1/data/xzepr/rbac/allow"
  bundle_url: "https://bundle-server.example.com/bundles/xzepr-rbac.tar.gz"
  cache_ttl_seconds: 300
  allowed_hosts:
    - "opa.example.com:8181"
    - "bundle-server.example.com"
  fail_safe_mode: "fail_closed"

security:
  cors:
    allowed_origins:
      - "https://app.example.com"
    allow_credentials: true
    max_age_seconds: 3600
  rate_limit:
    anonymous_rpm: 10
    authenticated_rpm: 100
    admin_rpm: 1000
    per_endpoint:
      /api/v1/auth/login: 5
    use_redis: true
    redis_url: "redis://redis:6379"
  validation:
    max_body_size: 1048576
    max_string_length: 5000
    max_array_length: 500
    strict_mode: true
  headers:
    enable_csp: true
    csp_directives: "default-src 'self'"
    enable_hsts: true
    hsts_max_age: 63072000
    hsts_include_subdomains: true
    hsts_preload: true
  monitoring:
    metrics_enabled: true
    tracing_enabled: true
    structured_logging: true
    log_level: "info"
    json_logs: true
    jaeger_endpoint: "http://jaeger:14268/api/traces"
    metrics_port: 9090

graphql:
  max_complexity: 50
  max_depth: 8
  enforce_complexity: true
"#
    }

    fn valid_production_settings() -> Settings {
        serde_yaml::from_str(valid_production_settings_yaml())
            .expect("valid production settings test fixture should deserialize")
    }

    /// Clean up environment variables used in tests
    fn cleanup_env_vars() {
        env::remove_var("XZEPR__KAFKA__AUTH__SECURITY_PROTOCOL");
        env::remove_var("XZEPR__KAFKA__AUTH__SASL__MECHANISM");
        env::remove_var("XZEPR__KAFKA__AUTH__SASL__USERNAME");
        env::remove_var("XZEPR__KAFKA__AUTH__SASL__PASSWORD");
        env::remove_var("XZEPR__KAFKA__AUTH__SSL__CA_LOCATION");
        env::remove_var("XZEPR__KAFKA__AUTH__SSL__CERT_LOCATION");
        env::remove_var("XZEPR__KAFKA__AUTH__SSL__KEY_LOCATION");
        env::remove_var("XZEPR__KAFKA__BROKERS");
        env::remove_var("XZEPR__KAFKA__DEFAULT_TOPIC");
    }

    #[test]
    fn test_kafka_config_deserialize_without_auth() {
        let yaml = r#"
            brokers: "localhost:9092"
            default_topic: "test.events"
            default_topic_partitions: 3
            default_topic_replication_factor: 1
        "#;

        let config: KafkaConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.brokers, "localhost:9092");
        assert_eq!(config.default_topic, "test.events");
        assert_eq!(config.default_topic_partitions, 3);
        assert_eq!(config.default_topic_replication_factor, 1);
        assert!(config.auth.is_none());
    }

    #[test]
    fn test_kafka_config_deserialize_with_plaintext_auth() {
        let yaml = r#"
            brokers: "localhost:9092"
            default_topic: "test.events"
            default_topic_partitions: 3
            default_topic_replication_factor: 1
            auth:
              security_protocol: PLAINTEXT
        "#;

        let config: KafkaConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.auth.is_some());
        let auth = config.auth.unwrap();
        assert_eq!(auth.security_protocol, SecurityProtocol::Plaintext);
        assert!(auth.sasl_config.is_none());
        assert!(auth.ssl_config.is_none());
    }

    #[test]
    fn test_kafka_config_deserialize_with_sasl_ssl_auth() {
        let yaml = r#"
            brokers: "kafka.example.com:9093"
            default_topic: "prod.events"
            default_topic_partitions: 6
            default_topic_replication_factor: 3
            auth:
              security_protocol: SASL_SSL
              sasl_config:
                mechanism: SCRAM-SHA-256
                username: "kafka_user"
                password: "secure_password"
              ssl_config:
                ca_location: "/path/to/ca.pem"
                certificate_location: "/path/to/cert.pem"
                key_location: "/path/to/key.pem"
        "#;

        let config: KafkaConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.brokers, "kafka.example.com:9093");
        assert_eq!(config.default_topic, "prod.events");

        assert!(config.auth.is_some());
        let auth = config.auth.unwrap();
        assert_eq!(auth.security_protocol, SecurityProtocol::SaslSsl);

        assert!(auth.sasl_config.is_some());
        let sasl = auth.sasl_config.unwrap();
        assert_eq!(sasl.mechanism, SaslMechanism::ScramSha256);
        assert_eq!(sasl.username, "kafka_user");
        assert_eq!(sasl.password, "secure_password");

        assert!(auth.ssl_config.is_some());
        let ssl = auth.ssl_config.unwrap();
        assert_eq!(ssl.ca_location, Some("/path/to/ca.pem".to_string()));
        assert_eq!(
            ssl.certificate_location,
            Some("/path/to/cert.pem".to_string())
        );
        assert_eq!(ssl.key_location, Some("/path/to/key.pem".to_string()));
    }

    #[test]
    fn test_kafka_config_deserialize_with_scram_sha512() {
        let yaml = r#"
            brokers: "kafka.example.com:9093"
            default_topic: "prod.events"
            default_topic_partitions: 6
            default_topic_replication_factor: 3
            auth:
              security_protocol: SASL_SSL
              sasl_config:
                mechanism: SCRAM-SHA-512
                username: "admin"
                password: "admin_pass"
              ssl_config:
                ca_location: "/certs/ca.pem"
        "#;

        let config: KafkaConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.auth.is_some());
        let auth = config.auth.unwrap();

        assert!(auth.sasl_config.is_some());
        let sasl = auth.sasl_config.unwrap();
        assert_eq!(sasl.mechanism, SaslMechanism::ScramSha512);
        assert_eq!(sasl.username, "admin");
    }

    #[test]
    fn test_kafka_config_serialize_roundtrip() {
        let auth = KafkaAuthConfig::new(
            SecurityProtocol::SaslSsl,
            Some(SaslConfig::new(
                SaslMechanism::ScramSha256,
                "user".to_string(),
                "pass".to_string(),
            )),
            Some(SslConfig::new(Some("/ca.pem".to_string()), None, None)),
        );

        let config = KafkaConfig {
            brokers: "localhost:9092".to_string(),
            default_topic: "events".to_string(),
            default_topic_partitions: 3,
            default_topic_replication_factor: 1,
            auth: Some(auth),
        };

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Deserialize back
        let deserialized: KafkaConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(deserialized.brokers, config.brokers);
        assert_eq!(deserialized.default_topic, config.default_topic);
        assert!(deserialized.auth.is_some());

        let auth = deserialized.auth.unwrap();
        assert_eq!(auth.security_protocol, SecurityProtocol::SaslSsl);
        assert!(auth.sasl_config.is_some());
        assert!(auth.ssl_config.is_some());
    }

    #[test]
    fn test_kafka_config_from_env_without_auth() {
        let _lock = ENV_MUTEX.lock().unwrap();
        cleanup_env_vars();

        env::set_var("XZEPR__KAFKA__BROKERS", "env-broker:9092");
        env::set_var("XZEPR__KAFKA__DEFAULT_TOPIC", "env.topic");

        let settings = Settings::new().unwrap();

        assert_eq!(settings.kafka.brokers, "env-broker:9092");
        assert_eq!(settings.kafka.default_topic, "env.topic");
        assert!(settings.kafka.auth.is_none());

        cleanup_env_vars();
    }

    #[test]
    fn test_settings_new_with_defaults() {
        let _lock = ENV_MUTEX.lock().unwrap();
        cleanup_env_vars();

        let settings = Settings::new().unwrap();

        // Verify Kafka defaults (from config/default.yaml)
        assert_eq!(settings.kafka.brokers, "localhost:19092");
        assert_eq!(settings.kafka.default_topic, "xzepr.dev.events");
        assert_eq!(settings.kafka.default_topic_partitions, 3);
        assert_eq!(settings.kafka.default_topic_replication_factor, 1);
        assert!(settings.kafka.auth.is_none());

        cleanup_env_vars();
    }

    #[test]
    fn test_kafka_config_with_minimal_fields() {
        let yaml = r#"
            brokers: "minimal:9092"
            default_topic: "minimal.events"
        "#;

        let config: KafkaConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.brokers, "minimal:9092");
        assert_eq!(config.default_topic, "minimal.events");
        // These fields should use defaults when deserializing via Settings::new()
        // but when deserializing directly, they are required or will error
    }

    #[test]
    fn test_kafka_config_auth_validation() {
        // Valid SASL/SSL config (validation will pass even if files don't exist for cert paths)
        let yaml = r#"
            brokers: "localhost:9092"
            default_topic: "test"
            default_topic_partitions: 1
            default_topic_replication_factor: 1
            auth:
              security_protocol: SASL_SSL
              sasl_config:
                mechanism: SCRAM-SHA-256
                username: "user"
                password: "pass"
        "#;

        let config: KafkaConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.auth.is_some());

        // Validate the auth config (without SSL cert paths, validation should pass)
        let auth = config.auth.unwrap();
        let validation_result = auth.validate();
        assert!(
            validation_result.is_ok(),
            "Validation failed: {:?}",
            validation_result
        );
    }

    #[test]
    fn test_kafka_config_default_values() {
        assert_eq!(default_kafka_topic(), "xzepr.dev.events");
        assert_eq!(default_kafka_partitions(), 3);
        assert_eq!(default_kafka_replication_factor(), 1);
    }

    #[test]
    fn test_debug_database_config_redacts_password() {
        let config = DatabaseConfig {
            url: "postgres://user:pass@host/db".to_string(),
            max_connections: 10,
            min_connections: 1,
            connection_timeout_seconds: 30,
        };
        let debug_str = format!("{:?}", config);
        assert!(
            !debug_str.contains("pass"),
            "Debug output must not expose the database password"
        );
        assert!(
            debug_str.contains("***"),
            "Debug output should show *** in place of the password"
        );
    }

    #[test]
    fn test_debug_jwt_auth_config_redacts_secret_key() {
        let config = JwtAuthConfig {
            access_token_expiration_seconds: 900,
            refresh_token_expiration_seconds: 604800,
            issuer: "xzepr".to_string(),
            audience: "xzepr-api".to_string(),
            algorithm: "RS256".to_string(),
            private_key_path: None,
            public_key_path: None,
            secret_key: Some("super-secret-key".to_string()),
            enable_token_rotation: true,
            leeway_seconds: 60,
        };
        let debug_str = format!("{:?}", config);
        assert!(
            !debug_str.contains("super-secret-key"),
            "Debug output must not expose the JWT secret key"
        );
        assert!(
            debug_str.contains("[REDACTED]"),
            "Debug output should show [REDACTED] for the secret key"
        );
    }

    #[test]
    fn test_debug_keycloak_config_redacts_client_secret() {
        let config = KeycloakConfig {
            issuer_url: "https://keycloak.example.com/realms/xzepr".to_string(),
            client_id: "xzepr".to_string(),
            client_secret: "my-super-secret".to_string(),
            redirect_url: "https://app.example.com/callback".to_string(),
            allowed_redirect_hosts: vec!["app.example.com".to_string()],
            session_ttl_seconds: 3600,
            max_sessions_per_user: 10,
        };
        let debug_str = format!("{:?}", config);
        assert!(
            !debug_str.contains("my-super-secret"),
            "Debug output must not expose the Keycloak client secret"
        );
        assert!(
            debug_str.contains("[REDACTED]"),
            "Debug output should show [REDACTED] for the client secret"
        );
    }

    #[test]
    fn test_validate_production_rejects_insecure_oidc() {
        let settings = Settings {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8443,
                enable_https: true,
                request_timeout_seconds: 30,
            },
            database: DatabaseConfig {
                url: "postgres://user:securepass@host/db".to_string(),
                max_connections: 10,
                min_connections: 1,
                connection_timeout_seconds: 30,
            },
            auth: AuthConfig {
                jwt: JwtAuthConfig {
                    access_token_expiration_seconds: 900,
                    refresh_token_expiration_seconds: 604800,
                    issuer: "xzepr".to_string(),
                    audience: "xzepr-api".to_string(),
                    algorithm: "RS256".to_string(),
                    private_key_path: Some("/keys/private.pem".to_string()),
                    public_key_path: Some("/keys/public.pem".to_string()),
                    secret_key: None,
                    enable_token_rotation: true,
                    leeway_seconds: 60,
                },
                enable_local_auth: true,
                enable_oidc: true,
                keycloak: Some(KeycloakConfig {
                    // http:// issuer should be rejected
                    issuer_url: "http://keycloak.example.com/realms/xzepr".to_string(),
                    client_id: "xzepr".to_string(),
                    client_secret: "secret".to_string(),
                    redirect_url: "https://app.example.com/callback".to_string(),
                    allowed_redirect_hosts: vec!["app.example.com".to_string()],
                    session_ttl_seconds: 3600,
                    max_sessions_per_user: 10,
                }),
            },
            tls: TlsConfig {
                cert_path: "/path/to/cert.pem".to_string(),
                key_path: "/path/to/key.pem".to_string(),
            },
            kafka: KafkaConfig {
                brokers: "localhost:9092".to_string(),
                default_topic: "test.events".to_string(),
                default_topic_partitions: 3,
                default_topic_replication_factor: 1,
                auth: None,
            },
            security: valid_production_security_config(),
            graphql: GraphqlConfig::default(),
            opa: None,
        };

        let result = settings.validate_production();
        assert!(
            result.is_err(),
            "validate_production should reject an http:// OIDC issuer URL"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("HTTPS"),
            "Error message should reference HTTPS: {}",
            err_msg
        );
    }

    #[test]
    fn test_config_yaml_files_do_not_contain_legacy_jwt_fields() {
        let files = [
            include_str!("../../config/default.yaml"),
            include_str!("../../config/development.yaml"),
            include_str!("../../config/production.yaml"),
        ];

        for contents in files {
            assert!(!contents.contains("jwt_secret"));
            assert!(!contents.contains("jwt_expiration_hours"));
        }
    }

    #[test]
    fn test_config_yaml_files_deserialize_into_settings() {
        let files = [
            include_str!("../../config/default.yaml"),
            include_str!("../../config/development.yaml"),
            include_str!("../../config/production.yaml"),
        ];

        for contents in files {
            let settings: Settings = serde_yaml::from_str(contents)
                .expect("configuration file should deserialize into Settings");
            assert!(settings.graphql.max_complexity > 0);
        }
    }

    #[test]
    fn test_production_yaml_deserializes_authoritative_sections() {
        let settings: Settings = serde_yaml::from_str(include_str!("../../config/production.yaml"))
            .expect("production YAML should deserialize");

        assert_eq!(settings.security.cors.allowed_origins.len(), 2);
        assert_eq!(settings.graphql.max_complexity, 50);
        assert_eq!(settings.graphql.max_depth, 8);
        assert!(settings.graphql.enforce_complexity);
        let keycloak = settings.auth.keycloak.as_ref().unwrap();
        assert_eq!(keycloak.allowed_redirect_hosts, vec!["xzepr.example.com"]);
        let opa = settings.opa.as_ref().unwrap();
        assert!(!opa.allowed_hosts.is_empty());
    }

    #[test]
    fn test_production_yaml_rejects_unknown_keys() {
        let yaml = format!("{}\nunknown_key: true\n", valid_production_settings_yaml());
        let result = serde_yaml::from_str::<Settings>(&yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_production_accepts_valid_settings() {
        let settings = valid_production_settings();
        assert!(settings.validate_production().is_ok());
    }

    #[test]
    fn test_validate_production_rejects_hs256() {
        let mut settings = valid_production_settings();
        settings.auth.jwt.algorithm = "HS256".to_string();
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::Hs256NotAllowed)
        ));
    }

    #[test]
    fn test_validate_production_rejects_rs256_without_private_key() {
        let mut settings = valid_production_settings();
        settings.auth.jwt.private_key_path = None;
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::MissingJwtPrivateKey)
        ));
    }

    #[test]
    fn test_validate_production_rejects_wildcard_cors() {
        let mut settings = valid_production_settings();
        settings.security.cors.allowed_origins = vec!["*".to_string()];
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::Security(
                SecurityConfigError::WildcardCorsOrigin
            ))
        ));
    }

    #[test]
    fn test_validate_production_rejects_non_https_cors() {
        let mut settings = valid_production_settings();
        settings.security.cors.allowed_origins = vec!["http://app.example.com".to_string()];
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::Security(
                SecurityConfigError::NonHttpsCorsOrigin { .. }
            ))
        ));
    }

    #[test]
    fn test_validate_production_rejects_missing_redis_url_when_required() {
        let mut settings = valid_production_settings();
        settings.security.rate_limit.redis_url = None;
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::Security(
                SecurityConfigError::MissingRedisUrl
            ))
        ));
    }

    #[test]
    fn test_validate_production_rejects_rs256_without_public_key() {
        let mut settings = valid_production_settings();
        settings.auth.jwt.public_key_path = None;
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::MissingJwtPublicKey)
        ));
    }

    #[test]
    fn test_validate_production_rejects_oidc_http_redirect() {
        let mut settings = valid_production_settings();
        let keycloak = settings.auth.keycloak.as_mut().unwrap();
        keycloak.redirect_url = "http://xzepr.example.com/auth/callback".to_string();
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::OidcRedirectNotHttps)
        ));
    }

    #[test]
    fn test_validate_production_rejects_missing_oidc_redirect_allowlist() {
        let mut settings = valid_production_settings();
        let keycloak = settings.auth.keycloak.as_mut().unwrap();
        keycloak.allowed_redirect_hosts.clear();
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::OidcRedirectAllowlistEmpty)
        ));
    }

    #[test]
    fn test_validate_production_rejects_redirect_host_not_allowlisted() {
        let mut settings = valid_production_settings();
        let keycloak = settings.auth.keycloak.as_mut().unwrap();
        keycloak.allowed_redirect_hosts = vec!["other.example.com".to_string()];
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::OidcRedirectHostNotAllowed { .. })
        ));
    }

    #[test]
    fn test_validate_production_rejects_insecure_opa_url() {
        let mut settings = valid_production_settings();
        let opa = settings.opa.as_mut().unwrap();
        opa.url = "http://opa.example.com:8181".to_string();
        let result = settings.validate_production();
        assert!(matches!(result, Err(SettingsValidationError::Opa(_))));
    }

    #[test]
    fn test_validate_production_rejects_opa_host_not_allowlisted() {
        let mut settings = valid_production_settings();
        let opa = settings.opa.as_mut().unwrap();
        opa.allowed_hosts = vec!["other.example.com".to_string()];
        let result = settings.validate_production();
        assert!(matches!(result, Err(SettingsValidationError::Opa(_))));
    }

    #[test]
    fn test_validate_production_rejects_opa_fail_open() {
        let mut settings = valid_production_settings();
        let opa = settings.opa.as_mut().unwrap();
        opa.fail_safe_mode = crate::opa::types::OpaFailSafeMode::FailOpenDevelopment;
        let result = settings.validate_production();
        assert!(matches!(result, Err(SettingsValidationError::Opa(_))));
    }

    #[test]
    fn test_validate_production_rejects_empty_tls_cert_path() {
        let mut settings = valid_production_settings();
        settings.tls.cert_path.clear();
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::EmptyTlsCertPath)
        ));
    }

    #[test]
    fn test_validate_production_rejects_empty_tls_key_path() {
        let mut settings = valid_production_settings();
        settings.tls.key_path.clear();
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::EmptyTlsKeyPath)
        ));
    }

    #[test]
    fn test_validate_production_rejects_zero_graphql_complexity() {
        let mut settings = valid_production_settings();
        settings.graphql.max_complexity = 0;
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::Graphql(
                GraphqlConfigError::ZeroMaxComplexity
            ))
        ));
    }

    #[test]
    fn test_validate_production_rejects_zero_graphql_depth() {
        let mut settings = valid_production_settings();
        settings.graphql.max_depth = 0;
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::Graphql(
                GraphqlConfigError::ZeroMaxDepth
            ))
        ));
    }

    #[test]
    fn test_validate_production_rejects_disabled_graphql_enforcement() {
        let mut settings = valid_production_settings();
        settings.graphql.enforce_complexity = false;
        let result = settings.validate_production();
        assert!(matches!(
            result,
            Err(SettingsValidationError::Graphql(
                GraphqlConfigError::EnforcementDisabled
            ))
        ));
    }

    #[test]
    fn test_oidc_session_config_defaults() {
        let yaml = "{}";
        let config: OidcSessionConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.session_ttl_seconds, 3600,
            "Default TTL should be 3600 seconds"
        );
        assert_eq!(
            config.max_sessions_per_user, 10,
            "Default max sessions should be 10"
        );
    }
}
