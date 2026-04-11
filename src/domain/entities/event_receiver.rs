// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/domain/entities/event_receiver.rs

use crate::domain::value_objects::{EventReceiverId, UserId};
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
    pub owner_id: UserId,
    pub resource_version: i64,
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
    owner_id: UserId,
    resource_version: i64,
    created_at: DateTime<Utc>,
}

impl EventReceiver {
    /// Creates a new event receiver with validation
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the event receiver
    /// * `receiver_type` - The type of the event receiver
    /// * `version` - The version of the event receiver
    /// * `description` - A description of the event receiver
    /// * `schema` - The JSON schema for event validation
    /// * `owner_id` - The user ID of the owner who created this receiver
    ///
    /// # Returns
    ///
    /// Returns a new EventReceiver or DomainError if validation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::domain::entities::event_receiver::EventReceiver;
    /// use xzepr::domain::value_objects::UserId;
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "object"});
    /// let owner_id = UserId::new();
    /// let receiver = EventReceiver::new(
    ///     "My Receiver".to_string(),
    ///     "webhook".to_string(),
    ///     "1.0.0".to_string(),
    ///     "Test receiver".to_string(),
    ///     schema,
    ///     owner_id,
    /// );
    /// assert!(receiver.is_ok());
    /// ```
    pub fn new(
        name: String,
        receiver_type: String,
        version: String,
        description: String,
        schema: JsonValue,
        owner_id: UserId,
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
            owner_id,
            resource_version: 1,
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
            owner_id: data.owner_id,
            resource_version: data.resource_version,
            created_at: data.created_at,
        })
    }

    /// Updates the event receiver with new data
    ///
    /// Increments the resource_version on any update to support cache invalidation.
    ///
    /// # Arguments
    ///
    /// * `name` - Optional new name
    /// * `receiver_type` - Optional new type
    /// * `version` - Optional new version
    /// * `description` - Optional new description
    /// * `schema` - Optional new schema
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if validation passes, otherwise DomainError
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
            self.resource_version += 1;
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

    /// Returns the owner user ID of this event receiver
    pub fn owner_id(&self) -> UserId {
        self.owner_id
    }

    /// Returns the current resource version for cache invalidation
    ///
    /// This version is incremented on every update and used by the
    /// authorization cache to detect stale entries.
    pub fn resource_version(&self) -> i64 {
        self.resource_version
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
        let owner_id = crate::domain::value_objects::UserId::new();
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
            owner_id,
        );

        assert!(receiver.is_ok());
        let receiver = receiver.unwrap();
        assert_eq!(receiver.name(), "Test Receiver");
        assert_eq!(receiver.receiver_type(), "webhook");
        assert_eq!(receiver.version(), "1.0.0");
        assert!(!receiver.fingerprint().is_empty());
        assert_eq!(receiver.owner_id(), owner_id);
        assert_eq!(receiver.resource_version(), 1);
    }

    #[test]
    fn test_validate_empty_name() {
        let schema = create_valid_schema();
        let owner_id = crate::domain::value_objects::UserId::new();
        let result = EventReceiver::new(
            "".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
            owner_id,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "name");
        }
    }

    #[test]
    fn test_validate_invalid_schema() {
        let invalid_schema = json!("not an object");
        let owner_id = crate::domain::value_objects::UserId::new();
        let result = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            invalid_schema,
            owner_id,
        );

        assert!(result.is_err());
        if let Err(DomainError::ValidationError { field, .. }) = result {
            assert_eq!(field, "schema");
        }
    }

    #[test]
    fn test_fingerprint_consistency() {
        let schema = create_valid_schema();
        let owner_id = crate::domain::value_objects::UserId::new();
        let receiver1 = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema.clone(),
            owner_id,
        )
        .unwrap();

        let receiver2 = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "Different description".to_string(), // Description doesn't affect fingerprint
            schema,
            owner_id,
        )
        .unwrap();

        assert_eq!(receiver1.fingerprint(), receiver2.fingerprint());
    }

    #[test]
    fn test_update_receiver() {
        let schema = create_valid_schema();
        let owner_id = crate::domain::value_objects::UserId::new();
        let mut receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
            owner_id,
        )
        .unwrap();

        let original_fingerprint = receiver.fingerprint().to_string();
        let original_version = receiver.resource_version();

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
        assert_eq!(receiver.resource_version(), original_version); // Version not incremented for description-only change

        // Update name (should change fingerprint)
        receiver
            .update(Some("Updated Receiver".to_string()), None, None, None, None)
            .unwrap();

        assert_eq!(receiver.name(), "Updated Receiver");
        assert_ne!(receiver.fingerprint(), original_fingerprint);
        assert_eq!(receiver.resource_version(), original_version + 1); // Version incremented
    }

    #[test]
    fn test_validate_event_payload() {
        let schema = create_valid_schema();
        let owner_id = crate::domain::value_objects::UserId::new();
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
            owner_id,
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
        let owner_id = crate::domain::value_objects::UserId::new();
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver with no schema constraints".to_string(),
            empty_schema,
            owner_id,
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
        let owner_id = crate::domain::value_objects::UserId::new();
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
            owner_id,
        );

        assert!(receiver.is_ok());
    }

    #[test]
    fn test_resource_version_increments_on_update() {
        let schema = create_valid_schema();
        let owner_id = crate::domain::value_objects::UserId::new();
        let mut receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
            owner_id,
        )
        .unwrap();

        assert_eq!(receiver.resource_version(), 1);

        // Update name (should increment version)
        receiver
            .update(Some("Updated Name".to_string()), None, None, None, None)
            .unwrap();
        assert_eq!(receiver.resource_version(), 2);

        // Update type (should increment version)
        receiver
            .update(None, Some("webhook_v2".to_string()), None, None, None)
            .unwrap();
        assert_eq!(receiver.resource_version(), 3);
    }

    #[test]
    fn test_owner_id_is_preserved() {
        let schema = create_valid_schema();
        let owner_id = crate::domain::value_objects::UserId::new();
        let receiver = EventReceiver::new(
            "Test Receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test event receiver".to_string(),
            schema,
            owner_id,
        )
        .unwrap();

        assert_eq!(receiver.owner_id(), owner_id);

        // Owner ID should not change on updates
        let mut receiver_copy = receiver.clone();
        receiver_copy
            .update(Some("New Name".to_string()), None, None, None, None)
            .unwrap();
        assert_eq!(receiver_copy.owner_id(), owner_id);
    }
}
