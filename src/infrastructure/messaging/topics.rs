// src/infrastructure/messaging/topics.rs

use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::error::{Error, InfrastructureError, Result};

/// Kafka topic manager for creating and managing topics
pub struct TopicManager {
    admin_client: AdminClient<DefaultClientContext>,
}

impl TopicManager {
    /// Create a new TopicManager
    ///
    /// # Arguments
    ///
    /// * `brokers` - Comma-separated list of Kafka broker addresses
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xzepr::infrastructure::messaging::topics::TopicManager;
    ///
    /// let manager = TopicManager::new("localhost:9092").unwrap();
    /// ```
    pub fn new(brokers: &str) -> Result<Self> {
        let admin_client: AdminClient<DefaultClientContext> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("client.id", "xzepr-topic-manager")
            .create()
            .map_err(|e| {
                Error::Infrastructure(InfrastructureError::KafkaProducerError {
                    message: format!("Failed to create admin client: {}", e),
                })
            })?;

        Ok(Self { admin_client })
    }

    /// Ensure a topic exists, creating it if necessary
    ///
    /// This function is idempotent - it will not fail if the topic already exists.
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic to create
    /// * `num_partitions` - Number of partitions for the topic
    /// * `replication_factor` - Replication factor for the topic
    ///
    /// # Returns
    ///
    /// Returns Ok(true) if topic was created, Ok(false) if it already existed
    ///
    /// # Errors
    ///
    /// Returns InfrastructureError if topic creation fails for reasons other than already existing
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xzepr::infrastructure::messaging::topics::TopicManager;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = TopicManager::new("localhost:9092")?;
    /// manager.ensure_topic_exists("xzepr.dev.events", 3, 1).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ensure_topic_exists(
        &self,
        topic_name: &str,
        num_partitions: i32,
        replication_factor: i32,
    ) -> Result<bool> {
        info!(
            "Checking if topic '{}' exists (partitions: {}, replication: {})",
            topic_name, num_partitions, replication_factor
        );

        // Create the topic specification
        let new_topic = NewTopic::new(
            topic_name,
            num_partitions,
            TopicReplication::Fixed(replication_factor),
        );

        // Set admin options with timeout
        let admin_opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(10)));

        // Attempt to create the topic
        match self
            .admin_client
            .create_topics(&[new_topic], &admin_opts)
            .await
        {
            Ok(results) => {
                // Check the result for this specific topic
                if let Some(result) = results.into_iter().next() {
                    match result {
                        Ok(topic) => {
                            info!("Successfully created topic: {}", topic);
                            Ok(true)
                        }
                        Err((topic, err)) => {
                            // Check if error is because topic already exists
                            let error_str = format!("{:?}", err);
                            if error_str.contains("TopicAlreadyExists")
                                || error_str.contains("already exists")
                            {
                                info!("Topic '{}' already exists, skipping creation", topic);
                                Ok(false)
                            } else {
                                error!("Failed to create topic '{}': {:?}", topic, err);
                                Err(Error::Infrastructure(
                                    InfrastructureError::KafkaProducerError {
                                        message: format!(
                                            "Failed to create topic '{}': {:?}",
                                            topic, err
                                        ),
                                    },
                                ))
                            }
                        }
                    }
                } else {
                    Ok(false)
                }
            }
            Err(e) => {
                // Check if the error is about topic already existing
                let error_str = format!("{:?}", e);
                if error_str.contains("TopicAlreadyExists") || error_str.contains("already exists")
                {
                    info!(
                        "Topic '{}' already exists (detected in error), skipping creation",
                        topic_name
                    );
                    Ok(false)
                } else {
                    error!("Failed to create topic '{}': {:?}", topic_name, e);
                    Err(Error::Infrastructure(
                        InfrastructureError::KafkaProducerError {
                            message: format!("Failed to create topic '{}': {:?}", topic_name, e),
                        },
                    ))
                }
            }
        }
    }

    /// Ensure multiple topics exist
    ///
    /// # Arguments
    ///
    /// * `topics` - Vector of (topic_name, num_partitions, replication_factor) tuples
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if all topics exist or were created successfully
    ///
    /// # Errors
    ///
    /// Returns the first error encountered
    pub async fn ensure_topics_exist(
        &self,
        topics: Vec<(&str, i32, i32)>,
    ) -> Result<Vec<(String, bool)>> {
        let mut results = Vec::new();

        for (topic_name, num_partitions, replication_factor) in topics {
            match self
                .ensure_topic_exists(topic_name, num_partitions, replication_factor)
                .await
            {
                Ok(created) => {
                    results.push((topic_name.to_string(), created));
                }
                Err(e) => {
                    warn!(
                        "Failed to ensure topic '{}' exists: {}. Continuing...",
                        topic_name, e
                    );
                    return Err(e);
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_manager_creation() {
        // Test with valid broker string
        let result = TopicManager::new("localhost:9092");
        assert!(result.is_ok());
    }

    #[test]
    fn test_topic_manager_creation_with_multiple_brokers() {
        // Test with multiple brokers
        let result = TopicManager::new("localhost:9092,localhost:9093");
        assert!(result.is_ok());
    }
}
