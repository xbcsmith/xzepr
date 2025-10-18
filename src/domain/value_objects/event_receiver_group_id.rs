// src/domain/value_objects/event_receiver_group_id.rs

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Value object representing a unique identifier for an event receiver group
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventReceiverGroupId(Uuid);

impl EventReceiverGroupId {
    /// Creates a new event receiver group ID with a random UUID v7
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Creates an event receiver group ID from an existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parses an event receiver group ID from a string representation
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    /// Returns the inner UUID
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Returns the string representation of the event receiver group ID
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for EventReceiverGroupId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventReceiverGroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for EventReceiverGroupId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EventReceiverGroupId> for Uuid {
    fn from(id: EventReceiverGroupId) -> Self {
        id.0
    }
}

impl std::str::FromStr for EventReceiverGroupId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_event_receiver_group_id() {
        let id1 = EventReceiverGroupId::new();
        let id2 = EventReceiverGroupId::new();

        // Each new ID should be unique
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_from_uuid() {
        let uuid = Uuid::now_v7();
        let id = EventReceiverGroupId::from_uuid(uuid);

        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_parse() {
        let uuid = Uuid::now_v7();
        let uuid_str = uuid.to_string();
        let id = EventReceiverGroupId::parse(&uuid_str).unwrap();

        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn test_display() {
        let uuid = Uuid::now_v7();
        let id = EventReceiverGroupId::from_uuid(uuid);

        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn test_serialization() {
        let id = EventReceiverGroupId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: EventReceiverGroupId = serde_json::from_str(&json).unwrap();

        assert_eq!(id, deserialized);
    }
}
