// src/domain/repositories/event_receiver_group_repo.rs

use crate::domain::entities::event_receiver_group::EventReceiverGroup;
use crate::domain::value_objects::{EventReceiverGroupId, EventReceiverId};
use crate::error::Result;
use async_trait::async_trait;

/// Repository trait for event receiver group persistence operations
#[async_trait]
pub trait EventReceiverGroupRepository: Send + Sync {
    /// Saves an event receiver group to the repository
    async fn save(&self, group: &EventReceiverGroup) -> Result<()>;

    /// Finds an event receiver group by its ID
    async fn find_by_id(&self, id: EventReceiverGroupId) -> Result<Option<EventReceiverGroup>>;

    /// Finds event receiver groups by name (partial match)
    async fn find_by_name(&self, name: &str) -> Result<Vec<EventReceiverGroup>>;

    /// Finds event receiver groups by type
    async fn find_by_type(&self, group_type: &str) -> Result<Vec<EventReceiverGroup>>;

    /// Finds event receiver groups by type and version
    async fn find_by_type_and_version(
        &self,
        group_type: &str,
        version: &str,
    ) -> Result<Vec<EventReceiverGroup>>;

    /// Finds enabled event receiver groups
    async fn find_enabled(&self) -> Result<Vec<EventReceiverGroup>>;

    /// Finds disabled event receiver groups
    async fn find_disabled(&self) -> Result<Vec<EventReceiverGroup>>;

    /// Finds groups that contain a specific event receiver
    async fn find_by_event_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Vec<EventReceiverGroup>>;

    /// Lists all event receiver groups with pagination
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<EventReceiverGroup>>;

    /// Counts total number of event receiver groups
    async fn count(&self) -> Result<usize>;

    /// Counts enabled event receiver groups
    async fn count_enabled(&self) -> Result<usize>;

    /// Counts disabled event receiver groups
    async fn count_disabled(&self) -> Result<usize>;

    /// Updates an existing event receiver group
    async fn update(&self, group: &EventReceiverGroup) -> Result<()>;

    /// Deletes an event receiver group by ID
    async fn delete(&self, id: EventReceiverGroupId) -> Result<()>;

    /// Enables an event receiver group
    async fn enable(&self, id: EventReceiverGroupId) -> Result<()>;

    /// Disables an event receiver group
    async fn disable(&self, id: EventReceiverGroupId) -> Result<()>;

    /// Checks if an event receiver group exists with the given name and type
    async fn exists_by_name_and_type(&self, name: &str, group_type: &str) -> Result<bool>;

    /// Finds event receiver groups that match multiple criteria
    async fn find_by_criteria(
        &self,
        criteria: FindEventReceiverGroupCriteria,
    ) -> Result<Vec<EventReceiverGroup>>;

    /// Adds an event receiver to a group
    async fn add_event_receiver_to_group(
        &self,
        group_id: EventReceiverGroupId,
        receiver_id: EventReceiverId,
    ) -> Result<()>;

    /// Removes an event receiver from a group
    async fn remove_event_receiver_from_group(
        &self,
        group_id: EventReceiverGroupId,
        receiver_id: EventReceiverId,
    ) -> Result<()>;

    /// Gets all event receiver IDs for a specific group
    async fn get_group_event_receivers(
        &self,
        group_id: EventReceiverGroupId,
    ) -> Result<Vec<EventReceiverId>>;
}

/// Criteria for finding event receiver groups
#[derive(Debug, Clone, Default)]
pub struct FindEventReceiverGroupCriteria {
    pub id: Option<EventReceiverGroupId>,
    pub name: Option<String>,
    pub group_type: Option<String>,
    pub version: Option<String>,
    pub enabled: Option<bool>,
    pub contains_receiver_id: Option<EventReceiverId>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl FindEventReceiverGroupCriteria {
    /// Creates a new criteria builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the ID filter
    pub fn with_id(mut self, id: EventReceiverGroupId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the name filter (partial match)
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the type filter
    pub fn with_type(mut self, group_type: String) -> Self {
        self.group_type = Some(group_type);
        self
    }

    /// Sets the version filter
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the enabled filter
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    /// Filters groups that contain a specific event receiver
    pub fn containing_receiver(mut self, receiver_id: EventReceiverId) -> Self {
        self.contains_receiver_id = Some(receiver_id);
        self
    }

    /// Sets pagination limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets pagination offset
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Checks if any criteria are set
    pub fn is_empty(&self) -> bool {
        self.id.is_none()
            && self.name.is_none()
            && self.group_type.is_none()
            && self.version.is_none()
            && self.enabled.is_none()
            && self.contains_receiver_id.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criteria_builder() {
        let receiver_id = EventReceiverId::new();
        let criteria = FindEventReceiverGroupCriteria::new()
            .with_name("test".to_string())
            .with_type("webhook_group".to_string())
            .with_enabled(true)
            .containing_receiver(receiver_id)
            .with_limit(10)
            .with_offset(0);

        assert_eq!(criteria.name, Some("test".to_string()));
        assert_eq!(criteria.group_type, Some("webhook_group".to_string()));
        assert_eq!(criteria.enabled, Some(true));
        assert_eq!(criteria.contains_receiver_id, Some(receiver_id));
        assert_eq!(criteria.limit, Some(10));
        assert_eq!(criteria.offset, Some(0));
        assert!(!criteria.is_empty());
    }

    #[test]
    fn test_empty_criteria() {
        let criteria = FindEventReceiverGroupCriteria::new();
        assert!(criteria.is_empty());
    }

    #[test]
    fn test_enabled_disabled_filters() {
        let enabled_criteria = FindEventReceiverGroupCriteria::new().with_enabled(true);
        assert_eq!(enabled_criteria.enabled, Some(true));

        let disabled_criteria = FindEventReceiverGroupCriteria::new().with_enabled(false);
        assert_eq!(disabled_criteria.enabled, Some(false));
    }
}
