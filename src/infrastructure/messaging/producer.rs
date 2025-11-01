// src/infrastructure/messaging/producer.rs

use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde_json;
use std::time::Duration;
use tracing::info;

use crate::domain::entities::event::Event;
use crate::error::{Error, InfrastructureError, Result};
use crate::infrastructure::messaging::cloudevents::CloudEventMessage;

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

    /// Publish an event to Kafka
    ///
    /// # Arguments
    ///
    /// * `event` - The event to publish
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the event was successfully published
    ///
    /// # Errors
    ///
    /// Returns InfrastructureError if publishing fails
    pub async fn publish(&self, event: &Event) -> Result<()> {
        // Convert Event to CloudEvents format for compatibility
        let cloudevent = CloudEventMessage::from_event(event);

        let payload = serde_json::to_string(&cloudevent).map_err(|e| {
            Error::Infrastructure(InfrastructureError::KafkaProducerError {
                message: format!("Failed to serialize CloudEvent message: {}", e),
            })
        })?;

        let key = event.id().to_string();

        let record = FutureRecord::to(&self.topic).key(&key).payload(&payload);

        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(err, _)| {
                Error::Infrastructure(InfrastructureError::KafkaProducerError {
                    message: format!("Failed to send event to Kafka: {}", err),
                })
            })?;

        info!(
            "Published CloudEvent {} (type: {}) to topic {}",
            event.id(),
            event.name(),
            self.topic
        );

        Ok(())
    }

    /// Publish a CloudEventMessage directly to Kafka
    ///
    /// # Arguments
    ///
    /// * `message` - The CloudEventMessage to publish
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the message was successfully published
    ///
    /// # Errors
    ///
    /// Returns InfrastructureError if publishing fails
    pub async fn publish_message(&self, message: &CloudEventMessage) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::event::CreateEventParams;
    use crate::domain::value_objects::EventReceiverId;

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
}
