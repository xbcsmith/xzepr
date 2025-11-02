// src/infrastructure/messaging/config.rs

use rdkafka::config::ClientConfig;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Errors that can occur during Kafka authentication configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required SASL credential: {0}")]
    MissingCredential(String),

    #[error("Invalid security protocol: {0}")]
    InvalidSecurityProtocol(String),

    #[error("Invalid SASL mechanism: {0}")]
    InvalidSaslMechanism(String),

    #[error("SASL configuration required for SASL security protocols")]
    SaslConfigRequired,

    #[error("SSL certificate file not found: {0}")]
    SslCertificateNotFound(String),
}

/// Security protocol for Kafka connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityProtocol {
    /// Plaintext connection (no encryption, no authentication)
    Plaintext,
    /// SSL/TLS encryption only (no SASL authentication)
    Ssl,
    /// SASL authentication over plaintext (not recommended for production)
    SaslPlaintext,
    /// SASL authentication over SSL/TLS (recommended for production)
    SaslSsl,
}

impl SecurityProtocol {
    /// Returns the rdkafka configuration value for this protocol
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityProtocol::Plaintext => "PLAINTEXT",
            SecurityProtocol::Ssl => "SSL",
            SecurityProtocol::SaslPlaintext => "SASL_PLAINTEXT",
            SecurityProtocol::SaslSsl => "SASL_SSL",
        }
    }

    /// Returns true if this protocol requires SASL configuration
    pub fn requires_sasl(&self) -> bool {
        matches!(
            self,
            SecurityProtocol::SaslPlaintext | SecurityProtocol::SaslSsl
        )
    }

    /// Returns true if this protocol uses SSL/TLS
    pub fn uses_ssl(&self) -> bool {
        matches!(self, SecurityProtocol::Ssl | SecurityProtocol::SaslSsl)
    }
}

impl fmt::Display for SecurityProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SecurityProtocol {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PLAINTEXT" => Ok(SecurityProtocol::Plaintext),
            "SSL" => Ok(SecurityProtocol::Ssl),
            "SASL_PLAINTEXT" => Ok(SecurityProtocol::SaslPlaintext),
            "SASL_SSL" => Ok(SecurityProtocol::SaslSsl),
            _ => Err(ConfigError::InvalidSecurityProtocol(s.to_string())),
        }
    }
}

/// SASL authentication mechanism
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaslMechanism {
    /// SASL/PLAIN (username/password in plaintext, use only over SSL)
    #[serde(rename = "PLAIN")]
    Plain,
    /// SASL/SCRAM-SHA-256 (recommended for production)
    #[serde(rename = "SCRAM-SHA-256")]
    ScramSha256,
    /// SASL/SCRAM-SHA-512 (recommended for production)
    #[serde(rename = "SCRAM-SHA-512")]
    ScramSha512,
    /// SASL/GSSAPI (Kerberos)
    #[serde(rename = "GSSAPI")]
    Gssapi,
    /// SASL/OAUTHBEARER
    #[serde(rename = "OAUTHBEARER")]
    OAuthBearer,
}

impl SaslMechanism {
    /// Returns the rdkafka configuration value for this mechanism
    pub fn as_str(&self) -> &'static str {
        match self {
            SaslMechanism::Plain => "PLAIN",
            SaslMechanism::ScramSha256 => "SCRAM-SHA-256",
            SaslMechanism::ScramSha512 => "SCRAM-SHA-512",
            SaslMechanism::Gssapi => "GSSAPI",
            SaslMechanism::OAuthBearer => "OAUTHBEARER",
        }
    }

    /// Returns true if this mechanism requires username and password
    pub fn requires_credentials(&self) -> bool {
        matches!(
            self,
            SaslMechanism::Plain | SaslMechanism::ScramSha256 | SaslMechanism::ScramSha512
        )
    }
}

