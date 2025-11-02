//! Kafka Authentication Example
//!
//! This example demonstrates how to create and configure a Kafka event publisher
//! with SASL/SCRAM authentication in XZepr.
//!
//! # Prerequisites
//!
//! - Kafka cluster with SASL/SCRAM authentication enabled
//! - Valid Kafka credentials (username and password)
//! - SSL certificates (if using SASL_SSL)
//!
//! # Running the Example
//!
//! ## Using Environment Variables
//!
//! ```bash
//! export XZEPR_KAFKA_SECURITY_PROTOCOL="SASL_SSL"
//! export XZEPR_KAFKA_SASL_MECHANISM="SCRAM-SHA-256"
//! export XZEPR_KAFKA_SASL_USERNAME="your-username"
//! export XZEPR_KAFKA_SASL_PASSWORD="your-password"
//! export XZEPR_KAFKA_SSL_CA_LOCATION="/path/to/ca-cert.pem"
//! export XZEPR_KAFKA_BROKERS="broker1.example.com:9093"
//!
//! cargo run --example kafka_with_auth
//! ```
//!
//! ## Using Configuration File
//!
//! Create a configuration file at `config/kafka-auth-example.yaml`:
//!
//! ```yaml
//! kafka:
//!   brokers: "broker1.example.com:9093"
//!   topic: "xzepr-events"
//!   auth:
//!     security_protocol: "SASL_SSL"
//!     sasl:
//!       mechanism: "SCRAM-SHA-256"
//!       username: "your-username"
//!       password: "your-password"
//!     ssl:
//!       ca_location: "/path/to/ca-cert.pem"
//! ```
//!
//! Then run:
//!
//! ```bash
//! XZEPR_CONFIG_FILE=config/kafka-auth-example.yaml cargo run --example kafka_with_auth
//! ```

use xzepr::infrastructure::messaging::config::{
    KafkaAuthConfig, SaslConfig, SaslMechanism, SecurityProtocol, SslConfig,
};
use xzepr::infrastructure::messaging::producer::KafkaEventPublisher;

/// Main example entry point
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,xzepr=debug,rdkafka=debug")
        .with_target(false)
        .with_thread_ids(true)
        .init();

    println!("=== XZepr Kafka Authentication Example ===\n");

    // Example 1: Load configuration from environment variables
    println!("Example 1: Loading authentication from environment variables");
    match load_from_environment().await {
        Ok(_) => println!("✓ Environment variable configuration successful\n"),
        Err(e) => println!("✗ Environment variable configuration failed: {}\n", e),
    }

    // Example 2: Create publisher with SCRAM-SHA-256 and SSL
    println!("Example 2: Creating publisher with SCRAM-SHA-256 + SSL");
    match create_scram_sha256_publisher().await {
        Ok(_) => println!("✓ SCRAM-SHA-256 publisher created successfully\n"),
        Err(e) => println!("✗ SCRAM-SHA-256 publisher creation failed: {}\n", e),
    }

    // Example 3: Create publisher with SCRAM-SHA-512 and SSL
    println!("Example 3: Creating publisher with SCRAM-SHA-512 + SSL");
    match create_scram_sha512_publisher().await {
        Ok(_) => println!("✓ SCRAM-SHA-512 publisher created successfully\n"),
        Err(e) => println!("✗ SCRAM-SHA-512 publisher creation failed: {}\n", e),
    }

    // Example 4: Show configuration summary
    println!("Example 4: Configuration Summary");
    print_configuration_summary();

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Example 1: Load authentication configuration from environment variables
///
/// This is the recommended approach for production deployments as it keeps
/// credentials out of configuration files and source control.
async fn load_from_environment() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Loading Kafka authentication from environment variables...");

    // Check if environment variables are set
    let security_protocol =
        std::env::var("XZEPR_KAFKA_SECURITY_PROTOCOL").unwrap_or_else(|_| "PLAINTEXT".to_string());

    println!("  Security protocol: {}", security_protocol);

    // Load authentication configuration from environment
    let auth_config = KafkaAuthConfig::from_env()?;

    // Validate configuration if present
    if let Some(ref config) = auth_config {
        config.validate()?;
        println!("  ✓ Authentication configuration loaded and validated");
    } else {
        println!("  ⚠ No authentication configured (using PLAINTEXT)");
    }

    // Get broker addresses
    let brokers =
        std::env::var("XZEPR_KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());

    println!("  Brokers: {}", brokers);

    // Create publisher with authentication
    let _publisher =
        KafkaEventPublisher::with_auth(&brokers, "xzepr-events", auth_config.as_ref())?;

    println!("  ✓ Publisher created with environment-based authentication");

    Ok(())
}

