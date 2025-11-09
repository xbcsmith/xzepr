# Event Publication Implementation

## Overview

This document describes the implementation of event publication functionality in XZepr, enabling the system to publish events to Kafka/Redpanda in CloudEvents 1.0.1 compatible format when events are created via the API, as well as publishing system events when event receivers and event receiver groups are created.

All messages are serialized in CloudEvents format to ensure compatibility with other systems expecting this industry-standard format.

## Problem Statement

XZepr was not producing events to Kafka when:

1. Events were POSTed to the API (`/api/v1/events`)
2. Event receivers were created (`/api/v1/event-receivers`)
3. Event receiver groups were created (`/api/v1/event-receiver-groups`)

The `KafkaEventPublisher` existed but was never instantiated or injected into the application handlers, resulting in events being stored in the database but not published to the message broker.

## Components Delivered

### Modified Files

- `src/infrastructure/messaging/cloudevents.rs` (343 lines, new file)

  - Created `CloudEventMessage` struct following CloudEvents 1.0.1 spec
  - Created `CloudEventData` struct for event payload
  - Implemented conversion from `Event` to `CloudEventMessage`
  - Added comprehensive tests for CloudEvents format

- `src/infrastructure/messaging/producer.rs` (~40 lines modified)

  - Updated `publish()` to convert Event to CloudEvents format
  - Added CloudEvents serialization before Kafka publication
  - Updated logging to reflect CloudEvents format

- `src/infrastructure/messaging/mod.rs` (1 line added)

  - Exported `cloudevents` module

- `src/application/handlers/event_handler.rs` (~120 lines modified)

  - Added `event_publisher` field to `EventHandler`
  - Added `with_publisher()` constructor
  - Implemented event publication after successful event creation

- `src/application/handlers/event_receiver_handler.rs` (~150 lines modified)

  - Added `event_publisher` field to `EventReceiverHandler`
  - Added `with_publisher()` constructor
  - Implemented system event creation and publication for receiver creation
  - Added `create_receiver_created_event()` helper method

- `src/application/handlers/event_receiver_group_handler.rs` (~180 lines modified)

  - Added `event_publisher` field to `EventReceiverGroupHandler`
  - Added `with_publisher()` constructor
  - Implemented system event creation and publication for group creation
  - Added `create_group_created_event()` helper method

- `src/main.rs` (~60 lines modified)

  - Instantiated `KafkaEventPublisher` at startup
  - Updated handler initialization to inject event publisher
  - Added graceful fallback if Kafka is unavailable
  - Added necessary imports (`warn`, `KafkaEventPublisher`)

- `examples/cloudevents_format.rs` (140 lines, new file)
  - Example demonstrating CloudEvents message format
  - Shows user events, system events, and failed events
  - Documents field mapping and compatibility

Total: ~1,034 lines of new/modified code

## Implementation Details

### 1. EventHandler Enhancement

The `EventHandler` now optionally holds a reference to a `KafkaEventPublisher`:

```rust
pub struct EventHandler {
    event_repository: Arc<dyn EventRepository>,
    receiver_repository: Arc<dyn EventReceiverRepository>,
    event_publisher: Option<Arc<KafkaEventPublisher>>,
}
```

**Key Design Decisions:**

- **Optional Publisher**: The publisher is wrapped in `Option` to allow the system to operate without Kafka (useful for testing and degraded operation)
- **Backward Compatibility**: The original `new()` constructor remains, setting `event_publisher` to `None`
- **New Constructor**: `with_publisher()` accepts an event publisher and enables Kafka publication

**Event Publication Flow:**

1. Event is validated and saved to the database
2. If publisher is configured, attempt to publish to Kafka
3. If publication fails, log error but don't fail the request
4. Event remains in database regardless of Kafka status (best-effort delivery)

```rust
// Publish event to Kafka if publisher is configured
if let Some(publisher) = &self.event_publisher {
    if let Err(e) = publisher.publish(&event).await {
        error!(
            event_id = %event_id,
            error = %e,
            "Failed to publish event to Kafka (event was saved to database)"
        );
        // Note: We don't fail the request since the event was saved to the database
    } else {
        info!(event_id = %event_id, "Event published to Kafka successfully");
    }
} else {
    warn!("Event publisher not configured, skipping Kafka publication");
}
```

