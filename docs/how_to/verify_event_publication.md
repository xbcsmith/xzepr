# How to Verify Event Publication

## Overview

This guide shows you how to verify that XZepr is correctly publishing events to Kafka/Redpanda when events are created via the API.

## Prerequisites

- XZepr running with Redpanda/Kafka
- `rpk` CLI installed (Redpanda CLI)
- `curl` or similar HTTP client

## Quick Verification Steps

### 1. Start Kafka Consumer

Open a terminal and start consuming from the XZepr events topic:

```bash
rpk topic consume xzepr.dev.events --brokers localhost:9092
```

Leave this running to see events as they arrive.

### 2. Create an Event Receiver

In another terminal, create an event receiver:

```bash
curl -X POST http://localhost:8080/api/v1/event-receivers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-receiver",
    "type": "webhook",
    "version": "1.0.0",
    "description": "Test receiver for verification",
    "schema": {
      "type": "object",
      "properties": {
        "message": {"type": "string"}
      }
    }
  }'
```

**Expected Result**: You should see a system event in the consumer terminal with type `xzepr.event.receiver.created`.

**Example Output** (CloudEvents 1.0.1 format):

```json
{
  "partition": 0,
  "offset": 0,
  "timestamp": "2024-01-15T10:30:00Z",
  "key": "01HJ1K2M3N4P5Q6R7S8T9V0W2B",
  "value": {
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
      "description": "Event receiver 'test-receiver' created",
      "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
      "created_at": "2024-01-15T10:30:00Z",
      "payload": {
        "receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
        "name": "test-receiver",
        "type": "webhook",
        "version": "1.0.0",
        "fingerprint": "sha256:...",
        "description": "Test receiver for verification"
      }
    }
  }
}
```

### 3. Post an Event

Using the receiver ID from step 2 (extract from response), post an event:

```bash
curl -X POST http://localhost:8080/api/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test.event",
    "version": "1.0.0",
    "release": "1.0.0",
    "platform_id": "test-platform",
    "package": "test-package",
    "description": "Test event for verification",
    "payload": {
      "message": "Hello from XZepr!"
    },
    "success": true,
    "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X"
  }'
```

**Expected Result**: You should see the event in the consumer terminal.

**Example Output** (CloudEvents 1.0.1 format):

```json
{
  "partition": 0,
  "offset": 1,
  "timestamp": "2024-01-15T10:31:00Z",
  "key": "01HJ1K2M3N4P5Q6R7S8T9V0W1Z",
  "value": {
    "success": true,
    "id": "01HJ1K2M3N4P5Q6R7S8T9V0W1Z",
    "specversion": "1.0.1",
    "type": "test.event",
    "source": "xzepr.event.receiver.01HJ1K2M3N4P5Q6R7S8T9V0W1X",
    "api_version": "v1",
    "name": "test.event",
    "version": "1.0.0",
    "release": "1.0.0",
    "platform_id": "test-platform",
    "package": "test-package",
    "data": {
      "description": "Test event for verification",
      "event_receiver_id": "01HJ1K2M3N4P5Q6R7S8T9V0W1X",
      "created_at": "2024-01-15T10:31:00Z",
      "payload": {
        "message": "Hello from XZepr!"
      }
    }
  }
}
```

### 4. Create an Event Receiver Group

Create a group with the receiver:

```bash
curl -X POST http://localhost:8080/api/v1/event-receiver-groups \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-group",
    "type": "webhook-cluster",
    "version": "1.0.0",
    "description": "Test group for verification",
    "enabled": true,
    "event_receiver_ids": ["01HJ1K2M3N4P5Q6R7S8T9V0W1X"]
  }'
```

**Expected Result**: You should see a system event with type `xzepr.event.receiver.group.created`.

## Checking Logs

XZepr logs provide visibility into event publication:

### Successful Publication

```bash
docker logs xzepr-app 2>&1 | grep "published"
```

**Expected Output**:

```
INFO Event published to Kafka successfully event_id=01HJ1K2M3N4P5Q6R7S8T9V0W1Z
INFO Receiver creation event published to Kafka successfully receiver_id=01HJ1K2M3N4P5Q6R7S8T9V0W1X event_id=01HJ1K2M3N4P5Q6R7S8T9V0W2B
INFO Group creation event published to Kafka successfully group_id=01HJ1K2M3N4P5Q6R7S8T9V0W1Y event_id=01HJ1K2M3N4P5Q6R7S8T9V0W2C
```

### Failed Publication

If Kafka is unavailable:

```bash
docker logs xzepr-app 2>&1 | grep -i kafka
```

**Expected Output** (when Kafka down):

```
WARN Failed to initialize Kafka event publisher: Failed to create Kafka producer: ...
WARN Event publisher not configured, skipping Kafka publication
```