/// Example 2: Create publisher with SCRAM-SHA-256 authentication
///
/// SCRAM-SHA-256 is recommended for most production use cases as it provides
/// good security without the complexity of Kerberos.
async fn create_scram_sha256_publisher() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating publisher with SCRAM-SHA-256 + SSL...");

    // Note: In production, these values should come from environment variables
    // or a secure secrets management system
    let username = std::env::var("KAFKA_USERNAME").unwrap_or_else(|_| "example-user".to_string());
    let password =
        std::env::var("KAFKA_PASSWORD").unwrap_or_else(|_| "example-password".to_string());
    let ca_cert =
        std::env::var("KAFKA_CA_CERT").unwrap_or_else(|_| "/path/to/ca-cert.pem".to_string());

    // Create SASL configuration
    let sasl_config = SaslConfig::new(SaslMechanism::ScramSha256, username, password);

    // Create SSL configuration
    let ssl_config = SslConfig::new(Some(ca_cert.clone()), None, None);

    // Create authentication configuration
    let auth_config = KafkaAuthConfig::new(
        SecurityProtocol::SaslSsl,
        Some(sasl_config),
        Some(ssl_config),
    );

    // Validate configuration before using
    if let Err(e) = auth_config.validate() {
        println!("  ⚠ Configuration validation failed: {}", e);
        println!("  (This is expected if certificate files don't exist)");
        return Err(e.into());
    }

    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9093".to_string());

    // Create publisher
    let _publisher = KafkaEventPublisher::with_auth(&brokers, "xzepr-events", Some(&auth_config))?;

    println!("  ✓ Publisher created with SCRAM-SHA-256 authentication");
    println!("  ✓ SSL/TLS encryption enabled");
    println!("  ✓ CA certificate: {}", ca_cert);

    Ok(())
}

/// Example 3: Create publisher with SCRAM-SHA-512 authentication
///
/// SCRAM-SHA-512 provides higher security than SCRAM-SHA-256 and should be
/// used when security requirements are stringent.
async fn create_scram_sha512_publisher() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating publisher with SCRAM-SHA-512 + SSL...");

    let username = std::env::var("KAFKA_USERNAME").unwrap_or_else(|_| "example-user".to_string());
    let password =
        std::env::var("KAFKA_PASSWORD").unwrap_or_else(|_| "example-password".to_string());
    let ca_cert =
        std::env::var("KAFKA_CA_CERT").unwrap_or_else(|_| "/path/to/ca-cert.pem".to_string());

    // Use the convenience constructor for SCRAM-SHA-512 with SSL
    let auth_config =
        KafkaAuthConfig::scram_sha512_ssl(username.clone(), password, Some(ca_cert.clone()));

    // Validate configuration
    if let Err(e) = auth_config.validate() {
        println!("  ⚠ Configuration validation failed: {}", e);
        println!("  (This is expected if certificate files don't exist)");
        return Err(e.into());
    }

    let brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9093".to_string());

    // Create publisher
    let _publisher = KafkaEventPublisher::with_auth(&brokers, "xzepr-events", Some(&auth_config))?;

    println!("  ✓ Publisher created with SCRAM-SHA-512 authentication");
    println!("  ✓ Using enhanced security mechanism");
    println!("  ✓ Username: {}", username);

    Ok(())
}

