// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/messaging/cloudevents.rs

use serde::{Deserialize, Serialize};

use crate::domain::entities::event::Event;
use crate::domain::entities::event_receiver::EventReceiver;
use crate::domain::entities::event_receiver_group::EventReceiverGroup;

/// CloudEvents 1.0.1 compatible message structure for Kafka publication
///
/// This structure ensures compatibility with other systems that expect
/// CloudEvents format with custom extensions.
///
/// Fields marked as "Extension" are custom additions to the CloudEvents spec.
/// Fields marked as "CloudEvents Spec 1.0.1" are standard CloudEvents fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEventMessage {
    /// Extension to Cloud Events Spec - indicates if the event represents success
    pub success: bool,

    /// Cloud Events Spec 1.0.1 - unique identifier for the event
    pub id: String,

    /// Cloud Events Spec 1.0.1 - version of the CloudEvents specification
    pub specversion: String,

    /// Cloud Events Spec 1.0.1 - type of the event
    #[serde(rename = "type")]
    pub event_type: String,

    /// Cloud Events Spec 1.0.1 - source of the event
    pub source: String,

    /// Extension to Cloud Events Spec - API version
    pub api_version: String,

    /// Extension to Cloud Events Spec - event name
    pub name: String,

    /// Extension to Cloud Events Spec - event version
    pub version: String,

    /// Extension to Cloud Events Spec - release identifier
    pub release: String,

    /// Extension to Cloud Events Spec - platform identifier
    pub platform_id: String,

    /// Extension to Cloud Events Spec - package name
    pub package: String,

    /// Cloud Events Spec 1.0.1 - event payload
    pub data: CloudEventData,
}

/// Data payload for CloudEvents messages
///
/// Contains arrays of events, event receivers, and event receiver groups
/// that triggered this CloudEvent message. Compatible with Go Data struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEventData {
    /// Events that are part of this message
    pub events: Vec<Event>,

    /// Event receivers that are part of this message
    pub event_receivers: Vec<EventReceiver>,

    /// Event receiver groups that are part of this message
    pub event_receiver_groups: Vec<EventReceiverGroup>,
}

impl CloudEventMessage {
    /// Creates a CloudEvents-compatible message from an Event domain entity
    ///
    /// # Arguments
    ///
    /// * `event` - The domain event to convert
    ///
    /// # Returns
    ///
    /// Returns a CloudEventMessage ready for Kafka publication
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xzepr::domain::entities::event::Event;
    /// use xzepr::infrastructure::messaging::cloudevents::CloudEventMessage;
    ///
    /// # fn example(event: Event) {
    /// let message = CloudEventMessage::from_event(&event);
    /// # }
    /// ```
    pub fn from_event(event: &Event) -> Self {
        Self {
            success: event.success(),
            id: event.id().to_string(),
            specversion: "1.0.1".to_string(),
            event_type: event.name().to_string(),
            source: format!("xzepr.event.receiver.{}", event.event_receiver_id()),
            api_version: "v1".to_string(),
            name: event.name().to_string(),
            version: event.version().to_string(),
            release: event.release().to_string(),
            platform_id: event.platform_id().to_string(),
            package: event.package().to_string(),
            data: CloudEventData {
                events: vec![event.clone()],
                event_receivers: vec![],
                event_receiver_groups: vec![],
            },
        }
    }

    /// Creates a CloudEvents-compatible message for an event receiver creation
    ///
    /// # Arguments
    ///
    /// * `event` - The system event
    /// * `receiver` - The event receiver that was created
    ///
    /// # Returns
    ///
    /// Returns a CloudEventMessage with the receiver in the data
    pub fn from_event_with_receiver(event: &Event, receiver: &EventReceiver) -> Self {
        Self {
            success: event.success(),
            id: event.id().to_string(),
            specversion: "1.0.1".to_string(),
            event_type: event.name().to_string(),
            source: format!("xzepr.event.receiver.{}", event.event_receiver_id()),
            api_version: "v1".to_string(),
            name: event.name().to_string(),
            version: event.version().to_string(),
            release: event.release().to_string(),
            platform_id: event.platform_id().to_string(),
            package: event.package().to_string(),
            data: CloudEventData {
                events: vec![event.clone()],
                event_receivers: vec![receiver.clone()],
                event_receiver_groups: vec![],
            },
        }
    }

    /// Creates a CloudEvents-compatible message for an event receiver group creation
    ///
    /// # Arguments
    ///
    /// * `event` - The system event
    /// * `group` - The event receiver group that was created
    ///
    /// # Returns
    ///
    /// Returns a CloudEventMessage with the group in the data
    pub fn from_event_with_group(event: &Event, group: &EventReceiverGroup) -> Self {
        Self {
            success: event.success(),
            id: event.id().to_string(),
            specversion: "1.0.1".to_string(),
            event_type: event.name().to_string(),
            source: format!("xzepr.event.receiver.{}", event.event_receiver_id()),
            api_version: "v1".to_string(),
            name: event.name().to_string(),
            version: event.version().to_string(),
            release: event.release().to_string(),
            platform_id: event.platform_id().to_string(),
            package: event.package().to_string(),
            data: CloudEventData {
                events: vec![event.clone()],
                event_receivers: vec![],
                event_receiver_groups: vec![group.clone()],
            },
        }
    }