impl fmt::Display for SaslMechanism {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SaslMechanism {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PLAIN" => Ok(SaslMechanism::Plain),
            "SCRAM-SHA-256" => Ok(SaslMechanism::ScramSha256),
            "SCRAM-SHA-512" => Ok(SaslMechanism::ScramSha512),
            "GSSAPI" => Ok(SaslMechanism::Gssapi),
            "OAUTHBEARER" => Ok(SaslMechanism::OAuthBearer),
            _ => Err(ConfigError::InvalidSaslMechanism(s.to_string())),
        }
    }
}

/// SASL authentication configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct SaslConfig {
    /// SASL mechanism to use
    pub mechanism: SaslMechanism,
    /// SASL username
    pub username: String,
    /// SASL password (will be redacted in debug output)
    pub password: String,
}

impl SaslConfig {
    /// Create a new SASL configuration
    ///
    /// # Arguments
    ///
    /// * `mechanism` - The SASL mechanism to use
    /// * `username` - The SASL username
    /// * `password` - The SASL password
    ///
    /// # Returns
    ///
    /// Returns a new SaslConfig instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::infrastructure::messaging::config::{SaslConfig, SaslMechanism};
    ///
    /// let config = SaslConfig::new(
    ///     SaslMechanism::ScramSha256,
    ///     "myuser".to_string(),
    ///     "mypassword".to_string()
    /// );
    /// ```
    pub fn new(mechanism: SaslMechanism, username: String, password: String) -> Self {
        Self {
            mechanism,
            username,
            password,
        }
    }

    /// Validate the SASL configuration
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the configuration is valid
    ///
    /// # Errors
    ///
    /// Returns ConfigError::MissingCredential if required credentials are missing
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.mechanism.requires_credentials() {
            if self.username.is_empty() {
                return Err(ConfigError::MissingCredential("username".to_string()));
            }
            if self.password.is_empty() {
                return Err(ConfigError::MissingCredential("password".to_string()));
            }
        }
        Ok(())
    }

    /// Apply this SASL configuration to an rdkafka ClientConfig
    ///
    /// # Arguments
    ///
    /// * `client_config` - The ClientConfig to modify
    pub fn apply_to_client_config(&self, client_config: &mut ClientConfig) {
        client_config.set("sasl.mechanism", self.mechanism.as_str());
        client_config.set("sasl.username", &self.username);
        client_config.set("sasl.password", &self.password);
    }
}

impl fmt::Debug for SaslConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaslConfig")
            .field("mechanism", &self.mechanism)
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

/// SSL/TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    /// Path to CA certificate file
    pub ca_location: Option<String>,
    /// Path to client certificate file
    pub certificate_location: Option<String>,
    /// Path to client key file
    pub key_location: Option<String>,
}

impl SslConfig {
    /// Create a new SSL configuration
    ///
    /// # Arguments
    ///
    /// * `ca_location` - Optional path to CA certificate file
    /// * `certificate_location` - Optional path to client certificate file
    /// * `key_location` - Optional path to client key file
    ///
    /// # Returns
    ///
    /// Returns a new SslConfig instance
    pub fn new(
        ca_location: Option<String>,
        certificate_location: Option<String>,
        key_location: Option<String>,
    ) -> Self {
        Self {
            ca_location,
            certificate_location,
            key_location,
        }
    }

    /// Validate the SSL configuration
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the configuration is valid
    ///
    /// # Errors
    ///
    /// Returns ConfigError::SslCertificateNotFound if a specified file does not exist
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(ca_path) = &self.ca_location {
            if !std::path::Path::new(ca_path).exists() {
                return Err(ConfigError::SslCertificateNotFound(ca_path.clone()));
            }
        }
        if let Some(cert_path) = &self.certificate_location {
            if !std::path::Path::new(cert_path).exists() {
                return Err(ConfigError::SslCertificateNotFound(cert_path.clone()));
            }
        }
        if let Some(key_path) = &self.key_location {
            if !std::path::Path::new(key_path).exists() {
                return Err(ConfigError::SslCertificateNotFound(key_path.clone()));
            }
        }
        Ok(())
    }

    /// Apply this SSL configuration to an rdkafka ClientConfig
    ///
    /// # Arguments
    ///
    /// * `client_config` - The ClientConfig to modify
    pub fn apply_to_client_config(&self, client_config: &mut ClientConfig) {
        if let Some(ca_location) = &self.ca_location {
            client_config.set("ssl.ca.location", ca_location);
        }
        if let Some(cert_location) = &self.certificate_location {
            client_config.set("ssl.certificate.location", cert_location);
        }
        if let Some(key_location) = &self.key_location {
            client_config.set("ssl.key.location", key_location);
        }
    }
}

