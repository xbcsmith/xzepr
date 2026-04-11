// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/infrastructure/config.rs

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

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
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
}
