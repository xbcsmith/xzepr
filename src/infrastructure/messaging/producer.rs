// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/messaging/producer.rs

use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde_json;
use std::time::Duration;
use tracing::info;

use crate::domain::entities::event::Event;
use crate::domain::entities::event_receiver::EventReceiver;
use crate::domain::entities::event_receiver_group::EventReceiverGroup;
use crate::domain::repositories::event_publisher::EventPublisher;
use crate::error::{Error, InfrastructureError, Result};
use crate::infrastructure::messaging::cloudevents::CloudEventMessage;
use crate::infrastructure::messaging::config::KafkaAuthConfig;

/// Kafka event publisher for sending events to Kafka topics
pub struct KafkaEventPublisher {
    producer: FutureProducer,
    topic: String,
}

impl KafkaEventPublisher {
    /// Create a new KafkaEventPublisher
    ///
    /// # Arguments
    ///
    /// * `brokers` - Comma-separated list of Kafka broker addresses
    /// * `topic` - The Kafka topic to publish events to
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xzepr::infrastructure::messaging::producer::KafkaEventPublisher;
    ///
    /// let publisher = KafkaEventPublisher::new("localhost:9092", "xzepr.dev.events").unwrap();
    /// ```
    pub fn new(brokers: &str, topic: &str) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("client.id", "xzepr-event-publisher")
            .create()
            .map_err(|e| {
                Error::Infrastructure(InfrastructureError::KafkaProducerError {
                    message: format!("Failed to create Kafka producer: {}", e),
                })
            })?;

        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    /// Create a new KafkaEventPublisher with authentication
    ///
    /// # Arguments
    ///
    /// * `brokers` - Comma-separated list of Kafka broker addresses
    /// * `topic` - The Kafka topic to publish events to
    /// * `auth_config` - Optional authentication configuration for SASL/SCRAM or SSL/TLS
    ///
    /// # Returns
    ///
    /// Returns a Result containing the KafkaEventPublisher or an error
    ///
    /// # Errors
    ///
    /// Returns InfrastructureError if:
    /// - Authentication configuration is invalid
    /// - Producer creation fails
    /// - SSL certificate files are missing or invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xzepr::infrastructure::messaging::producer::KafkaEventPublisher;
    /// use xzepr::infrastructure::messaging::config::KafkaAuthConfig;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create publisher with authentication
    /// let auth_config = KafkaAuthConfig::from_env()?;
    /// let publisher = KafkaEventPublisher::with_auth(
    ///     "localhost:9092",
    ///     "xzepr.dev.events",
    ///     auth_config.as_ref()
    /// )?;
    ///
    /// // Create publisher without authentication (backward compatible)
    /// let publisher = KafkaEventPublisher::with_auth(
    ///     "localhost:9092",
    ///     "xzepr.dev.events",
    ///     None
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_auth(
        brokers: &str,
        topic: &str,
        auth_config: Option<&KafkaAuthConfig>,
    ) -> Result<Self> {
        let mut client_config = ClientConfig::new();
        client_config
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("client.id", "xzepr-event-publisher");

        // Apply authentication configuration if provided
        if let Some(auth) = auth_config {
            auth.apply_to_client_config(&mut client_config);
        }

        let producer: FutureProducer = client_config.create().map_err(|e| {
            Error::Infrastructure(InfrastructureError::KafkaProducerError {
                message: format!("Failed to create Kafka producer: {}", e),
            })
        })?;

        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    /// Publish an event to Kafka.
    ///
    /// Constructs a CloudEvents envelope from the domain event and forwards it
    /// to the configured topic via `send_cloud_event_message_internal`.
    ///
    /// # Arguments
    ///
    /// * `event` - The domain event to publish.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was successfully delivered to Kafka.
    ///
    /// # Errors
    ///
    /// Returns `InfrastructureError::KafkaProducerError` if serialisation or
    /// delivery fails.
    pub async fn publish(&self, event: &Event) -> Result<()> {
        let cloudevent = CloudEventMessage::from_event(event);
        self.send_cloud_event_message_internal(&cloudevent).await
    }

    /// Publish a [`CloudEventMessage`] directly to Kafka.
    ///
    /// This is an inherent convenience method for callers that have already
    /// constructed a [`CloudEventMessage`].  It delegates to the private
    /// `send_cloud_event_message_internal` helper.
    ///
    /// # Arguments
    ///
    /// * `message` - The pre-built CloudEvents message to publish.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the message was successfully delivered.
    ///
    /// # Errors
    ///
    /// Returns `InfrastructureError::KafkaProducerError` if serialisation or
    /// delivery fails.
    pub async fn publish_message(&self, message: &CloudEventMessage) -> Result<()> {
        self.send_cloud_event_message_internal(message).await
    }

    /// Serialise and send a [`CloudEventMessage`] to the Kafka topic.
    ///
    /// This is the single private method that contains all Kafka I/O logic.
    /// Both the inherent `publish`/`publish_message` methods and the
    /// [`EventPublisher`] trait implementation delegate here so that the
    /// transport code lives in exactly one place.
    ///
    /// # Errors
    ///
    /// Returns `InfrastructureError::KafkaProducerError` on serialisation or
    /// delivery failure.
    async fn send_cloud_event_message_internal(&self, message: &CloudEventMessage) -> Result<()> {
        let payload = serde_json::to_string(message).map_err(|e| {
            Error::Infrastructure(InfrastructureError::KafkaProducerError {
                message: format!("Failed to serialize CloudEvent message: {}", e),
            })
        })?;

        let key = message.id.clone();
        let record = FutureRecord::to(&self.topic).key(&key).payload(&payload);

        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(err, _)| {
                Error::Infrastructure(InfrastructureError::KafkaProducerError {
                    message: format!("Failed to send message to Kafka: {}", err),
                })
            })?;

        info!(
            "Published CloudEvent {} (type: {}) to topic {}",
            message.id, message.event_type, self.topic
        );

        Ok(())
    }
}

