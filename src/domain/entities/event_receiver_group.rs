// src/domain/entities/event_receiver_group.rs

use crate::domain::value_objects::{EventReceiverGroupId, EventReceiverId};
use crate::error::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Parameters for creating an event receiver group from existing data
#[derive(Debug, Clone)]
pub struct EventReceiverGroupData {
    pub id: EventReceiverGroupId,
    pub name: String,
    pub group_type: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub event_receiver_ids: Vec<EventReceiverId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Event receiver group entity representing a collection of event receivers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventReceiverGroup {
    id: EventReceiverGroupId,
    name: String,
    group_type: String,
    version: String,
    description: String,
    enabled: bool,
    event_receiver_ids: Vec<EventReceiverId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl EventReceiverGroup {
    /// Creates a new event receiver group with validation
    pub fn new(
        name: String,
        group_type: String,
        version: String,
        description: String,
        enabled: bool,
        event_receiver_ids: Vec<EventReceiverId>,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;
        Self::validate_type(&group_type)?;
        Self::validate_version(&version)?;
        Self::validate_description(&description)?;
        Self::validate_event_receiver_ids(&event_receiver_ids)?;

        let now = Utc::now();

        Ok(Self {
            id: EventReceiverGroupId::new(),
            name,
            group_type,
            version,
            description,
            enabled,
            event_receiver_ids,
            created_at: now,
            updated_at: now,
        })
    }

    /// Creates an event receiver group from existing data (e.g., from database)
    pub fn from_existing(data: EventReceiverGroupData) -> Result<Self, DomainError> {
        Self::validate_name(&data.name)?;
        Self::validate_type(&data.group_type)?;
        Self::validate_version(&data.version)?;
        Self::validate_description(&data.description)?;
        Self::validate_event_receiver_ids(&data.event_receiver_ids)?;

        Ok(Self {
            id: data.id,
            name: data.name,
            group_type: data.group_type,
            version: data.version,
            description: data.description,
            enabled: data.enabled,
            event_receiver_ids: data.event_receiver_ids,
            created_at: data.created_at,
            updated_at: data.updated_at,
        })
    }

    /// Updates the event receiver group with new data
    pub fn update(
        &mut self,
        name: Option<String>,
        group_type: Option<String>,
        version: Option<String>,
        description: Option<String>,
        enabled: Option<bool>,
        event_receiver_ids: Option<Vec<EventReceiverId>>,
    ) -> Result<(), DomainError> {
        if let Some(new_name) = name {
            Self::validate_name(&new_name)?;
            self.name = new_name;
        }

        if let Some(new_type) = group_type {
            Self::validate_type(&new_type)?;
            self.group_type = new_type;
        }

        if let Some(new_version) = version {
            Self::validate_version(&new_version)?;
            self.version = new_version;
        }

        if let Some(new_description) = description {
            Self::validate_description(&new_description)?;
            self.description = new_description;
        }

        if let Some(new_enabled) = enabled {
            self.enabled = new_enabled;
        }

        if let Some(new_receiver_ids) = event_receiver_ids {
            Self::validate_event_receiver_ids(&new_receiver_ids)?;
            self.event_receiver_ids = new_receiver_ids;
        }

        self.updated_at = Utc::now();

        Ok(())
    }

    /// Enables the event receiver group
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
    }

    /// Disables the event receiver group
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }

