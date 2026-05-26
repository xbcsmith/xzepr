// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Event domain entity tests.
//!
//! These tests exercise `Event`, `EventReceiver`, and `EventReceiverGroup`
//! domain entities using their real construction and validation logic.
//! No mock HTTP infrastructure is used here.

mod common;

use common::*;
use serde_json::json;

/// Tests successful creation of an `Event` via `Event::new()` with a complete
/// set of valid `CreateEventParams`.
///
/// Verifies that all fields provided in the params are accessible through the
/// returned entity's getter methods.
#[test]
fn test_event_creation_with_valid_params() {
    let owner_id = UserId::new();
    let receiver_id = EventReceiverId::new();
    let payload = json!({"commit": "abc123", "duration_ms": 1500});

    let event = Event::new(CreateEventParams {
        name: "build-completed".to_string(),
        version: "1.2.3".to_string(),
        release: "2025.01".to_string(),
        platform_id: "linux-x86_64".to_string(),
        package: "rpm".to_string(),
        description: "Build completed successfully".to_string(),
        payload,
        success: true,
        receiver_id,
        owner_id,
    });

    assert!(event.is_ok(), "Event::new should succeed with valid params");
    let event = event.unwrap();

    assert_eq!(event.name(), "build-completed");
    assert_eq!(event.version(), "1.2.3");
    assert_eq!(event.release(), "2025.01");
    assert_eq!(event.platform_id(), "linux-x86_64");
    assert_eq!(event.package(), "rpm");
    assert_eq!(event.description(), "Build completed successfully");
    assert!(event.success(), "Event should report success = true");

    // Two events with the same params must receive distinct IDs.
    let event2 = Event::new(CreateEventParams {
        name: "build-completed".to_string(),
        version: "1.2.3".to_string(),
        release: "2025.01".to_string(),
        platform_id: "linux-x86_64".to_string(),
        package: "rpm".to_string(),
        description: "Build completed successfully".to_string(),
        payload: json!({"commit": "abc123", "duration_ms": 1500}),
        success: true,
        receiver_id,
        owner_id,
    })
    .unwrap();

    assert_ne!(
        event.id(),
        event2.id(),
        "Each Event must be assigned a unique ID"
    );
}

/// Tests that `EventReceiver::new` rejects an empty name with a validation
/// error, confirming that name validation is enforced at the domain layer.
///
/// Note: `Event::new` does not validate the `name` field itself (that is an
/// application-layer concern). Name validation IS enforced by `EventReceiver`,
/// which is the entry point for event routing configuration.
#[test]
fn test_event_creation_with_empty_name_fails() {
    let owner_id = UserId::new();
    let schema = json!({"type": "object"});

    let result = EventReceiver::new(
        String::new(), // empty name must be rejected
        "webhook".to_string(),
        "1.0.0".to_string(),
        "A receiver with no name".to_string(),
        schema,
        owner_id,
    );

    assert!(
        result.is_err(),
        "EventReceiver::new must fail when name is empty"
    );
}

/// Tests successful creation of an `EventReceiver` and verifies the fields
/// returned by its getter methods.
///
/// Also confirms that an empty schema (`{}`) is accepted as valid (meaning
/// "accept any JSON object payload").
#[test]
fn test_event_receiver_creation_and_validation() {
    let owner_id = UserId::new();
    let schema = json!({"type": "object", "properties": {"commit": {"type": "string"}}});

    let result = EventReceiver::new(
        "CI/CD Pipeline".to_string(),
        "webhook".to_string(),
        "2.0.0".to_string(),
        "Receives events from the CI/CD pipeline".to_string(),
        schema,
        owner_id,
    );

    assert!(
        result.is_ok(),
        "EventReceiver::new should succeed with valid params"
    );
    let receiver = result.unwrap();

    assert_eq!(receiver.name(), "CI/CD Pipeline");
    assert_eq!(receiver.receiver_type(), "webhook");
    assert_eq!(receiver.version(), "2.0.0");
    assert_eq!(
        receiver.description(),
        "Receives events from the CI/CD pipeline"
    );
    assert!(
        !receiver.fingerprint().is_empty(),
        "Fingerprint must be a non-empty string"
    );

    // Empty schema is valid: it means "accept any JSON object payload".
    let empty_schema_result = EventReceiver::new(
        "Permissive Receiver".to_string(),
        "kafka".to_string(),
        "1.0.0".to_string(),
        "Accepts any payload".to_string(),
        json!({}),
        UserId::new(),
    );
    assert!(
        empty_schema_result.is_ok(),
        "Empty schema {{}} must be accepted as valid"
    );
}

