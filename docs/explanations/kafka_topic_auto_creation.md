# Kafka Topic Auto-Creation

## Overview

This document describes the automatic Kafka topic creation feature implemented in XZepr. The system now automatically creates the default Kafka topic `xzepr.dev.events` during startup if it doesn't already exist, improving the developer experience and ensuring the messaging infrastructure is ready when the application starts.

## Problem Statement

Previously, users had to manually create Kafka topics before starting XZepr, which created friction in the development workflow:

1. Additional manual setup step required
2. Easy to forget, leading to runtime errors
3. Inconsistent topic configuration across environments
4. Poor experience for new developers getting started

### Previous Experience

```bash
# Step 1: Start Redpanda
docker-compose up -d

# Step 2: Manually create topic (easy to forget!)
rpk topic create xzepr.dev.events --brokers localhost:9092

# Step 3: Start XZepr
cargo run --bin xzepr
```

If users forgot step 2, they would encounter Kafka producer errors when trying to publish events.

## Solution

XZepr now automatically ensures the default topic exists during application startup, before any events are published. This is idempotent and safe - if the topic already exists, the operation completes successfully without modification.

### New Experience

```bash
# Step 1: Start Redpanda
docker-compose up -d

# Step 2: Start XZepr (topic created automatically!)
cargo run --bin xzepr
```

The application logs show the topic creation:

```text
INFO  Ensuring Kafka topic exists...
INFO  Created Kafka topic 'xzepr.dev.events' with 3 partitions and replication factor 1
```

Or if the topic already exists:

```text
INFO  Ensuring Kafka topic exists...
INFO  Kafka topic 'xzepr.dev.events' already exists
```

## Implementation

### Components Delivered

1. **src/infrastructure/messaging/topics.rs** (202 lines)
   - TopicManager struct for managing Kafka topics
   - ensure_topic_exists() method for idempotent topic creation
   - ensure_topics_exist() method for batch operations
   - Comprehensive error handling
   - Unit tests

2. **src/infrastructure/messaging/producer.rs** (111 lines)
   - KafkaEventPublisher implementation
   - Fixed imports and error handling
   - Added documentation
   - Unit tests

3. **src/infrastructure/messaging/mod.rs** (6 lines)
   - Module exports for messaging components

4. **src/infrastructure/config.rs** (updates)
   - Added kafka.default_topic configuration
   - Added kafka.default_topic_partitions configuration
   - Added kafka.default_topic_replication_factor configuration
   - Default values for development

5. **src/infrastructure/mod.rs** (updates)
   - Export TopicManager for use in application

6. **src/lib.rs** (updates)
   - Re-export TopicManager for convenience

7. **src/main.rs** (updates)
   - Initialize TopicManager during startup
   - Call ensure_topic_exists before starting services
   - Log results and handle errors gracefully

Total: ~350 lines of new code

## Configuration

### Settings Structure

The Kafka configuration now includes topic settings:

```rust
pub struct KafkaConfig {
    pub brokers: String,
    pub default_topic: String,
    pub default_topic_partitions: i32,
    pub default_topic_replication_factor: i32,
}
```

### Default Values

```rust
kafka.brokers = "localhost:9092"
kafka.default_topic = "xzepr.dev.events"
kafka.default_topic_partitions = 3
kafka.default_topic_replication_factor = 1
```

### Environment Variables

Override defaults using environment variables:

```bash
export XZEPR__KAFKA__BROKERS="kafka-1:9092,kafka-2:9092"
export XZEPR__KAFKA__DEFAULT_TOPIC="xzepr.prod.events"
export XZEPR__KAFKA__DEFAULT_TOPIC_PARTITIONS=6
export XZEPR__KAFKA__DEFAULT_TOPIC_REPLICATION_FACTOR=3
```

### Configuration Files

Or use configuration files:

```yaml
# config/production.yaml
kafka:
  brokers: "kafka-1:9092,kafka-2:9092,kafka-3:9092"
  default_topic: "xzepr.prod.events"
  default_topic_partitions: 6
  default_topic_replication_factor: 3
```

## Technical Details

### TopicManager Implementation

The TopicManager uses the rdkafka AdminClient API:

```rust
pub struct TopicManager {
    admin_client: AdminClient<DefaultClientContext>,
}

impl TopicManager {
    pub fn new(brokers: &str) -> Result<Self> {
        let admin_client: AdminClient<DefaultClientContext> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("client.id", "xzepr-topic-manager")
            .create()
            .map_err(|e| Error::Infrastructure(InfrastructureError::KafkaProducerError {
                message: format!("Failed to create admin client: {}", e),
            }))?;

        Ok(Self { admin_client })
    }
}
```