/// Complete Kafka authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaAuthConfig {
    /// Security protocol to use
    pub security_protocol: SecurityProtocol,
    /// SASL configuration (required if security_protocol requires SASL)
    pub sasl_config: Option<SaslConfig>,
    /// SSL configuration (optional, used when security_protocol uses SSL)
    pub ssl_config: Option<SslConfig>,
}

impl KafkaAuthConfig {
    /// Create a new Kafka authentication configuration
    ///
    /// # Arguments
    ///
    /// * `security_protocol` - The security protocol to use
    /// * `sasl_config` - Optional SASL configuration
    /// * `ssl_config` - Optional SSL configuration
    ///
    /// # Returns
    ///
    /// Returns a new KafkaAuthConfig instance
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::infrastructure::messaging::config::{
    ///     KafkaAuthConfig, SecurityProtocol, SaslConfig, SaslMechanism
    /// };
    ///
    /// let sasl = SaslConfig::new(
    ///     SaslMechanism::ScramSha256,
    ///     "user".to_string(),
    ///     "pass".to_string()
    /// );
    ///
    /// let config = KafkaAuthConfig::new(
    ///     SecurityProtocol::SaslSsl,
    ///     Some(sasl),
    ///     None
    /// );
    /// ```
    pub fn new(
        security_protocol: SecurityProtocol,
        sasl_config: Option<SaslConfig>,
        ssl_config: Option<SslConfig>,
    ) -> Self {
        Self {
            security_protocol,
            sasl_config,
            ssl_config,
        }
    }

    /// Create a configuration for SASL/SCRAM-SHA-256 over SSL
    ///
    /// # Arguments
    ///
    /// * `username` - SASL username
    /// * `password` - SASL password
    /// * `ca_location` - Optional path to CA certificate
    ///
    /// # Returns
    ///
    /// Returns a new KafkaAuthConfig configured for SCRAM-SHA-256 over SSL
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::infrastructure::messaging::config::KafkaAuthConfig;
    ///
    /// let config = KafkaAuthConfig::scram_sha256_ssl(
    ///     "myuser".to_string(),
    ///     "mypassword".to_string(),
    ///     Some("/path/to/ca.pem".to_string())
    /// );
    /// ```
    pub fn scram_sha256_ssl(
        username: String,
        password: String,
        ca_location: Option<String>,
    ) -> Self {
        let sasl_config = SaslConfig::new(SaslMechanism::ScramSha256, username, password);
        let ssl_config = SslConfig::new(ca_location, None, None);

        Self {
            security_protocol: SecurityProtocol::SaslSsl,
            sasl_config: Some(sasl_config),
            ssl_config: Some(ssl_config),
        }
    }

    /// Create a configuration for SASL/SCRAM-SHA-512 over SSL
    ///
    /// # Arguments
    ///
    /// * `username` - SASL username
    /// * `password` - SASL password
    /// * `ca_location` - Optional path to CA certificate
    ///
    /// # Returns
    ///
    /// Returns a new KafkaAuthConfig configured for SCRAM-SHA-512 over SSL
    pub fn scram_sha512_ssl(
        username: String,
        password: String,
        ca_location: Option<String>,
    ) -> Self {
        let sasl_config = SaslConfig::new(SaslMechanism::ScramSha512, username, password);
        let ssl_config = SslConfig::new(ca_location, None, None);

        Self {
            security_protocol: SecurityProtocol::SaslSsl,
            sasl_config: Some(sasl_config),
            ssl_config: Some(ssl_config),
        }
    }

    /// Validate the authentication configuration
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the configuration is valid
    ///
    /// # Errors
    ///
    /// Returns ConfigError if the configuration is invalid
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Verify SASL config is provided when required
        if self.security_protocol.requires_sasl() && self.sasl_config.is_none() {
            return Err(ConfigError::SaslConfigRequired);
        }

        // Validate SASL config if present
        if let Some(sasl_config) = &self.sasl_config {
            sasl_config.validate()?;
        }

        // Validate SSL config if present
        if let Some(ssl_config) = &self.ssl_config {
            ssl_config.validate()?;
        }

        Ok(())
    }