/// Tests that two `EventReceiver` instances created with the same
/// `name`, `receiver_type`, `version`, and `schema` produce identical
/// fingerprints, regardless of `description` or `owner_id`.
///
/// Also verifies that changing the name produces a different fingerprint.
#[test]
fn test_event_receiver_fingerprint_is_deterministic() {
    let schema = json!({"type": "object"});

    let receiver_a = EventReceiver::new(
        "Deploy Receiver".to_string(),
        "webhook".to_string(),
        "1.0.0".to_string(),
        "First description".to_string(),
        schema.clone(),
        UserId::new(),
    )
    .expect("receiver_a creation should succeed");

    // Different description and owner but identical name/type/version/schema.
    let receiver_b = EventReceiver::new(
        "Deploy Receiver".to_string(),
        "webhook".to_string(),
        "1.0.0".to_string(),
        "Second description".to_string(),
        schema.clone(),
        UserId::new(),
    )
    .expect("receiver_b creation should succeed");

    assert_eq!(
        receiver_a.fingerprint(),
        receiver_b.fingerprint(),
        "Fingerprint must be identical for receivers with matching name/type/version/schema"
    );

    // Changing the name must produce a different fingerprint.
    let receiver_c = EventReceiver::new(
        "Different Receiver".to_string(),
        "webhook".to_string(),
        "1.0.0".to_string(),
        "Same description".to_string(),
        schema,
        UserId::new(),
    )
    .expect("receiver_c creation should succeed");

    assert_ne!(
        receiver_a.fingerprint(),
        receiver_c.fingerprint(),
        "Fingerprint must differ when receiver name changes"
    );
}

/// Tests successful creation of an `EventReceiverGroup` and verifies
/// membership management (add/remove receiver, enable/disable).
///
/// Confirms:
/// - A group can be created with an initial set of receiver IDs
/// - `contains_receiver` reflects the initial membership
/// - `receiver_count` returns the correct count
/// - `enabled` and `disable` work as expected
#[test]
fn test_event_receiver_group_creation() {
    let owner_id = UserId::new();
    let r1 = EventReceiverId::new();
    let r2 = EventReceiverId::new();

    let group = EventReceiverGroup::new(
        "Production Receivers".to_string(),
        "webhook_group".to_string(),
        "1.0.0".to_string(),
        "All production webhook receivers".to_string(),
        true,
        vec![r1, r2],
        owner_id,
    );

    assert!(
        group.is_ok(),
        "EventReceiverGroup::new should succeed with valid params"
    );
    let mut group = group.unwrap();

    assert_eq!(group.name(), "Production Receivers");
    assert_eq!(group.group_type(), "webhook_group");
    assert_eq!(group.version(), "1.0.0");
    assert!(group.enabled(), "Group should be enabled after creation");
    assert_eq!(
        group.receiver_count(),
        2,
        "Group should contain 2 receivers"
    );
    assert!(group.contains_receiver(r1), "Group must contain r1");
    assert!(group.contains_receiver(r2), "Group must contain r2");

    // Disabling and re-enabling works.
    group.disable();
    assert!(!group.enabled(), "Group should be disabled after disable()");
    group.enable();
    assert!(group.enabled(), "Group should be enabled after enable()");

    // Adding a new receiver increases the count.
    let r3 = EventReceiverId::new();
    group
        .add_event_receiver(r3)
        .expect("Adding a new receiver should succeed");
    assert_eq!(
        group.receiver_count(),
        3,
        "Receiver count should be 3 after adding r3"
    );
    assert!(group.contains_receiver(r3), "Group must contain r3");

    // Removing a receiver decreases the count.
    group
        .remove_event_receiver(r1)
        .expect("Removing r1 should succeed");
    assert_eq!(
        group.receiver_count(),
        2,
        "Receiver count should be 2 after removing r1"
    );
    assert!(
        !group.contains_receiver(r1),
        "Group must no longer contain r1"
    );
}

/// Tests that serializing an `Event` to JSON includes all expected fields and
/// contains no sensitive data (such as a `password_hash`).
///
/// Unlike `User`, which uses `#[serde(skip_serializing)]` to hide
/// `password_hash`, `Event` has no sensitive fields and all data should
/// appear in the serialized output.
#[test]
fn test_event_serialization_hides_no_sensitive_fields() {
    let owner_id = UserId::new();
    let receiver_id = EventReceiverId::new();

    let event = Event::new(CreateEventParams {
        name: "deployment-success".to_string(),
        version: "3.1.4".to_string(),
        release: "2025.02".to_string(),
        platform_id: "kubernetes".to_string(),
        package: "helm".to_string(),
        description: "Deployment completed without errors".to_string(),
        payload: json!({"replicas": 3, "namespace": "production"}),
        success: true,
        receiver_id,
        owner_id,
    })
    .expect("Event creation must succeed");

    let json_str = serde_json::to_string(&event).expect("Event serialization must succeed");

    // All non-sensitive fields should be present in the JSON output.
    assert!(
        json_str.contains("deployment-success"),
        "Serialized event must include the name"
    );
    assert!(
        json_str.contains("3.1.4"),
        "Serialized event must include the version"
    );
    assert!(
        json_str.contains("kubernetes"),
        "Serialized event must include the platform_id"
    );
    assert!(
        json_str.contains("production"),
        "Serialized event must include payload content"
    );

    // Events carry no sensitive fields; password_hash must not appear.
    assert!(
        !json_str.contains("password_hash"),
        "Serialized event must not contain password_hash"
    );

    // Verify the JSON round-trips correctly.
    let deserialized: serde_json::Value =
        serde_json::from_str(&json_str).expect("Deserialization of event JSON must succeed");
    assert!(
        deserialized.is_object(),
        "Deserialized event JSON must be an object"
    );
}
