// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/rest/dtos.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::domain::entities::{
    event::Event, event_receiver::EventReceiver, event_receiver_group::EventReceiverGroup,
};
use crate::domain::value_objects::EventReceiverId;
use crate::error::DomainError;

/// Request DTO for creating an event receiver
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateEventReceiverRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub receiver_type: String,
    pub version: String,
    pub description: String,
    pub schema: JsonValue,
}

impl CreateEventReceiverRequest {
    /// Validates the request data
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "name".to_string(),
                message: "Name cannot be empty".to_string(),
            });
        }

        if self.receiver_type.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "type".to_string(),
                message: "Type cannot be empty".to_string(),
            });
        }

        if self.version.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "version".to_string(),
                message: "Version cannot be empty".to_string(),
            });
        }

        if !self.schema.is_object() {
            return Err(DomainError::ValidationError {
                field: "schema".to_string(),
                message: "Schema must be a JSON object".to_string(),
            });
        }

        Ok(())
    }
}

/// Response DTO for event receiver creation
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEventReceiverResponse {
    pub data: String, // ULID as string
}

/// Request DTO for creating an event
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateEventRequest {
    pub name: String,
    pub version: String,
    pub release: String,
    pub platform_id: String,
    pub package: String,
    pub description: String,
    pub payload: JsonValue,
    pub success: bool,
    pub event_receiver_id: String, // ULID as string
}

impl CreateEventRequest {
    /// Validates the request data
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "name".to_string(),
                message: "Name cannot be empty".to_string(),
            });
        }

        if self.version.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "version".to_string(),
                message: "Version cannot be empty".to_string(),
            });
        }

        if self.release.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "release".to_string(),
                message: "Release cannot be empty".to_string(),
            });
        }

        if self.platform_id.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "platform_id".to_string(),
                message: "Platform ID cannot be empty".to_string(),
            });
        }

        if self.package.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "package".to_string(),
                message: "Package cannot be empty".to_string(),
            });
        }

        if self.event_receiver_id.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "event_receiver_id".to_string(),
                message: "Event receiver ID cannot be empty".to_string(),
            });
        }

        if !self.payload.is_object() {
            return Err(DomainError::ValidationError {
                field: "payload".to_string(),
                message: "Payload must be a JSON object".to_string(),
            });
        }

        Ok(())
    }

    /// Converts to event receiver ID
    pub fn parse_event_receiver_id(&self) -> Result<EventReceiverId, DomainError> {
        EventReceiverId::parse(&self.event_receiver_id).map_err(|_| DomainError::ValidationError {
            field: "event_receiver_id".to_string(),
            message: "Invalid event receiver ID format".to_string(),
        })
    }
}

/// Response DTO for event creation
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEventResponse {
    pub data: String, // ULID as string
}

/// Request DTO for creating an event receiver group
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateEventReceiverGroupRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub event_receiver_ids: Vec<String>, // ULIDs as strings
}

impl CreateEventReceiverGroupRequest {
    /// Validates the request data
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "name".to_string(),
                message: "Name cannot be empty".to_string(),
            });
        }

        if self.group_type.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "type".to_string(),
                message: "Type cannot be empty".to_string(),
            });
        }

        if self.version.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "version".to_string(),
                message: "Version cannot be empty".to_string(),
            });
        }

        if self.event_receiver_ids.is_empty() {
            return Err(DomainError::ValidationError {
                field: "event_receiver_ids".to_string(),
                message: "At least one event receiver ID is required".to_string(),
            });
        }

        Ok(())
    }

    /// Converts event receiver ID strings to EventReceiverId values
    pub fn parse_event_receiver_ids(&self) -> Result<Vec<EventReceiverId>, DomainError> {
        self.event_receiver_ids
            .iter()
            .map(|id_str| {
                EventReceiverId::parse(id_str).map_err(|_| DomainError::ValidationError {
                    field: "event_receiver_ids".to_string(),
                    message: format!("Invalid event receiver ID format: {}", id_str),
                })
            })
            .collect()
    }
}

/// Response DTO for event receiver group creation
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEventReceiverGroupResponse {
    pub data: String, // ULID as string
}

/// Response DTO for event receiver details
#[derive(Debug, Serialize, Deserialize)]
pub struct EventReceiverResponse {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub receiver_type: String,
    pub version: String,
    pub description: String,
    pub schema: JsonValue,
    pub fingerprint: String,
    pub created_at: DateTime<Utc>,
}

impl From<EventReceiver> for EventReceiverResponse {
    fn from(receiver: EventReceiver) -> Self {
        Self {
            id: receiver.id().to_string(),
            name: receiver.name().to_string(),
            receiver_type: receiver.receiver_type().to_string(),
            version: receiver.version().to_string(),
            description: receiver.description().to_string(),
            schema: receiver.schema().clone(),
            fingerprint: receiver.fingerprint().to_string(),
            created_at: receiver.created_at(),
        }
    }
}

