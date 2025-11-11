// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/domain/entities/event.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{EventId, EventReceiverId};
use crate::error::DomainError;

/// Parameters for creating a new event
#[derive(Debug, Clone)]
pub struct CreateEventParams {
    pub name: String,
    pub version: String,
    pub release: String,
    pub platform_id: String,
    pub package: String,
    pub description: String,
    pub payload: serde_json::Value,
    pub success: bool,
    pub receiver_id: EventReceiverId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    id: EventId,
    name: String,
    version: String,
    release: String,
    platform_id: String,
    package: String,
    description: String,
    payload: serde_json::Value,
    success: bool,
    event_receiver_id: EventReceiverId,
    created_at: DateTime<Utc>,
}

impl Event {
    /// Creates a new event from parameters
    pub fn new(params: CreateEventParams) -> Result<Self, DomainError> {
        Self::validate_payload(&params.payload)?;

        Ok(Self {
            id: EventId::new(),
            name: params.name,
            version: params.version,
            release: params.release,
            platform_id: params.platform_id,
            package: params.package,
            description: params.description,
            payload: params.payload,
            success: params.success,
            event_receiver_id: params.receiver_id,
            created_at: Utc::now(),
        })
    }

    fn validate_payload(payload: &serde_json::Value) -> Result<(), DomainError> {
        // Validation logic
        if !payload.is_object() {
            return Err(DomainError::ValidationError {
                field: "payload".to_string(),
                message: "Event payload must be a JSON object".to_string(),
            });
        }
        Ok(())
    }

    // Getters
    pub fn id(&self) -> EventId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn release(&self) -> &str {
        &self.release
    }

    pub fn platform_id(&self) -> &str {
        &self.platform_id
    }

    pub fn package(&self) -> &str {
        &self.package
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    pub fn success(&self) -> bool {
        self.success
    }

    pub fn event_receiver_id(&self) -> EventReceiverId {
        self.event_receiver_id
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Reconstructs an event from database fields
    ///
    /// This method is used by repository implementations to reconstruct
    /// events from database rows with their original IDs and timestamps.
    ///
    /// # Arguments
    ///
    /// * `fields` - Database fields containing all event data
    ///
    /// # Returns
    ///
    /// Returns a reconstructed Event
    pub fn from_database(fields: DatabaseEventFields) -> Self {
        Self {
            id: fields.id,
            name: fields.name,
            version: fields.version,
            release: fields.release,
            platform_id: fields.platform_id,
            package: fields.package,
            description: fields.description,
            payload: fields.payload,
            success: fields.success,
            event_receiver_id: fields.event_receiver_id,
            created_at: fields.created_at,
        }
    }
}

/// Fields required to reconstruct an event from database
#[derive(Debug, Clone)]
pub struct DatabaseEventFields {
    pub id: EventId,
    pub name: String,
    pub version: String,
    pub release: String,
    pub platform_id: String,
    pub package: String,
    pub description: String,
    pub payload: serde_json::Value,
    pub success: bool,
    pub event_receiver_id: EventReceiverId,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_event() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({
            "message": "Test event",
            "status": "success"
        });

        let event = Event::new(CreateEventParams {
            name: "test-event".to_string(),
            version: "1.0.0".to_string(),
            release: "release-1".to_string(),
            platform_id: "platform-123".to_string(),
            package: "test-package".to_string(),
            description: "A test event".to_string(),
            payload: payload.clone(),
            success: true,
            receiver_id,
        });

        assert!(event.is_ok());
        let event = event.unwrap();
        assert_eq!(event.name(), "test-event");
        assert_eq!(event.version(), "1.0.0");
        assert_eq!(event.release(), "release-1");
        assert_eq!(event.platform_id(), "platform-123");
        assert_eq!(event.package(), "test-package");
        assert_eq!(event.description(), "A test event");
        assert_eq!(event.payload(), &payload);
        assert!(event.success());
        assert_eq!(event.event_receiver_id(), receiver_id);
    }

    #[test]
    fn test_create_successful_event() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({
            "deployment": "production",
            "version": "2.0.0"
        });

        let event = Event::new(CreateEventParams {
            name: "deployment-event".to_string(),
            version: "2.0.0".to_string(),
            release: "v2.0.0".to_string(),
            platform_id: "prod-platform".to_string(),
            package: "app-package".to_string(),
            description: "Production deployment".to_string(),
            payload,
            success: true,
            receiver_id,
        })
        .unwrap();

        assert!(event.success());
    }

    #[test]
    fn test_create_failed_event() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({
            "error": "Connection timeout",
            "retry_count": 3
        });

        let event = Event::new(CreateEventParams {
            name: "failed-build".to_string(),
            version: "1.2.3".to_string(),
            release: "rc-1".to_string(),
            platform_id: "ci-platform".to_string(),
            package: "build-package".to_string(),
            description: "Build failed".to_string(),
            payload,
            success: false,
            receiver_id,
        })
        .unwrap();

