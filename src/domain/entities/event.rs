// src/domain/entities/event.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{EventId, EventReceiverId};
use crate::error::DomainError;

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
    pub fn new(
        name: String,
        version: String,
        release: String,
        platform_id: String,
        package: String,
        description: String,
        payload: serde_json::Value,
        success: bool,
        receiver_id: EventReceiverId,
    ) -> Result<Self, DomainError> {
        Self::validate_payload(&payload)?;

        Ok(Self {
            id: EventId::new(),
            name,
            version,
            release,
            platform_id,
            package,
            description,
            payload,
            success,
            event_receiver_id: receiver_id,
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
}
