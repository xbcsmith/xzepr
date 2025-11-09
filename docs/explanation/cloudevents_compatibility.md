# CloudEvents Compatibility

## Overview

XZepr publishes all events to Kafka/Redpanda in CloudEvents 1.0.1 compatible format, ensuring interoperability with other systems that expect this industry-standard event format. This document explains the CloudEvents implementation and how it ensures compatibility with Go-based systems using the Message struct.

## CloudEvents Specification

XZepr implements CloudEvents Specification version 1.0.1, which defines a common format for describing event data in a standard way. This enables event producers and consumers to work together without tight coupling.

### Core CloudEvents Fields

XZepr uses the following CloudEvents 1.0.1 fields:

- **id**: Unique identifier for the event (ULID format in XZepr)
- **specversion**: Version of CloudEvents spec (always "1.0.1")
- **type**: Type of the event (event name in XZepr)
- **source**: URI identifying the context in which the event occurred
- **data**: Event payload containing the actual event data

### CloudEvents Extensions

In addition to core fields, XZepr uses custom extension attributes for compatibility with existing systems:

- **success**: Boolean indicating event success/failure
- **api_version**: API version (always "v1")
- **name**: Event name (mirrors type field)
- **version**: Event version
- **release**: Release identifier
- **platform_id**: Platform/environment identifier
- **package**: Package/application name

## Go Message Struct Compatibility

XZepr's CloudEvents format is designed to be fully compatible with the Go Message struct:

```go
type Message struct {
    Success     bool   `json:"success"`
    ID          string `json:"id"`
    Specversion string `json:"specversion"`
    Type        string `json:"type"`
    Source      string `json:"source"`
    APIVersion  string `json:"api_version"`
    Name        string `json:"name"`
    Version     string `json:"version"`
    Release     string `json:"release"`
    PlatformID  string `json:"platform_id"`
    Package     string `json:"package"`
    Data        Data   `json:"data"`
}
```

### Rust to Go Field Mapping

| Rust Field (CloudEventMessage) | Go Field (Message) | JSON Name   | Type   |
| ------------------------------ | ------------------ | ----------- | ------ |
| success                        | Success            | success     | bool   |
| id                             | ID                 | id          | string |
| specversion                    | Specversion        | specversion | string |
| event_type                     | Type               | type        | string |
| source                         | Source             | source      | string |
| api_version                    | APIVersion         | api_version | string |
| name                           | Name               | name        | string |
| version                        | Version            | version     | string |
| release                        | Release            | release     | string |
| platform_id                    | PlatformID         | platform_id | string |
| package                        | Package            | package     | string |
| data                           | Data               | data        | object |

### Data Structure Compatibility

The `data` field contains a `CloudEventData` struct in Rust, which maps to the Go `Data` struct:

**Go (Data struct)**:

```go
type Data struct {
    Events              []storage.Event              `json:"events"`
    EventReceivers      []storage.EventReceiver      `json:"event_receivers"`
    EventReceiverGroups []storage.EventReceiverGroup `json:"event_receiver_groups"`
}
```

**Rust (CloudEventData)**:

```rust
pub struct CloudEventData {
    pub events: Vec<Event>,
    pub event_receivers: Vec<EventReceiver>,
    pub event_receiver_groups: Vec<EventReceiverGroup>,
}
```

**JSON Output**:

```json
{
  "events": [
    {
      "id": "01HJ1K2M3N4P5Q6R7S8T9V0W1Z",
      "name": "deployment.success",
      "version": "2.1.0",
      "release": "2.1.0-rc.1",
      "platform_id": "kubernetes",
      "package": "myapp",
      "description": "Application deployed successfully",
      "payload": { "cluster": "prod-us-west" },
      "success": true,
      "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
      "created_at": "2024-01-15T10:30:00Z"
    }
  ],
  "event_receivers": [],
  "event_receiver_groups": []
}
```

## Message Format Examples

### Example 1: Deployment Success Event

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

