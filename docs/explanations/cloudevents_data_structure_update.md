# CloudEvents Data Structure Update

## Overview

This document describes the update to XZepr's CloudEvents `data` structure to match the Go system's `Data` struct format. The change replaces individual fields with arrays of domain entities, enabling batch operations and providing full context about what entities are involved in each CloudEvent message.

## Problem Statement

The original CloudEventData structure used individual fields:

```rust
// OLD structure
pub struct CloudEventData {
    pub description: String,
    pub event_receiver_id: String,
    pub created_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}
```

This structure was incompatible with the Go system's `Data` struct, which uses arrays:

```go
type Data struct {
    Events              []storage.Event              `json:"events"`
    EventReceivers      []storage.EventReceiver      `json:"event_receivers"`
    EventReceiverGroups []storage.EventReceiverGroup `json:"event_receiver_groups"`
}
```

The mismatch prevented seamless integration between XZepr and Go-based systems expecting this specific array-based format.

## Solution

### Updated CloudEventData Structure

```rust
// NEW structure
pub struct CloudEventData {
    pub events: Vec<Event>,
    pub event_receivers: Vec<EventReceiver>,
    pub event_receiver_groups: Vec<EventReceiverGroup>,
}
```

This structure now exactly matches the Go `Data` struct, with three arrays containing the actual domain entities involved in the CloudEvent.

## Implementation Details

### Changes Made

**Modified Files:**

1. **`src/infrastructure/messaging/cloudevents.rs`** (~150 lines modified)
   - Updated `CloudEventData` struct to use entity arrays
   - Added `from_event_with_receiver()` method for receiver system events
   - Added `from_event_with_group()` method for group system events
   - Updated all tests to work with new structure

2. **`src/infrastructure/messaging/producer.rs`** (~40 lines added)
   - Added `publish_message()` method to publish CloudEventMessage directly
   - Maintains backward compatibility with existing `publish()` method

3. **`src/application/handlers/event_receiver_handler.rs`** (~10 lines modified)
   - Updated to use `from_event_with_receiver()` when publishing
   - Includes actual EventReceiver entity in message

4. **`src/application/handlers/event_receiver_group_handler.rs`** (~10 lines modified)
   - Updated to use `from_event_with_group()` when publishing
   - Includes actual EventReceiverGroup entity in message

5. **`examples/cloudevents_format.rs`** (~10 lines modified)
   - Updated documentation to reflect new Data structure

6. **`docs/explanations/cloudevents_compatibility.md`** (updated)
   - Complete documentation of new Data structure
   - Added usage patterns and examples

Total: ~220 lines modified

### New Methods

**CloudEventMessage::from_event_with_receiver()**

Creates a CloudEvent message that includes both the system event and the receiver entity:

```rust
let message = CloudEventMessage::from_event_with_receiver(&event, &receiver);
// message.data.events = [event]
// message.data.event_receivers = [receiver]
// message.data.event_receiver_groups = []
```

**CloudEventMessage::from_event_with_group()**

Creates a CloudEvent message that includes both the system event and the group entity:

```rust
let message = CloudEventMessage::from_event_with_group(&event, &group);
// message.data.events = [event]
// message.data.event_receivers = []
// message.data.event_receiver_groups = [group]
```

**KafkaEventPublisher::publish_message()**

Publishes a CloudEventMessage directly without conversion:

```rust
pub async fn publish_message(&self, message: &CloudEventMessage) -> Result<()>
```

## Message Format Examples

### User Event (API POST)

```json
{
  "success": true,
  "id": "01HJ1K2M3N4P5Q6R7S8T9V0W1Z",
  "specversion": "1.0.1",
  "type": "deployment.success",
  "source": "xzepr.event.receiver.01HJ1K2M3N4P5Q6R7S8T9V0W1X",
  "api_version": "v1",
  "name": "deployment.success",
  "version": "2.1.0",
  "release": "2.1.0-rc.1",
  "platform_id": "kubernetes",
  "package": "myapp",
  "data": {
    "events": [
      {
        "id": "01HJ1K2M3N4P5Q6R7S8T9V0W1Z",
        "name": "deployment.success",
        "version": "2.1.0",
        "release": "2.1.0-rc.1",
        "platform_id": "kubernetes",
        "package": "myapp",
        "description": "Application deployed successfully",
        "payload": {
          "cluster": "prod-us-west",
          "namespace": "production",
          "replicas": 3
        },
        "success": true,
        "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
        "created_at": "2024-01-15T10:30:00Z"
      }
    ],
    "event_receivers": [],
    "event_receiver_groups": []
  }
}
```

### System Event: Receiver Created