/// [`EventPublisher`] trait implementation for [`KafkaEventPublisher`].
///
/// All three methods construct the appropriate [`CloudEventMessage`] variant
/// and forward it to the private `send_cloud_event_message_internal` helper.
/// The inherent `publish` and `publish_message` methods remain available for
/// direct use and are kept for backward compatibility.
#[async_trait::async_trait]
impl EventPublisher for KafkaEventPublisher {
    /// Publish a domain event as a basic CloudEvents envelope.
    ///
    /// Delegates to [`CloudEventMessage::from_event`] then sends via the
    /// internal Kafka helper.
    ///
    /// # Errors
    ///
    /// Returns an error if serialisation or Kafka delivery fails.
    async fn publish(&self, event: &Event) -> crate::error::Result<()> {
        // Explicitly call the inherent method to avoid ambiguity.
        KafkaEventPublisher::publish(self, event).await
    }

    /// Publish a domain event with receiver context included in the envelope.
    ///
    /// Delegates to [`CloudEventMessage::from_event_with_receiver`] then
    /// sends via the internal Kafka helper.
    ///
    /// # Errors
    ///
    /// Returns an error if serialisation or Kafka delivery fails.
    async fn publish_with_receiver(
        &self,
        event: &Event,
        receiver: &EventReceiver,
    ) -> crate::error::Result<()> {
        let message = CloudEventMessage::from_event_with_receiver(event, receiver);
        self.send_cloud_event_message_internal(&message).await
    }

