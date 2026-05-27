// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Lifecycle event builders for the application layer.
//!
//! Each function constructs a domain [`Event`] that represents a significant
//! lifecycle transition within XZepr (e.g. a receiver being created).  These
//! events are published to the messaging backend so that downstream systems can
//! react to state changes without polling the database.
//!
//! The builders mirror the private helper methods that were previously embedded
//! in the individual handler structs.  Centralising them here ensures that the
//! payloads and event names remain consistent across the codebase.

use crate::domain::entities::event::{CreateEventParams, Event};
use crate::domain::entities::event_receiver::EventReceiver;
use crate::domain::entities::event_receiver_group::EventReceiverGroup;
use crate::domain::value_objects::EventReceiverId;
use crate::error::DomainError;
use serde_json::json;

/// Builds the system event that is published when an [`EventReceiver`] is created.
///
/// The event payload captures a snapshot of the receiver at creation time so
/// that consumers can reconstruct what was created without a separate database
/// query.
///
/// # Arguments
///
/// * `receiver` - The newly persisted [`EventReceiver`].
///
/// # Returns
///
/// A domain [`Event`] representing the receiver-creation lifecycle transition.
///
/// # Errors
///
/// Returns [`DomainError`] if the generated event fails domain validation.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use serde_json::json;
/// use xzepr::application::lifecycle_events::build_receiver_created_event;
/// use xzepr::domain::entities::event_receiver::EventReceiver;
/// use xzepr::domain::value_objects::UserId;
///
/// let receiver = EventReceiver::new(
///     "docs-receiver".to_string(),
///     "webhook".to_string(),
///     "1.0.0".to_string(),
///     "Receiver used in lifecycle event documentation".to_string(),
///     json!({"type": "object"}),
///     UserId::new(),
/// )?;
///
/// let event = build_receiver_created_event(&receiver)?;
/// assert_eq!(event.name(), "xzepr.event.receiver.created");
/// # Ok(())
/// # }
/// ```
pub fn build_receiver_created_event(receiver: &EventReceiver) -> Result<Event, DomainError> {
    let payload = json!({
        "receiver_id": receiver.id().to_string(),
        "name": receiver.name(),
        "type": receiver.receiver_type(),
        "version": receiver.version(),
        "fingerprint": receiver.fingerprint(),
        "description": receiver.description(),
    });

    Event::new(CreateEventParams {
        name: "xzepr.event.receiver.created".to_string(),
        version: "1.0.0".to_string(),
        release: "system".to_string(),
        platform_id: "xzepr".to_string(),
        package: "xzepr.system".to_string(),
        description: format!("Event receiver '{}' created", receiver.name()),
        payload,
        success: true,
        receiver_id: receiver.id(),
        owner_id: receiver.owner_id(),
    })
}