    /// Serializes the CloudEvent message to JSON string
    ///
    /// # Returns
    ///
    /// Returns Result with JSON string or serialization error
    ///
    /// # Errors
    ///
    /// Returns error if the message cannot be serialized to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serializes the CloudEvent message to pretty-printed JSON string
    ///
    /// # Returns
    ///
    /// Returns Result with pretty-printed JSON string or serialization error
    ///
    /// # Errors
    ///
    /// Returns error if the message cannot be serialized to JSON
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::event::CreateEventParams;
    use crate::domain::value_objects::{EventReceiverId, UserId};

    #[test]
    fn test_cloudevent_message_from_event() {
        let receiver_id = EventReceiverId::new();
        let payload = serde_json::json!({
            "key": "value",
            "number": 42
        });

        let event = Event::new(CreateEventParams {
            name: "test.event".to_string(),
            version: "1.0.0".to_string(),
            release: "1.0.0-rc.1".to_string(),
            platform_id: "kubernetes".to_string(),
            package: "test-package".to_string(),
            description: "Test event description".to_string(),
            payload: payload.clone(),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let message = CloudEventMessage::from_event(&event);

        assert!(message.success);
        assert_eq!(message.id, event.id().to_string());
        assert_eq!(message.specversion, "1.0.1");
        assert_eq!(message.event_type, "test.event");
        assert_eq!(
            message.source,
            format!("xzepr.event.receiver.{}", receiver_id)
        );
        assert_eq!(message.api_version, "v1");
        assert_eq!(message.name, "test.event");
        assert_eq!(message.version, "1.0.0");
        assert_eq!(message.release, "1.0.0-rc.1");
        assert_eq!(message.platform_id, "kubernetes");
        assert_eq!(message.package, "test-package");
        assert_eq!(message.data.events.len(), 1);
        assert_eq!(message.data.event_receivers.len(), 0);
        assert_eq!(message.data.event_receiver_groups.len(), 0);
        assert_eq!(message.data.events[0].id(), event.id());
        assert_eq!(message.data.events[0].name(), event.name());
    }

    #[test]
    fn test_cloudevent_message_serialization() {
        let receiver_id = EventReceiverId::new();
        let event = Event::new(CreateEventParams {
            name: "deployment.success".to_string(),
            version: "2.1.0".to_string(),
            release: "2.1.0".to_string(),
            platform_id: "aws".to_string(),
            package: "myapp".to_string(),
            description: "Deployment successful".to_string(),
            payload: serde_json::json!({"status": "deployed"}),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let message = CloudEventMessage::from_event(&event);
        let json = message.to_json().unwrap();

        // Verify JSON can be parsed back
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["specversion"], "1.0.1");
        assert_eq!(parsed["type"], "deployment.success");
        assert_eq!(parsed["api_version"], "v1");
        assert_eq!(parsed["name"], "deployment.success");
        assert_eq!(parsed["version"], "2.1.0");
        assert_eq!(parsed["release"], "2.1.0");
        assert_eq!(parsed["platform_id"], "aws");
        assert_eq!(parsed["package"], "myapp");
        assert!(parsed["data"]["events"].is_array());
        assert!(parsed["data"]["event_receivers"].is_array());
        assert!(parsed["data"]["event_receiver_groups"].is_array());
        assert_eq!(parsed["data"]["events"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_cloudevent_message_with_system_event() {
        let receiver_id = EventReceiverId::new();
        let event = Event::new(CreateEventParams {
            name: "xzepr.event.receiver.created".to_string(),
            version: "1.0.0".to_string(),
            release: "system".to_string(),
            platform_id: "xzepr".to_string(),
            package: "xzepr.system".to_string(),
            description: "Event receiver created".to_string(),
            payload: serde_json::json!({
                "receiver_id": receiver_id.to_string(),
                "name": "test-receiver"
            }),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let message = CloudEventMessage::from_event(&event);

        assert_eq!(message.event_type, "xzepr.event.receiver.created");
        assert_eq!(message.release, "system");
        assert_eq!(message.platform_id, "xzepr");
        assert_eq!(message.package, "xzepr.system");
    }

    #[test]
    fn test_cloudevent_message_with_failed_event() {
        let receiver_id = EventReceiverId::new();
        let event = Event::new(CreateEventParams {
            name: "test.failed".to_string(),
            version: "1.0.0".to_string(),
            release: "1.0.0".to_string(),
            platform_id: "test".to_string(),
            package: "test-pkg".to_string(),
            description: "Test failed event".to_string(),
            payload: serde_json::json!({"error": "Something went wrong"}),
            success: false,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let message = CloudEventMessage::from_event(&event);

        assert!(!message.success);
        assert_eq!(message.event_type, "test.failed");
    }

    #[test]
    fn test_cloudevent_data_structure() {
        let receiver_id = EventReceiverId::new();
        let payload = serde_json::json!({
            "nested": {
                "field": "value"
            }
        });

        let event = Event::new(CreateEventParams {
            name: "test.event".to_string(),
            version: "1.0.0".to_string(),
            release: "1.0.0".to_string(),
            platform_id: "test".to_string(),
            package: "test-pkg".to_string(),
            description: "Test description".to_string(),
            payload: payload.clone(),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let message = CloudEventMessage::from_event(&event);

        // Verify data structure
        assert_eq!(message.data.events.len(), 1);
        assert_eq!(message.data.event_receivers.len(), 0);
        assert_eq!(message.data.event_receiver_groups.len(), 0);
        assert_eq!(message.data.events[0].id(), event.id());
    }

    #[test]
    fn test_cloudevent_json_field_names() {
        let receiver_id = EventReceiverId::new();
        let event = Event::new(CreateEventParams {
            name: "test.event".to_string(),
            version: "1.0.0".to_string(),
            release: "1.0.0-rc.1".to_string(),
            platform_id: "kubernetes".to_string(),
            package: "test-package".to_string(),
            description: "Test event".to_string(),
            payload: serde_json::json!({}),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let message = CloudEventMessage::from_event(&event);
        let json = message.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify exact JSON field names match Go struct tags
        assert!(parsed.get("success").is_some());
        assert!(parsed.get("id").is_some());
        assert!(parsed.get("specversion").is_some());
        assert!(parsed.get("type").is_some());
        assert!(parsed.get("source").is_some());
        assert!(parsed.get("api_version").is_some());
        assert!(parsed.get("name").is_some());
        assert!(parsed.get("version").is_some());
        assert!(parsed.get("release").is_some());
        assert!(parsed.get("platform_id").is_some());
        assert!(parsed.get("package").is_some());
        assert!(parsed.get("data").is_some());

        // Verify data has correct array fields
        let data = parsed.get("data").unwrap();
        assert!(data.get("events").is_some());
        assert!(data.get("event_receivers").is_some());
        assert!(data.get("event_receiver_groups").is_some());
    }

    #[test]
    fn test_cloudevent_with_receiver() {
        use crate::domain::value_objects::EventReceiverId;

        let receiver_id = EventReceiverId::new();
        let event = Event::new(CreateEventParams {
            name: "xzepr.event.receiver.created".to_string(),
            version: "1.0.0".to_string(),
            release: "system".to_string(),
            platform_id: "xzepr".to_string(),
            package: "xzepr.system".to_string(),
            description: "Receiver created".to_string(),
            payload: serde_json::json!({}),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let receiver = EventReceiver::new(
            "test-receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "Test receiver".to_string(),
            serde_json::json!({"type": "object"}),
            UserId::new(),
        )
        .unwrap();

        let message = CloudEventMessage::from_event_with_receiver(&event, &receiver);

        assert_eq!(message.data.events.len(), 1);
        assert_eq!(message.data.event_receivers.len(), 1);
        assert_eq!(message.data.event_receiver_groups.len(), 0);
        assert_eq!(message.data.event_receivers[0].name(), "test-receiver");
    }

    #[test]
    fn test_cloudevent_with_group() {
        use crate::domain::value_objects::EventReceiverId;

        let receiver_id = EventReceiverId::new();
        let event = Event::new(CreateEventParams {
            name: "xzepr.event.receiver.group.created".to_string(),
            version: "1.0.0".to_string(),
            release: "system".to_string(),
            platform_id: "xzepr".to_string(),
            package: "xzepr.system".to_string(),
            description: "Group created".to_string(),
            payload: serde_json::json!({}),
            success: true,
            receiver_id,
            owner_id: UserId::new(),
        })
        .unwrap();

        let group = EventReceiverGroup::new(
            "test-group".to_string(),
            "webhook-cluster".to_string(),
            "1.0.0".to_string(),
            "Test group".to_string(),
            true,
            vec![receiver_id],
            UserId::new(),
        )
        .unwrap();

        let message = CloudEventMessage::from_event_with_group(&event, &group);

        assert_eq!(message.data.events.len(), 1);
        assert_eq!(message.data.event_receivers.len(), 0);
        assert_eq!(message.data.event_receiver_groups.len(), 1);
        assert_eq!(message.data.event_receiver_groups[0].name(), "test-group");
    }
}