**Expected Output** (runtime failure):

```
ERROR Failed to publish event to Kafka (event was saved to database) event_id=... error=...
```

## Troubleshooting

### No Events Appearing in Kafka

**Problem**: Consumer shows no messages

**Solutions**:

1. Check Redpanda is running:

   ```bash
   docker ps | grep redpanda
   ```

2. Check topic exists:

   ```bash
   rpk topic list --brokers localhost:9092
   ```

3. Check XZepr logs for errors:

   ```bash
   docker logs xzepr-app 2>&1 | grep -i "kafka\|publisher"
   ```

4. Verify network connectivity:
   ```bash
   docker exec xzepr-app nc -zv redpanda 9092
   ```

### Events in Database but Not Kafka

**Problem**: API requests succeed but no Kafka messages

**Check**:

1. XZepr logs show publisher initialization:

   ```bash
   docker logs xzepr-app 2>&1 | grep "Kafka event publisher initialized"
   ```

2. If not initialized, check Kafka broker configuration in `config/default.yaml`:

   ```yaml
   kafka:
     brokers: "redpanda:9092" # Use correct hostname
     default_topic: "xzepr.dev.events"
   ```

3. Restart XZepr after configuration changes:
   ```bash
   docker-compose restart xzepr-app
   ```

### Wrong Topic

**Problem**: Events going to wrong topic

**Check configuration**:

```bash
cat config/default.yaml | grep -A 5 "kafka:"
```

**Update if needed**:

```yaml
kafka:
  brokers: "localhost:9092"
  default_topic: "xzepr.dev.events" # Change this
  default_topic_partitions: 3
  default_topic_replication_factor: 1
```

## Advanced Verification

### Check Message Count

Count total messages in topic:

```bash
rpk topic describe xzepr.dev.events --brokers localhost:9092
```

### Consume Specific Offset Range

View messages from specific offset:

```bash
rpk topic consume xzepr.dev.events --offset start --brokers localhost:9092
```

### Filter by Event Type

Use `jq` to filter system events (CloudEvents format):

```bash
rpk topic consume xzepr.dev.events --brokers localhost:9092 --format json | \
  jq 'select(.value.type | startswith("xzepr.event.receiver"))'
```

### Check Event Ordering

Verify events are in correct order by timestamp:

```bash
rpk topic consume xzepr.dev.events --brokers localhost:9092 --format json | \
  jq '.value.data.created_at' | sort
```

## Production Checklist

Before going to production, verify:

- [ ] Kafka/Redpanda cluster is running and accessible
- [ ] Topic `xzepr.dev.events` (or configured name) exists
- [ ] XZepr logs show successful publisher initialization
- [ ] Test event can be created and appears in Kafka
- [ ] Test receiver creation generates system event
- [ ] Test group creation generates system event
- [ ] Failed Kafka connection doesn't prevent API requests
- [ ] Monitoring/alerting configured for publication failures
- [ ] Network policies allow XZepr → Kafka communication

## CloudEvents Format

All messages published to Kafka follow CloudEvents 1.0.1 specification:

- `success`: Event success status (extension)
- `id`: Unique event ID (CloudEvents)
- `specversion`: Always "1.0.1" (CloudEvents)
- `type`: Event type/name (CloudEvents)
- `source`: Event source URI (CloudEvents)
- `api_version`: API version (extension)
- `name`: Event name (extension)
- `version`: Event version (extension)
- `release`: Release identifier (extension)
- `platform_id`: Platform identifier (extension)
- `package`: Package name (extension)
- `data`: Event payload with description, event_receiver_id, created_at, and payload (CloudEvents)

This format ensures compatibility with other systems expecting CloudEvents format or the Go Message struct.

## Summary

Event publication is working correctly if:

1. XZepr starts without errors
2. Creating receivers generates `xzepr.event.receiver.created` events in CloudEvents format
3. Creating groups generates `xzepr.event.receiver.group.created` events in CloudEvents format
4. Posting events publishes them to Kafka in CloudEvents format
5. Events appear in Kafka topic within seconds
6. Logs show successful publication messages
7. All messages have CloudEvents 1.0.1 structure with `specversion`, `type`, `source`, and `data` fields

If any step fails, check logs and configuration as described in the troubleshooting section.

## See Also

- Event Publication Implementation: `docs/explanation/event_publication_implementation.md`
- CloudEvents Format Example: `examples/cloudevents_format.rs` (run with `cargo run --example cloudevents_format`)
- CloudEvents Specification: https://github.com/cloudevents/spec/blob/v1.0.1/spec.md
- Kafka Topic Auto-Creation: `docs/explanation/kafka_topic_auto_creation.md`
- Docker Demo Tutorial: `docs/tutorials/docker_demo.md`
- API Reference: `docs/reference/api_specification.md`
