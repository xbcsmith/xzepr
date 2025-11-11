// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/application/handlers/event_handler.rs

use crate::domain::entities::event::{CreateEventParams, Event};
use crate::domain::repositories::event_receiver_repo::EventReceiverRepository;
use crate::domain::repositories::event_repo::EventRepository;
use crate::domain::value_objects::{EventId, EventReceiverId};
use crate::error::{DomainError, Result};
use crate::infrastructure::messaging::producer::KafkaEventPublisher;

use std::sync::Arc;
use tracing::{error, info, warn};

/// Application service for handling event operations
#[derive(Clone)]
pub struct EventHandler {
    event_repository: Arc<dyn EventRepository>,
    receiver_repository: Arc<dyn EventReceiverRepository>,
    event_publisher: Option<Arc<KafkaEventPublisher>>,
}

impl EventHandler {
    /// Creates a new event handler
    pub fn new(
        event_repository: Arc<dyn EventRepository>,
        receiver_repository: Arc<dyn EventReceiverRepository>,
    ) -> Self {
        Self {
            event_repository,
            receiver_repository,
            event_publisher: None,
        }
    }

    /// Creates a new event handler with event publisher
    pub fn with_publisher(
        event_repository: Arc<dyn EventRepository>,
        receiver_repository: Arc<dyn EventReceiverRepository>,
        event_publisher: Arc<KafkaEventPublisher>,
    ) -> Self {
        Self {
            event_repository,
            receiver_repository,
            event_publisher: Some(event_publisher),
        }
    }

    /// Creates a new event
    pub async fn create_event(&self, params: CreateEventParams) -> Result<EventId> {
        info!(
            name = %params.name,
            version = %params.version,
            release = %params.release,
            platform_id = %params.platform_id,
            package = %params.package,
            success = %params.success,
            receiver_id = %params.receiver_id,
            "Creating new event"
        );

        // Verify that the event receiver exists
        let receiver = self
            .receiver_repository
            .find_by_id(params.receiver_id)
            .await?;
        let receiver = receiver.ok_or_else(|| {
            warn!(receiver_id = %params.receiver_id, "Event receiver not found");
            DomainError::ReceiverNotFound
        })?;

        info!(
            receiver_id = %params.receiver_id,
            receiver_name = %receiver.name(),
            receiver_type = %receiver.receiver_type(),
            "Found event receiver for event creation"
        );

        // Validate the payload against the receiver's schema
        if let Err(e) = receiver.validate_event_payload(&params.payload) {
            error!(
                receiver_id = %params.receiver_id,
                error = %e,
                "Event payload validation failed"
            );
            return Err(e.into());
        }

        // Create the domain entity
        let event = Event::new(params)?;

        let event_id = event.id();

        // Save to repository
        self.event_repository.save(&event).await?;

        info!(
            event_id = %event_id,
            receiver_id = %event.event_receiver_id(),
            success = %event.success(),
            "Event created successfully"
        );

        // Publish event to Kafka if publisher is configured
        if let Some(publisher) = &self.event_publisher {
            if let Err(e) = publisher.publish(&event).await {
                error!(
                    event_id = %event_id,
                    error = %e,
                    "Failed to publish event to Kafka (event was saved to database)"
                );
                // Note: We don't fail the request since the event was saved to the database
                // The event publication is best-effort
            } else {
                info!(
                    event_id = %event_id,
                    "Event published to Kafka successfully"
                );
            }
        } else {
            warn!("Event publisher not configured, skipping Kafka publication");
        }

        Ok(event_id)
    }

    /// Gets an event by ID
    pub async fn get_event(&self, id: EventId) -> Result<Option<Event>> {
        info!(event_id = %id, "Retrieving event");

        let event = self.event_repository.find_by_id(id).await?;

        if event.is_none() {
            info!(event_id = %id, "Event not found");
        }

        Ok(event)
    }

