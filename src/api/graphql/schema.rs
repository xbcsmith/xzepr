// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/graphql/schema.rs

use async_graphql::*;
use std::sync::Arc;

use crate::api::graphql::types::*;
use crate::api::middleware::jwt::AuthenticatedUser;
use crate::application::handlers::event_receiver_group_handler::CreateEventReceiverGroupParams;
use crate::application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};
use crate::domain::entities::event::CreateEventParams;
use crate::domain::repositories::event_receiver_group_repo::FindEventReceiverGroupCriteria;
use crate::domain::repositories::event_receiver_repo::FindEventReceiverCriteria;
use crate::domain::repositories::event_repo::FindEventCriteria;
use crate::domain::value_objects::UserId;

pub struct Query;

#[Object]
impl Query {
    /// Get events by ID
    async fn events_by_id(&self, ctx: &Context<'_>, id: ID) -> Result<Vec<EventType>> {
        let handler = ctx.data::<Arc<EventHandler>>()?;
        let user = ctx.data::<AuthenticatedUser>()?;
        let owner_id = UserId::parse(user.user_id())
            .map_err(|e| Error::new(format!("Invalid user ID: {}", e)))?;
        let event_id = parse_event_id(&id)?;

        match handler.get_event_for_user(event_id, owner_id).await {
            Ok(Some(event)) => Ok(vec![event.into()]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(Error::new(format!("Failed to get event: {}", e))),
        }
    }

    /// Get event receivers by ID
    async fn event_receivers_by_id(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<Vec<EventReceiverType>> {
        let handler = ctx.data::<Arc<EventReceiverHandler>>()?;
        let receiver_id = parse_event_receiver_id(&id)?;

        match handler.get_event_receiver(receiver_id).await {
            Ok(Some(receiver)) => Ok(vec![receiver.into()]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(Error::new(format!("Failed to get event receiver: {}", e))),
        }
    }

    /// Get event receiver groups by ID
    async fn event_receiver_groups_by_id(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<Vec<EventReceiverGroupType>> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let group_id = parse_event_receiver_group_id(&id)?;

        match handler.get_event_receiver_group(group_id).await {
            Ok(Some(group)) => Ok(vec![group.into()]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(Error::new(format!(
                "Failed to get event receiver group: {}",
                e
            ))),
        }
    }

    /// Find events with criteria
    async fn events(&self, ctx: &Context<'_>, event: FindEventInput) -> Result<Vec<EventType>> {
        let handler = ctx.data::<Arc<EventHandler>>()?;
        let user = ctx.data::<AuthenticatedUser>()?;
        let owner_id = UserId::parse(user.user_id())
            .map_err(|e| Error::new(format!("Invalid user ID: {}", e)))?;

        let mut criteria = FindEventCriteria::new();

        if let Some(id) = event.id {
            criteria = criteria.with_id(parse_event_id(&id)?);
        }
        if let Some(name) = event.name {
            criteria = criteria.with_name(name);
        }
        if let Some(version) = event.version {
            criteria = criteria.with_version(version);
        }
        if let Some(release) = event.release {
            criteria = criteria.with_release(release);
        }
        if let Some(platform_id) = event.platform_id {
            criteria = criteria.with_platform_id(platform_id);
        }
        if let Some(package) = event.package {
            criteria = criteria.with_package(package);
        }
        if let Some(success) = event.success {
            criteria = criteria.with_success(success);
        }
        if let Some(receiver_id) = event.event_receiver_id {
            criteria = criteria.with_event_receiver_id(parse_event_receiver_id(&receiver_id)?);
        }

        match handler.find_events_for_user(criteria, owner_id).await {
            Ok(events) => Ok(events.into_iter().map(EventType::from).collect()),
            Err(e) => Err(Error::new(format!("Failed to find events: {}", e))),
        }
    }

    /// Find event receivers with criteria
    async fn event_receivers(
        &self,
        ctx: &Context<'_>,
        event_receiver: FindEventReceiverInput,
    ) -> Result<Vec<EventReceiverType>> {
        let handler = ctx.data::<Arc<EventReceiverHandler>>()?;

        let mut criteria = FindEventReceiverCriteria::new();

        if let Some(id) = event_receiver.id {
            let receiver_id = parse_event_receiver_id(&id)?;
            criteria = criteria.with_id(receiver_id);
        }

        if let Some(name) = event_receiver.name {
            criteria = criteria.with_name(name);
        }

        if let Some(receiver_type) = event_receiver.receiver_type {
            criteria = criteria.with_type(receiver_type);
        }

        if let Some(version) = event_receiver.version {
            criteria = criteria.with_version(version);
        }

        // Set default pagination if no specific criteria
        if criteria.is_empty() {
            criteria = criteria.with_limit(50).with_offset(0);
        }

        match handler.find_by_criteria(criteria).await {
            Ok(receivers) => Ok(receivers.into_iter().map(|r| r.into()).collect()),
            Err(e) => Err(Error::new(format!("Failed to find event receivers: {}", e))),
        }
    }

    /// Find event receiver groups with criteria
    async fn event_receiver_groups(
        &self,
        ctx: &Context<'_>,
        event_receiver_group: FindEventReceiverGroupInput,
    ) -> Result<Vec<EventReceiverGroupType>> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;

        let mut criteria = FindEventReceiverGroupCriteria::new();

        if let Some(id) = event_receiver_group.id {
            let group_id = parse_event_receiver_group_id(&id)?;
            criteria = criteria.with_id(group_id);
        }

        if let Some(name) = event_receiver_group.name {
            criteria = criteria.with_name(name);
        }

        if let Some(group_type) = event_receiver_group.group_type {
            criteria = criteria.with_type(group_type);
        }

        if let Some(version) = event_receiver_group.version {
            criteria = criteria.with_version(version);
        }

        // Set default pagination if no specific criteria
        if criteria.is_empty() {
            criteria = criteria.with_limit(50).with_offset(0);
        }

        match handler.find_by_criteria(criteria).await {
            Ok(groups) => Ok(groups.into_iter().map(|g| g.into()).collect()),
            Err(e) => Err(Error::new(format!(
                "Failed to find event receiver groups: {}",
                e
            ))),
        }
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    /// Create a new event
    async fn create_event(&self, ctx: &Context<'_>, event: CreateEventInput) -> Result<ID> {
        let handler = ctx.data::<Arc<EventHandler>>()?;
        let user = ctx.data::<AuthenticatedUser>()?;
        let owner_id = UserId::parse(user.user_id())
            .map_err(|e| Error::new(format!("Invalid user ID: {}", e)))?;
        let receiver_id = parse_event_receiver_id(&event.event_receiver_id)?;

        match handler
            .create_event(CreateEventParams {
                name: event.name,
                version: event.version,
                release: event.release,
                platform_id: event.platform_id,
                package: event.package,
                description: event.description,
                payload: event.payload.0,
                success: event.success,
                receiver_id,
                owner_id,
            })
            .await
        {
            Ok(event_id) => Ok(ID(event_id.to_string())),
            Err(e) => Err(Error::new(format!("Failed to create event: {}", e))),
        }
    }

    /// Create a new event receiver
    async fn create_event_receiver(
        &self,
        ctx: &Context<'_>,
        event_receiver: CreateEventReceiverInput,
    ) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverHandler>>()?;
        let user = ctx.data::<AuthenticatedUser>()?;

        // Parse user ID from authenticated user
        let owner_id = UserId::parse(user.user_id())
            .map_err(|e| Error::new(format!("Invalid user ID: {}", e)))?;

        match handler
            .create_event_receiver(
                event_receiver.name,
                event_receiver.receiver_type,
                event_receiver.version,
                event_receiver.description,
                event_receiver.schema.0,
                owner_id,
            )
            .await
        {
            Ok(receiver_id) => Ok(ID(receiver_id.to_string())),
            Err(e) => Err(Error::new(format!(
                "Failed to create event receiver: {}",
                e
            ))),
        }
    }

    /// Create a new event receiver group
    async fn create_event_receiver_group(
        &self,
        ctx: &Context<'_>,
        event_receiver_group: CreateEventReceiverGroupInput,
    ) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let user = ctx.data::<AuthenticatedUser>()?;

        // Parse user ID from authenticated user
        let owner_id = UserId::parse(user.user_id())
            .map_err(|e| Error::new(format!("Invalid user ID: {}", e)))?;

        let receiver_ids = parse_event_receiver_ids(&event_receiver_group.event_receiver_ids)?;

        match handler
            .create_event_receiver_group(CreateEventReceiverGroupParams {
                name: event_receiver_group.name,
                group_type: event_receiver_group.group_type,
                version: event_receiver_group.version,
                description: event_receiver_group.description,
                enabled: event_receiver_group.enabled,
                event_receiver_ids: receiver_ids,
                owner_id,
            })
            .await
        {
            Ok(group_id) => Ok(ID(group_id.to_string())),
            Err(e) => Err(Error::new(format!(
                "Failed to create event receiver group: {}",
                e
            ))),
        }
    }

    /// Enable an event receiver group
    async fn set_event_receiver_group_enabled(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let group_id = parse_event_receiver_group_id(&id)?;

        match handler.enable_event_receiver_group(group_id).await {
            Ok(enabled_group_id) => Ok(ID(enabled_group_id.to_string())),
            Err(e) => Err(Error::new(format!(
                "Failed to enable event receiver group: {}",
                e
            ))),
        }
    }

    /// Disable an event receiver group
    async fn set_event_receiver_group_disabled(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let group_id = parse_event_receiver_group_id(&id)?;

        match handler.disable_event_receiver_group(group_id).await {
            Ok(disabled_group_id) => Ok(ID(disabled_group_id.to_string())),
            Err(e) => Err(Error::new(format!(
                "Failed to disable event receiver group: {}",
                e
            ))),
        }
    }

    /// Add a member to an event receiver group
    ///
    /// # Arguments
    ///
    /// * `group_id` - The ID of the group to add the member to
    /// * `user_id` - The ID of the user to add as a member
    ///
    /// # Returns
    ///
    /// Returns the group ID on success
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Group not found
    /// - User is not the group owner
    /// - User is already a member
    /// - Invalid ID format
    async fn add_group_member(
        &self,
        ctx: &Context<'_>,
        group_id: ID,
        user_id: ID,
    ) -> Result<GroupMemberType> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let user = ctx.data::<AuthenticatedUser>()?;

        // Parse authenticated user ID (who is adding the member)
        let added_by = UserId::parse(user.user_id())
            .map_err(|e| Error::new(format!("Invalid user ID in token: {}", e)))?;

        // Parse group ID
        let group_id = parse_event_receiver_group_id(&group_id)?;

        // Parse user ID to add
        let member_user_id = parse_user_id(&user_id)?;

        // Verify the group exists and user is owner
        let group = handler
            .find_group_by_id(group_id)
            .await
            .map_err(|e| Error::new(format!("Failed to fetch group: {}", e)))?
            .ok_or_else(|| Error::new("Group not found"))?;

        if group.owner_id() != added_by {
            return Err(Error::new("Only the group owner can add members"));
        }

        let details = handler
            .add_group_member_details(group_id, member_user_id, added_by)
            .await
            .map_err(|e| Error::new(format!("Failed to add member: {}", e)))?;

        Ok(GroupMemberType {
            user_id: ID(details.user_id.to_string()),
            username: details.username,
            email: details.email,
            added_at: Time(details.added_at),
            added_by: ID(details.added_by.to_string()),
        })
    }

    /// Remove a member from an event receiver group
    ///
    /// # Arguments
    ///
    /// * `group_id` - The ID of the group to remove the member from
    /// * `user_id` - The ID of the user to remove
    ///
    /// # Returns
    ///
    /// Returns true on success
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Group not found
    /// - User is not the group owner
    /// - User is not a member
    /// - Invalid ID format
    async fn remove_group_member(
        &self,
        ctx: &Context<'_>,
        group_id: ID,
        user_id: ID,
    ) -> Result<bool> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let user = ctx.data::<AuthenticatedUser>()?;

        // Parse authenticated user ID (who is removing the member)
        let removed_by = UserId::parse(user.user_id())
            .map_err(|e| Error::new(format!("Invalid user ID in token: {}", e)))?;

        // Parse group ID
        let group_id = parse_event_receiver_group_id(&group_id)?;

        // Parse user ID to remove
        let member_user_id = parse_user_id(&user_id)?;

        // Verify the group exists and user is owner
        let group = handler
            .find_group_by_id(group_id)
            .await
            .map_err(|e| Error::new(format!("Failed to fetch group: {}", e)))?
            .ok_or_else(|| Error::new("Group not found"))?;

        if group.owner_id() != removed_by {
            return Err(Error::new("Only the group owner can remove members"));
        }

        // Remove the member
        handler
            .remove_group_member(group_id, member_user_id)
            .await
            .map_err(|e| Error::new(format!("Failed to remove member: {}", e)))?;

        Ok(true)
    }
}

pub type Schema = async_graphql::Schema<Query, Mutation, EmptySubscription>;

/// Creates a new GraphQL schema with the provided handlers
pub fn create_schema(
    event_handler: Arc<EventHandler>,
    event_receiver_handler: Arc<EventReceiverHandler>,
    event_receiver_group_handler: Arc<EventReceiverGroupHandler>,
) -> Schema {
    Schema::build(Query, Mutation, EmptySubscription)
        .data(event_handler)
        .data(event_receiver_handler)
        .data(event_receiver_group_handler)
        .finish()
}
