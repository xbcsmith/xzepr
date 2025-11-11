// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// examples/cloudevents_format.rs
//! Example demonstrating the CloudEvents message format
//!
//! This example shows how XZepr events are serialized to CloudEvents 1.0.1
//! compatible format for Kafka publication, ensuring compatibility with other
//! systems expecting this format.
//!
//! Run with: cargo run --example cloudevents_format

use xzepr::domain::entities::event::{CreateEventParams, Event};
use xzepr::domain::value_objects::EventReceiverId;
use xzepr::infrastructure::messaging::cloudevents::CloudEventMessage;

fn main() {
    println!("=== XZepr CloudEvents Format Example ===\n");

    // Example 1: User event from API
    println!("1. User Event (from API POST):");
    println!("{}\n", "-".repeat(80));

    let receiver_id = EventReceiverId::new();
    let user_event = Event::new(CreateEventParams {
        name: "deployment.success".to_string(),
        version: "2.1.0".to_string(),
        release: "2.1.0-rc.1".to_string(),
        platform_id: "kubernetes".to_string(),
        package: "myapp".to_string(),
        description: "Application deployed successfully to production".to_string(),
        payload: serde_json::json!({
            "cluster": "prod-us-west",
            "namespace": "production",
            "replicas": 3,
            "image": "myapp:2.1.0-rc.1",
            "previous_version": "2.0.5"
        }),
        success: true,
        receiver_id,
    })
    .expect("Failed to create user event");

    let cloudevent = CloudEventMessage::from_event(&user_event);
    let json = serde_json::to_string_pretty(&cloudevent).expect("Failed to serialize");
    println!("{}\n", json);

    // Example 2: System event for receiver creation
    println!("2. System Event (Receiver Created):");
    println!("{}\n", "-".repeat(80));

    let system_receiver_id = EventReceiverId::new();
    let system_event = Event::new(CreateEventParams {
        name: "xzepr.event.receiver.created".to_string(),
        version: "1.0.0".to_string(),
        release: "system".to_string(),
        platform_id: "xzepr".to_string(),
        package: "xzepr.system".to_string(),
        description: "Event receiver 'production-webhook' created".to_string(),
        payload: serde_json::json!({
            "receiver_id": system_receiver_id.to_string(),
            "name": "production-webhook",
            "type": "webhook",
            "version": "1.0.0",
            "fingerprint": "sha256:abc123def456...",
            "description": "Production webhook receiver for deployments"
        }),
        success: true,
        receiver_id: system_receiver_id,
    })
    .expect("Failed to create system event");

    let system_cloudevent = CloudEventMessage::from_event(&system_event);
    let system_json =
        serde_json::to_string_pretty(&system_cloudevent).expect("Failed to serialize");
    println!("{}\n", system_json);

    // Example 3: Failed event
    println!("3. Failed Event:");
    println!("{}\n", "-".repeat(80));

    let failed_receiver_id = EventReceiverId::new();
    let failed_event = Event::new(CreateEventParams {
        name: "deployment.failed".to_string(),
        version: "2.1.0".to_string(),
        release: "2.1.0-rc.2".to_string(),
        platform_id: "kubernetes".to_string(),
        package: "myapp".to_string(),
        description: "Application deployment failed".to_string(),
        payload: serde_json::json!({
            "cluster": "prod-us-west",
            "namespace": "production",
            "error": "ImagePullBackOff",
            "reason": "Image not found in registry",
            "exit_code": 1
        }),
        success: false,
        receiver_id: failed_receiver_id,
    })
    .expect("Failed to create failed event");

    let failed_cloudevent = CloudEventMessage::from_event(&failed_event);
    let failed_json =
        serde_json::to_string_pretty(&failed_cloudevent).expect("Failed to serialize");
    println!("{}\n", failed_json);

    // Show field mapping
    println!("=== CloudEvents Field Mapping ===\n");
    println!("CloudEvents Spec 1.0.1 Fields:");
    println!("  - id:          Event UUID (ULID format)");
    println!("  - specversion: Always '1.0.1'");
    println!("  - type:        Event name/type");
    println!("  - source:      xzepr.event.receiver.<receiver_id>");
    println!("  - data:        Event payload with entity arrays\n");

    println!("Custom Extension Fields:");
    println!("  - success:     Event success status (bool)");
    println!("  - api_version: Always 'v1'");
    println!("  - name:        Event name (same as type)");
    println!("  - version:     Event version");
    println!("  - release:     Release identifier");
    println!("  - platform_id: Platform/environment identifier");
    println!("  - package:     Package/application name\n");

    println!("Data Object Fields (matches Go Data struct):");
    println!("  - events:                 Array of Event entities");
    println!("  - event_receivers:        Array of EventReceiver entities");
    println!("  - event_receiver_groups:  Array of EventReceiverGroup entities\n");
    println!("Note: Arrays contain the actual entities involved in the event.");
    println!("      For user events, events array has 1 item, others empty.");
    println!("      For receiver.created, both events and event_receivers have 1 item.");
    println!("      For group.created, both events and event_receiver_groups have 1 item.\n");

    println!("=== Kafka Message Structure ===\n");
    println!("Key:   <event_id>");
    println!("Value: <cloudevents_json>");
    println!("Topic: xzepr.dev.events (configurable)\n");

    println!("=== Compatibility ===\n");
    println!("This format is compatible with:");
    println!("  - CloudEvents 1.0.1 specification");
    println!("  - Go systems expecting Message struct with same fields");
    println!("  - Any consumer expecting CloudEvents format");
    println!("  - Schema registries supporting CloudEvents\n");
}