```json
{
  "success": true,
  "id": "01HJ1K2M3N4P5Q6R7S8T9V0W2B",
  "specversion": "1.0.1",
  "type": "xzepr.event.receiver.created",
  "source": "xzepr.event.receiver.01HJ1K2M3N4P5Q6R7S8T9V0W1X",
  "api_version": "v1",
  "name": "xzepr.event.receiver.created",
  "version": "1.0.0",
  "release": "system",
  "platform_id": "xzepr",
  "package": "xzepr.system",
  "data": {
    "events": [
      {
        "id": "01HJ1K2M3N4P5Q6R7S8T9V0W2B",
        "name": "xzepr.event.receiver.created",
        "version": "1.0.0",
        "release": "system",
        "platform_id": "xzepr",
        "package": "xzepr.system",
        "description": "Event receiver 'prod-webhook' created",
        "payload": {
          "receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
          "name": "prod-webhook",
          "type": "webhook"
        },
        "success": true,
        "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
        "created_at": "2024-01-15T10:30:00Z"
      }
    ],
    "event_receivers": [
      {
        "id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
        "name": "prod-webhook",
        "receiver_type": "webhook",
        "version": "1.0.0",
        "description": "Production webhook receiver",
        "schema": {
          "type": "object",
          "properties": {
            "url": { "type": "string" }
          }
        },
        "fingerprint": "sha256:abc123...",
        "created_at": "2024-01-15T10:30:00Z"
      }
    ],
    "event_receiver_groups": []
  }
}
```

### System Event: Group Created

```json
{
  "success": true,
  "id": "01HJ1K2M3N4P5Q6R7S8T9V0W2C",
  "specversion": "1.0.1",
  "type": "xzepr.event.receiver.group.created",
  "source": "xzepr.event.receiver.01HJ1K2M3N4P5Q6R7S8T9V0W1X",
  "api_version": "v1",
  "name": "xzepr.event.receiver.group.created",
  "version": "1.0.0",
  "release": "system",
  "platform_id": "xzepr",
  "package": "xzepr.system",
  "data": {
    "events": [
      {
        "id": "01HJ1K2M3N4P5Q6R7S8T9V0W2C",
        "name": "xzepr.event.receiver.group.created",
        "version": "1.0.0",
        "release": "system",
        "platform_id": "xzepr",
        "package": "xzepr.system",
        "description": "Event receiver group 'prod-cluster' created",
        "payload": {
          "group_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1Y",
          "name": "prod-cluster"
        },
        "success": true,
        "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
        "created_at": "2024-01-15T10:30:00Z"
      }
    ],
    "event_receivers": [],
    "event_receiver_groups": [
      {
        "id": "01HJ1K2M3N4P5Q6R7S8T9V0W1Y",
        "name": "prod-cluster",
        "group_type": "webhook-cluster",
        "version": "1.0.0",
        "description": "Production webhook cluster",
        "enabled": true,
        "event_receiver_ids": [
          "01HJ1K2M3N4P5Q6R7S8T9V0W1X"
        ],
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T10:30:00Z"
      }
    ]
  }
}
```

## Data Array Usage Patterns

### Pattern 1: User Events (API POST)

- `events`: Contains 1 Event entity (the event that was posted)
- `event_receivers`: Empty array
- `event_receiver_groups`: Empty array

**Use Case**: Publishing business events from applications

### Pattern 2: Receiver System Events

- `events`: Contains 1 Event entity (system event describing the operation)
- `event_receivers`: Contains 1 EventReceiver entity (the receiver that was created/updated/deleted)
- `event_receiver_groups`: Empty array

**Use Cases**:
- `xzepr.event.receiver.created`
- `xzepr.event.receiver.updated`
- `xzepr.event.receiver.deleted`

### Pattern 3: Group System Events

- `events`: Contains 1 Event entity (system event describing the operation)
- `event_receivers`: Empty array
- `event_receiver_groups`: Contains 1 EventReceiverGroup entity (the group that was created/updated/deleted)

**Use Cases**:
- `xzepr.event.receiver.group.created`
- `xzepr.event.receiver.group.updated`
- `xzepr.event.receiver.group.deleted`

### Pattern 4: Batch Operations (Future)

Arrays can contain multiple items for batch operations:

- `events`: Multiple Event entities
- `event_receivers`: Multiple EventReceiver entities (if applicable)
- `event_receiver_groups`: Multiple EventReceiverGroup entities (if applicable)

**Use Cases** (not yet implemented):
- Bulk event creation
- Batch receiver updates
- Group membership changes

## Benefits of Array-Based Structure

### 1. Go System Compatibility

The structure now matches exactly with the Go `Data` struct, ensuring seamless interoperability:

```go
type Data struct {
    Events              []storage.Event              `json:"events"`
    EventReceivers      []storage.EventReceiver      `json:"event_receivers"`
    EventReceiverGroups []storage.EventReceiverGroup `json:"event_receiver_groups"`
}
```

Go systems can unmarshal XZepr CloudEvents directly into this struct without any transformation.

### 2. Full Entity Context

Consumers receive complete entity information:

- Full Event entity (not just ID and payload)
- Full EventReceiver entity with schema, fingerprint, etc.
- Full EventReceiverGroup entity with all members

This provides rich context for event processing, auditing, and analytics.

### 3. Batch Operation Support

The array structure naturally supports batch operations:

- Multiple events can be published in one message
- Bulk updates to receivers/groups can be communicated
- Reduces message count for batch operations