/// Response DTO for event details
#[derive(Debug, Serialize, Deserialize)]
pub struct EventResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub release: String,
    pub platform_id: String,
    pub package: String,
    pub description: String,
    pub payload: JsonValue,
    pub success: bool,
    pub event_receiver_id: String,
    pub created_at: DateTime<Utc>,
}

impl From<Event> for EventResponse {
    fn from(event: Event) -> Self {
        Self {
            id: event.id().to_string(),
            name: event.name().to_string(),
            version: event.version().to_string(),
            release: event.release().to_string(),
            platform_id: event.platform_id().to_string(),
            package: event.package().to_string(),
            description: event.description().to_string(),
            payload: event.payload().clone(),
            success: event.success(),
            event_receiver_id: event.event_receiver_id().to_string(),
            created_at: event.created_at(),
        }
    }
}

/// Response DTO for event receiver group details
#[derive(Debug, Serialize, Deserialize)]
pub struct EventReceiverGroupResponse {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub event_receiver_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<EventReceiverGroup> for EventReceiverGroupResponse {
    fn from(group: EventReceiverGroup) -> Self {
        Self {
            id: group.id().to_string(),
            name: group.name().to_string(),
            group_type: group.group_type().to_string(),
            version: group.version().to_string(),
            description: group.description().to_string(),
            enabled: group.enabled(),
            event_receiver_ids: group
                .event_receiver_ids()
                .iter()
                .map(|id| id.to_string())
                .collect(),
            created_at: group.created_at(),
            updated_at: group.updated_at(),
        }
    }
}

/// Generic error response DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl ErrorResponse {
    pub fn new(error: String, message: String) -> Self {
        Self {
            error,
            message,
            field: None,
        }
    }

    pub fn with_field(error: String, message: String, field: String) -> Self {
        Self {
            error,
            message,
            field: Some(field),
        }
    }
}

/// Request DTO for updating an event receiver
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateEventReceiverRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub receiver_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<JsonValue>,
}

impl UpdateEventReceiverRequest {
    /// Validates the request data
    pub fn validate(&self) -> Result<(), DomainError> {
        if let Some(ref name) = self.name {
            if name.trim().is_empty() {
                return Err(DomainError::ValidationError {
                    field: "name".to_string(),
                    message: "Name cannot be empty".to_string(),
                });
            }
        }

        if let Some(ref receiver_type) = self.receiver_type {
            if receiver_type.trim().is_empty() {
                return Err(DomainError::ValidationError {
                    field: "type".to_string(),
                    message: "Type cannot be empty".to_string(),
                });
            }
        }

        if let Some(ref version) = self.version {
            if version.trim().is_empty() {
                return Err(DomainError::ValidationError {
                    field: "version".to_string(),
                    message: "Version cannot be empty".to_string(),
                });
            }
        }

        if let Some(ref schema) = self.schema {
            if !schema.is_object() {
                return Err(DomainError::ValidationError {
                    field: "schema".to_string(),
                    message: "Schema must be a JSON object".to_string(),
                });
            }
        }

        Ok(())
    }
}

/// Request DTO for updating an event receiver group
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateEventReceiverGroupRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub group_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_receiver_ids: Option<Vec<String>>,
}

