// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/domain/repositories/event_repo.rs

use crate::domain::entities::event::Event;
use crate::domain::value_objects::{EventId, EventReceiverId};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Repository trait for event persistence operations
#[async_trait]
pub trait EventRepository: Send + Sync {
    /// Saves an event to the repository
    async fn save(&self, event: &Event) -> Result<()>;

    /// Finds an event by its ID
    async fn find_by_id(&self, id: EventId) -> Result<Option<Event>>;

    /// Finds events by event receiver ID
    async fn find_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<Vec<Event>>;

    /// Finds events by success status
    async fn find_by_success(&self, success: bool) -> Result<Vec<Event>>;

    /// Finds events by name (partial match)
    async fn find_by_name(&self, name: &str) -> Result<Vec<Event>>;

    /// Finds events by platform ID
    async fn find_by_platform_id(&self, platform_id: &str) -> Result<Vec<Event>>;

    /// Finds events by package
    async fn find_by_package(&self, package: &str) -> Result<Vec<Event>>;

    /// Lists all events with pagination
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<Event>>;

    /// Counts total number of events
    async fn count(&self) -> Result<usize>;

    /// Counts events by receiver ID
    async fn count_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<usize>;

    /// Counts successful events by receiver ID
    async fn count_successful_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<usize>;

    /// Deletes an event by ID
    async fn delete(&self, id: EventId) -> Result<()>;

    /// Finds the latest event for a receiver
    async fn find_latest_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Option<Event>>;

    /// Finds the latest successful event for a receiver
    async fn find_latest_successful_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Option<Event>>;

    /// Finds events within a time range
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>>;

    /// Finds events that match multiple criteria
    async fn find_by_criteria(&self, criteria: FindEventCriteria) -> Result<Vec<Event>>;
}

/// Criteria for finding events
#[derive(Debug, Clone, Default)]
pub struct FindEventCriteria {
    pub id: Option<EventId>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub release: Option<String>,
    pub platform_id: Option<String>,
    pub package: Option<String>,
    pub success: Option<bool>,
    pub event_receiver_id: Option<EventReceiverId>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl FindEventCriteria {
    /// Creates a new criteria builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the ID filter
    pub fn with_id(mut self, id: EventId) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the name filter (partial match)
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the version filter
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the release filter
    pub fn with_release(mut self, release: String) -> Self {
        self.release = Some(release);
        self
    }

    /// Sets the platform ID filter
    pub fn with_platform_id(mut self, platform_id: String) -> Self {
        self.platform_id = Some(platform_id);
        self
    }

    /// Sets the package filter
    pub fn with_package(mut self, package: String) -> Self {
        self.package = Some(package);
        self
    }

    /// Sets the success status filter
    pub fn with_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }

    /// Sets the event receiver ID filter
    pub fn with_event_receiver_id(mut self, receiver_id: EventReceiverId) -> Self {
        self.event_receiver_id = Some(receiver_id);
        self
    }

    /// Sets the start time filter
    pub fn with_start_time(mut self, start_time: DateTime<Utc>) -> Self {
        self.start_time = Some(start_time);
        self
    }

    /// Sets the end time filter
    pub fn with_end_time(mut self, end_time: DateTime<Utc>) -> Self {
        self.end_time = Some(end_time);
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
            && self.version.is_none()
            && self.release.is_none()
            && self.platform_id.is_none()
            && self.package.is_none()
            && self.success.is_none()
            && self.event_receiver_id.is_none()
            && self.start_time.is_none()
            && self.end_time.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criteria_builder() {
        let receiver_id = EventReceiverId::new();
        let criteria = FindEventCriteria::new()
            .with_name("test".to_string())
            .with_success(true)
            .with_event_receiver_id(receiver_id)
            .with_limit(10)
            .with_offset(0);

        assert_eq!(criteria.name, Some("test".to_string()));
        assert_eq!(criteria.success, Some(true));
        assert_eq!(criteria.event_receiver_id, Some(receiver_id));
        assert_eq!(criteria.limit, Some(10));
        assert_eq!(criteria.offset, Some(0));
        assert!(!criteria.is_empty());
    }

    #[test]
    fn test_empty_criteria() {
        let criteria = FindEventCriteria::new();
        assert!(criteria.is_empty());
    }
}
