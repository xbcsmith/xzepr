// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/graphql/types.rs

use async_graphql::*;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

use crate::domain::entities::{
    event_receiver::EventReceiver, event_receiver_group::EventReceiverGroup,
};
use crate::domain::value_objects::{EventReceiverGroupId, EventReceiverId, UserId};

/// Wrapper for JSON values to implement custom scalar
#[derive(Debug, Clone, PartialEq)]
pub struct JSON(pub JsonValue);

#[Scalar]
impl ScalarType for JSON {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => serde_json::from_str(&s)
                .map(JSON)
                .map_err(|e| InputValueError::custom(format!("Invalid JSON: {}", e))),
            _ => Ok(JSON(serde_json::to_value(value).unwrap())),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

/// Wrapper for DateTime<Utc> to implement custom scalar
#[derive(Debug, Clone, PartialEq)]
pub struct Time(pub DateTime<Utc>);

#[Scalar]
impl ScalarType for Time {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => DateTime::parse_from_rfc3339(&s)
                .map(|dt| Time(dt.with_timezone(&Utc)))
                .map_err(|e| InputValueError::custom(format!("Invalid datetime: {}", e))),
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_rfc3339())
    }
}

/// GraphQL type for EventReceiver
#[derive(SimpleObject)]
#[graphql(name = "EventReceiver")]
pub struct EventReceiverType {
    pub id: ID,
    pub name: String,
    #[graphql(name = "type")]
    pub receiver_type: String,
    pub version: String,
    pub description: String,
    pub schema: JSON,
    pub fingerprint: String,
    pub created_at: Time,
}

impl From<EventReceiver> for EventReceiverType {
    fn from(receiver: EventReceiver) -> Self {
        Self {
            id: ID(receiver.id().to_string()),
            name: receiver.name().to_string(),
            receiver_type: receiver.receiver_type().to_string(),
            version: receiver.version().to_string(),
            description: receiver.description().to_string(),
            schema: JSON(receiver.schema().clone()),
            fingerprint: receiver.fingerprint().to_string(),
            created_at: Time(receiver.created_at()),
        }
    }
}

/// GraphQL type for EventReceiverGroup
#[derive(SimpleObject)]
#[graphql(name = "EventReceiverGroup")]
pub struct EventReceiverGroupType {
    pub id: ID,
    pub name: String,
    #[graphql(name = "type")]
    pub group_type: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub event_receiver_ids: Vec<ID>,
    pub created_at: Time,
    pub updated_at: Time,
}

impl From<EventReceiverGroup> for EventReceiverGroupType {
    fn from(group: EventReceiverGroup) -> Self {
        Self {
            id: ID(group.id().to_string()),
            name: group.name().to_string(),
            group_type: group.group_type().to_string(),
            version: group.version().to_string(),
            description: group.description().to_string(),
            enabled: group.enabled(),
            event_receiver_ids: group
                .event_receiver_ids()
                .iter()
                .map(|id| ID(id.to_string()))
                .collect(),
            created_at: Time(group.created_at()),
            updated_at: Time(group.updated_at()),
        }
    }
}

/// Input type for creating an event receiver
#[derive(InputObject)]
#[graphql(name = "CreateEventReceiverInput")]
pub struct CreateEventReceiverInput {
    pub name: String,
    #[graphql(name = "type")]
    pub receiver_type: String,
    pub version: String,
    pub description: String,
    pub schema: JSON,
}

impl From<CreateEventReceiverInput> for (String, String, String, String, JsonValue) {
    fn from(input: CreateEventReceiverInput) -> Self {
        (
            input.name,
            input.receiver_type,
            input.version,
            input.description,
            input.schema.0,
        )
    }
}

/// Input type for finding event receivers
#[derive(InputObject)]
#[graphql(name = "FindEventReceiverInput")]
pub struct FindEventReceiverInput {
    pub id: Option<ID>,
    pub name: Option<String>,
    #[graphql(name = "type")]
    pub receiver_type: Option<String>,
    pub version: Option<String>,
}

/// Input type for creating an event receiver group
#[derive(InputObject)]
#[graphql(name = "CreateEventReceiverGroupInput")]
pub struct CreateEventReceiverGroupInput {
    pub name: String,
    #[graphql(name = "type")]
    pub group_type: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub event_receiver_ids: Vec<ID>,
}