        assert!(!event.success());
    }

    #[test]
    fn test_event_id_is_unique() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({
            "data": "test"
        });

        let event1 = Event::new(CreateEventParams {
            name: "event1".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload: payload.clone(),
            success: true,
            receiver_id,
        })
        .unwrap();

        let event2 = Event::new(CreateEventParams {
            name: "event2".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload,
            success: true,
            receiver_id,
        })
        .unwrap();

        assert_ne!(event1.id(), event2.id());
    }

    #[test]
    fn test_validate_payload_must_be_object() {
        let receiver_id = EventReceiverId::new();
        let invalid_payload = json!("not an object");

        let result = Event::new(CreateEventParams {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload: invalid_payload,
            success: true,
            receiver_id,
        });

        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::ValidationError { field, message } => {
                assert_eq!(field, "payload");
                assert!(message.contains("JSON object"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_validate_payload_array_fails() {
        let receiver_id = EventReceiverId::new();
        let invalid_payload = json!([1, 2, 3]);

        let result = Event::new(CreateEventParams {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload: invalid_payload,
            success: true,
            receiver_id,
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_payload_null_fails() {
        let receiver_id = EventReceiverId::new();
        let invalid_payload = json!(null);

        let result = Event::new(CreateEventParams {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload: invalid_payload,
            success: true,
            receiver_id,
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_event_created_at_is_set() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({"test": "data"});

        let before = Utc::now();
        let event = Event::new(CreateEventParams {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload,
            success: true,
            receiver_id,
        })
        .unwrap();
        let after = Utc::now();

        assert!(event.created_at() >= before);
        assert!(event.created_at() <= after);
    }

    #[test]
    fn test_event_getters() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({"key": "value"});

        let event = Event::new(CreateEventParams {
            name: "getter-test".to_string(),
            version: "3.2.1".to_string(),
            release: "stable".to_string(),
            platform_id: "linux-x64".to_string(),
            package: "myapp".to_string(),
            description: "Testing getters".to_string(),
            payload: payload.clone(),
            success: true,
            receiver_id,
        })
        .unwrap();

        assert_eq!(event.name(), "getter-test");
        assert_eq!(event.version(), "3.2.1");
        assert_eq!(event.release(), "stable");
        assert_eq!(event.platform_id(), "linux-x64");
        assert_eq!(event.package(), "myapp");
        assert_eq!(event.description(), "Testing getters");
        assert_eq!(event.payload(), &payload);
        assert!(event.success());
        assert_eq!(event.event_receiver_id(), receiver_id);
    }

    #[test]
    fn test_event_empty_payload_object() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({});

        let result = Event::new(CreateEventParams {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload,
            success: true,
            receiver_id,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_event_complex_payload() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({
            "nested": {
                "level1": {
                    "level2": "value"
                }
            },
            "array": [1, 2, 3],
            "boolean": true,
            "number": 42,
            "string": "text"
        });

        let result = Event::new(CreateEventParams {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload: payload.clone(),
            success: true,
            receiver_id,
        });

        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.payload(), &payload);
    }

    #[test]
    fn test_event_clone() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({"test": "clone"});

        let event1 = Event::new(CreateEventParams {
            name: "event1".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload: payload.clone(),
            success: true,
            receiver_id,
        })
        .unwrap();

        let event2 = event1.clone();

        assert_eq!(event1.id(), event2.id());
        assert_eq!(event1.name(), event2.name());
        assert_eq!(event1.version(), event2.version());
        assert_eq!(event1.success(), event2.success());
    }

    #[test]
    fn test_event_serialization() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({"data": "serialize"});

        let event = Event::new(CreateEventParams {
            name: "serialize-test".to_string(),
            version: "1.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload,
            success: true,
            receiver_id,
        })
        .unwrap();

        let serialized = serde_json::to_string(&event);
        assert!(serialized.is_ok());

        let json_str = serialized.unwrap();
        assert!(json_str.contains("serialize-test"));
        assert!(json_str.contains("1.0.0"));
    }

    #[test]
    fn test_event_deserialization() {
        let receiver_id = EventReceiverId::new();
        let payload = json!({"data": "deserialize"});

        let event = Event::new(CreateEventParams {
            name: "deserialize-test".to_string(),
            version: "2.0.0".to_string(),
            release: "release".to_string(),
            platform_id: "platform".to_string(),
            package: "package".to_string(),
            description: "desc".to_string(),
            payload,
            success: true,
            receiver_id,
        })
        .unwrap();

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: Result<Event, _> = serde_json::from_str(&serialized);

        assert!(deserialized.is_ok());
        let deserialized_event = deserialized.unwrap();
        assert_eq!(deserialized_event.name(), "deserialize-test");
        assert_eq!(deserialized_event.version(), "2.0.0");
    }
}
