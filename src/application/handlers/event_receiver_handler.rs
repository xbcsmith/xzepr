// src/application/handlers/event_receiver_handler.rs

use crate::domain::entities::event_receiver::EventReceiver;
use crate::domain::repositories::event_receiver_repo::{
    EventReceiverRepository, FindEventReceiverCriteria,
};
use crate::domain::value_objects::EventReceiverId;
use crate::error::{DomainError, Result};

use std::sync::Arc;
use tracing::{info, warn};

/// Application service for handling event receiver operations
#[derive(Clone)]
pub struct EventReceiverHandler {
    repository: Arc<dyn EventReceiverRepository>,
}

impl EventReceiverHandler {
    /// Creates a new event receiver handler
    pub fn new(repository: Arc<dyn EventReceiverRepository>) -> Self {
        Self { repository }
    }

    /// Creates a new event receiver
    pub async fn create_event_receiver(
        &self,
        name: String,
        receiver_type: String,
        version: String,
        description: String,
        schema: serde_json::Value,
    ) -> Result<EventReceiverId> {
        info!(
            name = %name,
            receiver_type = %receiver_type,
            version = %version,
            "Creating new event receiver"
        );

        // Check if a receiver with the same name and type already exists
        if self
            .repository
            .exists_by_name_and_type(&name, &receiver_type)
            .await?
        {
            warn!(
                name = %name,
                receiver_type = %receiver_type,
                "Event receiver with same name and type already exists"
            );
            return Err(DomainError::BusinessRuleViolation {
                rule: "Event receiver with the same name and type already exists".to_string(),
            }
            .into());
        }

        // Create the domain entity
        let event_receiver = EventReceiver::new(name, receiver_type, version, description, schema)?;

        let receiver_id = event_receiver.id();

        // Save to repository
        self.repository.save(&event_receiver).await?;

        info!(
            receiver_id = %receiver_id,
            fingerprint = %event_receiver.fingerprint(),
            "Event receiver created successfully"
        );

        Ok(receiver_id)
    }

    /// Gets an event receiver by ID
    pub async fn get_event_receiver(&self, id: EventReceiverId) -> Result<Option<EventReceiver>> {
        info!(receiver_id = %id, "Retrieving event receiver");

        let receiver = self.repository.find_by_id(id).await?;

        if receiver.is_none() {
            info!(receiver_id = %id, "Event receiver not found");
        }

        Ok(receiver)
    }

    /// Gets an event receiver by ID, returning an error if not found
    pub async fn get_event_receiver_or_error(&self, id: EventReceiverId) -> Result<EventReceiver> {
        self.get_event_receiver(id)
            .await?
            .ok_or_else(|| DomainError::ReceiverNotFound.into())
    }

    /// Lists event receivers by name (partial match)
    pub async fn find_by_name(&self, name: &str) -> Result<Vec<EventReceiver>> {
        info!(name = %name, "Finding event receivers by name");
        self.repository.find_by_name(name).await
    }

    /// Lists event receivers by type
    pub async fn find_by_type(&self, receiver_type: &str) -> Result<Vec<EventReceiver>> {
        info!(receiver_type = %receiver_type, "Finding event receivers by type");
        self.repository.find_by_type(receiver_type).await
    }

    /// Lists event receivers by type and version
    pub async fn find_by_type_and_version(
        &self,
        receiver_type: &str,
        version: &str,
    ) -> Result<Vec<EventReceiver>> {
        info!(
            receiver_type = %receiver_type,
            version = %version,
            "Finding event receivers by type and version"
        );
        self.repository
            .find_by_type_and_version(receiver_type, version)
            .await
    }

    /// Finds event receivers using multiple criteria
    pub async fn find_by_criteria(
        &self,
        criteria: FindEventReceiverCriteria,
    ) -> Result<Vec<EventReceiver>> {
        info!(?criteria, "Finding event receivers by criteria");

        if criteria.is_empty() {
            warn!("Empty criteria provided, using default pagination");
            let criteria = FindEventReceiverCriteria::new()
                .with_limit(50)
                .with_offset(0);
            return self.repository.find_by_criteria(criteria).await;
        }

        self.repository.find_by_criteria(criteria).await
    }