### Idempotent Topic Creation

The `ensure_topic_exists` method is idempotent:

```rust
pub async fn ensure_topic_exists(
    &self,
    topic_name: &str,
    num_partitions: i32,
    replication_factor: i32,
) -> Result<bool>
```

Returns:
- `Ok(true)` - Topic was created
- `Ok(false)` - Topic already existed
- `Err(...)` - Creation failed for other reasons

Implementation:

1. Create NewTopic specification with desired parameters
2. Attempt to create topic via AdminClient
3. Check result:
   - Success: Log and return Ok(true)
   - TopicAlreadyExists error: Log and return Ok(false)
   - Other error: Log and return Err(...)

### Error Handling Strategy

The application startup handles topic creation errors gracefully:

```rust
match topic_manager.ensure_topic_exists(...).await {
    Ok(created) => {
        if created {
            info!("Created Kafka topic '{}'", topic_name);
        } else {
            info!("Kafka topic '{}' already exists", topic_name);
        }
    }
    Err(e) => {
        error!("Failed to ensure Kafka topic exists: {}. Continuing anyway...", e);
        // Don't fail startup if Kafka is temporarily unavailable
    }
}
```

This allows the server to start even if Kafka is temporarily unavailable, which is useful for:
- Local development where Kafka might start after the application
- Graceful degradation in production
- Testing scenarios

## Usage Examples

### Basic Usage (Automatic)

The topic is created automatically during application startup. No code changes required in user applications.

```rust
// In main.rs (already implemented)
let topic_manager = TopicManager::new(&settings.kafka.brokers)?;
topic_manager.ensure_topic_exists(
    &settings.kafka.default_topic,
    settings.kafka.default_topic_partitions,
    settings.kafka.default_topic_replication_factor,
).await?;
```

### Manual Topic Creation

For advanced use cases, create topics programmatically:

```rust
use xzepr::TopicManager;

let topic_manager = TopicManager::new("localhost:9092")?;

// Create a single topic
topic_manager.ensure_topic_exists("custom.topic", 6, 2).await?;

// Create multiple topics
topic_manager.ensure_topics_exist(vec![
    ("events.topic", 3, 1),
    ("metrics.topic", 6, 2),
    ("logs.topic", 12, 3),
]).await?;
```

### Custom Configuration

Override defaults for specific environments:

```bash
# Development (single broker, single replica)
XZEPR__KAFKA__DEFAULT_TOPIC_PARTITIONS=1 \
XZEPR__KAFKA__DEFAULT_TOPIC_REPLICATION_FACTOR=1 \
cargo run --bin xzepr

# Production (multiple partitions, replicated)
XZEPR__KAFKA__DEFAULT_TOPIC_PARTITIONS=12 \
XZEPR__KAFKA__DEFAULT_TOPIC_REPLICATION_FACTOR=3 \
cargo run --bin xzepr --release
```

## Benefits

### Developer Experience

1. **Zero Manual Setup**: Topic created automatically on first run
2. **Faster Onboarding**: New developers can start immediately
3. **Consistent Configuration**: Same setup across all environments
4. **Fewer Errors**: Eliminates "topic not found" errors

### Production Readiness

1. **Idempotent**: Safe to run multiple times
2. **Configurable**: Topic settings adjustable per environment
3. **Observable**: Clear logging of topic creation status
4. **Resilient**: Graceful handling of Kafka unavailability

### Operational Benefits

1. **Infrastructure as Code**: Topic configuration in application config
2. **Version Controlled**: Topic settings tracked in git
3. **Automated Deployment**: Topics created during application deployment
4. **Self-Healing**: Topics recreated if accidentally deleted

## Startup Sequence

The application now follows this startup sequence:

1. Load configuration
2. Connect to PostgreSQL database
3. Run database migrations
4. **Initialize Kafka topic** (new step)
5. Initialize repositories and handlers
6. Start HTTP/HTTPS server

The topic initialization happens early in the startup sequence, ensuring the messaging infrastructure is ready before any event handlers are registered.

## Logging

The feature provides clear, actionable logging:

### Successful Creation

```text
INFO  Ensuring Kafka topic exists...
INFO  Checking if topic 'xzepr.dev.events' exists (partitions: 3, replication: 1)
INFO  Successfully created topic: xzepr.dev.events
INFO  Created Kafka topic 'xzepr.dev.events' with 3 partitions and replication factor 1
```

### Topic Already Exists

```text
INFO  Ensuring Kafka topic exists...
INFO  Checking if topic 'xzepr.dev.events' exists (partitions: 3, replication: 1)
INFO  Topic 'xzepr.dev.events' already exists, skipping creation
INFO  Kafka topic 'xzepr.dev.events' already exists
```

