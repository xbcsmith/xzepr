// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/domain/entities/event.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    id: EventId,
    name: String,
    version: Version,
    release: String,
    platform_id: PlatformId,
    package: String,
    description: String,
    payload: serde_json::Value,
    success: bool,
    event_receiver_id: ReceiverId,
    created_at: DateTime<Utc>,
}

impl Event {
    pub fn new(
        name: String,
        version: Version,
        receiver_id: ReceiverId,
        payload: serde_json::Value,
    ) -> Result<Self, DomainError> {
        Self::validate_payload(&payload)?;
        
        Ok(Self {
            id: EventId::new(),
            name,
            version,
            event_receiver_id: receiver_id,
            payload,
            created_at: Utc::now(),
            // ... other fields with defaults
        })
    }
    
    fn validate_payload(payload: &serde_json::Value) -> Result<(), DomainError> {
        // Validation logic
        Ok(())
    }
}