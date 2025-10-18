// src/application/handlers/event_receiver_group_handler.rs

use crate::domain::entities::event_receiver_group::EventReceiverGroup;
use crate::domain::repositories::event_receiver_group_repo::{EventReceiverGroupRepository, FindEventReceiverGroupCriteria};
use crate::domain::repositories::event_receiver_repo::EventReceiverRepository;
use crate::domain::value_objects::{EventReceiverId, EventReceiverGroupId};
use crate::error::{DomainError, Result};

use std::sync::Arc;
use tracing::{info, warn, error};

/// Application service for handling event receiver group operations
#[derive(Clone)]
pub struct EventReceiverGroupHandler {
    group_repository: Arc<dyn EventReceiverGroupRepository>,
    receiver_repository: Arc<dyn EventReceiverRepository>,
}

impl EventReceiverGroupHandler {
    /// Creates a new event receiver group handler
    pub fn new(
        group_repository: Arc<dyn EventReceiverGroupRepository>,
        receiver_repository: Arc<dyn EventReceiverRepository>,
    ) -> Self {
        Self {
            group_repository,
            receiver_repository,
        }
    }

    /// Creates a new event receiver group
    pub async fn create_event_receiver_group(
        &self,
        name: String,
        group_type: String,
        version: String,
        description: String,
        enabled: bool,
        event_receiver_ids: Vec<EventReceiverId>,
    ) -> Result<EventReceiverGroupId> {
        info!(
            name = %name,
            group_type = %group_type,
            version = %version,
            enabled = %enabled,
            receiver_count = %event_receiver_ids.len(),
            "Creating new event receiver group"
        );

        // Check if a group with the same name and type already exists
        if self.group_repository.exists_by_name_and_type(&name, &group_type).await? {
            warn!(
                name = %name,
                group_type = %group_type,
                "Event receiver group with same name and type already exists"
            );
            return Err(DomainError::BusinessRuleViolation {
                rule: "Event receiver group with the same name and type already exists".to_string(),
            }.into());
        }

        // Validate that all event receivers exist
        for receiver_id in &event_receiver_ids {
            if self.receiver_repository.find_by_id(*receiver_id).await?.is_none() {
                error!(receiver_id = %receiver_id, "Event receiver not found");
                return Err(DomainError::ReceiverNotFound.into());
            }
        }

        // Create the domain entity
        let event_receiver_group = EventReceiverGroup::new(
            name,
            group_type,
            version,
            description,
            enabled,
            event_receiver_ids,
        ).map_err(|e| e)?;

        let group_id = event_receiver_group.id();

        // Save to repository
        self.group_repository.save(&event_receiver_group).await?;

        info!(
            group_id = %group_id,
            receiver_count = %event_receiver_group.receiver_count(),
            "Event receiver group created successfully"
        );

        Ok(group_id)
    }

    /// Gets an event receiver group by ID
    pub async fn get_event_receiver_group(&self, id: EventReceiverGroupId) -> Result<Option<EventReceiverGroup>> {
        info!(group_id = %id, "Retrieving event receiver group");

        let group = self.group_repository.find_by_id(id).await?;

        if group.is_none() {
            info!(group_id = %id, "Event receiver group not found");
        }

        Ok(group)
    }

    /// Gets an event receiver group by ID, returning an error if not found
    pub async fn get_event_receiver_group_or_error(&self, id: EventReceiverGroupId) -> Result<EventReceiverGroup> {
        self.get_event_receiver_group(id)
            .await?
            .ok_or_else(|| DomainError::GroupNotFound.into())
    }

    /// Lists event receiver groups by name (partial match)
    pub async fn find_by_name(&self, name: &str) -> Result<Vec<EventReceiverGroup>> {
        info!(name = %name, "Finding event receiver groups by name");
        self.group_repository.find_by_name(name).await
    }

    /// Lists event receiver groups by type
    pub async fn find_by_type(&self, group_type: &str) -> Result<Vec<EventReceiverGroup>> {
        info!(group_type = %group_type, "Finding event receiver groups by type");
        self.group_repository.find_by_type(group_type).await
    }

    /// Lists event receiver groups by type and version
    pub async fn find_by_type_and_version(
        &self,
        group_type: &str,
        version: &str,
    ) -> Result<Vec<EventReceiverGroup>> {
        info!(
            group_type = %group_type,
            version = %version,
            "Finding event receiver groups by type and version"
        );
        self.group_repository.find_by_type_and_version(group_type, version).await
    }