### 2. EventReceiverHandler Enhancement

When an event receiver is created, a system event is generated and published:

**System Event Structure:**

- **Type**: `xzepr.event.receiver.created`
- **Version**: `1.0.0`
- **Release**: `system`
- **Platform**: `xzepr`
- **Package**: `xzepr.system`
- **Success**: `true`

**Payload Example:**

```json
{
  "receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
  "name": "production-api-receiver",
  "type": "webhook",
  "version": "1.0.0",
  "fingerprint": "sha256:abc123...",
  "description": "Production API webhook receiver"
}
```

**Implementation:**

```rust
fn create_receiver_created_event(&self, receiver: &EventReceiver) -> Event {
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
    })
    .expect("Failed to create system event for receiver creation")
}
```

### 3. EventReceiverGroupHandler Enhancement

When an event receiver group is created, a system event is generated:

**System Event Structure:**

- **Type**: `xzepr.event.receiver.group.created`
- **Version**: `1.0.0`
- **Release**: `system`
- **Platform**: `xzepr`
- **Package**: `xzepr.system`
- **Success**: `true`

**Payload Example:**

```json
{
  "group_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1Y",
  "name": "production-group",
  "type": "webhook-cluster",
  "version": "1.0.0",
  "description": "Production webhook cluster",
  "enabled": true,
  "receiver_ids": ["01HJ1K2M3N4P5Q6R7S8T9V0W1X", "01HJ1K2M3N4P5Q6R7S8T9V0W2A"],
  "receiver_count": 2
}
```

**Special Case Handling:**

Groups require a `receiver_id` for event creation. The implementation handles empty groups by using a synthetic receiver ID:

```rust
let receiver_id = group
    .event_receiver_ids()
    .first()
    .copied()
    .unwrap_or_else(|| {
        // If group has no receivers, use the group's ID as a synthetic receiver ID
        EventReceiverId::from(group.id().as_ulid())
    });
```

This ensures system events can be created even for empty groups.

### 4. Main Application Integration

**Kafka Publisher Initialization:**

```rust
// Initialize Kafka event publisher
info!("Initializing Kafka event publisher...");
let event_publisher = match KafkaEventPublisher::new(
    &settings.kafka.brokers,
    &settings.kafka.default_topic,
) {
    Ok(publisher) => {
        info!(
            "Kafka event publisher initialized successfully (topic: {})",
            settings.kafka.default_topic
        );
        Some(Arc::new(publisher))
    }
    Err(e) => {
        warn!(
            "Failed to initialize Kafka event publisher: {}. Event publication will be disabled.",
            e
        );
        None
    }
};
```

**Handler Initialization with Publisher:**

```rust
// Create application handlers with event publisher
let event_handler = if let Some(ref publisher) = event_publisher {
    EventHandler::with_publisher(event_repo, receiver_repo.clone(), publisher.clone())
} else {
    EventHandler::new(event_repo, receiver_repo.clone())
};

let receiver_handler = if let Some(ref publisher) = event_publisher {
    EventReceiverHandler::with_publisher(receiver_repo.clone(), publisher.clone())
} else {
    EventReceiverHandler::new(receiver_repo.clone())
};

let group_handler = if let Some(ref publisher) = event_publisher {
    EventReceiverGroupHandler::with_publisher(group_repo, receiver_repo, publisher.clone())
} else {
    EventReceiverGroupHandler::new(group_repo, receiver_repo)
};
```

## Error Handling and Resilience

### Graceful Degradation

The implementation follows a "best-effort" approach:

1. **Kafka Unavailable at Startup**: System starts without event publisher, logs warning
2. **Kafka Unavailable During Operation**: Events saved to database, publication failure logged
3. **No Request Failures**: API requests succeed even if Kafka publication fails

### Logging Strategy

**Success Cases:**

- Info-level logs when publisher initializes
- Info-level logs when events publish successfully

**Failure Cases:**

- Warn-level log if publisher fails to initialize (startup)
- Error-level logs if publication fails (runtime)
- Warn-level log if publisher not configured (per operation)

### Benefits of This Approach