    /// Lists all event receivers with pagination
    pub async fn list_event_receivers(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventReceiver>> {
        info!(limit = %limit, offset = %offset, "Listing event receivers with pagination");

        // Validate pagination parameters
        if limit == 0 || limit > 1000 {
            return Err(DomainError::ValidationError {
                field: "limit".to_string(),
                message: "Limit must be between 1 and 1000".to_string(),
            }
            .into());
        }

        self.repository.list(limit, offset).await
    }

    /// Counts total number of event receivers
    pub async fn count_event_receivers(&self) -> Result<usize> {
        info!("Counting total event receivers");
        self.repository.count().await
    }

    /// Updates an existing event receiver
    pub async fn update_event_receiver(
        &self,
        id: EventReceiverId,
        name: Option<String>,
        receiver_type: Option<String>,
        version: Option<String>,
        description: Option<String>,
        schema: Option<serde_json::Value>,
    ) -> Result<()> {
        info!(receiver_id = %id, "Updating event receiver");

        // Get the existing receiver
        let mut receiver = self.get_event_receiver_or_error(id).await?;

        // If name or type is being changed, check for conflicts
        if let (Some(ref new_name), Some(ref new_type)) = (&name, &receiver_type) {
            if (new_name != receiver.name() || new_type != receiver.receiver_type())
                && self
                    .repository
                    .exists_by_name_and_type(new_name, new_type)
                    .await?
            {
                return Err(DomainError::BusinessRuleViolation {
                    rule: "Event receiver with the same name and type already exists".to_string(),
                }
                .into());
            }
        } else if let Some(ref new_name) = name {
            if new_name != receiver.name()
                && self
                    .repository
                    .exists_by_name_and_type(new_name, receiver.receiver_type())
                    .await?
            {
                return Err(DomainError::BusinessRuleViolation {
                    rule: "Event receiver with the same name and type already exists".to_string(),
                }
                .into());
            }
        } else if let Some(ref new_type) = receiver_type {
            if new_type != receiver.receiver_type()
                && self
                    .repository
                    .exists_by_name_and_type(receiver.name(), new_type)
                    .await?
            {
                return Err(DomainError::BusinessRuleViolation {
                    rule: "Event receiver with the same name and type already exists".to_string(),
                }
                .into());
            }
        }

        // Update the receiver
        receiver.update(name, receiver_type, version, description, schema)?;

        // Save the updated receiver
        self.repository.update(&receiver).await?;

        info!(
            receiver_id = %id,
            fingerprint = %receiver.fingerprint(),
            "Event receiver updated successfully"
        );

        Ok(())
    }

    /// Deletes an event receiver
    pub async fn delete_event_receiver(&self, id: EventReceiverId) -> Result<()> {
        info!(receiver_id = %id, "Deleting event receiver");

        // Check if the receiver exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(DomainError::ReceiverNotFound.into());
        }

        // TODO: Check if receiver is being used by any events or groups
        // This should be done by checking with other repositories

        self.repository.delete(id).await?;

        info!(receiver_id = %id, "Event receiver deleted successfully");

        Ok(())
    }

    /// Validates an event payload against a receiver's schema
    pub async fn validate_event_payload(
        &self,
        receiver_id: EventReceiverId,
        payload: &serde_json::Value,
    ) -> Result<()> {
        info!(receiver_id = %receiver_id, "Validating event payload");

        let receiver = self.get_event_receiver_or_error(receiver_id).await?;
        receiver.validate_event_payload(payload)?;

        Ok(())
    }

    /// Gets event receiver by fingerprint
    pub async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Option<EventReceiver>> {
        info!(fingerprint = %fingerprint, "Finding event receiver by fingerprint");
        self.repository.find_by_fingerprint(fingerprint).await
    }