    /// Finds enabled event receiver groups
    pub async fn find_enabled(&self) -> Result<Vec<EventReceiverGroup>> {
        info!("Finding enabled event receiver groups");
        self.group_repository.find_enabled().await
    }

    /// Finds disabled event receiver groups
    pub async fn find_disabled(&self) -> Result<Vec<EventReceiverGroup>> {
        info!("Finding disabled event receiver groups");
        self.group_repository.find_disabled().await
    }

    /// Finds groups that contain a specific event receiver
    pub async fn find_by_event_receiver_id(&self, receiver_id: EventReceiverId) -> Result<Vec<EventReceiverGroup>> {
        info!(receiver_id = %receiver_id, "Finding groups containing event receiver");
        self.group_repository.find_by_event_receiver_id(receiver_id).await
    }

    /// Finds event receiver groups using multiple criteria
    pub async fn find_by_criteria(&self, criteria: FindEventReceiverGroupCriteria) -> Result<Vec<EventReceiverGroup>> {
        info!(?criteria, "Finding event receiver groups by criteria");

        if criteria.is_empty() {
            warn!("Empty criteria provided, using default pagination");
            let criteria = FindEventReceiverGroupCriteria::new()
                .with_limit(50)
                .with_offset(0);
            return self.group_repository.find_by_criteria(criteria).await;
        }

        self.group_repository.find_by_criteria(criteria).await
    }

    /// Lists all event receiver groups with pagination
    pub async fn list_event_receiver_groups(&self, limit: usize, offset: usize) -> Result<Vec<EventReceiverGroup>> {
        info!(limit = %limit, offset = %offset, "Listing event receiver groups with pagination");

        // Validate pagination parameters
        if limit == 0 || limit > 1000 {
            return Err(DomainError::ValidationError {
                field: "limit".to_string(),
                message: "Limit must be between 1 and 1000".to_string(),
            }.into());
        }

        self.group_repository.list(limit, offset).await
    }

    /// Counts total number of event receiver groups
    pub async fn count_event_receiver_groups(&self) -> Result<usize> {
        info!("Counting total event receiver groups");
        self.group_repository.count().await
    }

    /// Counts enabled event receiver groups
    pub async fn count_enabled(&self) -> Result<usize> {
        info!("Counting enabled event receiver groups");
        self.group_repository.count_enabled().await
    }

    /// Counts disabled event receiver groups
    pub async fn count_disabled(&self) -> Result<usize> {
        info!("Counting disabled event receiver groups");
        self.group_repository.count_disabled().await
    }

    /// Updates an existing event receiver group
    pub async fn update_event_receiver_group(
        &self,
        id: EventReceiverGroupId,
        name: Option<String>,
        group_type: Option<String>,
        version: Option<String>,
        description: Option<String>,
        enabled: Option<bool>,
        event_receiver_ids: Option<Vec<EventReceiverId>>,
    ) -> Result<()> {
        info!(group_id = %id, "Updating event receiver group");

        // Get the existing group
        let mut group = self.get_event_receiver_group_or_error(id).await?;

        // If name or type is being changed, check for conflicts
        if let (Some(ref new_name), Some(ref new_type)) = (&name, &group_type) {
            if new_name != group.name() || new_type != group.group_type() {
                if self.group_repository.exists_by_name_and_type(new_name, new_type).await? {
                    return Err(DomainError::BusinessRuleViolation {
                        rule: "Event receiver group with the same name and type already exists".to_string(),
                    }.into());
                }
            }
        } else if let Some(ref new_name) = name {
            if new_name != group.name()
                && self.group_repository.exists_by_name_and_type(new_name, group.group_type()).await? {
                return Err(DomainError::BusinessRuleViolation {
                    rule: "Event receiver group with the same name and type already exists".to_string(),
                }.into());
            }
        } else if let Some(ref new_type) = group_type {
            if new_type != group.group_type()
                && self.group_repository.exists_by_name_and_type(group.name(), new_type).await? {
                return Err(DomainError::BusinessRuleViolation {
                    rule: "Event receiver group with the same name and type already exists".to_string(),
                }.into());
            }
        }

        // If event receiver IDs are being updated, validate they exist
        if let Some(ref new_receiver_ids) = event_receiver_ids {
            for receiver_id in new_receiver_ids {
                if self.receiver_repository.find_by_id(*receiver_id).await?.is_none() {
                    error!(receiver_id = %receiver_id, "Event receiver not found");
                    return Err(DomainError::ReceiverNotFound.into());
                }
            }
        }

        // Update the group
        group.update(name, group_type, version, description, enabled, event_receiver_ids)?;

        // Save the updated group
        self.group_repository.update(&group).await?;

        info!(
            group_id = %id,
            receiver_count = %group.receiver_count(),
            enabled = %group.enabled(),
            "Event receiver group updated successfully"
        );

        Ok(())
    }