/// Example 4: Configuration Summary
///
/// Shows the current configuration state and what authentication is active.
fn print_configuration_summary() {
    println!("  Current Kafka Configuration:");
    println!();

    let security_protocol = std::env::var("XZEPR_KAFKA_SECURITY_PROTOCOL")
        .unwrap_or_else(|_| "PLAINTEXT (default)".to_string());
    println!("  Security Protocol: {}", security_protocol);

    if security_protocol.contains("SASL") {
        let mechanism =
            std::env::var("XZEPR_KAFKA_SASL_MECHANISM").unwrap_or_else(|_| "Not set".to_string());
        let username =
            std::env::var("XZEPR_KAFKA_SASL_USERNAME").unwrap_or_else(|_| "Not set".to_string());
        println!("  SASL Mechanism: {}", mechanism);
        println!("  SASL Username: {}", username);
        println!("  SASL Password: [REDACTED]");
    }

    if security_protocol.contains("SSL") {
        let ca_location =
            std::env::var("XZEPR_KAFKA_SSL_CA_LOCATION").unwrap_or_else(|_| "Not set".to_string());
        println!("  SSL CA Location: {}", ca_location);
    }

    let brokers = std::env::var("XZEPR_KAFKA_BROKERS")
        .unwrap_or_else(|_| "localhost:9092 (default)".to_string());
    let topic =
        std::env::var("XZEPR_KAFKA_TOPIC").unwrap_or_else(|_| "xzepr-events (default)".to_string());

    println!();
    println!("  Brokers: {}", brokers);
    println!("  Topic: {}", topic);
    println!();

    // Note about event publishing
    println!("  Note: This example demonstrates authentication configuration.");
    println!("  To publish events, use the XZepr API endpoints with the server");
    println!("  configured to use this authentication.");
}

/// Helper function to demonstrate error handling
#[allow(dead_code)]
fn handle_authentication_error(error: Box<dyn std::error::Error>) {
    eprintln!("Authentication Error: {}", error);

    // Check common error scenarios
    let error_msg = error.to_string();

    if error_msg.contains("Authentication failed") {
        eprintln!("Tip: Verify your username and password are correct");
        eprintln!("     Check XZEPR_KAFKA_SASL_USERNAME and XZEPR_KAFKA_SASL_PASSWORD");
    } else if error_msg.contains("SSL") || error_msg.contains("certificate") {
        eprintln!("Tip: Verify SSL certificate paths are correct");
        eprintln!("     Check XZEPR_KAFKA_SSL_CA_LOCATION exists and is readable");
    } else if error_msg.contains("Connection") || error_msg.contains("timeout") {
        eprintln!("Tip: Verify Kafka broker addresses and network connectivity");
        eprintln!("     Check XZEPR_KAFKA_BROKERS and firewall rules");
    } else if error_msg.contains("Missing credential") {
        eprintln!("Tip: Ensure all required environment variables are set");
        eprintln!("     For SASL_SSL, you need: SECURITY_PROTOCOL, SASL_MECHANISM,");
        eprintln!("     SASL_USERNAME, SASL_PASSWORD, and SSL_CA_LOCATION");
    }
}

/// Example configuration for reference
#[allow(dead_code)]
fn print_example_configuration() {
    println!("Example Environment Variables:");
    println!();
    println!("# Security Protocol");
    println!("export XZEPR_KAFKA_SECURITY_PROTOCOL=\"SASL_SSL\"");
    println!();
    println!("# SASL Configuration");
    println!("export XZEPR_KAFKA_SASL_MECHANISM=\"SCRAM-SHA-256\"");
    println!("export XZEPR_KAFKA_SASL_USERNAME=\"your-kafka-username\"");
    println!("export XZEPR_KAFKA_SASL_PASSWORD=\"your-kafka-password\"");
    println!();
    println!("# SSL Configuration");
    println!("export XZEPR_KAFKA_SSL_CA_LOCATION=\"/path/to/ca-cert.pem\"");
    println!("export XZEPR_KAFKA_SSL_CERTIFICATE_LOCATION=\"/path/to/client-cert.pem\"");
    println!("export XZEPR_KAFKA_SSL_KEY_LOCATION=\"/path/to/client-key.pem\"");
    println!();
    println!("# Kafka Brokers");
    println!("export XZEPR_KAFKA_BROKERS=\"broker1.example.com:9093,broker2.example.com:9093\"");
    println!("export XZEPR_KAFKA_TOPIC=\"xzepr-events\"");
}