### 4. Type Safety

Using domain entities instead of arbitrary JSON:

- Compile-time type checking in Rust
- Schema validation in Go
- Consistent structure across all messages

### 5. Queryability

Consumers can easily:

- Count how many events are in a message
- Iterate over receivers in a batch
- Filter groups by criteria

## Backward Compatibility

### Breaking Changes

This is a **breaking change** for existing consumers that expect the old structure with individual fields:

**Old structure (no longer used)**:
```json
{
  "data": {
    "description": "...",
    "event_receiver_id": "...",
    "created_at": "...",
    "payload": {...}
  }
}
```

**New structure**:
```json
{
  "data": {
    "events": [...],
    "event_receivers": [...],
    "event_receiver_groups": [...]
  }
}
```

### Migration Path for Consumers

Consumers need to update to access events from arrays:

**Old consumer code**:
```go
// OLD - no longer works
description := message.Data.Description
payload := message.Data.Payload
```

**New consumer code**:
```go
// NEW - access from events array
if len(message.Data.Events) > 0 {
    event := message.Data.Events[0]
    description := event.Description
    payload := event.Payload
}
```

**Python example**:
```python
# OLD
description = message['data']['description']

# NEW
if message['data']['events']:
    event = message['data']['events'][0]
    description = event['description']
```

## Testing

### Unit Tests

Added comprehensive tests for the new structure:

- `test_cloudevent_with_receiver()` - Verifies receiver in data.event_receivers
- `test_cloudevent_with_group()` - Verifies group in data.event_receiver_groups
- Updated existing tests to check array fields

### Test Results

```
test result: ok. 390 passed; 0 failed; 4 ignored
```

All tests pass, including:
- CloudEvents structure tests
- Serialization/deserialization tests
- Handler integration tests

### Manual Verification

Run the example to see actual message format:

```bash
cargo run --example cloudevents_format
```

Output shows:
- User event with single item in events array
- System event with both event and receiver
- System event with both event and group

## Validation Results

### Code Quality

- ✅ `cargo fmt --all` - passed
- ✅ `cargo check --all-targets --all-features` - 0 errors
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` - 0 warnings
- ✅ `cargo test --all-features` - 390 tests passed, 0 failures
- ✅ `cargo build --release` - successful

### Message Format Validation

Verified CloudEvents messages contain correct structure:

```bash
# Check field existence
rpk topic consume xzepr.dev.events --format json | \
  jq '.value.data | {
    has_events: has("events"),
    has_event_receivers: has("event_receivers"),
    has_event_receiver_groups: has("event_receiver_groups"),
    events_is_array: (.events | type == "array"),
    event_receivers_is_array: (.event_receivers | type == "array"),
    event_receiver_groups_is_array: (.event_receiver_groups | type == "array")
  }'
```

Expected output:
```json
{
  "has_events": true,
  "has_event_receivers": true,
  "has_event_receiver_groups": true,
  "events_is_array": true,
  "event_receivers_is_array": true,
  "event_receiver_groups_is_array": true
}
```

## Future Enhancements

### 1. Batch Operations API

Add endpoints for batch operations:

```
POST /api/v1/events/batch
POST /api/v1/event-receivers/batch
```

These would produce CloudEvents with multiple items in arrays.

### 2. Query Support

Add filtering in data arrays:

```rust
pub struct CloudEventData {
    pub events: Vec<Event>,
    pub event_receivers: Vec<EventReceiver>,
    pub event_receiver_groups: Vec<EventReceiverGroup>,
    pub query: Option<QueryCriteria>, // New field for explaining array contents
}
```

### 3. Pagination Support

For large batches, add pagination metadata:

```rust
pub struct CloudEventData {
    pub events: Vec<Event>,
    pub event_receivers: Vec<EventReceiver>,
    pub event_receiver_groups: Vec<EventReceiverGroup>,
    pub pagination: Option<PaginationMetadata>,
}
```

### 4. Delta/Diff Support

For update operations, include before/after:

```rust
pub struct UpdateData {
    pub before: Vec<EventReceiver>,
    pub after: Vec<EventReceiver>,
}
```

## References

- CloudEvents Specification 1.0.1: https://github.com/cloudevents/spec/blob/v1.0.1/spec.md
- Implementation: `src/infrastructure/messaging/cloudevents.rs`
- Example: `examples/cloudevents_format.rs`
- Compatibility Guide: `docs/explanations/cloudevents_compatibility.md`
- Go Data struct: Go system's storage package

## Summary

The CloudEventData structure has been updated to match the Go system's `Data` struct format, using arrays of domain entities instead of individual fields. This change:

1. **Ensures compatibility** with Go-based systems expecting array-based Data structure
2. **Provides full context** by including complete entity information
3. **Enables batch operations** through natural array support
4. **Maintains type safety** with strongly-typed domain entities
5. **Follows CloudEvents spec** for extensible data payloads

All existing functionality continues to work, with CloudEvents now carrying richer, more structured information about the entities involved in each event. This is a breaking change for consumers, who must update to access data from the arrays.