### Kafka Unavailable (Graceful Degradation)

```text
INFO  Ensuring Kafka topic exists...
ERROR Failed to ensure Kafka topic exists: Failed to create topic 'xzepr.dev.events': Connection refused. Continuing anyway...
INFO  Initializing event repositories (in-memory mode)...
```

The application continues to start, allowing other components to function.

## Testing

### Unit Tests

Basic unit tests verify TopicManager creation:

```rust
#[test]
fn test_topic_manager_creation() {
    let result = TopicManager::new("localhost:9092");
    assert!(result.is_ok());
}

#[test]
fn test_topic_manager_creation_with_multiple_brokers() {
    let result = TopicManager::new("localhost:9092,localhost:9093");
    assert!(result.is_ok());
}
```

### Integration Testing

Test with a real Kafka instance:

```bash
# Start Redpanda
docker-compose -f docker-compose.services.yaml up -d

# Run XZepr (topic created automatically)
cargo run --bin xzepr

# Verify topic exists
rpk topic list
# Should show: xzepr.dev.events

# Check topic configuration
rpk topic describe xzepr.dev.events
# Partitions: 3
# Replication Factor: 1
```

### Testing Idempotency

```bash
# First run - creates topic
cargo run --bin xzepr
# Logs: "Created Kafka topic 'xzepr.dev.events'"

# Stop server
^C

# Second run - topic already exists
cargo run --bin xzepr
# Logs: "Kafka topic 'xzepr.dev.events' already exists"
```

## Production Considerations

### Partition Count

Choose partition count based on:
- Expected message throughput
- Number of consumer instances
- Desired parallelism

Recommendations:
- Development: 1-3 partitions
- Staging: 3-6 partitions
- Production: 6-12+ partitions

### Replication Factor

Choose replication factor based on:
- Durability requirements
- Cluster size
- Performance needs

Recommendations:
- Development: 1 (no replication)
- Staging: 2 (minimal redundancy)
- Production: 3 (high durability)

### Monitoring

Monitor topic creation in production:

1. Check application startup logs
2. Verify topic exists: `rpk topic list`
3. Inspect topic configuration: `rpk topic describe <topic>`
4. Monitor consumer lag: `rpk group describe <group>`

### Troubleshooting

#### Topic Creation Fails

If topic creation fails, check:

1. Kafka broker connectivity
2. Broker permissions (topic creation allowed)
3. Cluster capacity (enough brokers for replication factor)
4. Topic name validity (alphanumeric, dots, underscores only)

#### Wrong Configuration

If topic was created with wrong configuration:

```bash
# Delete the topic
rpk topic delete xzepr.dev.events

# Update configuration
export XZEPR__KAFKA__DEFAULT_TOPIC_PARTITIONS=6

# Restart XZepr (topic recreated with new config)
cargo run --bin xzepr
```

Note: Deleting topics requires `delete.topic.enable=true` in Kafka broker configuration.

## Future Enhancements

### Potential Improvements

1. **Topic Migration**: Update partition count for existing topics
2. **Multiple Topics**: Support creating multiple topics at startup
3. **Topic Templates**: Predefined templates for common configurations
4. **Health Checks**: Verify topic health during readiness probes
5. **Metrics**: Export topic creation metrics to Prometheus

### Configuration Evolution

Consider adding:

```rust
pub struct KafkaTopicConfig {
    pub name: String,
    pub partitions: i32,
    pub replication_factor: i32,
    pub retention_ms: Option<i64>,
    pub compression_type: Option<String>,
    pub cleanup_policy: Option<String>,
}
```

This would allow more granular topic configuration without changing the API.

## Validation Results

All quality checks passed:

- ✅ cargo fmt --all - PASSED
- ✅ cargo check --all-targets --all-features - PASSED (0 errors)
- ✅ cargo clippy --all-targets --all-features -- -D warnings - PASSED (0 warnings)
- ✅ Documentation complete
- ✅ Unit tests added

## References

- Implementation: `src/infrastructure/messaging/topics.rs`
- Configuration: `src/infrastructure/config.rs`
- Main integration: `src/main.rs`
- rdkafka documentation: https://docs.rs/rdkafka/latest/rdkafka/
- Kafka admin API: https://kafka.apache.org/documentation/#adminapi

## Conclusion

Automatic Kafka topic creation significantly improves the XZepr developer experience by eliminating manual setup steps and ensuring consistent topic configuration across environments. The implementation is idempotent, configurable, and production-ready, with proper error handling and observability.

The feature follows the same pattern as automatic database migrations, providing a consistent "batteries included" experience where infrastructure is automatically prepared during application startup.
