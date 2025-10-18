// src/bin/server.rs
//! XZepr Event Tracking Server
//!
//! A high-performance event tracking server with REST API for managing
//! event receivers, events, and event receiver groups.

use std::sync::Arc;
use tokio::signal;
use tracing::info;

use xzepr::api::rest::{build_router, AppState};
use xzepr::application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(true)
        .init();

    info!("Starting XZepr Event Tracking Server");

    // For this example, we'll use mock repositories
    // In a real implementation, these would be database-backed
    let event_repo = Arc::new(MockEventRepository::new());
    let receiver_repo = Arc::new(MockEventReceiverRepository::new());
    let group_repo = Arc::new(MockEventReceiverGroupRepository::new());

    // Create application handlers
    let event_handler = EventHandler::new(event_repo, receiver_repo.clone());
    let receiver_handler = EventReceiverHandler::new(receiver_repo.clone());
    let group_handler = EventReceiverGroupHandler::new(group_repo, receiver_repo.clone());

    // Create application state
    let app_state = AppState {
        event_handler,
        event_receiver_handler: receiver_handler,
        event_receiver_group_handler: group_handler,
    };

    // Build the router
    let app = build_router(app_state);

    // Server configuration
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8042".to_string())
        .parse()
        .unwrap_or(8042);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    info!("Server listening on http://{}:{}", host, port);
    info!("Health check: http://{}:{}/health", host, port);
    info!("API documentation: http://{}:{}/api/v1", host, port);

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("XZepr server ready to accept connections");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down gracefully...");
        },
    }
}

// Mock implementations for demonstration purposes
// In a real application, these would be database-backed repositories

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Mutex;

use xzepr::domain::entities::{
    event::Event, event_receiver::EventReceiver, event_receiver_group::EventReceiverGroup,
};
use xzepr::domain::repositories::{
    event_receiver_group_repo::{EventReceiverGroupRepository, FindEventReceiverGroupCriteria},
    event_receiver_repo::{EventReceiverRepository, FindEventReceiverCriteria},
    event_repo::{EventRepository, FindEventCriteria},
};
use xzepr::domain::value_objects::{EventId, EventReceiverGroupId, EventReceiverId};
use xzepr::error::Result;

/// Mock event repository that stores events in memory
pub struct MockEventRepository {
    events: Arc<Mutex<HashMap<EventId, Event>>>,
}

impl Default for MockEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventRepository {
    pub fn new() -> Self {
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
        info!("Saved event: {} ({})", event.name(), event.id());
        Ok(())
    }

    async fn find_by_id(&self, id: EventId) -> Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events.get(&id).cloned())
    }

    async fn find_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id)
            .cloned()
            .collect())
    }

    async fn find_by_success(&self, success: bool) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.success() == success)
            .cloned()
            .collect())
    }

    async fn find_by_name(&self, name: &str) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.name().contains(name))
            .cloned()
            .collect())
    }

    async fn find_by_platform_id(&self, platform_id: &str) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.platform_id() == platform_id)
            .cloned()
            .collect())
    }

    async fn find_by_package(&self, package: &str) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.package() == package)
            .cloned()
            .collect())
    }

    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events.values().skip(offset).take(limit).cloned().collect())
    }

    async fn count(&self) -> Result<usize> {
        let events = self.events.lock().unwrap();
        Ok(events.len())
    }

    async fn count_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<usize> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id)
            .count())
    }

    async fn count_successful_by_receiver_id(&self, receiver_id: EventReceiverId) -> Result<usize> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id && e.success())
            .count())
    }

    async fn delete(&self, id: EventId) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        events.remove(&id);
        info!("Deleted event: {}", id);
        Ok(())
    }

    async fn find_latest_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id)
            .max_by_key(|e| e.created_at())
            .cloned())
    }

    async fn find_latest_successful_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id && e.success())
            .max_by_key(|e| e.created_at())
            .cloned())
    }

    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.created_at() >= start && e.created_at() <= end)
            .cloned()
            .collect())
    }

    async fn find_by_criteria(&self, _criteria: FindEventCriteria) -> Result<Vec<Event>> {
        // Simplified implementation
        let events = self.events.lock().unwrap();
        Ok(events.values().cloned().collect())
    }
}

/// Mock event receiver repository that stores receivers in memory
pub struct MockEventReceiverRepository {
    receivers: Arc<Mutex<HashMap<EventReceiverId, EventReceiver>>>,
    name_type_index: Arc<Mutex<HashMap<(String, String), EventReceiverId>>>,
}

impl Default for MockEventReceiverRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventReceiverRepository {
    pub fn new() -> Self {
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

        info!(
            "Saved event receiver: {} ({})",
            event_receiver.name(),
            event_receiver.id()
        );
        Ok(())
    }

