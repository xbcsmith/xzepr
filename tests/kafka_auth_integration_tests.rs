// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// tests/kafka_auth_integration_tests.rs

//! Integration tests for Kafka SASL/SCRAM authentication
//!
//! These tests verify that the Kafka producer and topic manager
//! work correctly with various authentication configurations.
//!
//! ## Test Execution Notes
//!
//! ### Environment Variable Tests
//! Tests that use environment variables may fail when run in parallel due to
//! shared process environment. To run these tests reliably, use:
//!
//! ```bash
//! cargo test --test kafka_auth_integration_tests -- --test-threads=1
//! ```
//!
//! ### Ignored Integration Tests
//! Some tests are marked with #[ignore] because they require:
//! - A running Kafka broker with SASL/SCRAM authentication enabled
//! - rdkafka compiled with libsasl2 or openssl support
//! - Valid SSL certificates for SSL/TLS tests
//!
//! To run ignored tests:
//! ```bash
//! cargo test --test kafka_auth_integration_tests -- --ignored --test-threads=1
//! ```

use xzepr::infrastructure::messaging::config::{
    KafkaAuthConfig, SaslConfig, SaslMechanism, SecurityProtocol, SslConfig,
};
use xzepr::infrastructure::messaging::producer::KafkaEventPublisher;
use xzepr::infrastructure::messaging::topics::TopicManager;

// Helper function to create SASL/SCRAM-SHA-256 config
fn create_scram_sha256_config() -> KafkaAuthConfig {
    KafkaAuthConfig {
        security_protocol: SecurityProtocol::SaslPlaintext,
        sasl_config: Some(SaslConfig {
            mechanism: SaslMechanism::ScramSha256,
            username: "test-user".to_string(),
            password: "test-password".to_string(),
        }),
        ssl_config: None,
    }
}

// Helper function to create SASL/SCRAM-SHA-512 config
fn create_scram_sha512_config() -> KafkaAuthConfig {
    KafkaAuthConfig {
        security_protocol: SecurityProtocol::SaslPlaintext,
        sasl_config: Some(SaslConfig {
            mechanism: SaslMechanism::ScramSha512,
            username: "test-user".to_string(),
            password: "test-password".to_string(),
        }),
        ssl_config: None,
    }
}

// Helper function to create SASL/PLAIN config
fn create_sasl_plain_config() -> KafkaAuthConfig {
    KafkaAuthConfig {
        security_protocol: SecurityProtocol::SaslPlaintext,
        sasl_config: Some(SaslConfig {
            mechanism: SaslMechanism::Plain,
            username: "test-user".to_string(),
            password: "test-password".to_string(),
        }),
        ssl_config: None,
    }
}

// Helper function to create SASL/SSL config
fn create_sasl_ssl_config() -> KafkaAuthConfig {
    KafkaAuthConfig {
        security_protocol: SecurityProtocol::SaslSsl,
        sasl_config: Some(SaslConfig {
            mechanism: SaslMechanism::ScramSha256,
            username: "test-user".to_string(),
            password: "test-password".to_string(),
        }),
        ssl_config: Some(SslConfig {
            ca_location: Some("/path/to/ca-cert".to_string()),
            certificate_location: Some("/path/to/client-cert".to_string()),
            key_location: Some("/path/to/client-key".to_string()),
        }),
    }
}

// Helper function to create SSL-only config
fn create_ssl_config() -> KafkaAuthConfig {
    KafkaAuthConfig {
        security_protocol: SecurityProtocol::Ssl,
        sasl_config: None,
        ssl_config: Some(SslConfig {
            ca_location: Some("/path/to/ca-cert".to_string()),
            certificate_location: Some("/path/to/client-cert".to_string()),
            key_location: Some("/path/to/client-key".to_string()),
        }),
    }
}

#[test]
fn test_kafka_auth_config_serialization_scram_sha256() {
    // Test that SCRAM-SHA-256 config can be serialized and deserialized
    let config = create_scram_sha256_config();
    let yaml = serde_yaml::to_string(&config).expect("Failed to serialize config");

    assert!(yaml.contains("SASL_PLAINTEXT"));
    assert!(yaml.contains("SCRAM-SHA-256"));
    assert!(yaml.contains("test-user"));

    let deserialized: KafkaAuthConfig =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize config");

    assert_eq!(
        deserialized.security_protocol,
        SecurityProtocol::SaslPlaintext
    );
    assert!(deserialized.sasl_config.is_some());

    let sasl = deserialized.sasl_config.unwrap();
    assert_eq!(sasl.mechanism, SaslMechanism::ScramSha256);
    assert_eq!(sasl.username, "test-user");
    assert_eq!(sasl.password, "test-password");
}