    /// Checks if an event receiver exists
    pub async fn exists(&self, id: EventReceiverId) -> Result<bool> {
        Ok(self.repository.find_by_id(id).await?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::event_receiver_repo::EventReceiverRepository;
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Mock repository for testing
    struct MockEventReceiverRepository {
        receivers: Arc<Mutex<HashMap<EventReceiverId, EventReceiver>>>,
        name_type_index: Arc<Mutex<HashMap<(String, String), EventReceiverId>>>,
    }

    impl MockEventReceiverRepository {
        fn new() -> Self {
            Self {
                receivers: Arc::new(Mutex::new(HashMap::new())),
                name_type_index: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl EventReceiverRepository for MockEventReceiverRepository {
        async fn save(&self, event_receiver: &EventReceiver) -> Result<()> {
            let mut receivers = self.receivers.lock().unwrap();
            let mut index = self.name_type_index.lock().unwrap();

            receivers.insert(event_receiver.id(), event_receiver.clone());
            index.insert(
                (
                    event_receiver.name().to_string(),
                    event_receiver.receiver_type().to_string(),
                ),
                event_receiver.id(),
            );

            Ok(())
        }

        async fn find_by_id(&self, id: EventReceiverId) -> Result<Option<EventReceiver>> {
            let receivers = self.receivers.lock().unwrap();
            Ok(receivers.get(&id).cloned())
        }

        async fn exists_by_name_and_type(&self, name: &str, receiver_type: &str) -> Result<bool> {
            let index = self.name_type_index.lock().unwrap();
            Ok(index.contains_key(&(name.to_string(), receiver_type.to_string())))
        }

        // Implement other required methods with basic functionality
        async fn find_by_name(&self, _name: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_type(&self, _receiver_type: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_type_and_version(
            &self,
            _receiver_type: &str,
            _version: &str,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_fingerprint(&self, _fingerprint: &str) -> Result<Option<EventReceiver>> {
            Ok(None)
        }

        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn count(&self) -> Result<usize> {
            let receivers = self.receivers.lock().unwrap();
            Ok(receivers.len())
        }

        async fn update(&self, event_receiver: &EventReceiver) -> Result<()> {
            self.save(event_receiver).await
        }

        async fn delete(&self, id: EventReceiverId) -> Result<()> {
            let mut receivers = self.receivers.lock().unwrap();
            receivers.remove(&id);
            Ok(())
        }

        async fn find_by_criteria(
            &self,
            _criteria: FindEventReceiverCriteria,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_create_event_receiver() {
        let repository = Arc::new(MockEventReceiverRepository::new());
        let handler = EventReceiverHandler::new(repository);

        let schema = json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            }
        });

        let result = handler
            .create_event_receiver(
                "Test Receiver".to_string(),
                "webhook".to_string(),
                "1.0.0".to_string(),
                "A test receiver".to_string(),
                schema,
            )
            .await;

        assert!(result.is_ok());
        let receiver_id = result.unwrap();

        // Verify the receiver was created
        let receiver = handler.get_event_receiver(receiver_id).await.unwrap();
        assert!(receiver.is_some());
        assert_eq!(receiver.unwrap().name(), "Test Receiver");
    }

    #[tokio::test]
    async fn test_create_duplicate_receiver() {
        let repository = Arc::new(MockEventReceiverRepository::new());
        let handler = EventReceiverHandler::new(repository);

        let schema = json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            }
        });

        // Create first receiver
        let result1 = handler
            .create_event_receiver(
                "Test Receiver".to_string(),
                "webhook".to_string(),
                "1.0.0".to_string(),
                "A test receiver".to_string(),
                schema.clone(),
            )
            .await;
        assert!(result1.is_ok());

        // Try to create duplicate receiver
        let result2 = handler
            .create_event_receiver(
                "Test Receiver".to_string(),
                "webhook".to_string(),
                "2.0.0".to_string(), // Different version, but same name and type
                "Another test receiver".to_string(),
                schema,
            )
            .await;
        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_get_nonexistent_receiver() {
        let repository = Arc::new(MockEventReceiverRepository::new());
        let handler = EventReceiverHandler::new(repository);

        let receiver_id = EventReceiverId::new();
        let result = handler.get_event_receiver_or_error(receiver_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_pagination_limits() {
        let repository = Arc::new(MockEventReceiverRepository::new());
        let handler = EventReceiverHandler::new(repository);

        // Test invalid limits
        let result1 = handler.list_event_receivers(0, 0).await;
        assert!(result1.is_err());

        let result2 = handler.list_event_receivers(1001, 0).await;
        assert!(result2.is_err());

        // Test valid limits
        let result3 = handler.list_event_receivers(10, 0).await;
        assert!(result3.is_ok());
    }
}