1. **High Availability**: API remains operational even if Kafka is down
2. **Data Integrity**: Events always saved to database first
3. **Observability**: Clear logging of all publication attempts
4. **Backward Compatibility**: Existing code works without changes

## Testing

### Unit Tests

All existing tests continue to pass. The optional publisher design means:

- Tests without publisher: Use `Handler::new()` constructors
- Tests with publisher: Can use `Handler::with_publisher()` and mock publishers

### Integration Testing

To test event publication:

1. Start XZepr with Redpanda running
2. Create an event receiver:
   ```bash
   curl -X POST http://localhost:8080/api/v1/event-receivers \
     -H "Content-Type: application/json" \
     -d '{
       "name": "test-receiver",
       "type": "webhook",
       "version": "1.0.0",
       "description": "Test receiver",
       "schema": {}
     }'
   ```
3. Verify system event published to `xzepr.dev.events` topic
4. Post an event:
   ```bash
   curl -X POST http://localhost:8080/api/v1/events \
     -H "Content-Type: application/json" \
     -d '{
       "name": "test-event",
       "version": "1.0.0",
       "release": "1.0.0",
       "platform_id": "test",
       "package": "test-pkg",
       "description": "Test event",
       "payload": {},
       "success": true,
       "event_receiver_id": "<receiver-id>"
     }'
   ```
5. Verify event published to `xzepr.dev.events` topic

### Verifying Kafka Messages

Use `rpk` (Redpanda CLI) to consume messages:

```bash
rpk topic consume xzepr.dev.events
```

Expected output includes:

- System events for receiver/group creation
- User events from API posts

## Configuration

Event publication is configured via `config/default.yaml`:

```yaml
kafka:
  brokers: "localhost:9092"
  default_topic: "xzepr.dev.events"
  default_topic_partitions: 3
  default_topic_replication_factor: 1
```

**Configuration Options:**

- `brokers`: Comma-separated list of Kafka/Redpanda brokers
- `default_topic`: Topic name for event publication
- `default_topic_partitions`: Number of partitions (for auto-creation)
- `default_topic_replication_factor`: Replication factor (for auto-creation)

### Authentication Configuration

XZepr supports authenticated connections to Kafka clusters using SASL/SCRAM
mechanisms. Authentication is optional and configured separately.

**YAML Configuration with Authentication:**

```yaml
kafka:
  brokers: "broker1.example.com:9093,broker2.example.com:9093"
  default_topic: "xzepr.events"
  default_topic_partitions: 3
  default_topic_replication_factor: 3
  auth:
    security_protocol: "SASL_SSL"
    sasl:
      mechanism: "SCRAM-SHA-256"
      username: "kafka-user"
      password: "kafka-password"
    ssl:
      ca_location: "/path/to/ca-cert.pem"
      certificate_location: "/path/to/client-cert.pem"
      key_location: "/path/to/client-key.pem"
```

**Environment Variables (Recommended for Production):**

```bash
# Security protocol
export XZEPR_KAFKA_SECURITY_PROTOCOL="SASL_SSL"

# SASL credentials
export XZEPR_KAFKA_SASL_MECHANISM="SCRAM-SHA-256"
export XZEPR_KAFKA_SASL_USERNAME="kafka-user"
export XZEPR_KAFKA_SASL_PASSWORD="kafka-password"

# SSL certificates
export XZEPR_KAFKA_SSL_CA_LOCATION="/path/to/ca-cert.pem"
export XZEPR_KAFKA_SSL_CERTIFICATE_LOCATION="/path/to/client-cert.pem"
export XZEPR_KAFKA_SSL_KEY_LOCATION="/path/to/client-key.pem"
```

**Authentication Options:**

- `security_protocol`: Authentication mode
  - `PLAINTEXT` - No authentication (default)
  - `SASL_PLAINTEXT` - SASL without encryption
  - `SASL_SSL` - SASL with SSL/TLS (recommended)
  - `SSL` - Certificate-based authentication
- `sasl.mechanism`: SASL authentication mechanism
  - `SCRAM-SHA-256` - Recommended for production
  - `SCRAM-SHA-512` - Higher security requirements
  - `PLAIN` - Simple username/password
  - `GSSAPI` - Kerberos
  - `OAUTHBEARER` - OAuth 2.0
