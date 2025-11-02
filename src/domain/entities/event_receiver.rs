// src/domain/entities/event_receiver.rs

use crate::domain::value_objects::EventReceiverId;
use crate::error::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

/// Parameters for creating an event receiver from existing data
#[derive(Debug, Clone)]
pub struct EventReceiverData {
    pub id: EventReceiverId,
    pub name: String,
    pub receiver_type: String,
    pub version: String,
    pub description: String,
    pub schema: JsonValue,
    pub fingerprint: String,
    pub created_at: DateTime<Utc>,
}

/// Event receiver entity representing a destination for events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventReceiver {
    id: EventReceiverId,
    name: String,
    receiver_type: String,
    version: String,
    description: String,
    schema: JsonValue,
    fingerprint: String,
    created_at: DateTime<Utc>,
}

impl EventReceiver {
    /// Creates a new event receiver with validation
    pub fn new(
        name: String,
        receiver_type: String,
        version: String,
        description: String,
        schema: JsonValue,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&name)?;
        Self::validate_type(&receiver_type)?;
        Self::validate_version(&version)?;
        Self::validate_description(&description)?;
        Self::validate_schema(&schema)?;

        let fingerprint = Self::generate_fingerprint(&name, &receiver_type, &version, &schema);

        Ok(Self {
            id: EventReceiverId::new(),
            name,
            receiver_type,
            version,
            description,
            schema,
            fingerprint,
            created_at: Utc::now(),
        })
    }

    /// Creates an event receiver from existing data (e.g., from database)
    pub fn from_existing(data: EventReceiverData) -> Result<Self, DomainError> {
        Self::validate_name(&data.name)?;
        Self::validate_type(&data.receiver_type)?;
        Self::validate_version(&data.version)?;
        Self::validate_description(&data.description)?;
        Self::validate_schema(&data.schema)?;

        Ok(Self {
            id: data.id,
            name: data.name,
            receiver_type: data.receiver_type,
            version: data.version,
            description: data.description,
            schema: data.schema,
            fingerprint: data.fingerprint,
            created_at: data.created_at,
        })
    }

    /// Updates the event receiver with new data
    pub fn update(
        &mut self,
        name: Option<String>,
        receiver_type: Option<String>,
        version: Option<String>,
        description: Option<String>,
        schema: Option<JsonValue>,
    ) -> Result<(), DomainError> {
        let mut updated = false;

        if let Some(new_name) = name {
            Self::validate_name(&new_name)?;
            self.name = new_name;
            updated = true;
        }

        if let Some(new_type) = receiver_type {
            Self::validate_type(&new_type)?;
            self.receiver_type = new_type;
            updated = true;
        }

        if let Some(new_version) = version {
            Self::validate_version(&new_version)?;
            self.version = new_version;
            updated = true;
        }

        if let Some(new_description) = description {
            Self::validate_description(&new_description)?;
            self.description = new_description;
        }

        if let Some(new_schema) = schema {
            Self::validate_schema(&new_schema)?;
            self.schema = new_schema;
            updated = true;
        }

        // Regenerate fingerprint if critical fields changed
        if updated {
            self.fingerprint = Self::generate_fingerprint(
                &self.name,
                &self.receiver_type,
                &self.version,
                &self.schema,
            );
        }

        Ok(())
    }

    /// Generates a unique fingerprint for the event receiver
    fn generate_fingerprint(
        name: &str,
        receiver_type: &str,
        version: &str,
        schema: &JsonValue,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(receiver_type.as_bytes());
        hasher.update(version.as_bytes());
        hasher.update(schema.to_string().as_bytes());

        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Validates event receiver name
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "name".to_string(),
                message: "Event receiver name cannot be empty".to_string(),
            });
        }

        if name.len() > 255 {
            return Err(DomainError::ValidationError {
                field: "name".to_string(),
                message: "Event receiver name cannot exceed 255 characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validates event receiver type
    fn validate_type(receiver_type: &str) -> Result<(), DomainError> {
        if receiver_type.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "type".to_string(),
                message: "Event receiver type cannot be empty".to_string(),
            });
        }

        if receiver_type.len() > 100 {
            return Err(DomainError::ValidationError {
                field: "type".to_string(),
                message: "Event receiver type cannot exceed 100 characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validates version string
    fn validate_version(version: &str) -> Result<(), DomainError> {
        if version.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: "version".to_string(),
                message: "Version cannot be empty".to_string(),
            });
        }

        if version.len() > 50 {
            return Err(DomainError::ValidationError {
                field: "version".to_string(),
                message: "Version cannot exceed 50 characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validates description
    fn validate_description(description: &str) -> Result<(), DomainError> {
        if description.len() > 1000 {
            return Err(DomainError::ValidationError {
                field: "description".to_string(),
                message: "Description cannot exceed 1000 characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validates JSON schema
    fn validate_schema(schema: &JsonValue) -> Result<(), DomainError> {
        // Ensure it's a valid JSON object
        // An empty schema {} is valid and means "accept any valid JSON"
        if !schema.is_object() {
            return Err(DomainError::ValidationError {
                field: "schema".to_string(),
                message: "Schema must be a valid JSON object".to_string(),
            });
        }

        // Empty schema is valid - no further validation needed
        // This allows for free-form event payloads
        Ok(())
    }

    /// Validates an event payload against this receiver's schema
    pub fn validate_event_payload(&self, payload: &JsonValue) -> Result<(), DomainError> {
        // Basic validation - in a real implementation, you'd use a proper JSON schema validator
        if !payload.is_object() {
            return Err(DomainError::ValidationError {
                field: "payload".to_string(),
                message: "Event payload must be a JSON object".to_string(),
            });
        }

        // Additional schema validation could be implemented here
        Ok(())
    }

    // Getters
    pub fn id(&self) -> EventReceiverId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn receiver_type(&self) -> &str {
        &self.receiver_type
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn schema(&self) -> &JsonValue {
        &self.schema
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_valid_schema() -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"},
                "timestamp": {"type": "string", "format": "date-time"}
            },
            "required": ["message"]
        })
    }

    #[test]
    fn test_create_event_receiver() {
        let schema = create_valid_schema();
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
        );

        assert!(receiver.is_ok());
        let receiver = receiver.unwrap();
        assert_eq!(receiver.name(), "Test Receiver");
        assert_eq!(receiver.receiver_type(), "webhook");
        assert_eq!(receiver.version(), "1.0.0");
        assert!(!receiver.fingerprint().is_empty());
    }

    #[test]
    fn test_validate_empty_name() {
        let schema = create_valid_schema();
        let result = EventReceiver::new(
            "".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "name");
        }
    }

    #[test]
    fn test_validate_invalid_schema() {
        let invalid_schema = json!("not an object");
        let result = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            invalid_schema,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "schema");
        }
    }

    #[test]
    fn test_fingerprint_consistency() {
        let schema = create_valid_schema();
        let receiver1 = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema.clone(),
        )
        .unwrap();

        let receiver2 = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "Different description".to_string(), // Description doesn't affect fingerprint
            schema,
        )
        .unwrap();

        assert_eq!(receiver1.fingerprint(), receiver2.fingerprint());
    }

    #[test]
    fn test_update_receiver() {
        let schema = create_valid_schema();
        let mut receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
        )
        .unwrap();

        let original_fingerprint = receiver.fingerprint().to_string();

        // Update description (should not change fingerprint)
        receiver
            .update(
                None,
                None,
                None,
                Some("Updated description".to_string()),
                None,
            )
            .unwrap();

        assert_eq!(receiver.description(), "Updated description");
        assert_eq!(receiver.fingerprint(), original_fingerprint);

        // Update name (should change fingerprint)
        receiver
            .update(Some("Updated Receiver".to_string()), None, None, None, None)
            .unwrap();

        assert_eq!(receiver.name(), "Updated Receiver");
        assert_ne!(receiver.fingerprint(), original_fingerprint);
    }

    #[test]
    fn test_validate_event_payload() {
        let schema = create_valid_schema();
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
        )
        .unwrap();

        let valid_payload = json!({
            "message": "Hello, world!",
            "timestamp": "2023-01-01T00:00:00Z"
        });

        assert!(receiver.validate_event_payload(&valid_payload).is_ok());

        let invalid_payload = json!("not an object");
        assert!(receiver.validate_event_payload(&invalid_payload).is_err());
    }

    #[test]
    fn test_empty_schema_is_valid() {
        // Empty schema {} should be valid - allows free-form event payloads
        let empty_schema = json!({});
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver with no schema constraints".to_string(),
            empty_schema,
        );

        assert!(receiver.is_ok());
        let receiver = receiver.unwrap();
        assert_eq!(receiver.schema(), &json!({}));
    }

    #[test]
    fn test_schema_without_type_field_is_valid() {
        // Schema without "type" field should be valid
        let schema = json!({
            "properties": {
                "any_field": {"type": "string"}
            }
        });
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
        );

        assert!(receiver.is_ok());
    }
}
