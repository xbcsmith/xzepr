// src/domain/value_objects/event_id.rs

use serde::{Deserialize, Serialize};
use std::fmt;
use ulid::Ulid;

/// Value object representing a unique identifier for an event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(Ulid);

impl EventId {
    /// Creates a new event ID with a new ULID
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Creates an event ID from an existing ULID
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Parses an event ID from a string representation
    pub fn parse(s: &str) -> Result<Self, ulid::DecodeError> {
        Ok(Self(Ulid::from_string(s)?))
    }

    /// Returns the inner ULID
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }

    /// Returns the string representation of the event ID
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Returns the timestamp component of the ULID in milliseconds since Unix epoch
    pub fn timestamp_ms(&self) -> u64 {
        self.0.timestamp_ms()
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Ulid> for EventId {
    fn from(ulid: Ulid) -> Self {
        Self(ulid)
    }
}

impl From<EventId> for Ulid {
    fn from(id: EventId) -> Self {
        id.0
    }
}

impl std::str::FromStr for EventId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

// SQLx support for EventId
impl sqlx::Type<sqlx::Postgres> for EventId {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for EventId {
    fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(EventId::parse(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Postgres> for EventId {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        <String as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.to_string(), buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_event_id() {
        let id1 = EventId::new();
        let id2 = EventId::new();

        // Each new ID should be unique
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_from_ulid() {
        let ulid = Ulid::new();
        let id = EventId::from_ulid(ulid);

        assert_eq!(id.as_ulid(), ulid);
    }

    #[test]
    fn test_parse() {
        let ulid = Ulid::new();
        let ulid_str = ulid.to_string();
        let id = EventId::parse(&ulid_str).unwrap();

        assert_eq!(id.as_ulid(), ulid);
    }

    #[test]
    fn test_display() {
        let ulid = Ulid::new();
        let id = EventId::from_ulid(ulid);

        assert_eq!(id.to_string(), ulid.to_string());
    }

    #[test]
    fn test_serialization() {
        let id = EventId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: EventId = serde_json::from_str(&json).unwrap();

        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_parse_invalid_ulid() {
        let result = EventId::parse("invalid-ulid");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_str() {
        let ulid = Ulid::new();
        let ulid_str = ulid.to_string();
        let id: EventId = ulid_str.parse().unwrap();

        assert_eq!(id.as_ulid(), ulid);
    }

    #[test]
    fn test_timestamp_ms() {
        let id = EventId::new();
        let timestamp = id.timestamp_ms();

        // Timestamp should be reasonable (after 2020 and before far future)
        assert!(timestamp > 1_577_836_800_000); // Jan 1, 2020
        assert!(timestamp < 2_000_000_000_000); // Some date far in future
    }

    #[test]
    fn test_ordering_by_time() {
        let id1 = EventId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = EventId::new();

        // Later IDs should have higher timestamps
        assert!(id2.timestamp_ms() >= id1.timestamp_ms());
    }
}