- `sasl.username`: Kafka username
- `sasl.password`: Kafka password
- `ssl.ca_location`: Path to CA certificate
- `ssl.certificate_location`: Path to client certificate (optional)
- `ssl.key_location`: Path to client key (optional)

**Security Best Practices:**

1. Always use `SASL_SSL` in production for encrypted connections
2. Store credentials in environment variables or secrets management systems
3. Never commit credentials to version control
4. Rotate credentials regularly
5. Use `SCRAM-SHA-256` or `SCRAM-SHA-512` for SASL mechanism
6. Ensure certificate files have appropriate permissions (600 for keys)

For detailed authentication setup instructions, see:

- [Configure Kafka Authentication](../how_to/configure_kafka_authentication.md)
- [Kafka Authentication Example](../../examples/kafka_with_auth.rs)

## Event Schemas

All events are published to Kafka in CloudEvents 1.0.1 compatible format. The CloudEvents envelope wraps the event data and provides standard fields for event routing and processing.

### CloudEvents Message Structure

```json
{
  "success": true, // Extension: Event success status
  "id": "string", // CloudEvents: Unique event ID (ULID)
  "specversion": "1.0.1", // CloudEvents: Spec version
  "type": "string", // CloudEvents: Event type/name
  "source": "string", // CloudEvents: Event source URI
  "api_version": "string", // Extension: API version
  "name": "string", // Extension: Event name
  "version": "string", // Extension: Event version
  "release": "string", // Extension: Release identifier
  "platform_id": "string", // Extension: Platform identifier
  "package": "string", // Extension: Package name
  "data": {
    // CloudEvents: Event payload
    "description": "string",
    "event_receiver_id": "string",
    "created_at": "2024-01-15T10:30:00Z",
    "payload": {}
  }
}
```