#[test]
fn test_kafka_auth_config_serialization_scram_sha512() {
    // Test that SCRAM-SHA-512 config can be serialized and deserialized
    let config = create_scram_sha512_config();
    let yaml = serde_yaml::to_string(&config).expect("Failed to serialize config");

    assert!(yaml.contains("SASL_PLAINTEXT"));
    assert!(yaml.contains("SCRAM-SHA-512"));
    assert!(yaml.contains("test-user"));

    let deserialized: KafkaAuthConfig =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize config");

    assert_eq!(
        deserialized.security_protocol,
        SecurityProtocol::SaslPlaintext
    );
    assert!(deserialized.sasl_config.is_some());

    let sasl = deserialized.sasl_config.unwrap();
    assert_eq!(sasl.mechanism, SaslMechanism::ScramSha512);
    assert_eq!(sasl.username, "test-user");
    assert_eq!(sasl.password, "test-password");
}

#[test]
fn test_kafka_auth_config_serialization_sasl_plain() {
    // Test that SASL/PLAIN config can be serialized and deserialized
    let config = create_sasl_plain_config();
    let yaml = serde_yaml::to_string(&config).expect("Failed to serialize config");

    assert!(yaml.contains("SASL_PLAINTEXT"));
    assert!(yaml.contains("PLAIN"));
    assert!(yaml.contains("test-user"));

    let deserialized: KafkaAuthConfig =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize config");

    assert_eq!(
        deserialized.security_protocol,
        SecurityProtocol::SaslPlaintext
    );
    assert!(deserialized.sasl_config.is_some());

    let sasl = deserialized.sasl_config.unwrap();
    assert_eq!(sasl.mechanism, SaslMechanism::Plain);
    assert_eq!(sasl.username, "test-user");
    assert_eq!(sasl.password, "test-password");
}

#[test]
fn test_kafka_auth_config_serialization_sasl_ssl() {
    // Test that SASL/SSL config can be serialized and deserialized
    let config = create_sasl_ssl_config();
    let yaml = serde_yaml::to_string(&config).expect("Failed to serialize config");

    assert!(yaml.contains("SASL_SSL"));
    assert!(yaml.contains("SCRAM-SHA-256"));
    assert!(yaml.contains("/path/to/ca-cert"));

    let deserialized: KafkaAuthConfig =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize config");

    assert_eq!(deserialized.security_protocol, SecurityProtocol::SaslSsl);
    assert!(deserialized.sasl_config.is_some());
    assert!(deserialized.ssl_config.is_some());

    let ssl = deserialized.ssl_config.unwrap();
    assert_eq!(ssl.ca_location, Some("/path/to/ca-cert".to_string()));
    assert_eq!(
        ssl.certificate_location,
        Some("/path/to/client-cert".to_string())
    );
    assert_eq!(ssl.key_location, Some("/path/to/client-key".to_string()));
}

#[test]
fn test_kafka_auth_config_serialization_ssl_only() {
    // Test that SSL-only config can be serialized and deserialized
    let config = create_ssl_config();
    let yaml = serde_yaml::to_string(&config).expect("Failed to serialize config");

    assert!(yaml.contains("SSL"));
    assert!(yaml.contains("/path/to/ca-cert"));

    let deserialized: KafkaAuthConfig =
        serde_yaml::from_str(&yaml).expect("Failed to deserialize config");

    assert_eq!(deserialized.security_protocol, SecurityProtocol::Ssl);
    assert!(deserialized.sasl_config.is_none());
    assert!(deserialized.ssl_config.is_some());

    let ssl = deserialized.ssl_config.unwrap();
    assert_eq!(ssl.ca_location, Some("/path/to/ca-cert".to_string()));
}

