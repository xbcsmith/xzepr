// src/domain/repositories/event_receiver_repo.rs

use crate::domain::entities::event_receiver::EventReceiver;
use crate::domain::value_objects::EventReceiverId;
use crate::error::Result;
use async_trait::async_trait;

/// Repository trait for event receiver persistence operations
#[async_trait]
pub trait EventReceiverRepository: Send + Sync {
    /// Saves an event receiver to the repository
    async fn save(&self, event_receiver: &EventReceiver) -> Result<()>;

    /// Finds an event receiver by its ID
    async fn find_by_id(&self, id: EventReceiverId) -> Result<Option<EventReceiver>>;

    /// Finds event receivers by name (partial match)
    async fn find_by_name(&self, name: &str) -> Result<Vec<EventReceiver>>;

    /// Finds event receivers by type
    async fn find_by_type(&self, receiver_type: &str) -> Result<Vec<EventReceiver>>;

    /// Finds event receivers by type and version
    async fn find_by_type_and_version(
        &self,
        receiver_type: &str,
        version: &str,
    ) -> Result<Vec<EventReceiver>>;

    /// Finds event receivers by fingerprint (should be unique)
    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Option<EventReceiver>>;

    /// Lists all event receivers with pagination
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<EventReceiver>>;

    /// Counts total number of event receivers
    async fn count(&self) -> Result<usize>;

    /// Updates an existing event receiver
    async fn update(&self, event_receiver: &EventReceiver) -> Result<()>;

    /// Deletes an event receiver by ID
    async fn delete(&self, id: EventReceiverId) -> Result<()>;

    /// Checks if an event receiver exists with the given name and type
    async fn exists_by_name_and_type(&self, name: &str, receiver_type: &str) -> Result<bool>;

    /// Finds event receivers that match multiple criteria
    async fn find_by_criteria(
        &self,
        criteria: FindEventReceiverCriteria,
    ) -> Result<Vec<EventReceiver>>;
}

/// Criteria for finding event receivers
#[derive(Debug, Clone, Default)]
pub struct FindEventReceiverCriteria {
    pub id: Option<EventReceiverId>,
    pub name: Option<String>,
    pub receiver_type: Option<String>,
    pub version: Option<String>,
    pub fingerprint: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl FindEventReceiverCriteria {
    /// Creates a new criteria builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the ID filter
    pub fn with_id(mut self, id: EventReceiverId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the name filter (partial match)
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the type filter
    pub fn with_type(mut self, receiver_type: String) -> Self {
        self.receiver_type = Some(receiver_type);
        self
    }

    /// Sets the version filter
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the fingerprint filter
    pub fn with_fingerprint(mut self, fingerprint: String) -> Self {
        self.fingerprint = Some(fingerprint);
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
            && self.receiver_type.is_none()
            && self.version.is_none()
            && self.fingerprint.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criteria_builder() {
        let criteria = FindEventReceiverCriteria::new()
            .with_name("test".to_string())
            .with_type("webhook".to_string())
            .with_limit(10)
            .with_offset(0);

        assert_eq!(criteria.name, Some("test".to_string()));
        assert_eq!(criteria.receiver_type, Some("webhook".to_string()));
        assert_eq!(criteria.limit, Some(10));
        assert_eq!(criteria.offset, Some(0));
        assert!(!criteria.is_empty());
    }

    #[test]
    fn test_empty_criteria() {
        let criteria = FindEventReceiverCriteria::new();
        assert!(criteria.is_empty());
    }
}