### Example 2: System Event - Receiver Created

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
          "type": "webhook",
          "version": "1.0.0"
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
        "schema": {},
        "fingerprint": "sha256:abc123...",
        "created_at": "2024-01-15T10:30:00Z"
      }
    ],
    "event_receiver_groups": []
  }
}
```

### Example 3: Failed Event

```json
{
  "success": false,
  "id": "01HJ1K2M3N4P5Q6R7S8T9V0W3C",
  "specversion": "1.0.1",
  "type": "deployment.failed",
  "source": "xzepr.event.receiver.01HJ1K2M3N4P5Q6R7S8T9V0W1X",
  "api_version": "v1",
  "name": "deployment.failed",
  "version": "2.1.0",
  "release": "2.1.0-rc.2",
  "platform_id": "kubernetes",
  "package": "myapp",
  "data": {
    "events": [
      {
        "id": "01HJ1K2M3N4P5Q6R7S8T9V0W3C",
        "name": "deployment.failed",
        "version": "2.1.0",
        "release": "2.1.0-rc.2",
        "platform_id": "kubernetes",
        "package": "myapp",
        "description": "Application deployment failed",
        "payload": {
          "cluster": "prod-us-west",
          "error": "ImagePullBackOff",
          "exit_code": 1
        },
        "success": false,
        "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
        "created_at": "2024-01-15T10:35:00Z"
      }
    ],
    "event_receivers": [],
    "event_receiver_groups": []
  }
}
```

## Implementation Details

### Rust Implementation

The CloudEvents format is implemented in `src/infrastructure/messaging/cloudevents.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEventMessage {
    pub success: bool,
    pub id: String,
    pub specversion: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub source: String,
    pub api_version: String,
    pub name: String,
    pub version: String,
    pub release: String,
    pub platform_id: String,
    pub package: String,
    pub data: CloudEventData,
}
```

Key implementation details:

- **Serde rename**: The `event_type` field is renamed to `"type"` in JSON
- **Field naming**: All field names use snake_case in Rust, serialized to lowercase with underscores in JSON
- **Type safety**: Strong typing ensures all required fields are present
- **Conversion**: `CloudEventMessage::from_event()` converts domain Event to CloudEvents format

### Kafka Publication

Events are converted to CloudEvents format before publication:

```rust
pub async fn publish(&self, event: &Event) -> Result<()> {
    // Convert Event to CloudEvents format for compatibility
    let cloudevent = CloudEventMessage::from_event(event);

    let payload = serde_json::to_string(&cloudevent)?;

    // Publish to Kafka...
}
```

This ensures that ALL messages sent to Kafka follow the CloudEvents format, maintaining consistency across the system.

## Consuming CloudEvents Messages

### From Go Systems

Go systems can consume XZepr events using the standard Message struct:

```go
import (
    "encoding/json"
    "github.com/confluentinc/confluent-kafka-go/kafka"
)

func consumeEvent(kafkaMessage *kafka.Message) error {
    var msg Message
    if err := json.Unmarshal(kafkaMessage.Value, &msg); err != nil {
        return err
    }

    // Access fields directly
    if msg.Success {
        log.Printf("Event %s succeeded: %s", msg.ID, msg.Name)
    }

    // Access payload
    payload := msg.Data.Payload

    return nil
}
```

### From Other Systems

Any system that supports CloudEvents 1.0.1 can consume XZepr events:

**Python Example**:

```python
from cloudevents.http import from_json
import json

def consume_event(kafka_message):
    event_dict = json.loads(kafka_message.value)

    # Access CloudEvents fields
    event_id = event_dict['id']
    event_type = event_dict['type']
    success = event_dict['success']

    # Access data
    data = event_dict['data']
    payload = data['payload']
```

**JavaScript/TypeScript Example**:

```typescript
interface CloudEventMessage {
  success: boolean;
  id: string;
  specversion: string;
  type: string;
  source: string;
  api_version: string;
  name: string;
  version: string;
  release: string;
  platform_id: string;
  package: string;
  data: {
    description: string;
    event_receiver_id: string;
    created_at: string;
    payload: any;
  };
}

function consumeEvent(kafkaMessage: any): void {
  const event: CloudEventMessage = JSON.parse(kafkaMessage.value);

  console.log(`Event ${event.id} (${event.type}): ${event.data.description}`);

  if (event.success) {
    console.log("Event succeeded");
  }
}
```

## Validation and Testing

### Running the Example

View example CloudEvents messages:

```bash
cargo run --example cloudevents_format
```

This shows:

- User events from API
- System events for receiver/group creation
- Failed events
- Field mapping documentation

### Testing CloudEvents Format

Unit tests verify CloudEvents format:

```bash
cargo test cloudevents
```

Tests verify:

- Correct field mapping from Event to CloudEventMessage
- JSON serialization matches expected structure
- All required CloudEvents fields present
- Field names match Go struct tags exactly

### Manual Verification

Verify CloudEvents format in Kafka:

```bash
# Consume from Kafka
rpk topic consume xzepr.dev.events --format json

# Verify structure with jq
rpk topic consume xzepr.dev.events --format json | \
  jq '.value | {
    has_id: has("id"),
    has_specversion: has("specversion"),
    has_type: has("type"),
    has_source: has("source"),
    has_data: has("data"),
    specversion: .specversion
  }'
