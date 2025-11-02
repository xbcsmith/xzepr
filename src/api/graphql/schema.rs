// src/api/graphql/schema.rs

use async_graphql::*;
use std::sync::Arc;

use crate::api::graphql::types::*;
use crate::application::handlers::{EventReceiverGroupHandler, EventReceiverHandler};
use crate::domain::repositories::event_receiver_group_repo::FindEventReceiverGroupCriteria;
use crate::domain::repositories::event_receiver_repo::FindEventReceiverCriteria;

pub struct Query;

#[Object]
impl Query {
    /// Get events by ID
    async fn events_by_id(&self, _ctx: &Context<'_>, _id: ID) -> Result<Vec<EventType>> {
        // TODO: Implement event queries once Event entity is complete
        Ok(vec![])
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
    async fn events(&self, _ctx: &Context<'_>, _event: FindEventInput) -> Result<Vec<EventType>> {
        // TODO: Implement event queries once Event entity is complete
        Ok(vec![])
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
    async fn create_event(&self, _ctx: &Context<'_>, _event: CreateEventInput) -> Result<ID> {
        // TODO: Implement event creation once Event entity is complete
        Err(Error::new("Event creation not implemented yet"))
    }

    /// Create a new event receiver
    async fn create_event_receiver(
        &self,
        ctx: &Context<'_>,
        event_receiver: CreateEventReceiverInput,
    ) -> Result<ID> {
        let handler = ctx.data::<Arc<EventReceiverHandler>>()?;

        match handler
            .create_event_receiver(
                event_receiver.name,
                event_receiver.receiver_type,
                event_receiver.version,
                event_receiver.description,
                event_receiver.schema.0,
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

        let receiver_ids = parse_event_receiver_ids(&event_receiver_group.event_receiver_ids)?;

        match handler
            .create_event_receiver_group(
                event_receiver_group.name,
                event_receiver_group.group_type,
                event_receiver_group.version,
                event_receiver_group.description,
                event_receiver_group.enabled,
                receiver_ids,
            )
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
}

pub type Schema = async_graphql::Schema<Query, Mutation, EmptySubscription>;

/// Creates a new GraphQL schema with the provided handlers
pub fn create_schema(
    event_receiver_handler: Arc<EventReceiverHandler>,
    event_receiver_group_handler: Arc<EventReceiverGroupHandler>,
) -> Schema {
    Schema::build(Query, Mutation, EmptySubscription)
        .data(event_receiver_handler)
        .data(event_receiver_group_handler)
        .finish()
}