    /// Gets an event by ID, returning an error if not found
    pub async fn get_event_or_error(&self, id: EventId) -> Result<Event> {
        self.get_event(id).await?.ok_or_else(|| {
            DomainError::EventCreationFailed {
                reason: "Event not found".to_string(),
            }
            .into()
        })
    }

    /// Lists events by receiver ID
    pub async fn find_by_receiver(&self, receiver_id: EventReceiverId) -> Result<Vec<Event>> {
        info!(receiver_id = %receiver_id, "Finding events by receiver");
        self.event_repository.find_by_receiver_id(receiver_id).await
    }

    /// Lists events by success status
    pub async fn find_by_success(&self, success: bool) -> Result<Vec<Event>> {
        info!(success = %success, "Finding events by success status");
        self.event_repository.find_by_success(success).await
    }

    /// Lists events by name pattern
    pub async fn find_by_name(&self, name_pattern: &str) -> Result<Vec<Event>> {
        info!(name_pattern = %name_pattern, "Finding events by name pattern");
        self.event_repository.find_by_name(name_pattern).await
    }

    /// Lists events by platform ID
    pub async fn find_by_platform(&self, platform_id: &str) -> Result<Vec<Event>> {
        info!(platform_id = %platform_id, "Finding events by platform");
        self.event_repository.find_by_platform_id(platform_id).await
    }

    /// Lists events by package
    pub async fn find_by_package(&self, package: &str) -> Result<Vec<Event>> {
        info!(package = %package, "Finding events by package");
        self.event_repository.find_by_package(package).await
    }

    /// Lists all events with pagination
    pub async fn list_events(&self, limit: usize, offset: usize) -> Result<Vec<Event>> {
        info!(limit = %limit, offset = %offset, "Listing events with pagination");

        // Validate pagination parameters
        if limit == 0 || limit > 1000 {
            return Err(DomainError::ValidationError {
                field: "limit".to_string(),
                message: "Limit must be between 1 and 1000".to_string(),
            }
            .into());
        }

        self.event_repository.list(limit, offset).await
    }

    /// Counts total number of events
    pub async fn count_events(&self) -> Result<usize> {
        info!("Counting total events");
        self.event_repository.count().await
    }

    /// Counts events by receiver ID
    pub async fn count_events_by_receiver(&self, receiver_id: EventReceiverId) -> Result<usize> {
        info!(receiver_id = %receiver_id, "Counting events by receiver");
        self.event_repository
            .count_by_receiver_id(receiver_id)
            .await
    }

