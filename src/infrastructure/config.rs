// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use serde::Deserialize;

use config::{Config, ConfigError, Environment, File};

use crate::infrastructure::messaging::config::KafkaAuthConfig;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub tls: TlsConfig,
    pub kafka: KafkaConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opa: Option<crate::opa::types::OpaConfig>,
}

/// Database connection configuration.
///
/// The `Debug` implementation redacts the password from the connection URL
/// so that it cannot be accidentally exposed via logging.
#[derive(Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL.
    pub url: String,
}

impl fmt::Debug for DatabaseConfig {
    /// Formats the config, masking the password component of the URL.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &mask_password(&self.url))
            .finish()
    }
}

#[derive(Debug, Deserialize, serde::Serialize)]
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
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_https: bool,
}

/// Authentication configuration including JWT settings and provider options.
#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    // JWT configuration
    pub jwt: JwtAuthConfig,

    // Authentication providers
    pub enable_local_auth: bool,
    pub enable_oidc: bool,
    pub keycloak: Option<KeycloakConfig>,
}

#[derive(Deserialize, Clone)]
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
pub struct KeycloakConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

impl fmt::Debug for KeycloakConfig {
    /// Formats the config, showing `[REDACTED]` for `client_secret`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeycloakConfig")
            .field("issuer_url", &self.issuer_url)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("redirect_url", &self.redirect_url)
            .finish()
    }
}

/// OIDC session configuration.
///
/// Controls the lifetime and concurrency limits for OIDC-authenticated sessions.
#[derive(Debug, Clone, Deserialize)]
pub struct OidcSessionConfig {
    /// Session time-to-live in seconds (default: 3600 = 1 hour).
    #[serde(default = "default_oidc_session_ttl")]
    pub session_ttl_seconds: u64,
    /// Maximum number of concurrent sessions per user (default: 10).
    #[serde(default = "default_oidc_max_sessions")]
    pub max_sessions_per_user: usize,
}

#[derive(Debug, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
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
            .set_default("kafka.brokers", "localhost:9092")?
            .set_default("kafka.default_topic", "xzepr.dev.events")?
            .set_default("kafka.default_topic_partitions", 3)?
            .set_default("kafka.default_topic_replication_factor", 1)?
            .set_default("opa.enabled", false)?
            .set_default("opa.url", "http://localhost:8181")?
            .set_default("opa.timeout_seconds", 5)?
            .set_default("opa.policy_path", "/v1/data/xzepr/rbac/allow")?
            .set_default("opa.cache_ttl_seconds", 300)?;

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
    /// Returns `Err(String)` with a descriptive message when a critical
    /// misconfiguration is detected.
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
    pub fn validate_production(&self) -> Result<(), String> {
        let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".into());

        // Hard error: HTTPS must be enabled in production.
        if env == "production" && !self.server.enable_https {
            return Err(
                "HTTPS must be enabled in production. Set server.enable_https = true.".to_string(),
            );
        }

        // Advisory: warn about default or insecure database passwords.
        if self.database.url.contains(":password@") {
            tracing::warn!(
                "Database URL appears to use a default password. \
                 Update the database password before deploying to production."
            );
        }

        // Advisory: warn when using the HS256 symmetric algorithm.
        if self.auth.jwt.algorithm == "HS256" {
            tracing::warn!(
                "JWT algorithm is HS256. RS256 with asymmetric keys is recommended \
                 for production deployments."
            );
        }

        // Hard error: all OIDC endpoints must use HTTPS.
        if self.auth.enable_oidc {
            if let Some(keycloak) = &self.auth.keycloak {
                if !keycloak.issuer_url.starts_with("https://") {
                    return Err(format!(
                        "OIDC issuer URL must use https:// when OIDC is enabled. \
                         Got: {}",
                        keycloak.issuer_url
                    ));
                }
                if !keycloak.redirect_url.starts_with("https://") {
                    return Err(format!(
                        "OIDC redirect URL must use https:// when OIDC is enabled. \
                         Got: {}",
                        keycloak.redirect_url
                    ));
                }
            }
        }

        Ok(())
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
            },
            database: DatabaseConfig {
                url: "postgres://user:securepass@host/db".to_string(),
            },
            auth: AuthConfig {
                jwt: JwtAuthConfig {
                    access_token_expiration_seconds: 900,
                    refresh_token_expiration_seconds: 604800,
                    issuer: "xzepr".to_string(),
                    audience: "xzepr-api".to_string(),
                    algorithm: "RS256".to_string(),
                    private_key_path: None,
                    public_key_path: None,
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
            opa: None,
        };

        let result = settings.validate_production();
        assert!(
            result.is_err(),
            "validate_production should reject an http:// OIDC issuer URL"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("https"),
            "Error message should reference https: {}",
            err_msg
        );
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