impl UpdateEventReceiverGroupRequest {
    /// Validates the request data
    pub fn validate(&self) -> Result<(), DomainError> {
        if let Some(ref name) = self.name {
            if name.trim().is_empty() {
                return Err(DomainError::ValidationError {
                    field: "name".to_string(),
                    message: "Name cannot be empty".to_string(),
                });
            }
        }

        if let Some(ref group_type) = self.group_type {
            if group_type.trim().is_empty() {
                return Err(DomainError::ValidationError {
                    field: "type".to_string(),
                    message: "Type cannot be empty".to_string(),
                });
            }
        }

        if let Some(ref version) = self.version {
            if version.trim().is_empty() {
                return Err(DomainError::ValidationError {
                    field: "version".to_string(),
                    message: "Version cannot be empty".to_string(),
                });
            }
        }

        if let Some(ref receiver_ids) = self.event_receiver_ids {
            if receiver_ids.is_empty() {
                return Err(DomainError::ValidationError {
                    field: "event_receiver_ids".to_string(),
                    message: "At least one event receiver ID is required".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Converts event receiver ID strings to EventReceiverId values
    pub fn parse_event_receiver_ids(&self) -> Result<Option<Vec<EventReceiverId>>, DomainError> {
        if let Some(ref ids) = self.event_receiver_ids {
            let parsed_ids: Result<Vec<_>, _> = ids
                .iter()
                .map(|id_str| {
                    EventReceiverId::parse(id_str).map_err(|_| DomainError::ValidationError {
                        field: "event_receiver_ids".to_string(),
                        message: format!("Invalid event receiver ID format: {}", id_str),
                    })
                })
                .collect();
            Ok(Some(parsed_ids?))
        } else {
            Ok(None)
        }
    }
}

/// List query parameters for event receivers
#[derive(Debug, Deserialize)]
pub struct EventReceiverQueryParams {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub receiver_type: Option<String>,
    pub version: Option<String>,
}

fn default_limit() -> usize {
    50
}

impl EventReceiverQueryParams {
    /// Validates query parameters
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.limit == 0 || self.limit > 1000 {
            return Err(DomainError::ValidationError {
                field: "limit".to_string(),
                message: "Limit must be between 1 and 1000".to_string(),
            });
        }

        Ok(())
    }
}

/// Paginated response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

/// Pagination metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub limit: usize,
    pub offset: usize,
    pub total: usize,
    pub has_more: bool,
}

impl PaginationMeta {
    pub fn new(limit: usize, offset: usize, total: usize) -> Self {
        let has_more = offset + limit < total;
        Self {
            limit,
            offset,
            total,
            has_more,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_event_receiver_request_validation() {
        let valid_request = CreateEventReceiverRequest {
            name: "Test Receiver".to_string(),
            receiver_type: "webhook".to_string(),
            version: "1.0.0".to_string(),
            description: "A test receiver".to_string(),
            schema: json!({"type": "object"}),
        };
        assert!(valid_request.validate().is_ok());

        // Empty schema is valid - allows free-form payloads
        let empty_schema_request = CreateEventReceiverRequest {
            name: "Test Receiver".to_string(),
            receiver_type: "webhook".to_string(),
            version: "1.0.0".to_string(),
            description: "A test receiver with no schema constraints".to_string(),
            schema: json!({}),
        };
        assert!(empty_schema_request.validate().is_ok());

        let invalid_request = CreateEventReceiverRequest {
            name: "".to_string(), // Empty name
            receiver_type: "webhook".to_string(),
            version: "1.0.0".to_string(),
            description: "A test receiver".to_string(),
            schema: json!({"type": "object"}),
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_create_event_request_validation() {
        let valid_request = CreateEventRequest {
            name: "test-event".to_string(),
            version: "1.0.0".to_string(),
            release: "2023.11.16".to_string(),
            platform_id: "linux".to_string(),
            package: "docker".to_string(),
            description: "Test event".to_string(),
            payload: json!({"test": "data"}),
            success: true,
            event_receiver_id: EventReceiverId::new().to_string(),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateEventRequest {
            name: "".to_string(), // Empty name
            version: "1.0.0".to_string(),
            release: "2023.11.16".to_string(),
            platform_id: "linux".to_string(),
            package: "docker".to_string(),
            description: "Test event".to_string(),
            payload: json!({"test": "data"}),
            success: true,
            event_receiver_id: EventReceiverId::new().to_string(),
        };
        assert!(invalid_request.validate().is_err());
    }

    #[test]
    fn test_query_params_validation() {
        let valid_params = EventReceiverQueryParams {
            limit: 10,
            offset: 0,
            name: None,
            receiver_type: None,
            version: None,
        };
        assert!(valid_params.validate().is_ok());

        let invalid_params = EventReceiverQueryParams {
            limit: 0, // Invalid limit
            offset: 0,
            name: None,
            receiver_type: None,
            version: None,
        };
        assert!(invalid_params.validate().is_err());
    }

    #[test]
    fn test_pagination_meta() {
        let meta = PaginationMeta::new(10, 0, 100);
        assert_eq!(meta.limit, 10);
        assert_eq!(meta.offset, 0);
        assert_eq!(meta.total, 100);
        assert!(meta.has_more);

        let meta_last_page = PaginationMeta::new(10, 90, 100);
        assert!(!meta_last_page.has_more);
    }

    #[test]
    fn test_event_receiver_id_parsing() {
        let receiver_id = EventReceiverId::new();
        let request = CreateEventRequest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            release: "2023.11.16".to_string(),
            platform_id: "linux".to_string(),
            package: "docker".to_string(),
            description: "Test".to_string(),
            payload: json!({}),
            success: true,
            event_receiver_id: receiver_id.to_string(),
        };

        let parsed_id = request.parse_event_receiver_id().unwrap();
        assert_eq!(parsed_id, receiver_id);

        let invalid_request = CreateEventRequest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            release: "2023.11.16".to_string(),
            platform_id: "linux".to_string(),
            package: "docker".to_string(),
            description: "Test".to_string(),
            payload: json!({}),
            success: true,
            event_receiver_id: "invalid-id".to_string(),
        };

        assert!(invalid_request.parse_event_receiver_id().is_err());
    }
}