### User Event (from API POST)

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
    "description": "Application deployed successfully",
    "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
    "created_at": "2024-01-15T10:30:00Z",
    "payload": {
      "cluster": "prod-us-west",
      "namespace": "production",
      "replicas": 3
    }
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
    "description": "Event receiver 'prod-webhook' created",
    "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
    "created_at": "2024-01-15T10:30:00Z",
    "payload": {
      "receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
      "name": "prod-webhook",
      "type": "webhook",
      "version": "1.0.0",
      "fingerprint": "sha256:abc123...",
      "description": "Production webhook receiver"
    }
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
    "description": "Event receiver group 'prod-cluster' created",
    "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
    "created_at": "2024-01-15T10:30:00Z",
    "payload": {
      "group_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1Y",
      "name": "prod-cluster",
      "type": "webhook-cluster",
      "version": "1.0.0",
      "description": "Production webhook cluster",
      "enabled": true,
      "receiver_ids": ["01HJ1K2M3N4P5Q6R7S8T9V0W1X"],
      "receiver_count": 1
    }
  }
}
```

### CloudEvents Field Mapping

**CloudEvents Specification 1.0.1 Fields:**

- `id`: Unique event identifier (ULID format)
- `specversion`: Always "1.0.1"
- `type`: Event type/name (e.g., "deployment.success")
- `source`: Event source (format: "xzepr.event.receiver.<receiver_id>")
- `data`: Event payload and metadata

**Custom Extension Fields (Compatible with Go Message struct):**

- `success`: Event success status (boolean)
- `api_version`: API version (always "v1")
- `name`: Event name (same as type)
- `version`: Event version
- `release`: Release identifier
- `platform_id`: Platform/environment identifier
- `package`: Package/application name

**Data Object Fields:**

- `description`: Human-readable event description
- `event_receiver_id`: Associated receiver ID
- `created_at`: Event creation timestamp (ISO 8601)
- `payload`: Arbitrary JSON payload with event-specific data

## Performance Considerations

### Async Publishing

Events are published asynchronously using `rdkafka`'s `FutureProducer`:

- Non-blocking operation
- Timeout: 5 seconds per message
- Fire-and-forget with error logging

### Message Ordering

Events are keyed by event ID, ensuring:

- Events with same ID go to same partition
- Ordering preserved within partition
- Parallel processing across partitions

### Backpressure

If Kafka is slow or unavailable:

- Events accumulate in database
- Publication failures logged
- No request blocking or queueing
- Manual replay possible via database

## Operational Considerations

### Monitoring

Monitor these metrics:

- Event creation rate (API requests)
- Kafka publication success rate
- Publication latency
- Error logs for publication failures

### Troubleshooting

**Problem**: Events not appearing in Kafka

**Checklist**:

1. Check Redpanda is running: `rpk cluster info`
2. Check topic exists: `rpk topic list`
3. Check XZepr logs for publisher initialization
4. Check XZepr logs for publication errors
5. Verify broker connectivity from XZepr container

**Problem**: System events for receivers not appearing

**Checklist**:

1. Verify receiver creation succeeded (200 response)
2. Check XZepr logs for publication attempts
3. Verify receiver ID in database matches event payload

### Recovery Procedures

**Scenario**: Kafka was down, events in database but not published

**Solution**: Implement event replay mechanism (future work):

```sql
-- Query events not yet published
SELECT * FROM events
WHERE created_at > '2024-01-15'
ORDER BY created_at;
```

Then manually publish using admin tool or API endpoint.

## Validation Results

### Code Quality

- ✅ `cargo fmt --all` passed
- ✅ `cargo check --all-targets --all-features` passed (0 errors)
- ✅ `cargo clippy --all-targets --all-features -- -D warnings` passed (0 warnings)
- ✅ `cargo test --all-features` passed (385 tests, 0 failures)

### Test Coverage

All existing tests pass without modification, demonstrating:

- Backward compatibility maintained
- No regressions introduced
- Optional publisher design works correctly

## Future Enhancements

### 1. Event Replay API

Add endpoint to replay events from database to Kafka:

```
POST /api/v1/admin/replay-events?start=<timestamp>&end=<timestamp>
```

### 2. Publisher Metrics

Add Prometheus metrics:

- `xzepr_events_published_total{status="success|failure"}`
- `xzepr_event_publish_duration_seconds`
- `xzepr_kafka_connection_status`

### 3. Dead Letter Queue

For failed publications:

- Store failed events in separate table
- Retry with exponential backoff
- Alert on persistent failures

### 4. Event Batching

Improve throughput:

- Batch multiple events in single Kafka message
- Configurable batch size and timeout
- Maintain ordering guarantees

### 5. Schema Registry Integration

Validate event schemas:

- Register CloudEvents schemas with Confluent Schema Registry
- Validate events before publishing
- Version compatibility checks
- CloudEvents schema evolution

### 6. Multi-Topic Support

Allow routing events to different topics:

- Topic per event type
- Topic per receiver type
- Topic per environment

### 7. CloudEvents Extensions

Support additional CloudEvents extensions:

- `datacontenttype`: Specify payload content type
- `dataschema`: Reference to payload schema
- `subject`: Subject of the event
- `time`: Event occurrence time (distinct from created_at)

## References

- Architecture: `docs/explanation/architecture.md`
- Kafka Topic Auto-Creation: `docs/explanation/kafka_topic_auto_creation.md`
- API Reference: `docs/reference/api_specification.md`
- CloudEvents Message Format: `src/infrastructure/messaging/cloudevents.rs`
- KafkaEventPublisher: `src/infrastructure/messaging/producer.rs`
- TopicManager: `src/infrastructure/messaging/topics.rs`
- CloudEvents Format Example: `examples/cloudevents_format.rs`
- CloudEvents Specification 1.0.1: https://github.com/cloudevents/spec/blob/v1.0.1/spec.md
- rdkafka documentation: https://docs.rs/rdkafka/

## Summary

This implementation successfully adds event publication to XZepr:

1. **CloudEvents 1.0.1 compatible** format for all Kafka messages
2. **Compatible with Go Message struct** from other systems
3. **Events published to Kafka** when posted via API
4. **System events generated** for receiver and group creation
5. **Graceful degradation** when Kafka unavailable
6. **Zero breaking changes** to existing APIs or tests
7. **Production-ready** with proper error handling and logging

The system now provides real-time event streaming in industry-standard CloudEvents format while maintaining high availability and data integrity through a database-first, best-effort publication model. All messages are serialized with the exact field names and structure expected by Go systems, ensuring cross-language compatibility.