    /// Counts successful events by receiver ID
    pub async fn count_successful_events_by_receiver(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<usize> {
        info!(receiver_id = %receiver_id, "Counting successful events by receiver");
        self.event_repository
            .count_successful_by_receiver_id(receiver_id)
            .await
    }

    /// Deletes an event
    pub async fn delete_event(&self, id: EventId) -> Result<()> {
        info!(event_id = %id, "Deleting event");

        // Check if the event exists
        if self.event_repository.find_by_id(id).await?.is_none() {
            return Err(DomainError::EventCreationFailed {
                reason: "Event not found".to_string(),
            }
            .into());
        }

        self.event_repository.delete(id).await?;

        info!(event_id = %id, "Event deleted successfully");

        Ok(())
    }

    /// Gets the latest event for a receiver
    pub async fn get_latest_event_for_receiver(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Option<Event>> {
        info!(receiver_id = %receiver_id, "Getting latest event for receiver");
        self.event_repository
            .find_latest_by_receiver_id(receiver_id)
            .await
    }

    /// Gets the latest successful event for a receiver
    pub async fn get_latest_successful_event_for_receiver(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Option<Event>> {
        info!(receiver_id = %receiver_id, "Getting latest successful event for receiver");
        self.event_repository
            .find_latest_successful_by_receiver_id(receiver_id)
            .await
    }

    /// Checks if an event exists
    pub async fn exists(&self, id: EventId) -> Result<bool> {
        Ok(self.event_repository.find_by_id(id).await?.is_some())
    }

    /// Gets events within a time range
    pub async fn find_events_in_time_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Event>> {
        info!(
            start = %start,
            end = %end,
            "Finding events within time range"
        );

        if start >= end {
            return Err(DomainError::ValidationError {
                field: "time_range".to_string(),
                message: "Start time must be before end time".to_string(),
            }
            .into());
        }

        self.event_repository.find_by_time_range(start, end).await
    }

    /// Gets events statistics for a receiver
    pub async fn get_receiver_statistics(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<ReceiverStatistics> {
        info!(receiver_id = %receiver_id, "Getting receiver statistics");

        // Verify receiver exists
        if self
            .receiver_repository
            .find_by_id(receiver_id)
            .await?
            .is_none()
        {
            return Err(DomainError::ReceiverNotFound.into());
        }

        let total_events = self.count_events_by_receiver(receiver_id).await?;
        let successful_events = self
            .count_successful_events_by_receiver(receiver_id)
            .await?;
        let failed_events = total_events - successful_events;
        let latest_event = self.get_latest_event_for_receiver(receiver_id).await?;
        let latest_successful_event = self
            .get_latest_successful_event_for_receiver(receiver_id)
            .await?;

        let success_rate = if total_events > 0 {
            (successful_events as f64 / total_events as f64) * 100.0
        } else {
            0.0
        };

        Ok(ReceiverStatistics {
            receiver_id,
            total_events,
            successful_events,
            failed_events,
            success_rate,
            latest_event_id: latest_event.map(|e| e.id()),
            latest_successful_event_id: latest_successful_event.map(|e| e.id()),
        })
    }
}

/// Statistics for an event receiver
#[derive(Debug, Clone)]
pub struct ReceiverStatistics {
    pub receiver_id: EventReceiverId,
    pub total_events: usize,
    pub successful_events: usize,
    pub failed_events: usize,
    pub success_rate: f64, // Percentage
    pub latest_event_id: Option<EventId>,
    pub latest_successful_event_id: Option<EventId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::event_receiver::EventReceiver;
    use crate::domain::repositories::event_repo::EventRepository;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Mock repositories for testing
    struct MockEventRepository {
        events: Arc<Mutex<HashMap<EventId, Event>>>,
    }

    impl MockEventRepository {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl EventRepository for MockEventRepository {
        async fn save(&self, event: &Event) -> Result<()> {
            let mut events = self.events.lock().unwrap();
            events.insert(event.id(), event.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: EventId) -> Result<Option<Event>> {
            let events = self.events.lock().unwrap();
            Ok(events.get(&id).cloned())
        }

        async fn find_by_receiver_id(&self, _receiver_id: EventReceiverId) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_success(&self, _success: bool) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_name(&self, _name: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_platform_id(&self, _platform_id: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_package(&self, _package: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn count(&self) -> Result<usize> {
            let events = self.events.lock().unwrap();
            Ok(events.len())
        }

        async fn count_by_receiver_id(&self, _receiver_id: EventReceiverId) -> Result<usize> {
            Ok(0)
        }

        async fn count_successful_by_receiver_id(
            &self,
            _receiver_id: EventReceiverId,
        ) -> Result<usize> {
            Ok(0)
        }

        async fn delete(&self, id: EventId) -> Result<()> {
            let mut events = self.events.lock().unwrap();
            events.remove(&id);
            Ok(())
        }

        async fn find_latest_by_receiver_id(
            &self,
            _receiver_id: EventReceiverId,
        ) -> Result<Option<Event>> {
            Ok(None)
        }

        async fn find_latest_successful_by_receiver_id(
            &self,
            _receiver_id: EventReceiverId,
        ) -> Result<Option<Event>> {
            Ok(None)
        }

        async fn find_by_time_range(
            &self,
            _start: DateTime<Utc>,
            _end: DateTime<Utc>,
        ) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_criteria(
            &self,
            _criteria: crate::domain::repositories::event_repo::FindEventCriteria,
        ) -> Result<Vec<Event>> {
            Ok(vec![])
        }
    }

    struct MockEventReceiverRepository {
        receivers: Arc<Mutex<HashMap<EventReceiverId, EventReceiver>>>,
    }

    impl MockEventReceiverRepository {
        fn new() -> Self {
            Self {
                receivers: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn insert(&self, receiver: EventReceiver) {
            let mut receivers = self.receivers.lock().unwrap();
            receivers.insert(receiver.id(), receiver);
        }
    }

    #[async_trait]
    impl EventReceiverRepository for MockEventReceiverRepository {
        async fn save(&self, _event_receiver: &EventReceiver) -> Result<()> {
            Ok(())
        }

        async fn find_by_id(&self, id: EventReceiverId) -> Result<Option<EventReceiver>> {
            let receivers = self.receivers.lock().unwrap();
            Ok(receivers.get(&id).cloned())
        }

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
            Ok(0)
        }

        async fn update(&self, _event_receiver: &EventReceiver) -> Result<()> {
            Ok(())
        }

        async fn delete(&self, _id: EventReceiverId) -> Result<()> {
            Ok(())
        }

        async fn exists_by_name_and_type(&self, _name: &str, _receiver_type: &str) -> Result<bool> {
            Ok(false)
        }

        async fn find_by_criteria(
            &self,
            _criteria: crate::domain::repositories::event_receiver_repo::FindEventReceiverCriteria,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
    }

    fn create_test_receiver() -> EventReceiver {
        let schema = json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            }
        });

        EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test receiver".to_string(),
            schema,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_event() {
        let event_repo = Arc::new(MockEventRepository::new());
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());

        let receiver = create_test_receiver();
        let receiver_id = receiver.id();
        receiver_repo.insert(receiver);

        let handler = EventHandler::new(event_repo, receiver_repo);

        let payload = json!({"message": "Hello, world!"});
        let result = handler
            .create_event(CreateEventParams {
                name: "test-event".to_string(),
                version: "1.0.0".to_string(),
                release: "2023.11.16".to_string(),
                platform_id: "linux".to_string(),
                package: "docker".to_string(),
                description: "Test event".to_string(),
                payload,
                success: true,
                receiver_id,
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_event_with_nonexistent_receiver() {
        let event_repo = Arc::new(MockEventRepository::new());
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let handler = EventHandler::new(event_repo, receiver_repo);

        let _payload = json!({"message": "Hello, world!"});
        let _receiver_id = EventReceiverId::new(); // Non-existent receiver

        let payload = json!({"message": "Non-existent receiver"});
        let result = handler
            .create_event(CreateEventParams {
                name: "test-event".to_string(),
                version: "1.0.0".to_string(),
                release: "2023.11.16".to_string(),
                platform_id: "linux".to_string(),
                package: "docker".to_string(),
                description: "Test event".to_string(),
                payload,
                success: true,
                receiver_id: EventReceiverId::new(),
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_pagination_limits() {
        let event_repo = Arc::new(MockEventRepository::new());
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let handler = EventHandler::new(event_repo, receiver_repo);

        // Test invalid limits
        let result1 = handler.list_events(0, 0).await;
        assert!(result1.is_err());

        let result2 = handler.list_events(1001, 0).await;
        assert!(result2.is_err());

        // Test valid limits
        let result3 = handler.list_events(10, 0).await;
        assert!(result3.is_ok());
    }

    #[tokio::test]
    async fn test_time_range_validation() {
        let event_repo = Arc::new(MockEventRepository::new());
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let handler = EventHandler::new(event_repo, receiver_repo);

        let end_time = Utc::now();
        let start_time = end_time + chrono::Duration::hours(1); // Start after end

        let result = handler
            .find_events_in_time_range(start_time, end_time)
            .await;
        assert!(result.is_err());
    }
}