```

Expected output:

```json
{
  "has_id": true,
  "has_specversion": true,
  "has_type": true,
  "has_source": true,
  "has_data": true,
  "specversion": "1.0.1"
}
```

## Benefits of CloudEvents Format

### Interoperability

- **Cross-language compatibility**: Works with Go, Python, JavaScript, Java, etc.
- **Standard tooling**: Compatible with CloudEvents SDKs and libraries
- **Schema registry support**: Can be registered in schema registries
- **Industry standard**: Widely adopted in cloud-native applications

### Discoverability

- **Self-describing**: Events include metadata about their structure
- **Type system**: Event types clearly identified in `type` field
- **Source tracking**: Origin of events tracked in `source` field
- **Version management**: Multiple versions can coexist with `version` field

### Extensibility

- **Custom extensions**: XZepr adds custom fields while maintaining compatibility
- **Backward compatibility**: New fields can be added without breaking consumers
- **Payload flexibility**: Arbitrary JSON allowed in `data.payload`
- **Metadata separation**: Event metadata separate from payload

### Observability

- **Tracing**: CloudEvents integrates with distributed tracing
- **Monitoring**: Standard format enables consistent monitoring
- **Debugging**: Clear structure makes debugging easier
- **Auditing**: Timestamps and IDs enable event auditing

## CloudEvents Best Practices

### Event Type Naming

XZepr follows CloudEvents type naming conventions:

- Use reverse-DNS notation: `com.example.myapp.event`
- Or dot-separated hierarchy: `deployment.success`
- System events prefixed: `xzepr.event.receiver.created`
- Versioning in separate field, not type name

### Source URI Format

XZepr uses consistent source URI format:

```
xzepr.event.receiver.<receiver_id>
```

This identifies:

- System origin: `xzepr`
- Event category: `event.receiver`
- Specific receiver: `<receiver_id>`

### Data Content

The `data` field contains:

- **description**: Human-readable event description
- **event_receiver_id**: Associated receiver for correlation
- **created_at**: Event creation timestamp
- **payload**: Event-specific data (arbitrary structure)

Keep payload structure consistent within event types for easier consumption.

### Data Arrays Usage

The `data` field contains three arrays:

- **events**: Always contains at least one Event entity that triggered this CloudEvent
- **event_receivers**: Contains EventReceiver entities when the CloudEvent is about receiver operations (create, update, delete)
- **event_receiver_groups**: Contains EventReceiverGroup entities when the CloudEvent is about group operations (create, update, delete)

**Usage patterns**:

1. **User events** (posted via API): `events` has 1 item, others empty
2. **Receiver system events**: `events` has 1 item, `event_receivers` has 1 item, `event_receiver_groups` empty
3. **Group system events**: `events` has 1 item, `event_receiver_groups` has 1 item, `event_receivers` empty
4. **Batch operations** (future): Arrays may contain multiple items

This structure allows batch operations and provides full context about what entities are involved in each CloudEvent.

### Version Management

Use the `version` field for event schema versions:

- Start with `1.0.0`
- Increment minor version for backward-compatible changes
- Increment major version for breaking changes
- Document schema changes in release notes

## Migration Guide

### For Existing Consumers

If you have existing consumers expecting a different format:

1. **Update consumers to parse CloudEvents format**:

   - Access `type` instead of direct `name`
   - Access `data.payload` instead of direct `payload`
   - Use `success` field consistently

2. **Use adapter pattern** if needed:

   ```go
   func adaptToLegacyFormat(ce Message) LegacyEvent {
       return LegacyEvent{
           Name:    ce.Name,
           Payload: ce.Data.Payload,
           Success: ce.Success,
           // Map other fields...
       }
   }
   ```

3. **Test compatibility** with both formats during transition period

### For New Consumers

Start with CloudEvents-aware consumers:

1. Use CloudEvents SDKs for your language
2. Parse the entire CloudEvents envelope
3. Validate `specversion` is "1.0.1"
4. Handle both success and failure events (`success` field)
5. Log unrecognized event types for monitoring

## Troubleshooting

### Problem: Consumer Cannot Parse Events

**Solution**: Verify consumer expects CloudEvents 1.0.1 format with custom extensions.

### Problem: Missing Fields

**Solution**: XZepr always includes all required fields. Check consumer is not filtering them.

### Problem: Wrong Field Names

**Solution**: Field names use snake_case with underscores (e.g., `api_version`, not `apiVersion`).

### Problem: Incompatible with Go Struct

**Solution**: Verify Go struct uses correct JSON tags matching XZepr field names exactly.

## References

- CloudEvents Specification 1.0.1: https://github.com/cloudevents/spec/blob/v1.0.1/spec.md
- CloudEvents Primer: https://github.com/cloudevents/spec/blob/v1.0.1/cloudevents/primer.md
- Implementation: `src/infrastructure/messaging/cloudevents.rs`
- Example: `examples/cloudevents_format.rs`
- Event Publication: `docs/explanation/event_publication_implementation.md`
- Verification Guide: `docs/how_to/verify_event_publication.md`

## Summary

XZepr implements CloudEvents 1.0.1 specification with custom extensions for full compatibility with Go-based systems. The format is:

- **Standards-compliant**: Follows CloudEvents 1.0.1 exactly
- **Go-compatible**: Matches Go Message struct field-for-field
- **Cross-language**: Works with any CloudEvents-aware consumer
- **Extensible**: Custom fields don't break compatibility
- **Production-ready**: Tested and validated format

All events published to Kafka follow this format consistently, ensuring seamless integration with existing systems and future consumers.