    /// Enables an event receiver group
    pub async fn enable_event_receiver_group(&self, id: EventReceiverGroupId) -> Result<EventReceiverGroupId> {
        info!(group_id = %id, "Enabling event receiver group");

        let mut group = self.get_event_receiver_group_or_error(id).await?;

        if group.enabled() {
            info!(group_id = %id, "Event receiver group is already enabled");
            return Ok(id);
        }

        group.enable();
        self.group_repository.update(&group).await?;

        info!(group_id = %id, "Event receiver group enabled successfully");
        Ok(id)
    }

    /// Disables an event receiver group
    pub async fn disable_event_receiver_group(&self, id: EventReceiverGroupId) -> Result<EventReceiverGroupId> {
        info!(group_id = %id, "Disabling event receiver group");

        let mut group = self.get_event_receiver_group_or_error(id).await?;

        if !group.enabled() {
            info!(group_id = %id, "Event receiver group is already disabled");
            return Ok(id);
        }

        group.disable();
        self.group_repository.update(&group).await?;

        info!(group_id = %id, "Event receiver group disabled successfully");
        Ok(id)
    }

    /// Adds an event receiver to a group
    pub async fn add_event_receiver_to_group(
        &self,
        group_id: EventReceiverGroupId,
        receiver_id: EventReceiverId,
    ) -> Result<()> {
        info!(
            group_id = %group_id,
            receiver_id = %receiver_id,
            "Adding event receiver to group"
        );

        // Verify the receiver exists
        if self.receiver_repository.find_by_id(receiver_id).await?.is_none() {
            return Err(DomainError::ReceiverNotFound.into());
        }

        // Get the group
        let mut group = self.get_event_receiver_group_or_error(group_id).await?;

        // Add the receiver to the group
        group.add_event_receiver(receiver_id)?;

        // Save the updated group
        self.group_repository.update(&group).await?;

        info!(
            group_id = %group_id,
            receiver_id = %receiver_id,
            receiver_count = %group.receiver_count(),
            "Event receiver added to group successfully"
        );

        Ok(())
    }

    /// Removes an event receiver from a group
    pub async fn remove_event_receiver_from_group(
        &self,
        group_id: EventReceiverGroupId,
        receiver_id: EventReceiverId,
    ) -> Result<()> {
        info!(
            group_id = %group_id,
            receiver_id = %receiver_id,
            "Removing event receiver from group"
        );

        // Get the group
        let mut group = self.get_event_receiver_group_or_error(group_id).await?;

        // Remove the receiver from the group
        group.remove_event_receiver(receiver_id)?;

        // Save the updated group
        self.group_repository.update(&group).await?;

        info!(
            group_id = %group_id,
            receiver_id = %receiver_id,
            receiver_count = %group.receiver_count(),
            "Event receiver removed from group successfully"
        );

        Ok(())
    }

    /// Deletes an event receiver group
    pub async fn delete_event_receiver_group(&self, id: EventReceiverGroupId) -> Result<()> {
        info!(group_id = %id, "Deleting event receiver group");

        // Check if the group exists
        if self.group_repository.find_by_id(id).await?.is_none() {
            return Err(DomainError::GroupNotFound.into());
        }

        // TODO: Check if group is being referenced by any events
        // This should be done by checking with event repository

        self.group_repository.delete(id).await?;

        info!(group_id = %id, "Event receiver group deleted successfully");

        Ok(())
    }

    /// Checks if an event receiver group exists
    pub async fn exists(&self, id: EventReceiverGroupId) -> Result<bool> {
        Ok(self.group_repository.find_by_id(id).await?.is_some())
    }

    /// Checks if a group contains a specific event receiver
    pub async fn group_contains_receiver(
        &self,
        group_id: EventReceiverGroupId,
        receiver_id: EventReceiverId,
    ) -> Result<bool> {
        let group = self.get_event_receiver_group_or_error(group_id).await?;
        Ok(group.contains_receiver(receiver_id))
    }