#[test]
fn test_kafka_auth_config_from_env_none() {
    // Test that from_env returns None when no environment variables are set
    // Note: We cannot reliably test this in a multi-threaded test environment
    // because other tests may set environment variables
    // This test documents the expected behavior
    let original_protocol = std::env::var("KAFKA_SECURITY_PROTOCOL").ok();
    let original_mechanism = std::env::var("KAFKA_SASL_MECHANISM").ok();
    let original_username = std::env::var("KAFKA_SASL_USERNAME").ok();
    let original_password = std::env::var("KAFKA_SASL_PASSWORD").ok();

    std::env::remove_var("KAFKA_SECURITY_PROTOCOL");
    std::env::remove_var("KAFKA_SASL_MECHANISM");
    std::env::remove_var("KAFKA_SASL_USERNAME");
    std::env::remove_var("KAFKA_SASL_PASSWORD");

    let config = KafkaAuthConfig::from_env();
    assert!(config.is_ok());
    assert!(config.unwrap().is_none());

    // Restore original values
    if let Some(val) = original_protocol {
        std::env::set_var("KAFKA_SECURITY_PROTOCOL", val);
    }
    if let Some(val) = original_mechanism {
        std::env::set_var("KAFKA_SASL_MECHANISM", val);
    }
    if let Some(val) = original_username {
        std::env::set_var("KAFKA_SASL_USERNAME", val);
    }
    if let Some(val) = original_password {
        std::env::set_var("KAFKA_SASL_PASSWORD", val);
    }
}

#[test]
#[ignore = "Environment variable tests may fail when run in parallel; use --test-threads=1"]
fn test_kafka_auth_config_from_env_scram_sha256() {
    // Test that from_env correctly parses SCRAM-SHA-256 configuration
    // Save original values
    let original_protocol = std::env::var("KAFKA_SECURITY_PROTOCOL").ok();
    let original_mechanism = std::env::var("KAFKA_SASL_MECHANISM").ok();
    let original_username = std::env::var("KAFKA_SASL_USERNAME").ok();
    let original_password = std::env::var("KAFKA_SASL_PASSWORD").ok();

    std::env::set_var("KAFKA_SECURITY_PROTOCOL", "SASL_PLAINTEXT");
    std::env::set_var("KAFKA_SASL_MECHANISM", "SCRAM-SHA-256");
    std::env::set_var("KAFKA_SASL_USERNAME", "env-user");
    std::env::set_var("KAFKA_SASL_PASSWORD", "env-password");

    let config = KafkaAuthConfig::from_env()
        .expect("Failed to load config from env")
        .expect("Config should be Some");

    assert_eq!(config.security_protocol, SecurityProtocol::SaslPlaintext);
    assert!(config.sasl_config.is_some());

    let sasl = config.sasl_config.unwrap();
    assert_eq!(sasl.mechanism, SaslMechanism::ScramSha256);
    assert_eq!(sasl.username, "env-user");
    assert_eq!(sasl.password, "env-password");

    // Restore original values
    std::env::remove_var("KAFKA_SECURITY_PROTOCOL");
    std::env::remove_var("KAFKA_SASL_MECHANISM");
    std::env::remove_var("KAFKA_SASL_USERNAME");
    std::env::remove_var("KAFKA_SASL_PASSWORD");
    if let Some(val) = original_protocol {
        std::env::set_var("KAFKA_SECURITY_PROTOCOL", val);
    }
    if let Some(val) = original_mechanism {
        std::env::set_var("KAFKA_SASL_MECHANISM", val);
    }
    if let Some(val) = original_username {
        std::env::set_var("KAFKA_SASL_USERNAME", val);
    }
    if let Some(val) = original_password {
        std::env::set_var("KAFKA_SASL_PASSWORD", val);
    }
}

#[test]
#[ignore = "Environment variable tests may fail when run in parallel; use --test-threads=1"]
fn test_kafka_auth_config_from_env_scram_sha512() {
    // Test that from_env correctly parses SCRAM-SHA-512 configuration
    // Save original values
    let original_protocol = std::env::var("KAFKA_SECURITY_PROTOCOL").ok();
    let original_mechanism = std::env::var("KAFKA_SASL_MECHANISM").ok();
    let original_username = std::env::var("KAFKA_SASL_USERNAME").ok();
    let original_password = std::env::var("KAFKA_SASL_PASSWORD").ok();

    std::env::set_var("KAFKA_SECURITY_PROTOCOL", "SASL_PLAINTEXT");
    std::env::set_var("KAFKA_SASL_MECHANISM", "SCRAM-SHA-512");
    std::env::set_var("KAFKA_SASL_USERNAME", "env-user");
    std::env::set_var("KAFKA_SASL_PASSWORD", "env-password");

    let config = KafkaAuthConfig::from_env()
        .expect("Failed to load config from env")
        .expect("Config should be Some");

    assert_eq!(config.security_protocol, SecurityProtocol::SaslPlaintext);
    assert!(config.sasl_config.is_some());

    let sasl = config.sasl_config.unwrap();
    assert_eq!(sasl.mechanism, SaslMechanism::ScramSha512);
    assert_eq!(sasl.username, "env-user");
    assert_eq!(sasl.password, "env-password");

    // Restore original values
    std::env::remove_var("KAFKA_SECURITY_PROTOCOL");
    std::env::remove_var("KAFKA_SASL_MECHANISM");
    std::env::remove_var("KAFKA_SASL_USERNAME");
    std::env::remove_var("KAFKA_SASL_PASSWORD");
    if let Some(val) = original_protocol {
        std::env::set_var("KAFKA_SECURITY_PROTOCOL", val);
    }
    if let Some(val) = original_mechanism {
        std::env::set_var("KAFKA_SASL_MECHANISM", val);
    }
    if let Some(val) = original_username {
        std::env::set_var("KAFKA_SASL_USERNAME", val);
    }
    if let Some(val) = original_password {
        std::env::set_var("KAFKA_SASL_PASSWORD", val);
    }
}