/// Input type for finding event receiver groups
#[derive(InputObject)]
#[graphql(name = "FindEventReceiverGroupInput")]
pub struct FindEventReceiverGroupInput {
    pub id: Option<ID>,
    pub name: Option<String>,
    #[graphql(name = "type")]
    pub group_type: Option<String>,
    pub version: Option<String>,
}

impl From<CreateEventInput>
    for (
        String,
        String,
        String,
        String,
        String,
        String,
        JsonValue,
        String,
        bool,
    )
{
    fn from(input: CreateEventInput) -> Self {
        (
            input.name,
            input.version,
            input.release,
            input.platform_id,
            input.package,
            input.description,
            input.payload.0,
            input.event_receiver_id.0,
            input.success,
        )
    }
}

/// Input type for creating events
#[derive(InputObject)]
#[graphql(name = "CreateEventInput")]
pub struct CreateEventInput {
    pub name: String,
    pub version: String,
    pub release: String,
    pub platform_id: String,
    pub package: String,
    pub description: String,
    pub payload: JSON,
    pub event_receiver_id: ID,
    pub success: bool,
}

/// Input type for finding events
#[derive(InputObject)]
#[graphql(name = "FindEventInput")]
pub struct FindEventInput {
    pub id: Option<ID>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub release: Option<String>,
    pub platform_id: Option<String>,
    pub package: Option<String>,
    pub success: Option<bool>,
    pub event_receiver_id: Option<ID>,
}

/// GraphQL type for Event
#[derive(SimpleObject)]
#[graphql(name = "Event")]
pub struct EventType {
    pub id: ID,
    pub name: String,
    pub version: String,
    pub release: String,
    pub platform_id: String,
    pub package: String,
    pub description: String,
    pub payload: JSON,
    pub event_receiver_id: ID,
    pub success: bool,
    pub created_at: Time,
}

/// Helper functions for ID parsing
pub fn parse_event_receiver_id(id: &ID) -> Result<EventReceiverId, Error> {
    EventReceiverId::parse(&id.0).map_err(|e| Error::new(format!("Invalid EventReceiverId: {}", e)))
}

pub fn parse_event_receiver_group_id(id: &ID) -> Result<EventReceiverGroupId, Error> {
    EventReceiverGroupId::parse(&id.0)
        .map_err(|e| Error::new(format!("Invalid EventReceiverGroupId: {}", e)))
}

pub fn parse_event_receiver_ids(ids: &[ID]) -> Result<Vec<EventReceiverId>, Error> {
    ids.iter()
        .map(parse_event_receiver_id)
        .collect::<Result<Vec<_>, _>>()
}

pub fn parse_user_id(id: &ID) -> Result<UserId, Error> {
    UserId::parse(&id.0).map_err(|e| Error::new(format!("Invalid UserId: {}", e)))
}

/// GraphQL type for a group member
#[derive(Debug, Clone, SimpleObject)]
pub struct GroupMemberType {
    pub user_id: ID,
    pub username: String,
    pub email: String,
    pub added_at: Time,
    pub added_by: ID,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::event_receiver::EventReceiver;
    use serde_json::json;

    #[test]
    fn test_event_receiver_type_conversion() {
        let schema = json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            }
        });

        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test receiver".to_string(),
            schema.clone(),
            UserId::new(),
        )
        .unwrap();

        let graphql_type: EventReceiverType = receiver.into();

        assert_eq!(graphql_type.name, "Test Receiver");
        assert_eq!(graphql_type.receiver_type, "webhook");
        assert_eq!(graphql_type.version, "1.0.0");
        assert_eq!(graphql_type.description, "A test receiver");
        assert_eq!(graphql_type.schema.0, schema);
    }

    #[test]
    fn test_id_parsing() {
        let receiver_id = EventReceiverId::new();
        let id = ID(receiver_id.to_string());

        let parsed = parse_event_receiver_id(&id).unwrap();
        assert_eq!(parsed, receiver_id);
    }

    #[test]
    fn test_invalid_id_parsing() {
        let invalid_id = ID("invalid-uuid".to_string());
        let result = parse_event_receiver_id(&invalid_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_id_parsing() {
        let receiver_ids = vec![EventReceiverId::new(), EventReceiverId::new()];
        let graphql_ids: Vec<ID> = receiver_ids.iter().map(|id| ID(id.to_string())).collect();

        let parsed = parse_event_receiver_ids(&graphql_ids).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed, receiver_ids);
    }
}