    /// Gets all event receiver IDs for a specific group
    pub async fn get_group_event_receivers(&self, group_id: EventReceiverGroupId) -> Result<Vec<EventReceiverId>> {
        let group = self.get_event_receiver_group_or_error(group_id).await?;
        Ok(group.event_receiver_ids().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::event_receiver_repo::EventReceiverRepository;
    use crate::domain::repositories::event_receiver_group_repo::EventReceiverGroupRepository;
    use crate::domain::entities::event_receiver::EventReceiver;
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Mock repositories for testing
    struct MockEventReceiverRepository {
        receivers: Arc<Mutex<HashMap<EventReceiverId, EventReceiver>>>,
    }

    impl MockEventReceiverRepository {
        fn new() -> Self {
            Self {
                receivers: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn add_receiver(&self, receiver: EventReceiver) {
            let mut receivers = self.receivers.lock().unwrap();
            receivers.insert(receiver.id(), receiver);
        }
    }

    #[async_trait]
    impl EventReceiverRepository for MockEventReceiverRepository {
        async fn find_by_id(&self, id: EventReceiverId) -> Result<Option<EventReceiver>> {
            let receivers = self.receivers.lock().unwrap();
            Ok(receivers.get(&id).cloned())
        }

        // Implement other required methods with basic functionality
        async fn save(&self, _event_receiver: &EventReceiver) -> Result<()> { Ok(()) }
        async fn find_by_name(&self, _name: &str) -> Result<Vec<EventReceiver>> { Ok(vec![]) }
        async fn find_by_type(&self, _receiver_type: &str) -> Result<Vec<EventReceiver>> { Ok(vec![]) }
        async fn find_by_type_and_version(&self, _receiver_type: &str, _version: &str) -> Result<Vec<EventReceiver>> { Ok(vec![]) }
        async fn find_by_fingerprint(&self, _fingerprint: &str) -> Result<Option<EventReceiver>> { Ok(None) }
        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<EventReceiver>> { Ok(vec![]) }
        async fn count(&self) -> Result<usize> { Ok(0) }
        async fn update(&self, _event_receiver: &EventReceiver) -> Result<()> { Ok(()) }
        async fn delete(&self, _id: EventReceiverId) -> Result<()> { Ok(()) }
        async fn exists_by_name_and_type(&self, _name: &str, _receiver_type: &str) -> Result<bool> { Ok(false) }
        async fn find_by_criteria(&self, _criteria: crate::domain::repositories::event_receiver_repo::FindEventReceiverCriteria) -> Result<Vec<EventReceiver>> { Ok(vec![]) }
    }

    struct MockEventReceiverGroupRepository {
        groups: Arc<Mutex<HashMap<EventReceiverGroupId, EventReceiverGroup>>>,
        name_type_index: Arc<Mutex<HashMap<(String, String), EventReceiverGroupId>>>,
    }

    impl MockEventReceiverGroupRepository {
        fn new() -> Self {
            Self {
                groups: Arc::new(Mutex::new(HashMap::new())),
                name_type_index: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl EventReceiverGroupRepository for MockEventReceiverGroupRepository {
        async fn save(&self, group: &EventReceiverGroup) -> Result<()> {
            let mut groups = self.groups.lock().unwrap();
            let mut index = self.name_type_index.lock().unwrap();

            groups.insert(group.id(), group.clone());
            index.insert(
                (group.name().to_string(), group.group_type().to_string()),
                group.id(),
            );

            Ok(())
        }

        async fn find_by_id(&self, id: EventReceiverGroupId) -> Result<Option<EventReceiverGroup>> {
            let groups = self.groups.lock().unwrap();
            Ok(groups.get(&id).cloned())
        }

        async fn exists_by_name_and_type(&self, name: &str, group_type: &str) -> Result<bool> {
            let index = self.name_type_index.lock().unwrap();
            Ok(index.contains_key(&(name.to_string(), group_type.to_string())))
        }

        async fn update(&self, group: &EventReceiverGroup) -> Result<()> {
            self.save(group).await
        }

        // Implement other required methods with basic functionality
        async fn find_by_name(&self, _name: &str) -> Result<Vec<EventReceiverGroup>> { Ok(vec![]) }
        async fn find_by_type(&self, _group_type: &str) -> Result<Vec<EventReceiverGroup>> { Ok(vec![]) }
        async fn find_by_type_and_version(&self, _group_type: &str, _version: &str) -> Result<Vec<EventReceiverGroup>> { Ok(vec![]) }
        async fn find_enabled(&self) -> Result<Vec<EventReceiverGroup>> { Ok(vec![]) }
        async fn find_disabled(&self) -> Result<Vec<EventReceiverGroup>> { Ok(vec![]) }
        async fn find_by_event_receiver_id(&self, _receiver_id: EventReceiverId) -> Result<Vec<EventReceiverGroup>> { Ok(vec![]) }
        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<EventReceiverGroup>> { Ok(vec![]) }
        async fn count(&self) -> Result<usize> { Ok(0) }
        async fn count_enabled(&self) -> Result<usize> { Ok(0) }
        async fn count_disabled(&self) -> Result<usize> { Ok(0) }
        async fn delete(&self, _id: EventReceiverGroupId) -> Result<()> { Ok(()) }
        async fn enable(&self, _id: EventReceiverGroupId) -> Result<()> { Ok(()) }
        async fn disable(&self, _id: EventReceiverGroupId) -> Result<()> { Ok(()) }
        async fn find_by_criteria(&self, _criteria: FindEventReceiverGroupCriteria) -> Result<Vec<EventReceiverGroup>> { Ok(vec![]) }
        async fn add_event_receiver_to_group(&self, _group_id: EventReceiverGroupId, _receiver_id: EventReceiverId) -> Result<()> { Ok(()) }
        async fn remove_event_receiver_from_group(&self, _group_id: EventReceiverGroupId, _receiver_id: EventReceiverId) -> Result<()> { Ok(()) }
        async fn get_group_event_receivers(&self, _group_id: EventReceiverGroupId) -> Result<Vec<EventReceiverId>> { Ok(vec![]) }
    }

    #[tokio::test]
    async fn test_create_event_receiver_group() {
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let group_repo = Arc::new(MockEventReceiverGroupRepository::new());
        let handler = EventReceiverGroupHandler::new(group_repo, receiver_repo.clone());

        // Create a mock receiver
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test receiver".to_string(),
            json!({"type": "object"}),
        ).unwrap();
        let receiver_id = receiver.id();
        receiver_repo.add_receiver(receiver);

        let result = handler.create_event_receiver_group(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test group".to_string(),
            true,
            vec![receiver_id],
        ).await;

        assert!(result.is_ok());
        let group_id = result.unwrap();

        // Verify the group was created
        let group = handler.get_event_receiver_group(group_id).await.unwrap();
        assert!(group.is_some());
        let group = group.unwrap();
        assert_eq!(group.name(), "Test Group");
        assert!(group.enabled());
        assert_eq!(group.receiver_count(), 1);
    }

    #[tokio::test]
    async fn test_create_group_with_nonexistent_receiver() {
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let group_repo = Arc::new(MockEventReceiverGroupRepository::new());
        let handler = EventReceiverGroupHandler::new(group_repo, receiver_repo);

        let nonexistent_receiver_id = EventReceiverId::new();

        let result = handler.create_event_receiver_group(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test group".to_string(),
            true,
            vec![nonexistent_receiver_id],
        ).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enable_disable_group() {
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let group_repo = Arc::new(MockEventReceiverGroupRepository::new());
        let handler = EventReceiverGroupHandler::new(group_repo, receiver_repo.clone());

        // Create a mock receiver
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test receiver".to_string(),
            json!({"type": "object"}),
        ).unwrap();
        let receiver_id = receiver.id();
        receiver_repo.add_receiver(receiver);

        // Create a disabled group
        let group_id = handler.create_event_receiver_group(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test group".to_string(),
            false,
            vec![receiver_id],
        ).await.unwrap();

        // Enable the group
        let result = handler.enable_event_receiver_group(group_id).await;
        assert!(result.is_ok());

        // Disable the group
        let result = handler.disable_event_receiver_group(group_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_nonexistent_group() {
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let group_repo = Arc::new(MockEventReceiverGroupRepository::new());
        let handler = EventReceiverGroupHandler::new(group_repo, receiver_repo);

        let group_id = EventReceiverGroupId::new();
        let result = handler.get_event_receiver_group_or_error(group_id).await;
        assert!(result.is_err());
    }
}