    async fn find_by_id(&self, id: EventReceiverId) -> Result<Option<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers.get(&id).cloned())
    }

    async fn find_by_name(&self, name: &str) -> Result<Vec<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .filter(|r| r.name().contains(name))
            .cloned()
            .collect())
    }

    async fn find_by_type(&self, receiver_type: &str) -> Result<Vec<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .filter(|r| r.receiver_type() == receiver_type)
            .cloned()
            .collect())
    }

    async fn find_by_type_and_version(
        &self,
        receiver_type: &str,
        version: &str,
    ) -> Result<Vec<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .filter(|r| r.receiver_type() == receiver_type && r.version() == version)
            .cloned()
            .collect())
    }

    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Option<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .find(|r| r.fingerprint() == fingerprint)
            .cloned())
    }

    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
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
        let mut index = self.name_type_index.lock().unwrap();

        if let Some(receiver) = receivers.remove(&id) {
            index.remove(&(
                receiver.name().to_string(),
                receiver.receiver_type().to_string(),
            ));
            info!("Deleted event receiver: {} ({})", receiver.name(), id);
        }
        Ok(())
    }

    async fn exists_by_name_and_type(&self, name: &str, receiver_type: &str) -> Result<bool> {
        let index = self.name_type_index.lock().unwrap();
        Ok(index.contains_key(&(name.to_string(), receiver_type.to_string())))
    }

    async fn find_by_criteria(
        &self,
        _criteria: FindEventReceiverCriteria,
    ) -> Result<Vec<EventReceiver>> {
        // Simplified implementation
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers.values().cloned().collect())
    }
}

/// Mock event receiver group repository that stores groups in memory
pub struct MockEventReceiverGroupRepository {
    groups: Arc<Mutex<HashMap<EventReceiverGroupId, EventReceiverGroup>>>,
    name_type_index: Arc<Mutex<HashMap<(String, String), EventReceiverGroupId>>>,
}

impl Default for MockEventReceiverGroupRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventReceiverGroupRepository {
    pub fn new() -> Self {
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

        info!(
            "Saved event receiver group: {} ({})",
            group.name(),
            group.id()
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

    async fn find_by_name(&self, _name: &str) -> Result<Vec<EventReceiverGroup>> {
        Ok(vec![])
    }

    async fn find_by_type(&self, _group_type: &str) -> Result<Vec<EventReceiverGroup>> {
        Ok(vec![])
    }

    async fn find_by_type_and_version(
        &self,
        _group_type: &str,
        _version: &str,
    ) -> Result<Vec<EventReceiverGroup>> {
        Ok(vec![])
    }

    async fn find_enabled(&self) -> Result<Vec<EventReceiverGroup>> {
        Ok(vec![])
    }

    async fn find_disabled(&self) -> Result<Vec<EventReceiverGroup>> {
        Ok(vec![])
    }

    async fn find_by_event_receiver_id(
        &self,
        _receiver_id: EventReceiverId,
    ) -> Result<Vec<EventReceiverGroup>> {
        Ok(vec![])
    }

    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<EventReceiverGroup>> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.values().skip(offset).take(limit).cloned().collect())
    }

    async fn count(&self) -> Result<usize> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.len())
    }

    async fn count_enabled(&self) -> Result<usize> {
        Ok(0)
    }

    async fn count_disabled(&self) -> Result<usize> {
        Ok(0)
    }

    async fn delete(&self, id: EventReceiverGroupId) -> Result<()> {
        let mut groups = self.groups.lock().unwrap();
        let mut index = self.name_type_index.lock().unwrap();

        if let Some(group) = groups.remove(&id) {
            index.remove(&(group.name().to_string(), group.group_type().to_string()));
            info!("Deleted event receiver group: {} ({})", group.name(), id);
        }
        Ok(())
    }

    async fn enable(&self, _id: EventReceiverGroupId) -> Result<()> {
        Ok(())
    }

    async fn disable(&self, _id: EventReceiverGroupId) -> Result<()> {
        Ok(())
    }

    async fn find_by_criteria(
        &self,
        _criteria: FindEventReceiverGroupCriteria,
    ) -> Result<Vec<EventReceiverGroup>> {
        Ok(vec![])
    }

    async fn add_event_receiver_to_group(
        &self,
        _group_id: EventReceiverGroupId,
        _receiver_id: EventReceiverId,
    ) -> Result<()> {
        Ok(())
    }

    async fn remove_event_receiver_from_group(
        &self,
        _group_id: EventReceiverGroupId,
        _receiver_id: EventReceiverId,
    ) -> Result<()> {
        Ok(())
    }

    async fn get_group_event_receivers(
        &self,
        _group_id: EventReceiverGroupId,
    ) -> Result<Vec<EventReceiverId>> {
        Ok(vec![])
    }
}