    /// Adds an event receiver to the group
    pub fn add_event_receiver(&mut self, receiver_id: EventReceiverId) -> Result<(), DomainError> {
        if self.event_receiver_ids.contains(&receiver_id) {
            return Err(DomainError::BusinessRuleViolation {
                rule: "Event receiver already exists in the group".to_string(),
            });
        }

        self.event_receiver_ids.push(receiver_id);
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Removes an event receiver from the group
    pub fn remove_event_receiver(
        &mut self,
        receiver_id: EventReceiverId,
    ) -> Result<(), DomainError> {
        let initial_len = self.event_receiver_ids.len();
        self.event_receiver_ids.retain(|&id| id != receiver_id);

        if self.event_receiver_ids.len() == initial_len {
            return Err(DomainError::BusinessRuleViolation {
                rule: "Event receiver not found in the group".to_string(),
            });
        }

        self.updated_at = Utc::now();

        Ok(())
    }

    /// Checks if the group contains a specific event receiver
    pub fn contains_receiver(&self, receiver_id: EventReceiverId) -> bool {
        self.event_receiver_ids.contains(&receiver_id)
    }

    /// Gets the count of event receivers in the group
    pub fn receiver_count(&self) -> usize {
        self.event_receiver_ids.len()
    }

    /// Validates group name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "name".to_string(),
                message: "Event receiver group name cannot be empty".to_string(),
            });
        }

        if name.len() > 255 {
            return Err(DomainError::ValidationError {
                field: "name".to_string(),
                message: "Event receiver group name cannot exceed 255 characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validates group type
    fn validate_type(group_type: &str) -> Result<(), DomainError> {
        if group_type.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "type".to_string(),
                message: "Event receiver group type cannot be empty".to_string(),
            });
        }

        if group_type.len() > 100 {
            return Err(DomainError::ValidationError {
                field: "type".to_string(),
                message: "Event receiver group type cannot exceed 100 characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validates version string
    fn validate_version(version: &str) -> Result<(), DomainError> {
        if version.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "version".to_string(),
                message: "Version cannot be empty".to_string(),
            });
        }

        if version.len() > 50 {
            return Err(DomainError::ValidationError {
                field: "version".to_string(),
                message: "Version cannot exceed 50 characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validates description
    fn validate_description(description: &str) -> Result<(), DomainError> {
        if description.len() > 1000 {
            return Err(DomainError::ValidationError {
                field: "description".to_string(),
                message: "Description cannot exceed 1000 characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validates event receiver IDs for uniqueness
    fn validate_event_receiver_ids(receiver_ids: &[EventReceiverId]) -> Result<(), DomainError> {
        if receiver_ids.len() > 100 {
            return Err(DomainError::ValidationError {
                field: "event_receiver_ids".to_string(),
                message: "Cannot have more than 100 event receivers in a group".to_string(),
            });
        }

        // Check for duplicates
        let unique_ids: HashSet<_> = receiver_ids.iter().collect();
        if unique_ids.len() != receiver_ids.len() {
            return Err(DomainError::ValidationError {
                field: "event_receiver_ids".to_string(),
                message: "Duplicate event receiver IDs are not allowed".to_string(),
            });
        }

        Ok(())
    }

    // Getters
    pub fn id(&self) -> EventReceiverGroupId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn group_type(&self) -> &str {
        &self.group_type
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn event_receiver_ids(&self) -> &[EventReceiverId] {
        &self.event_receiver_ids
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_event_receiver_group() {
        let receiver_ids = vec![EventReceiverId::new(), EventReceiverId::new()];

        let group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids.clone(),
        );

        assert!(group.is_ok());
        let group = group.unwrap();
        assert_eq!(group.name(), "Test Group");
        assert_eq!(group.group_type(), "webhook_group");
        assert_eq!(group.version(), "1.0.0");
        assert!(group.enabled());
        assert_eq!(group.event_receiver_ids().len(), 2);
        assert_eq!(group.receiver_count(), 2);
    }

    #[test]
    fn test_validate_empty_name() {
        let receiver_ids = vec![EventReceiverId::new()];
        let result = EventReceiverGroup::new(
            "".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "name");
        }
    }

    #[test]
    fn test_validate_duplicate_receiver_ids() {
        let receiver_id = EventReceiverId::new();
        let receiver_ids = vec![receiver_id, receiver_id]; // Duplicate IDs

        let result = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "event_receiver_ids");
        }
    }

    #[test]
    fn test_enable_disable_group() {
        let receiver_ids = vec![EventReceiverId::new()];
        let mut group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            false,
            receiver_ids,
        )
        .unwrap();

        assert!(!group.enabled());

        group.enable();
        assert!(group.enabled());

        group.disable();
        assert!(!group.enabled());
    }

    #[test]
    fn test_add_remove_event_receiver() {
        let receiver_ids = vec![EventReceiverId::new()];
        let mut group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
        )
        .unwrap();

        let initial_count = group.receiver_count();
        let new_receiver_id = EventReceiverId::new();

        // Add new receiver
        assert!(group.add_event_receiver(new_receiver_id).is_ok());
        assert_eq!(group.receiver_count(), initial_count + 1);
        assert!(group.contains_receiver(new_receiver_id));

        // Try to add duplicate receiver
        assert!(group.add_event_receiver(new_receiver_id).is_err());

        // Remove receiver
        assert!(group.remove_event_receiver(new_receiver_id).is_ok());
        assert_eq!(group.receiver_count(), initial_count);
        assert!(!group.contains_receiver(new_receiver_id));

        // Try to remove non-existent receiver
        assert!(group.remove_event_receiver(new_receiver_id).is_err());
    }

    #[test]
    fn test_update_group() {
        let receiver_ids = vec![EventReceiverId::new()];
        let mut group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
        )
        .unwrap();

        let original_updated_at = group.updated_at();

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        // Update description
        group
            .update(
                None,
                None,
                None,
                Some("Updated description".to_string()),
                None,
                None,
            )
            .unwrap();

        assert_eq!(group.description(), "Updated description");
        assert!(group.updated_at() > original_updated_at);

        // Update name
        group
            .update(
                Some("Updated Group".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();

        assert_eq!(group.name(), "Updated Group");
    }

    #[test]
    fn test_too_many_receivers() {
        let receiver_ids: Vec<EventReceiverId> = (0..101).map(|_| EventReceiverId::new()).collect();

        let result = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "event_receiver_ids");
        }
    }
}