    /// Apply this authentication configuration to an rdkafka ClientConfig
    ///
    /// # Arguments
    ///
    /// * `client_config` - The ClientConfig to modify
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rdkafka::config::ClientConfig;
    /// use xzepr::infrastructure::messaging::config::KafkaAuthConfig;
    ///
    /// let auth_config = KafkaAuthConfig::scram_sha256_ssl(
    ///     "user".to_string(),
    ///     "pass".to_string(),
    ///     None
    /// );
    ///
    /// let mut client_config = ClientConfig::new();
    /// auth_config.apply_to_client_config(&mut client_config);
    /// ```
    pub fn apply_to_client_config(&self, client_config: &mut ClientConfig) {
        client_config.set("security.protocol", self.security_protocol.as_str());

        if let Some(sasl_config) = &self.sasl_config {
            sasl_config.apply_to_client_config(client_config);
        }

        if let Some(ssl_config) = &self.ssl_config {
            ssl_config.apply_to_client_config(client_config);
        }
    }

    /// Load authentication configuration from environment variables
    ///
    /// # Environment Variables
    ///
    /// * `KAFKA_SECURITY_PROTOCOL` - Security protocol (PLAINTEXT, SSL, SASL_PLAINTEXT, SASL_SSL)
    /// * `KAFKA_SASL_MECHANISM` - SASL mechanism (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512, etc.)
    /// * `KAFKA_SASL_USERNAME` - SASL username
    /// * `KAFKA_SASL_PASSWORD` - SASL password
    /// * `KAFKA_SSL_CA_LOCATION` - Path to CA certificate file
    /// * `KAFKA_SSL_CERT_LOCATION` - Path to client certificate file
    /// * `KAFKA_SSL_KEY_LOCATION` - Path to client key file
    ///
    /// # Returns
    ///
    /// Returns Some(KafkaAuthConfig) if KAFKA_SECURITY_PROTOCOL is set, None otherwise
    ///
    /// # Errors
    ///
    /// Returns ConfigError if environment variables are invalid or missing required values
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xzepr::infrastructure::messaging::config::KafkaAuthConfig;
    ///
    /// if let Some(auth_config) = KafkaAuthConfig::from_env().unwrap() {
    ///     // Use the configuration
    /// }
    /// ```
    pub fn from_env() -> Result<Option<Self>, ConfigError> {
        let security_protocol = match std::env::var("KAFKA_SECURITY_PROTOCOL") {
            Ok(val) => val.parse::<SecurityProtocol>()?,
            Err(_) => return Ok(None), // No security protocol specified, use defaults
        };

        let sasl_config = if security_protocol.requires_sasl() {
            let mechanism = std::env::var("KAFKA_SASL_MECHANISM")
                .unwrap_or_else(|_| "SCRAM-SHA-256".to_string())
                .parse::<SaslMechanism>()?;

            let username = std::env::var("KAFKA_SASL_USERNAME")
                .map_err(|_| ConfigError::MissingCredential("KAFKA_SASL_USERNAME".to_string()))?;

            let password = std::env::var("KAFKA_SASL_PASSWORD")
                .map_err(|_| ConfigError::MissingCredential("KAFKA_SASL_PASSWORD".to_string()))?;

            Some(SaslConfig::new(mechanism, username, password))
        } else {
            None
        };

        let ssl_config = if security_protocol.uses_ssl() {
            let ca_location = std::env::var("KAFKA_SSL_CA_LOCATION").ok();
            let cert_location = std::env::var("KAFKA_SSL_CERT_LOCATION").ok();
            let key_location = std::env::var("KAFKA_SSL_KEY_LOCATION").ok();

            if ca_location.is_some() || cert_location.is_some() || key_location.is_some() {
                Some(SslConfig::new(ca_location, cert_location, key_location))
            } else {
                None
            }
        } else {
            None
        };

        let config = Self::new(security_protocol, sasl_config, ssl_config);
        config.validate()?;

        Ok(Some(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that use environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // Helper to clean up all Kafka-related environment variables
    fn cleanup_env_vars() {
        std::env::remove_var("KAFKA_SECURITY_PROTOCOL");
        std::env::remove_var("KAFKA_SASL_MECHANISM");
        std::env::remove_var("KAFKA_SASL_USERNAME");
        std::env::remove_var("KAFKA_SASL_PASSWORD");
        std::env::remove_var("KAFKA_SSL_CA_LOCATION");
        std::env::remove_var("KAFKA_SSL_CERT_LOCATION");
        std::env::remove_var("KAFKA_SSL_KEY_LOCATION");
    }

    #[test]
    fn test_security_protocol_as_str() {
        assert_eq!(SecurityProtocol::Plaintext.as_str(), "PLAINTEXT");
        assert_eq!(SecurityProtocol::Ssl.as_str(), "SSL");
        assert_eq!(SecurityProtocol::SaslPlaintext.as_str(), "SASL_PLAINTEXT");
        assert_eq!(SecurityProtocol::SaslSsl.as_str(), "SASL_SSL");
    }

    #[test]
    fn test_security_protocol_requires_sasl() {
        assert!(!SecurityProtocol::Plaintext.requires_sasl());
        assert!(!SecurityProtocol::Ssl.requires_sasl());
        assert!(SecurityProtocol::SaslPlaintext.requires_sasl());
        assert!(SecurityProtocol::SaslSsl.requires_sasl());
    }

    #[test]
    fn test_security_protocol_uses_ssl() {
        assert!(!SecurityProtocol::Plaintext.uses_ssl());
        assert!(SecurityProtocol::Ssl.uses_ssl());
        assert!(!SecurityProtocol::SaslPlaintext.uses_ssl());
        assert!(SecurityProtocol::SaslSsl.uses_ssl());
    }

    #[test]
    fn test_security_protocol_from_str() {
        assert_eq!(
            "PLAINTEXT".parse::<SecurityProtocol>().unwrap(),
            SecurityProtocol::Plaintext
        );
        assert_eq!(
            "plaintext".parse::<SecurityProtocol>().unwrap(),
            SecurityProtocol::Plaintext
        );
        assert_eq!(
            "SASL_SSL".parse::<SecurityProtocol>().unwrap(),
            SecurityProtocol::SaslSsl
        );
        assert!("INVALID".parse::<SecurityProtocol>().is_err());
    }

    #[test]
    fn test_sasl_mechanism_as_str() {
        assert_eq!(SaslMechanism::Plain.as_str(), "PLAIN");
        assert_eq!(SaslMechanism::ScramSha256.as_str(), "SCRAM-SHA-256");
        assert_eq!(SaslMechanism::ScramSha512.as_str(), "SCRAM-SHA-512");
        assert_eq!(SaslMechanism::Gssapi.as_str(), "GSSAPI");
        assert_eq!(SaslMechanism::OAuthBearer.as_str(), "OAUTHBEARER");
    }

    #[test]
    fn test_sasl_mechanism_requires_credentials() {
        assert!(SaslMechanism::Plain.requires_credentials());
        assert!(SaslMechanism::ScramSha256.requires_credentials());
        assert!(SaslMechanism::ScramSha512.requires_credentials());
        assert!(!SaslMechanism::Gssapi.requires_credentials());
        assert!(!SaslMechanism::OAuthBearer.requires_credentials());
    }

    #[test]
    fn test_sasl_mechanism_from_str() {
        assert_eq!(
            "SCRAM-SHA-256".parse::<SaslMechanism>().unwrap(),
            SaslMechanism::ScramSha256
        );
        assert_eq!(
            "scram-sha-256".parse::<SaslMechanism>().unwrap(),
            SaslMechanism::ScramSha256
        );
        assert!("INVALID".parse::<SaslMechanism>().is_err());
    }

    #[test]
    fn test_sasl_config_new() {
        let config = SaslConfig::new(
            SaslMechanism::ScramSha256,
            "testuser".to_string(),
            "testpass".to_string(),
        );
        assert_eq!(config.mechanism, SaslMechanism::ScramSha256);
        assert_eq!(config.username, "testuser");
        assert_eq!(config.password, "testpass");
    }

    #[test]
    fn test_sasl_config_validate_with_valid_credentials() {
        let config = SaslConfig::new(
            SaslMechanism::ScramSha256,
            "testuser".to_string(),
            "testpass".to_string(),
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_sasl_config_validate_with_empty_username() {
        let config = SaslConfig::new(
            SaslMechanism::ScramSha256,
            "".to_string(),
            "testpass".to_string(),
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sasl_config_validate_with_empty_password() {
        let config = SaslConfig::new(
            SaslMechanism::ScramSha256,
            "testuser".to_string(),
            "".to_string(),
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sasl_config_debug_redacts_password() {
        let config = SaslConfig::new(
            SaslMechanism::ScramSha256,
            "testuser".to_string(),
            "secret".to_string(),
        );
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("testuser"));
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("secret"));
    }

    #[test]
    fn test_ssl_config_new() {
        let config = SslConfig::new(
            Some("/path/to/ca.pem".to_string()),
            Some("/path/to/cert.pem".to_string()),
            Some("/path/to/key.pem".to_string()),
        );
        assert_eq!(config.ca_location, Some("/path/to/ca.pem".to_string()));
        assert_eq!(
            config.certificate_location,
            Some("/path/to/cert.pem".to_string())
        );
        assert_eq!(config.key_location, Some("/path/to/key.pem".to_string()));
    }

    #[test]
    fn test_kafka_auth_config_new() {
        let sasl = SaslConfig::new(
            SaslMechanism::ScramSha256,
            "user".to_string(),
            "pass".to_string(),
        );
        let config = KafkaAuthConfig::new(SecurityProtocol::SaslSsl, Some(sasl), None);
        assert_eq!(config.security_protocol, SecurityProtocol::SaslSsl);
        assert!(config.sasl_config.is_some());
        assert!(config.ssl_config.is_none());
    }

    #[test]
    fn test_kafka_auth_config_scram_sha256_ssl() {
        let config = KafkaAuthConfig::scram_sha256_ssl(
            "user".to_string(),
            "pass".to_string(),
            Some("/path/to/ca.pem".to_string()),
        );
        assert_eq!(config.security_protocol, SecurityProtocol::SaslSsl);
        assert!(config.sasl_config.is_some());
        assert_eq!(
            config.sasl_config.unwrap().mechanism,
            SaslMechanism::ScramSha256
        );
        assert!(config.ssl_config.is_some());
    }

    #[test]
    fn test_kafka_auth_config_scram_sha512_ssl() {
        let config =
            KafkaAuthConfig::scram_sha512_ssl("user".to_string(), "pass".to_string(), None);
        assert_eq!(config.security_protocol, SecurityProtocol::SaslSsl);
        assert!(config.sasl_config.is_some());
        assert_eq!(
            config.sasl_config.unwrap().mechanism,
            SaslMechanism::ScramSha512
        );
    }

    #[test]
    fn test_kafka_auth_config_validate_with_sasl_required_but_missing() {
        let config = KafkaAuthConfig::new(SecurityProtocol::SaslSsl, None, None);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_kafka_auth_config_validate_with_valid_config() {
        let sasl = SaslConfig::new(
            SaslMechanism::ScramSha256,
            "user".to_string(),
            "pass".to_string(),
        );
        let config = KafkaAuthConfig::new(SecurityProtocol::SaslSsl, Some(sasl), None);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_kafka_auth_config_validate_with_plaintext() {
        let config = KafkaAuthConfig::new(SecurityProtocol::Plaintext, None, None);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_kafka_auth_config_apply_to_client_config() {
        let sasl = SaslConfig::new(
            SaslMechanism::ScramSha256,
            "testuser".to_string(),
            "testpass".to_string(),
        );
        let auth_config = KafkaAuthConfig::new(SecurityProtocol::SaslSsl, Some(sasl), None);

        let mut client_config = ClientConfig::new();
        auth_config.apply_to_client_config(&mut client_config);

        // Note: We can't easily test the actual values set in ClientConfig
        // because rdkafka doesn't provide a way to read them back.
        // This test just verifies that the method doesn't panic.
    }

    #[test]
    fn test_kafka_auth_config_from_env_with_no_env_vars() {
        let _lock = ENV_MUTEX.lock().unwrap();
        cleanup_env_vars();

        let result = KafkaAuthConfig::from_env().unwrap();
        assert!(result.is_none());

        cleanup_env_vars();
    }

    #[test]
    fn test_kafka_auth_config_from_env_with_plaintext() {
        let _lock = ENV_MUTEX.lock().unwrap();
        cleanup_env_vars();

        std::env::set_var("KAFKA_SECURITY_PROTOCOL", "PLAINTEXT");

        let result = KafkaAuthConfig::from_env().unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.security_protocol, SecurityProtocol::Plaintext);

        cleanup_env_vars();
    }

    #[test]
    fn test_kafka_auth_config_from_env_with_sasl_ssl() {
        let _lock = ENV_MUTEX.lock().unwrap();
        cleanup_env_vars();

        std::env::set_var("KAFKA_SECURITY_PROTOCOL", "SASL_SSL");
        std::env::set_var("KAFKA_SASL_MECHANISM", "SCRAM-SHA-256");
        std::env::set_var("KAFKA_SASL_USERNAME", "testuser");
        std::env::set_var("KAFKA_SASL_PASSWORD", "testpass");

        let result = KafkaAuthConfig::from_env().unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        assert_eq!(config.security_protocol, SecurityProtocol::SaslSsl);
        assert!(config.sasl_config.is_some());

        let sasl = config.sasl_config.unwrap();
        assert_eq!(sasl.mechanism, SaslMechanism::ScramSha256);
        assert_eq!(sasl.username, "testuser");
        assert_eq!(sasl.password, "testpass");

        cleanup_env_vars();
    }

    #[test]
    fn test_kafka_auth_config_from_env_with_missing_sasl_credentials() {
        let _lock = ENV_MUTEX.lock().unwrap();
        cleanup_env_vars();

        std::env::set_var("KAFKA_SECURITY_PROTOCOL", "SASL_SSL");
        // Missing KAFKA_SASL_USERNAME and KAFKA_SASL_PASSWORD

        let result = KafkaAuthConfig::from_env();
        assert!(result.is_err());

        cleanup_env_vars();
    }

    #[test]
    fn test_kafka_auth_config_from_env_defaults_to_scram_sha256() {
        let _lock = ENV_MUTEX.lock().unwrap();
        cleanup_env_vars();

        std::env::set_var("KAFKA_SECURITY_PROTOCOL", "SASL_SSL");
        // Don't set KAFKA_SASL_MECHANISM - should default to SCRAM-SHA-256
        std::env::set_var("KAFKA_SASL_USERNAME", "testuser");
        std::env::set_var("KAFKA_SASL_PASSWORD", "testpass");

        let result = KafkaAuthConfig::from_env().unwrap();
        assert!(result.is_some());

        let config = result.unwrap();
        assert!(config.sasl_config.is_some());
        assert_eq!(
            config.sasl_config.unwrap().mechanism,
            SaslMechanism::ScramSha256
        );

        cleanup_env_vars();
    }
}