    /// Publish a domain event with receiver-group context included in the envelope.
    ///
    /// Delegates to [`CloudEventMessage::from_event_with_group`] then sends
    /// via the internal Kafka helper.
    ///
    /// # Errors
    ///
    /// Returns an error if serialisation or Kafka delivery fails.
    async fn publish_with_group(
        &self,
        event: &Event,
        group: &EventReceiverGroup,
    ) -> crate::error::Result<()> {
        let message = CloudEventMessage::from_event_with_group(event, group);
        self.send_cloud_event_message_internal(&message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::event::CreateEventParams;
    use crate::domain::entities::event_receiver::EventReceiver;
    use crate::domain::entities::event_receiver_group::EventReceiverGroup;
    use crate::domain::value_objects::{EventReceiverId, UserId};

    #[test]
    fn test_kafka_publisher_creation() {
        // Test with valid broker string
        let result = KafkaEventPublisher::new("localhost:9092", "test-topic");
        assert!(result.is_ok());
    }

    #[test]
    fn test_kafka_publisher_creation_with_multiple_brokers() {
        // Test with multiple brokers
        let result = KafkaEventPublisher::new("localhost:9092,localhost:9093", "test-topic");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cloudevents_message_creation() {
        // Verify CloudEvents message is created correctly
        let receiver_id = EventReceiverId::new();
        let event = Event::new(CreateEventParams {
            name: "test.event".to_string(),
            version: "1.0.0".to_string(),
            release: "1.0.0".to_string(),
            platform_id: "test".to_string(),
            package: "test-pkg".to_string(),
            description: "Test event".to_string(),
            payload: serde_json::json!({"key": "value"}),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let cloudevent = CloudEventMessage::from_event(&event);
        let json = serde_json::to_string(&cloudevent).unwrap();

        // Verify JSON structure matches expected CloudEvents format
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["specversion"], "1.0.1");
        assert_eq!(parsed["type"], "test.event");
        assert_eq!(parsed["api_version"], "v1");
        assert!(parsed["data"].is_object());
    }

    #[test]
    fn test_kafka_publisher_with_auth_none() {
        // Test with_auth with no authentication (backward compatible)
        let result = KafkaEventPublisher::with_auth("localhost:9092", "test-topic", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kafka_publisher_with_auth_plaintext() {
        // Test with_auth with plaintext security protocol
        use crate::infrastructure::messaging::config::{KafkaAuthConfig, SecurityProtocol};

        let auth_config = KafkaAuthConfig {
            security_protocol: SecurityProtocol::Plaintext,
            sasl_config: None,
            ssl_config: None,
        };

        let result =
            KafkaEventPublisher::with_auth("localhost:9092", "test-topic", Some(&auth_config));
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires rdkafka compiled with libsasl2 or openssl support for SCRAM-SHA-256"]
    fn test_kafka_publisher_with_auth_sasl_scram_sha256() {
        // Test with_auth with SASL/SCRAM-SHA-256
        // Note: This test requires rdkafka to be compiled with SASL/SCRAM support
        use crate::infrastructure::messaging::config::{
            KafkaAuthConfig, SaslConfig, SaslMechanism, SecurityProtocol,
        };

        let sasl_config = SaslConfig {
            mechanism: SaslMechanism::ScramSha256,
            username: "test-user".to_string(),
            password: "test-password".to_string(),
        };

        let auth_config = KafkaAuthConfig {
            security_protocol: SecurityProtocol::SaslPlaintext,
            sasl_config: Some(sasl_config),
            ssl_config: None,
        };

        let result =
            KafkaEventPublisher::with_auth("localhost:9092", "test-topic", Some(&auth_config));
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires rdkafka compiled with libsasl2 or openssl support for SCRAM-SHA-512"]
    fn test_kafka_publisher_with_auth_sasl_scram_sha512() {
        // Test with_auth with SASL/SCRAM-SHA-512
        // Note: This test requires rdkafka to be compiled with SASL/SCRAM support
        use crate::infrastructure::messaging::config::{
            KafkaAuthConfig, SaslConfig, SaslMechanism, SecurityProtocol,
        };

        let sasl_config = SaslConfig {
            mechanism: SaslMechanism::ScramSha512,
            username: "test-user".to_string(),
            password: "test-password".to_string(),
        };

        let auth_config = KafkaAuthConfig {
            security_protocol: SecurityProtocol::SaslPlaintext,
            sasl_config: Some(sasl_config),
            ssl_config: None,
        };

        let result =
            KafkaEventPublisher::with_auth("localhost:9092", "test-topic", Some(&auth_config));
        assert!(result.is_ok());
    }

    #[test]
    fn test_kafka_publisher_with_auth_sasl_plain() {
        // Test with_auth with SASL/PLAIN
        use crate::infrastructure::messaging::config::{
            KafkaAuthConfig, SaslConfig, SaslMechanism, SecurityProtocol,
        };

        let sasl_config = SaslConfig {
            mechanism: SaslMechanism::Plain,
            username: "test-user".to_string(),
            password: "test-password".to_string(),
        };

        let auth_config = KafkaAuthConfig {
            security_protocol: SecurityProtocol::SaslPlaintext,
            sasl_config: Some(sasl_config),
            ssl_config: None,
        };

        let result =
            KafkaEventPublisher::with_auth("localhost:9092", "test-topic", Some(&auth_config));
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires rdkafka compiled with libsasl2 or openssl support for SCRAM-SHA-256"]
    fn test_kafka_publisher_with_auth_multiple_brokers() {
        // Test with_auth with multiple brokers and SASL/SCRAM authentication
        // Note: This test requires rdkafka to be compiled with SASL/SCRAM support
        use crate::infrastructure::messaging::config::{
            KafkaAuthConfig, SaslConfig, SaslMechanism, SecurityProtocol,
        };

        let sasl_config = SaslConfig {
            mechanism: SaslMechanism::ScramSha256,
            username: "test-user".to_string(),
            password: "test-password".to_string(),
        };

        let auth_config = KafkaAuthConfig {
            security_protocol: SecurityProtocol::SaslPlaintext,
            sasl_config: Some(sasl_config),
            ssl_config: None,
        };

        let result = KafkaEventPublisher::with_auth(
            "localhost:9092,localhost:9093,localhost:9094",
            "test-topic",
            Some(&auth_config),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_kafka_publisher_backward_compatibility() {
        // Verify both new() and with_auth(None) produce equivalent results
        let result_new = KafkaEventPublisher::new("localhost:9092", "test-topic");
        let result_with_auth = KafkaEventPublisher::with_auth("localhost:9092", "test-topic", None);

        assert!(result_new.is_ok());
        assert!(result_with_auth.is_ok());
    }

    /// Helper: build a minimal valid Event for tests.
    fn make_test_event() -> Event {
        let receiver_id = EventReceiverId::new();
        // SAFETY: All inputs are controlled test values; validation cannot fail.
        Event::new(CreateEventParams {
            name: "test.event".to_string(),
            version: "1.0.0".to_string(),
            release: "1.0.0".to_string(),
            platform_id: "test".to_string(),
            package: "test-pkg".to_string(),
            description: "Test event".to_string(),
            payload: serde_json::json!({"key": "value"}),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .expect("SAFETY: All inputs are controlled test values; validation cannot fail.")
    }

    /// Helper: build a minimal valid EventReceiver for tests.
    fn make_test_receiver() -> EventReceiver {
        // SAFETY: All inputs are controlled test values; validation cannot fail.
        EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            serde_json::json!({"type": "object"}),
            UserId::new(),
        )
        .expect("SAFETY: All inputs are controlled test values; validation cannot fail.")
    }

    /// Helper: build a minimal valid EventReceiverGroup for tests.
    fn make_test_group() -> EventReceiverGroup {
        // SAFETY: All inputs are controlled test values; validation cannot fail.
        EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            vec![EventReceiverId::new()],
            UserId::new(),
        )
        .expect("SAFETY: All inputs are controlled test values; validation cannot fail.")
    }

    #[test]
    fn test_event_publisher_trait_impl_exists() {
        // Verify KafkaEventPublisher satisfies the EventPublisher trait bound.
        // SAFETY: publisher creation with a broker string succeeds without a live broker.
        let publisher = KafkaEventPublisher::new("localhost:9092", "test-topic").expect(
            "SAFETY: publisher creation with a broker string succeeds without a live broker.",
        );
        // Coerce to trait object to confirm the impl compiles.
        let _: &dyn EventPublisher = &publisher;
    }

    #[tokio::test]
    #[ignore = "Requires a running Kafka / Redpanda instance"]
    async fn test_event_publisher_publish_delegates_to_kafka() {
        // SAFETY: test is ignored without a live broker.
        let publisher = KafkaEventPublisher::new("localhost:9092", "test-topic").unwrap();
        let event = make_test_event();
        let result = EventPublisher::publish(&publisher, &event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires a running Kafka / Redpanda instance"]
    async fn test_event_publisher_publish_with_receiver() {
        // SAFETY: test is ignored without a live broker.
        let publisher = KafkaEventPublisher::new("localhost:9092", "test-topic").unwrap();
        let event = make_test_event();
        let receiver = make_test_receiver();
        let result = EventPublisher::publish_with_receiver(&publisher, &event, &receiver).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires a running Kafka / Redpanda instance"]
    async fn test_event_publisher_publish_with_group() {
        // SAFETY: test is ignored without a live broker.
        let publisher = KafkaEventPublisher::new("localhost:9092", "test-topic").unwrap();
        let event = make_test_event();
        let group = make_test_group();
        let result = EventPublisher::publish_with_group(&publisher, &event, &group).await;
        assert!(result.is_ok());
    }
}