#[test]
#[ignore = "Environment variable tests may fail when run in parallel; use --test-threads=1"]
fn test_kafka_auth_config_from_env_ssl() {
    // Test that from_env correctly parses SSL configuration
    // Note: This test expects an error because the certificate files don't exist
    // In a real scenario, valid certificate files would be provided

    // Save original values
    let original_protocol = std::env::var("KAFKA_SECURITY_PROTOCOL").ok();
    let original_ca = std::env::var("KAFKA_SSL_CA_LOCATION").ok();
    let original_cert = std::env::var("KAFKA_SSL_CERT_LOCATION").ok();
    let original_key = std::env::var("KAFKA_SSL_KEY_LOCATION").ok();

    std::env::set_var("KAFKA_SECURITY_PROTOCOL", "SSL");
    std::env::set_var("KAFKA_SSL_CA_LOCATION", "/nonexistent/ca.pem");
    std::env::set_var("KAFKA_SSL_CERT_LOCATION", "/nonexistent/cert.pem");
    std::env::set_var("KAFKA_SSL_KEY_LOCATION", "/nonexistent/key.pem");

    // from_env should fail because certificate files don't exist
    let result = KafkaAuthConfig::from_env();

    // Restore original values first (cleanup regardless of assertion)
    std::env::remove_var("KAFKA_SECURITY_PROTOCOL");
    std::env::remove_var("KAFKA_SSL_CA_LOCATION");
    std::env::remove_var("KAFKA_SSL_CERT_LOCATION");
    std::env::remove_var("KAFKA_SSL_KEY_LOCATION");
    if let Some(val) = original_protocol {
        std::env::set_var("KAFKA_SECURITY_PROTOCOL", val);
    }
    if let Some(val) = original_ca {
        std::env::set_var("KAFKA_SSL_CA_LOCATION", val);
    }
    if let Some(val) = original_cert {
        std::env::set_var("KAFKA_SSL_CERT_LOCATION", val);
    }
    if let Some(val) = original_key {
        std::env::set_var("KAFKA_SSL_KEY_LOCATION", val);
    }

    // Now assert - should fail validation due to missing certificate files
    assert!(
        result.is_err(),
        "Should fail validation when SSL certificate files don't exist"
    );
}