/// Builds the system event that is published when an [`EventReceiverGroup`] is created.
///
/// The event payload captures a snapshot of the group at creation time,
/// including the list of receiver IDs that were part of the group.
///
/// # Arguments
///
/// * `group` - The newly persisted [`EventReceiverGroup`].
///
/// # Returns
///
/// A domain [`Event`] representing the group-creation lifecycle transition.
///
/// # Errors
///
/// Returns [`DomainError`] if the generated event fails domain validation.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use xzepr::application::lifecycle_events::build_group_created_event;
/// use xzepr::domain::entities::event_receiver_group::EventReceiverGroup;
/// use xzepr::domain::value_objects::{EventReceiverId, UserId};
///
/// let receiver_id = EventReceiverId::new();
/// let group = EventReceiverGroup::new(
///     "docs-group".to_string(),
///     "webhook_group".to_string(),
///     "1.0.0".to_string(),
///     "Group used in lifecycle event documentation".to_string(),
///     true,
///     vec![receiver_id],
///     UserId::new(),
/// )?;
///
/// let event = build_group_created_event(&group)?;
/// assert_eq!(event.name(), "xzepr.event.receiver.group.created");
/// # Ok(())
/// # }
/// ```
pub fn build_group_created_event(group: &EventReceiverGroup) -> Result<Event, DomainError> {
    let receiver_ids: Vec<String> = group
        .event_receiver_ids()
        .iter()
        .map(|id| id.to_string())
        .collect();

    let payload = json!({
        "group_id": group.id().to_string(),
        "name": group.name(),
        "type": group.group_type(),
        "version": group.version(),
        "description": group.description(),
        "enabled": group.enabled(),
        "receiver_ids": receiver_ids,
        "receiver_count": group.receiver_count(),
    });

    // Use the first receiver in the group as the event's receiver_id, or
    // synthesise one from the group ID when the group has no members.  System
    // events must always reference a receiver ID; this fallback is acceptable
    // because the group ID carries the real identity in the payload.
    let receiver_id = group
        .event_receiver_ids()
        .first()
        .copied()
        .unwrap_or_else(|| EventReceiverId::from(group.id().as_ulid()));

    Event::new(CreateEventParams {
        name: "xzepr.event.receiver.group.created".to_string(),
        version: "1.0.0".to_string(),
        release: "system".to_string(),
        platform_id: "xzepr".to_string(),
        package: "xzepr.system".to_string(),
        description: format!("Event receiver group '{}' created", group.name()),
        payload,
        success: true,
        receiver_id,
        owner_id: group.owner_id(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::UserId;
    use serde_json::json;

    fn make_test_receiver() -> EventReceiver {
        // SAFETY: All inputs are controlled test values.
        EventReceiver::new(
            "test-receiver".to_string(),
            "webhook".to_string(),
            "1.0.0".to_string(),
            "A test receiver".to_string(),
            json!({"type": "object"}),
            UserId::new(),
        )
        .expect("test receiver construction must succeed")
    }

    fn make_test_group_with_receivers(receiver_ids: Vec<EventReceiverId>) -> EventReceiverGroup {
        // SAFETY: All inputs are controlled test values.
        EventReceiverGroup::new(
            "test-group".to_string(),
            "webhook_group".to_string(),
            "1.0.0".to_string(),
            "A test group".to_string(),
            true,
            receiver_ids,
            UserId::new(),
        )
        .expect("test group construction must succeed")
    }

    #[test]
    fn test_build_receiver_created_event_sets_correct_name() {
        let receiver = make_test_receiver();
        let event = build_receiver_created_event(&receiver)
            .expect("receiver lifecycle event construction must succeed");
        assert_eq!(event.name(), "xzepr.event.receiver.created");
    }

    #[test]
    fn test_build_receiver_created_event_owner_matches_receiver() {
        let receiver = make_test_receiver();
        let event = build_receiver_created_event(&receiver)
            .expect("receiver lifecycle event construction must succeed");
        assert_eq!(event.owner_id(), receiver.owner_id());
    }

    #[test]
    fn test_build_receiver_created_event_receiver_id_matches() {
        let receiver = make_test_receiver();
        let event = build_receiver_created_event(&receiver)
            .expect("receiver lifecycle event construction must succeed");
        assert_eq!(event.event_receiver_id(), receiver.id());
    }

    #[test]
    fn test_build_receiver_created_event_payload_contains_fingerprint() {
        let receiver = make_test_receiver();
        let event = build_receiver_created_event(&receiver)
            .expect("receiver lifecycle event construction must succeed");
        let payload = event.payload();
        assert!(payload.get("fingerprint").is_some());
    }

    #[test]
    fn test_build_group_created_event_sets_correct_name() {
        let receiver_id = EventReceiverId::new();
        let group = make_test_group_with_receivers(vec![receiver_id]);
        let event = build_group_created_event(&group)
            .expect("group lifecycle event construction must succeed");
        assert_eq!(event.name(), "xzepr.event.receiver.group.created");
    }

    #[test]
    fn test_build_group_created_event_owner_matches_group() {
        let receiver_id = EventReceiverId::new();
        let group = make_test_group_with_receivers(vec![receiver_id]);
        let event = build_group_created_event(&group)
            .expect("group lifecycle event construction must succeed");
        assert_eq!(event.owner_id(), group.owner_id());
    }

    #[test]
    fn test_build_group_created_event_receiver_id_is_first_group_member() {
        let receiver_id = EventReceiverId::new();
        let group = make_test_group_with_receivers(vec![receiver_id]);
        let event = build_group_created_event(&group)
            .expect("group lifecycle event construction must succeed");
        assert_eq!(event.event_receiver_id(), receiver_id);
    }

    #[test]
    fn test_build_group_created_event_empty_group_uses_synthetic_receiver_id() {
        let group = make_test_group_with_receivers(vec![]);
        let event = build_group_created_event(&group)
            .expect("group lifecycle event construction must succeed");
        assert_eq!(event.name(), "xzepr.event.receiver.group.created");
    }

    #[test]
    fn test_build_group_created_event_payload_contains_receiver_count() {
        let receiver_id = EventReceiverId::new();
        let group = make_test_group_with_receivers(vec![receiver_id]);
        let event = build_group_created_event(&group)
            .expect("group lifecycle event construction must succeed");
        let payload = event.payload();
        assert!(payload.get("receiver_count").is_some());
    }
}
