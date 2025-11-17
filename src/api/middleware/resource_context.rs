// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/middleware/resource_context.rs

//! Resource context builders for OPA authorization
//!
//! This module provides trait and implementations for building resource context
//! from domain entities. Resource context is used by OPA to make authorization
//! decisions based on ownership, group membership, and resource state.

use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

use crate::domain::repositories::event_receiver_group_repo::EventReceiverGroupRepository;
use crate::domain::repositories::event_receiver_repo::EventReceiverRepository;
use crate::domain::repositories::event_repo::EventRepository;
use crate::domain::value_objects::{EventId, EventReceiverGroupId, EventReceiverId};
use crate::opa::types::ResourceContext;

/// Trait for building resource context from domain entities
///
/// Implementations of this trait query repositories to gather all necessary
/// information for authorization decisions, including:
/// - Resource ownership (owner_id)
/// - Group membership (group_id, group_members)
/// - Resource version for cache invalidation
#[async_trait]
pub trait ResourceContextBuilder: Send + Sync {
    /// Build resource context for a given resource
    ///
    /// # Arguments
    ///
    /// * `resource_id` - The ID of the resource as a string
    ///
    /// # Returns
    ///
    /// Returns `ResourceContext` with ownership and membership information,
    /// or an error if the resource is not found or cannot be loaded.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Resource does not exist
    /// - Repository query fails
    /// - Group membership cannot be retrieved
    async fn build_context(&self, resource_id: &str) -> Result<ResourceContext, String>;
}

/// Resource context builder for EventReceiver entities
pub struct EventReceiverContextBuilder {
    /// Repository for querying event receivers
    receiver_repo: Arc<dyn EventReceiverRepository>,
    /// Repository for querying event receiver groups
    #[allow(dead_code)]
    group_repo: Arc<dyn EventReceiverGroupRepository>,
}

impl EventReceiverContextBuilder {
    /// Create a new EventReceiver context builder
    pub fn new(
        receiver_repo: Arc<dyn EventReceiverRepository>,
        group_repo: Arc<dyn EventReceiverGroupRepository>,
    ) -> Self {
        Self {
            receiver_repo,
            group_repo,
        }
    }
}

#[async_trait]
impl ResourceContextBuilder for EventReceiverContextBuilder {
    async fn build_context(&self, resource_id: &str) -> Result<ResourceContext, String> {
        debug!(
            resource_id = %resource_id,
            "Building resource context for EventReceiver"
        );

        // Parse receiver ID
        let receiver_id = EventReceiverId::parse(resource_id)
            .map_err(|e| format!("Invalid receiver ID: {}", e))?;

        // Load receiver from repository
        let receiver = self
            .receiver_repo
            .find_by_id(receiver_id)
            .await
            .map_err(|e| format!("Failed to load receiver: {}", e))?
            .ok_or_else(|| format!("Receiver not found: {}", resource_id))?;

        let owner_id = Some(receiver.owner_id().to_string());
        let resource_version = receiver.resource_version();

        // TODO: Implement group membership lookup
        // EventReceiver doesn't have a direct group_id field
        // Need to query which groups contain this receiver
        let group_id = None;
        let group_members = Vec::new();

        Ok(ResourceContext {
            resource_type: "event_receiver".to_string(),
            resource_id: Some(resource_id.to_string()),
            owner_id,
            group_id,
            members: group_members,
            resource_version,
        })
    }
}

/// Resource context builder for Event entities
pub struct EventContextBuilder {
    /// Repository for querying events
    event_repo: Arc<dyn EventRepository>,
    /// Repository for querying event receivers
    receiver_repo: Arc<dyn EventReceiverRepository>,
    /// Repository for querying event receiver groups
    #[allow(dead_code)]
    group_repo: Arc<dyn EventReceiverGroupRepository>,
}

impl EventContextBuilder {
    /// Create a new Event context builder
    pub fn new(
        event_repo: Arc<dyn EventRepository>,
        receiver_repo: Arc<dyn EventReceiverRepository>,
        group_repo: Arc<dyn EventReceiverGroupRepository>,
    ) -> Self {
        Self {
            event_repo,
            receiver_repo,
            group_repo,
        }
    }
}

#[async_trait]
impl ResourceContextBuilder for EventContextBuilder {
    async fn build_context(&self, resource_id: &str) -> Result<ResourceContext, String> {
        debug!(
            resource_id = %resource_id,
            "Building resource context for Event"
        );

        // Parse event ID
        let event_id =
            EventId::parse(resource_id).map_err(|e| format!("Invalid event ID: {}", e))?;

        // Load event from repository
        let event = self
            .event_repo
            .find_by_id(event_id)
            .await
            .map_err(|e| format!("Failed to load event: {}", e))?
            .ok_or_else(|| format!("Event not found: {}", resource_id))?;

        // Events inherit ownership from their parent receiver
        let receiver_id = event.event_receiver_id();
        let receiver = self
            .receiver_repo
            .find_by_id(receiver_id)
            .await
            .map_err(|e| format!("Failed to load event receiver: {}", e))?
            .ok_or_else(|| format!("Event receiver not found: {}", receiver_id))?;

        let owner_id = Some(receiver.owner_id().to_string());
        let resource_version = event.resource_version();

        // TODO: Implement group membership lookup
        // EventReceiver doesn't have a direct group_id field
        // Need to query which groups contain this receiver
        let group_id = None;
        let group_members = Vec::new();

        Ok(ResourceContext {
            resource_type: "event".to_string(),
            resource_id: Some(resource_id.to_string()),
            owner_id,
            group_id,
            members: group_members,
            resource_version,
        })
    }
}

/// Resource context builder for EventReceiverGroup entities
pub struct EventReceiverGroupContextBuilder {
    /// Repository for querying event receiver groups
    group_repo: Arc<dyn EventReceiverGroupRepository>,
}

impl EventReceiverGroupContextBuilder {
    /// Create a new EventReceiverGroup context builder
    pub fn new(group_repo: Arc<dyn EventReceiverGroupRepository>) -> Self {
        Self { group_repo }
    }
}

#[async_trait]
impl ResourceContextBuilder for EventReceiverGroupContextBuilder {
    async fn build_context(&self, resource_id: &str) -> Result<ResourceContext, String> {
        debug!(
            resource_id = %resource_id,
            "Building resource context for EventReceiverGroup"
        );

        // Parse group ID
        let group_id = EventReceiverGroupId::parse(resource_id)
            .map_err(|e| format!("Invalid group ID: {}", e))?;

        // Load group from repository
        let group = self
            .group_repo
            .find_by_id(group_id)
            .await
            .map_err(|e| format!("Failed to load group: {}", e))?
            .ok_or_else(|| format!("Group not found: {}", resource_id))?;

        let owner_id = Some(group.owner_id().to_string());
        let resource_version = group.resource_version();

        // TODO: Query group members from membership table
        let group_members = Vec::new();

        Ok(ResourceContext {
            resource_type: "event_receiver_group".to_string(),
            resource_id: Some(resource_id.to_string()),
            owner_id,
            group_id: Some(resource_id.to_string()),
            members: group_members,
            resource_version,
        })
    }
}

// TODO: Add tests once mock repositories are available
// #[cfg(test)]
// mod tests {
//     use super::*;
//     // Tests will be added in Phase 3 testing task
// }