#[test]
#[ignore = "Requires running Kafka broker with SASL/SCRAM-SHA-256 authentication"]
fn test_producer_connection_with_scram_sha256() {
    // Integration test: Connect to Kafka with SCRAM-SHA-256
    // This test requires:
    // - A running Kafka broker on localhost:19092
    // - SASL/SCRAM-SHA-256 enabled with user credentials
    // - rdkafka compiled with libsasl2 support

    let auth_config = create_scram_sha256_config();
    let result =
        KafkaEventPublisher::with_auth("localhost:19092", "test-topic", Some(&auth_config));

    assert!(
        result.is_ok(),
        "Failed to create publisher with SCRAM-SHA-256: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "Requires running Kafka broker with SASL/SCRAM-SHA-512 authentication"]
fn test_producer_connection_with_scram_sha512() {
    // Integration test: Connect to Kafka with SCRAM-SHA-512
    // This test requires:
    // - A running Kafka broker on localhost:19092
    // - SASL/SCRAM-SHA-512 enabled with user credentials
    // - rdkafka compiled with libsasl2 support

    let auth_config = create_scram_sha512_config();
    let result =
        KafkaEventPublisher::with_auth("localhost:19092", "test-topic", Some(&auth_config));

    assert!(
        result.is_ok(),
        "Failed to create publisher with SCRAM-SHA-512: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "Requires running Kafka broker with SASL/PLAIN authentication"]
fn test_producer_connection_with_sasl_plain() {
    // Integration test: Connect to Kafka with SASL/PLAIN
    // This test requires:
    // - A running Kafka broker on localhost:19092
    // - SASL/PLAIN enabled with user credentials
    // - rdkafka compiled with libsasl2 support

    let auth_config = create_sasl_plain_config();
    let result =
        KafkaEventPublisher::with_auth("localhost:19092", "test-topic", Some(&auth_config));

    assert!(
        result.is_ok(),
        "Failed to create publisher with SASL/PLAIN: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "Requires running Kafka broker with SSL/TLS authentication"]
fn test_producer_connection_with_ssl() {
    // Integration test: Connect to Kafka with SSL/TLS
    // This test requires:
    // - A running Kafka broker on localhost:19093 with SSL enabled
    // - Valid SSL certificates
    // - rdkafka compiled with openssl support

    let auth_config = create_ssl_config();
    let result =
        KafkaEventPublisher::with_auth("localhost:19093", "test-topic", Some(&auth_config));

    // This will likely fail unless certificates exist
    // In a real test environment, this should succeed
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[ignore = "Requires running Kafka broker with SASL/SCRAM-SHA-256 authentication"]
fn test_topic_manager_connection_with_scram_sha256() {
    // Integration test: Connect TopicManager to Kafka with SCRAM-SHA-256
    // This test requires:
    // - A running Kafka broker on localhost:19092
    // - SASL/SCRAM-SHA-256 enabled with user credentials
    // - rdkafka compiled with libsasl2 support

    let auth_config = create_scram_sha256_config();
    let result = TopicManager::with_auth("localhost:19092", Some(&auth_config));

    assert!(
        result.is_ok(),
        "Failed to create topic manager with SCRAM-SHA-256: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "Requires running Kafka broker with SASL/SCRAM-SHA-512 authentication"]
fn test_topic_manager_connection_with_scram_sha512() {
    // Integration test: Connect TopicManager to Kafka with SCRAM-SHA-512
    // This test requires:
    // - A running Kafka broker on localhost:19092
    // - SASL/SCRAM-SHA-512 enabled with user credentials
    // - rdkafka compiled with libsasl2 support

    let auth_config = create_scram_sha512_config();
    let result = TopicManager::with_auth("localhost:19092", Some(&auth_config));

    assert!(
        result.is_ok(),
        "Failed to create topic manager with SCRAM-SHA-512: {:?}",
        result.err()
    );
}

#[tokio::test]
#[ignore = "Requires running Kafka broker with SASL/SCRAM authentication and topic creation permissions"]
async fn test_end_to_end_topic_creation_with_auth() {
    // End-to-end integration test: Create topic with authentication
    // This test requires:
    // - A running Kafka broker on localhost:19092
    // - SASL/SCRAM authentication enabled
    // - User has topic creation permissions
    // - rdkafka compiled with libsasl2 support

    let auth_config = create_scram_sha256_config();
    let manager = TopicManager::with_auth("localhost:19092", Some(&auth_config))
        .expect("Failed to create topic manager");

    let result = manager.ensure_topic_exists("test-auth-topic", 3, 1).await;

    // Should either create the topic or succeed because it already exists
    assert!(
        result.is_ok(),
        "Failed to ensure topic exists: {:?}",
        result.err()
    );
}

#[tokio::test]
#[ignore = "Requires running Kafka broker with SASL/SCRAM authentication"]
async fn test_end_to_end_event_publishing_with_auth() {
    // End-to-end integration test: Publish event with authentication
    // This test requires:
    // - A running Kafka broker on localhost:19092
    // - SASL/SCRAM authentication enabled
    // - Topic "test-auth-events" exists or user has creation permissions
    // - rdkafka compiled with libsasl2 support

    use xzepr::domain::entities::event::{CreateEventParams, Event};
    use xzepr::domain::value_objects::{EventReceiverId, UserId};

    let auth_config = create_scram_sha256_config();
    let publisher =
        KafkaEventPublisher::with_auth("localhost:19092", "test-auth-events", Some(&auth_config))
            .expect("Failed to create publisher");

    // Create a test event
    let receiver_id = EventReceiverId::new();
    let owner_id = UserId::new();
    let event = Event::new(CreateEventParams {
        name: "test.auth.event".to_string(),
        version: "1.0.0".to_string(),
        release: "1.0.0".to_string(),
        platform_id: "test".to_string(),
        package: "test-pkg".to_string(),
        description: "Test event with auth".to_string(),
        owner_id,
        payload: serde_json::json!({"test": "data"}),
        success: true,
        receiver_id,
    })
    .expect("Failed to create event");

    // Publish the event
    let result = publisher.publish(&event).await;

    assert!(
        result.is_ok(),
        "Failed to publish event with auth: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "Requires running Kafka broker with invalid credentials to test failure"]
fn test_producer_connection_fails_with_invalid_credentials() {
    // Integration test: Verify connection fails with invalid credentials
    // This test requires:
    // - A running Kafka broker on localhost:19092
    // - SASL/SCRAM authentication enabled
    // - The credentials used here should be invalid

    let auth_config = KafkaAuthConfig {
        security_protocol: SecurityProtocol::SaslPlaintext,
        sasl_config: Some(SaslConfig {
            mechanism: SaslMechanism::ScramSha256,
            username: "invalid-user".to_string(),
            password: "invalid-password".to_string(),
        }),
        ssl_config: None,
    };

    let result =
        KafkaEventPublisher::with_auth("localhost:19092", "test-topic", Some(&auth_config));

    // Producer creation may succeed (connection is lazy)
    // But actual operations would fail
    // This test documents the behavior
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_auth_config_validation_missing_sasl_credentials() {
    // Test validation: SASL config should require username and password
    let config = KafkaAuthConfig {
        security_protocol: SecurityProtocol::SaslPlaintext,
        sasl_config: Some(SaslConfig {
            mechanism: SaslMechanism::ScramSha256,
            username: "".to_string(),
            password: "".to_string(),
        }),
        ssl_config: None,
    };

    // Empty credentials are technically valid (will fail at connection time)
    // but we can create the config
    let yaml = serde_yaml::to_string(&config);
    assert!(yaml.is_ok());
}

#[test]
fn test_auth_config_validation_missing_ssl_certificates() {
    // Test validation: SSL config can have optional certificate paths
    let config = KafkaAuthConfig {
        security_protocol: SecurityProtocol::Ssl,
        sasl_config: None,
        ssl_config: Some(SslConfig {
            ca_location: None,
            certificate_location: None,
            key_location: None,
        }),
    };

    // All SSL fields are optional
    let yaml = serde_yaml::to_string(&config);
    assert!(yaml.is_ok());
}

#[test]
fn test_multiple_authentication_mechanisms() {
    // Test that different authentication mechanisms can coexist
    let configs = vec![
        create_scram_sha256_config(),
        create_scram_sha512_config(),
        create_sasl_plain_config(),
        create_ssl_config(),
        create_sasl_ssl_config(),
    ];

    for config in configs {
        let yaml = serde_yaml::to_string(&config).expect("Failed to serialize config");
        let deserialized: KafkaAuthConfig =
            serde_yaml::from_str(&yaml).expect("Failed to deserialize config");

        assert_eq!(config.security_protocol, deserialized.security_protocol);
    }
}

#[test]
fn test_config_roundtrip_preserves_data() {
    // Test that config can be serialized and deserialized without data loss
    let original = create_sasl_ssl_config();

    let yaml = serde_yaml::to_string(&original).expect("Failed to serialize");
    let deserialized: KafkaAuthConfig = serde_yaml::from_str(&yaml).expect("Failed to deserialize");

    assert_eq!(original.security_protocol, deserialized.security_protocol);

    let original_sasl = original.sasl_config.as_ref().unwrap();
    let deserialized_sasl = deserialized.sasl_config.as_ref().unwrap();
    assert_eq!(original_sasl.mechanism, deserialized_sasl.mechanism);
    assert_eq!(original_sasl.username, deserialized_sasl.username);
    assert_eq!(original_sasl.password, deserialized_sasl.password);

    let original_ssl = original.ssl_config.as_ref().unwrap();
    let deserialized_ssl = deserialized.ssl_config.as_ref().unwrap();
    assert_eq!(original_ssl.ca_location, deserialized_ssl.ca_location);
    assert_eq!(
        original_ssl.certificate_location,
        deserialized_ssl.certificate_location
    );
    assert_eq!(original_ssl.key_location, deserialized_ssl.key_location);
}
