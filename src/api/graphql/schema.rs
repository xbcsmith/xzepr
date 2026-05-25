// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/graphql/schema.rs

use async_graphql::*;
use std::sync::Arc;

use crate::api::graphql::error_codes;
use crate::api::graphql::guards::{
    parse_caller_user_id, require_authenticated_user, ComplexityConfig,
};
use crate::api::graphql::types::*;
use crate::application::handlers::event_receiver_group_handler::CreateEventReceiverGroupParams;
use crate::application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};
use crate::domain::entities::event::CreateEventParams;
use crate::domain::repositories::event_receiver_group_repo::FindEventReceiverGroupCriteria;
use crate::domain::repositories::event_receiver_repo::FindEventReceiverCriteria;
use crate::domain::repositories::event_repo::FindEventCriteria;

/// Root GraphQL query type.
pub struct Query;

#[Object]
impl Query {
    /// Get events by ID scoped to the authenticated owner.
    async fn events_by_id(&self, ctx: &Context<'_>, id: ID) -> Result<Vec<EventType>> {
        let handler = ctx.data::<Arc<EventHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;
        let event_id = parse_event_id(&id)?;

        match handler.get_event_for_user(event_id, owner_id).await {
            Ok(Some(event)) => Ok(vec![event.into()]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Get event receivers by ID scoped to the authenticated owner.
    async fn event_receivers_by_id(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<Vec<EventReceiverType>> {
        let handler = ctx.data::<Arc<EventReceiverHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;
        let receiver_id = parse_event_receiver_id(&id)?;

        match handler
            .get_event_receiver_for_user(receiver_id, owner_id)
            .await
        {
            Ok(Some(receiver)) => Ok(vec![receiver.into()]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Get event receiver groups by ID scoped to the authenticated owner.
    async fn event_receiver_groups_by_id(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<Vec<EventReceiverGroupType>> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;
        let group_id = parse_event_receiver_group_id(&id)?;

        match handler
            .get_event_receiver_group_for_user(group_id, owner_id)
            .await
        {
            Ok(Some(group)) => Ok(vec![group.into()]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Find events with criteria scoped to the authenticated owner.
    async fn events(&self, ctx: &Context<'_>, event: FindEventInput) -> Result<Vec<EventType>> {
        let handler = ctx.data::<Arc<EventHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;

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
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Find event receivers with criteria scoped to the authenticated owner.
    async fn event_receivers(
        &self,
        ctx: &Context<'_>,
        event_receiver: FindEventReceiverInput,
    ) -> Result<Vec<EventReceiverType>> {
        let handler = ctx.data::<Arc<EventReceiverHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;

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

        match handler
            .find_event_receivers_for_user(criteria, owner_id)
            .await
        {
            Ok(receivers) => Ok(receivers.into_iter().map(|r| r.into()).collect()),
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Find event receiver groups with criteria scoped to the authenticated owner.
    async fn event_receiver_groups(
        &self,
        ctx: &Context<'_>,
        event_receiver_group: FindEventReceiverGroupInput,
    ) -> Result<Vec<EventReceiverGroupType>> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;

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

        match handler
            .find_event_receiver_groups_for_user(criteria, owner_id)
            .await
        {
            Ok(groups) => Ok(groups.into_iter().map(|g| g.into()).collect()),
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }
}

/// Root GraphQL mutation type.
pub struct Mutation;

#[Object]
impl Mutation {
    /// Create a new event owned by the authenticated user.
    async fn create_event(&self, ctx: &Context<'_>, event: CreateEventInput) -> Result<ID> {
        let handler = ctx.data::<Arc<EventHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;
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
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Create a new event receiver owned by the authenticated user.
    async fn create_event_receiver(
        &self,
        ctx: &Context<'_>,
        event_receiver: CreateEventReceiverInput,
    ) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;

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
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Create a new event receiver group owned by the authenticated user.
    async fn create_event_receiver_group(
        &self,
        ctx: &Context<'_>,
        event_receiver_group: CreateEventReceiverGroupInput,
    ) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;

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
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Enable an event receiver group; caller must be the owner.
    async fn set_event_receiver_group_enabled(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;
        let group_id = parse_event_receiver_group_id(&id)?;

        match handler
            .enable_event_receiver_group_for_user(group_id, owner_id)
            .await
        {
            Ok(enabled_group_id) => Ok(ID(enabled_group_id.to_string())),
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Disable an event receiver group; caller must be the owner.
    async fn set_event_receiver_group_disabled(&self, ctx: &Context<'_>, id: ID) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverGroupHandler>>()?;
        let user = require_authenticated_user(ctx)?;
        let owner_id = parse_caller_user_id(user)?;
        let group_id = parse_event_receiver_group_id(&id)?;

        match handler
            .disable_event_receiver_group_for_user(group_id, owner_id)
            .await
        {
            Ok(disabled_group_id) => Ok(ID(disabled_group_id.to_string())),
            Err(e) => Err(error_codes::map_app_error(e)),
        }
    }

    /// Add a member to an event receiver group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The ID of the group to add the member to
    /// * `user_id` - The ID of the user to add as a member
    ///
    /// # Returns
    ///
    /// Returns the group member details on success
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
        let user = require_authenticated_user(ctx)?;

        // Parse authenticated user ID (the caller who is adding the member)
        let added_by = parse_caller_user_id(user)?;

        // Parse group ID
        let group_id = parse_event_receiver_group_id(&group_id)?;

        // Parse user ID to add
        let member_user_id = parse_user_id(&user_id)?;

        // Verify the group exists and the caller is the owner
        let group = handler
            .find_group_by_id(group_id)
            .await
            .map_err(error_codes::map_app_error)?
            .ok_or_else(|| error_codes::not_found("group"))?;

        if group.owner_id() != added_by {
            return Err(error_codes::forbidden(
                "Only the group owner can add members",
            ));
        }

        let details = handler
            .add_group_member_details(group_id, member_user_id, added_by)
            .await
            .map_err(error_codes::map_app_error)?;

        Ok(GroupMemberType {
            user_id: ID(details.user_id.to_string()),
            username: details.username,
            email: details.email,
            added_at: Time(details.added_at),
            added_by: ID(details.added_by.to_string()),
        })
    }

    /// Remove a member from an event receiver group.
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
        let user = require_authenticated_user(ctx)?;

        // Parse authenticated user ID (the caller who is removing the member)
        let removed_by = parse_caller_user_id(user)?;

        // Parse group ID
        let group_id = parse_event_receiver_group_id(&group_id)?;

        // Parse user ID to remove
        let member_user_id = parse_user_id(&user_id)?;

        // Verify the group exists and the caller is the owner
        let group = handler
            .find_group_by_id(group_id)
            .await
            .map_err(error_codes::map_app_error)?
            .ok_or_else(|| error_codes::not_found("group"))?;

        if group.owner_id() != removed_by {
            return Err(error_codes::forbidden(
                "Only the group owner can remove members",
            ));
        }

        // Remove the member
        handler
            .remove_group_member(group_id, member_user_id)
            .await
            .map_err(error_codes::map_app_error)?;

        Ok(true)
    }
}

/// Compiled GraphQL schema type alias.
pub type Schema = async_graphql::Schema<Query, Mutation, EmptySubscription>;

/// Creates a new GraphQL schema with the provided handlers.
///
/// Complexity and depth limits are set to the compiled-in defaults.
///
/// # Arguments
///
/// * `event_handler` - Application handler for event operations.
/// * `event_receiver_handler` - Application handler for receiver operations.
/// * `event_receiver_group_handler` - Application handler for group operations.
///
/// # Returns
///
/// Returns an executable GraphQL schema.
pub fn create_schema(
    event_handler: Arc<EventHandler>,
    event_receiver_handler: Arc<EventReceiverHandler>,
    event_receiver_group_handler: Arc<EventReceiverGroupHandler>,
) -> Schema {
    create_schema_with_config(
        event_handler,
        event_receiver_handler,
        event_receiver_group_handler,
        ComplexityConfig::default(),
    )
}

/// Creates a new GraphQL schema with runtime complexity and depth settings.
///
/// # Arguments
///
/// * `event_handler` - Application handler for event operations.
/// * `event_receiver_handler` - Application handler for receiver operations.
/// * `event_receiver_group_handler` - Application handler for group operations.
/// * `complexity_config` - Runtime GraphQL complexity and depth configuration.
///
/// # Returns
///
/// Returns an executable GraphQL schema configured with runtime limits.
pub fn create_schema_with_config(
    event_handler: Arc<EventHandler>,
    event_receiver_handler: Arc<EventReceiverHandler>,
    event_receiver_group_handler: Arc<EventReceiverGroupHandler>,
    complexity_config: ComplexityConfig,
) -> Schema {
    let mut builder = Schema::build(Query, Mutation, EmptySubscription)
        .data(event_handler)
        .data(event_receiver_handler)
        .data(event_receiver_group_handler);

    if complexity_config.enforce {
        builder = builder
            .limit_complexity(complexity_config.max_complexity)
            .limit_depth(complexity_config.max_depth);
    }

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::handlers::{
        EventHandler, EventReceiverGroupHandler, EventReceiverHandler,
    };
    use crate::domain::entities::event::Event;
    use crate::domain::entities::event_receiver::EventReceiver;
    use crate::domain::entities::event_receiver_group::EventReceiverGroup;
    use crate::domain::repositories::{
        event_receiver_group_repo::{EventReceiverGroupRepository, FindEventReceiverGroupCriteria},
        event_receiver_repo::{EventReceiverRepository, FindEventReceiverCriteria},
        event_repo::{EventRepository, FindEventCriteria},
    };
    use crate::domain::value_objects::{EventId, EventReceiverGroupId, EventReceiverId, UserId};
    use crate::error::Result;
    use async_trait::async_trait;

    /// Minimal no-op mock for EventRepository. All methods return empty/default values.
    struct MockEventRepo;

    #[async_trait]
    impl EventRepository for MockEventRepo {
        async fn save(&self, _: &Event) -> Result<()> {
            Ok(())
        }
        async fn find_by_id(&self, _: EventId) -> Result<Option<Event>> {
            Ok(None)
        }
        async fn find_by_receiver_id(&self, _: EventReceiverId) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn find_by_success(&self, _: bool) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn find_by_name(&self, _: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn find_by_platform_id(&self, _: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn find_by_package(&self, _: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn list(&self, _: usize, _: usize) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn count(&self) -> Result<usize> {
            Ok(0)
        }
        async fn count_by_receiver_id(&self, _: EventReceiverId) -> Result<usize> {
            Ok(0)
        }
        async fn count_successful_by_receiver_id(&self, _: EventReceiverId) -> Result<usize> {
            Ok(0)
        }
        async fn delete(&self, _: EventId) -> Result<()> {
            Ok(())
        }
        async fn find_latest_by_receiver_id(&self, _: EventReceiverId) -> Result<Option<Event>> {
            Ok(None)
        }
        async fn find_latest_successful_by_receiver_id(
            &self,
            _: EventReceiverId,
        ) -> Result<Option<Event>> {
            Ok(None)
        }
        async fn find_by_time_range(
            &self,
            _: chrono::DateTime<chrono::Utc>,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn find_by_criteria(&self, _: FindEventCriteria) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn find_by_owner(&self, _: UserId) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn find_by_owner_paginated(
            &self,
            _: UserId,
            _: usize,
            _: usize,
        ) -> Result<Vec<Event>> {
            Ok(vec![])
        }
        async fn is_owner(&self, _: EventId, _: UserId) -> Result<bool> {
            Ok(false)
        }
        async fn get_resource_version(&self, _: EventId) -> Result<Option<i64>> {
            Ok(None)
        }
    }

    /// Minimal no-op mock for EventReceiverRepository.
    struct MockReceiverRepo;

    #[async_trait]
    impl EventReceiverRepository for MockReceiverRepo {
        async fn save(&self, _: &EventReceiver) -> Result<()> {
            Ok(())
        }
        async fn find_by_id(&self, _: EventReceiverId) -> Result<Option<EventReceiver>> {
            Ok(None)
        }
        async fn find_by_name(&self, _: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
        async fn find_by_type(&self, _: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
        async fn find_by_type_and_version(&self, _: &str, _: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
        async fn find_by_fingerprint(&self, _: &str) -> Result<Option<EventReceiver>> {
            Ok(None)
        }
        async fn list(&self, _: usize, _: usize) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
        async fn count(&self) -> Result<usize> {
            Ok(0)
        }
        async fn update(&self, _: &EventReceiver) -> Result<()> {
            Ok(())
        }
        async fn delete(&self, _: EventReceiverId) -> Result<()> {
            Ok(())
        }
        async fn exists_by_name_and_type(&self, _: &str, _: &str) -> Result<bool> {
            Ok(false)
        }
        async fn find_by_criteria(
            &self,
            _: FindEventReceiverCriteria,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
        async fn find_by_owner(&self, _: UserId) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
        async fn find_by_owner_paginated(
            &self,
            _: UserId,
            _: usize,
            _: usize,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
        async fn is_owner(&self, _: EventReceiverId, _: UserId) -> Result<bool> {
            Ok(false)
        }
        async fn get_resource_version(&self, _: EventReceiverId) -> Result<Option<i64>> {
            Ok(None)
        }
    }

    /// Minimal no-op mock for EventReceiverGroupRepository.
    struct MockGroupRepo;

    #[async_trait]
    impl EventReceiverGroupRepository for MockGroupRepo {
        async fn save(&self, _: &EventReceiverGroup) -> Result<()> {
            Ok(())
        }
        async fn find_by_id(&self, _: EventReceiverGroupId) -> Result<Option<EventReceiverGroup>> {
            Ok(None)
        }
        async fn find_by_name(&self, _: &str) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
        async fn find_by_type(&self, _: &str) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
        async fn find_by_type_and_version(
            &self,
            _: &str,
            _: &str,
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
            _: EventReceiverId,
        ) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
        async fn list(&self, _: usize, _: usize) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
        async fn count(&self) -> Result<usize> {
            Ok(0)
        }
        async fn count_enabled(&self) -> Result<usize> {
            Ok(0)
        }
        async fn count_disabled(&self) -> Result<usize> {
            Ok(0)
        }
        async fn update(&self, _: &EventReceiverGroup) -> Result<()> {
            Ok(())
        }
        async fn delete(&self, _: EventReceiverGroupId) -> Result<()> {
            Ok(())
        }
        async fn enable(&self, _: EventReceiverGroupId) -> Result<()> {
            Ok(())
        }
        async fn disable(&self, _: EventReceiverGroupId) -> Result<()> {
            Ok(())
        }
        async fn exists_by_name_and_type(&self, _: &str, _: &str) -> Result<bool> {
            Ok(false)
        }
        async fn add_event_receiver_to_group(
            &self,
            _: EventReceiverGroupId,
            _: EventReceiverId,
        ) -> Result<()> {
            Ok(())
        }
        async fn remove_event_receiver_from_group(
            &self,
            _: EventReceiverGroupId,
            _: EventReceiverId,
        ) -> Result<()> {
            Ok(())
        }
        async fn get_group_event_receivers(
            &self,
            _: EventReceiverGroupId,
        ) -> Result<Vec<EventReceiverId>> {
            Ok(vec![])
        }
        async fn find_by_criteria(
            &self,
            _: FindEventReceiverGroupCriteria,
        ) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
        async fn find_by_owner(&self, _: UserId) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
        async fn find_by_owner_paginated(
            &self,
            _: UserId,
            _: usize,
            _: usize,
        ) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
        async fn is_owner(&self, _: EventReceiverGroupId, _: UserId) -> Result<bool> {
            Ok(false)
        }
        async fn get_resource_version(&self, _: EventReceiverGroupId) -> Result<Option<i64>> {
            Ok(None)
        }
        async fn is_member(&self, _: EventReceiverGroupId, _: UserId) -> Result<bool> {
            Ok(false)
        }
        async fn get_group_members(&self, _: EventReceiverGroupId) -> Result<Vec<UserId>> {
            Ok(vec![])
        }
        async fn add_member(&self, _: EventReceiverGroupId, _: UserId, _: UserId) -> Result<()> {
            Ok(())
        }
        async fn remove_member(&self, _: EventReceiverGroupId, _: UserId) -> Result<()> {
            Ok(())
        }
        async fn find_groups_for_user(&self, _: UserId) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
    }

    /// Builds a test schema backed by no-op mock repositories.
    fn create_test_schema() -> Schema {
        let event_repo = Arc::new(MockEventRepo);
        let receiver_repo = Arc::new(MockReceiverRepo);
        let group_repo = Arc::new(MockGroupRepo);

        let event_handler = Arc::new(EventHandler::new(event_repo, receiver_repo.clone()));
        let receiver_handler = Arc::new(EventReceiverHandler::new(receiver_repo.clone()));
        let group_handler = Arc::new(EventReceiverGroupHandler::new(group_repo, receiver_repo));

        create_schema(event_handler, receiver_handler, group_handler)
    }

    /// Extracts the first error extension code from a GraphQL response.
    fn first_error_code(response: &async_graphql::Response) -> Option<String> {
        response.errors.first().and_then(|e| {
            e.extensions.as_ref().and_then(|ext| {
                ext.get("code").and_then(|v| match v {
                    async_graphql::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
            })
        })
    }

    /// Executes a GraphQL request against the test schema without any auth context.
    async fn execute_unauthenticated(schema: &Schema, query: &str) -> async_graphql::Response {
        schema.execute(async_graphql::Request::new(query)).await
    }

    #[tokio::test]
    async fn test_event_receivers_by_id_requires_auth() {
        let schema = create_test_schema();
        let response = execute_unauthenticated(
            &schema,
            r#"{ eventReceiversById(id: "00000000000000000000000000") { id } }"#,
        )
        .await;

        assert!(
            !response.errors.is_empty(),
            "unauthenticated request should return errors"
        );
        assert_eq!(
            first_error_code(&response).as_deref(),
            Some("UNAUTHENTICATED"),
            "error code should be UNAUTHENTICATED"
        );
    }

    #[tokio::test]
    async fn test_event_receiver_groups_by_id_requires_auth() {
        let schema = create_test_schema();
        let response = execute_unauthenticated(
            &schema,
            r#"{ eventReceiverGroupsById(id: "00000000000000000000000000") { id } }"#,
        )
        .await;

        assert!(
            !response.errors.is_empty(),
            "unauthenticated request should return errors"
        );
        assert_eq!(
            first_error_code(&response).as_deref(),
            Some("UNAUTHENTICATED"),
            "error code should be UNAUTHENTICATED"
        );
    }

    #[tokio::test]
    async fn test_event_receivers_requires_auth() {
        let schema = create_test_schema();
        let response =
            execute_unauthenticated(&schema, r#"{ eventReceivers(eventReceiver: {}) { id } }"#)
                .await;

        assert!(
            !response.errors.is_empty(),
            "unauthenticated request should return errors"
        );
        assert_eq!(
            first_error_code(&response).as_deref(),
            Some("UNAUTHENTICATED"),
            "error code should be UNAUTHENTICATED"
        );
    }

    #[tokio::test]
    async fn test_event_receiver_groups_requires_auth() {
        let schema = create_test_schema();
        let response = execute_unauthenticated(
            &schema,
            r#"{ eventReceiverGroups(eventReceiverGroup: {}) { id } }"#,
        )
        .await;

        assert!(
            !response.errors.is_empty(),
            "unauthenticated request should return errors"
        );
        assert_eq!(
            first_error_code(&response).as_deref(),
            Some("UNAUTHENTICATED"),
            "error code should be UNAUTHENTICATED"
        );
    }

    #[tokio::test]
    async fn test_set_event_receiver_group_enabled_requires_auth() {
        let schema = create_test_schema();
        let response = execute_unauthenticated(
            &schema,
            r#"mutation { setEventReceiverGroupEnabled(id: "00000000000000000000000000") }"#,
        )
        .await;

        assert!(
            !response.errors.is_empty(),
            "unauthenticated mutation should return errors"
        );
        assert_eq!(
            first_error_code(&response).as_deref(),
            Some("UNAUTHENTICATED"),
            "error code should be UNAUTHENTICATED"
        );
    }

    #[tokio::test]
    async fn test_set_event_receiver_group_disabled_requires_auth() {
        let schema = create_test_schema();
        let response = execute_unauthenticated(
            &schema,
            r#"mutation { setEventReceiverGroupDisabled(id: "00000000000000000000000000") }"#,
        )
        .await;

        assert!(
            !response.errors.is_empty(),
            "unauthenticated mutation should return errors"
        );
        assert_eq!(
            first_error_code(&response).as_deref(),
            Some("UNAUTHENTICATED"),
            "error code should be UNAUTHENTICATED"
        );
    }
}
