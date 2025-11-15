// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/domain/entities/event_receiver_group.rs

use crate::domain::value_objects::{EventReceiverGroupId, EventReceiverId, UserId};
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
    pub owner_id: UserId,
    pub resource_version: i64,
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
    owner_id: UserId,
    resource_version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl EventReceiverGroup {
    /// Creates a new event receiver group with validation
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the event receiver group
    /// * `group_type` - The type of the group
    /// * `version` - The version of the group
    /// * `description` - A description of the group
    /// * `enabled` - Whether the group is enabled
    /// * `event_receiver_ids` - List of event receiver IDs in this group
    /// * `owner_id` - The user ID of the owner who created this group
    ///
    /// # Returns
    ///
    /// Returns a new EventReceiverGroup or DomainError if validation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::domain::entities::event_receiver_group::EventReceiverGroup;
    /// use xzepr::domain::value_objects::{UserId, EventReceiverId};
    ///
    /// let owner_id = UserId::new();
    /// let receiver_ids = vec![EventReceiverId::new()];
    /// let group = EventReceiverGroup::new(
    ///     "My Group".to_string(),
    ///     "webhook_group".to_string(),
    ///     "1.0.0".to_string(),
    ///     "Test group".to_string(),
    ///     true,
    ///     receiver_ids,
    ///     owner_id,
    /// );
    /// assert!(group.is_ok());
    /// ```
    pub fn new(
        name: String,
        group_type: String,
        version: String,
        description: String,
        enabled: bool,
        event_receiver_ids: Vec<EventReceiverId>,
        owner_id: UserId,
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
            owner_id,
            resource_version: 1,
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
            owner_id: data.owner_id,
            resource_version: data.resource_version,
            created_at: data.created_at,
            updated_at: data.updated_at,
        })
    }

    /// Updates the event receiver group with new data
    ///
    /// Increments the resource_version on any update to support cache invalidation.
    ///
    /// # Arguments
    ///
    /// * `name` - Optional new name
    /// * `group_type` - Optional new type
    /// * `version` - Optional new version
    /// * `description` - Optional new description
    /// * `enabled` - Optional new enabled state
    /// * `event_receiver_ids` - Optional new list of receiver IDs
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if validation passes, otherwise DomainError
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
        self.resource_version += 1;

        Ok(())
    }

    /// Enables the event receiver group
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
        self.resource_version += 1;
    }

    /// Disables the event receiver group
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
        self.resource_version += 1;
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
        self.resource_version += 1;

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
        self.resource_version += 1;

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

    /// Returns the owner user ID of this event receiver group
    pub fn owner_id(&self) -> UserId {
        self.owner_id
    }

    /// Returns the current resource version for cache invalidation
    ///
    /// This version is incremented on every update and used by the
    /// authorization cache to detect stale entries.
    pub fn resource_version(&self) -> i64 {
        self.resource_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_event_receiver_group() {
        let receiver_ids = vec![EventReceiverId::new(), EventReceiverId::new()];
        let owner_id = UserId::new();

        let group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids.clone(),
            owner_id,
        );

        assert!(group.is_ok());
        let group = group.unwrap();
        assert_eq!(group.name(), "Test Group");
        assert_eq!(group.group_type(), "webhook_group");
        assert_eq!(group.version(), "1.0.0");
        assert!(group.enabled());
        assert_eq!(group.event_receiver_ids().len(), 2);
        assert_eq!(group.receiver_count(), 2);
        assert_eq!(group.owner_id(), owner_id);
        assert_eq!(group.resource_version(), 1);
    }

    #[test]
    fn test_validate_empty_name() {
        let receiver_ids = vec![EventReceiverId::new()];
        let owner_id = UserId::new();
        let result = EventReceiverGroup::new(
            "".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
            owner_id,
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
        let owner_id = UserId::new();

        let result = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
            owner_id,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "event_receiver_ids");
        }
    }

    #[test]
    fn test_enable_disable_group() {
        let receiver_ids = vec![EventReceiverId::new()];
        let owner_id = UserId::new();
        let mut group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            false,
            receiver_ids,
            owner_id,
        )
        .unwrap();

        assert!(!group.enabled());
        let initial_version = group.resource_version();

        group.enable();
        assert!(group.enabled());
        assert_eq!(group.resource_version(), initial_version + 1);

        group.disable();
        assert!(!group.enabled());
        assert_eq!(group.resource_version(), initial_version + 2);
    }

    #[test]
    fn test_add_remove_event_receiver() {
        let receiver_ids = vec![EventReceiverId::new()];
        let owner_id = UserId::new();
        let mut group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
            owner_id,
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
        let owner_id = UserId::new();
        let mut group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
            owner_id,
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
        let owner_id = UserId::new();

        let result = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
            owner_id,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "event_receiver_ids");
        }
    }

    #[test]
    fn test_resource_version_increments_on_update() {
        let receiver_ids = vec![EventReceiverId::new()];
        let owner_id = UserId::new();
        let mut group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
            owner_id,
        )
        .unwrap();

        assert_eq!(group.resource_version(), 1);

        // Update name (should increment version)
        group
            .update(
                Some("Updated Name".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(group.resource_version(), 2);

        // Add receiver (should increment version)
        group.add_event_receiver(EventReceiverId::new()).unwrap();
        assert_eq!(group.resource_version(), 3);
    }

    #[test]
    fn test_owner_id_is_preserved() {
        let receiver_ids = vec![EventReceiverId::new()];
        let owner_id = UserId::new();
        let group = EventReceiverGroup::new(
            "Test Group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test event receiver group".to_string(),
            true,
            receiver_ids,
            owner_id,
        )
        .unwrap();

        assert_eq!(group.owner_id(), owner_id);

        // Owner ID should not change on updates
        let mut group_copy = group.clone();
        group_copy
            .update(Some("New Name".to_string()), None, None, None, None, None)
            .unwrap();
        assert_eq!(group_copy.owner_id(), owner_id);
    }
}
