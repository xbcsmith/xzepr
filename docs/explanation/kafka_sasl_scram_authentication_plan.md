# Kafka SASL/SCRAM Authentication Implementation Plan

## Overview

This document outlines the implementation plan for adding SASL/SCRAM authentication support to XZepr's Kafka integration. Currently, the `KafkaEventPublisher` and `TopicManager` only support unauthenticated connections, which is not suitable for production environments.

## Current State

### Existing Implementation

The current Kafka integration (`src/infrastructure/messaging/producer.rs` and `src/infrastructure/messaging/topics.rs`) only configures:

- `bootstrap.servers` - Broker addresses
- `message.timeout.ms` - Message timeout
- `client.id` - Client identifier

**No security protocol, SASL mechanism, or authentication credentials are configured.**

### Limitations

- Cannot connect to production Kafka clusters requiring authentication
- No support for secure communication (SSL/TLS)
- Not compliant with enterprise security requirements
- Limited to development/local Kafka instances only

## Goals

1. Add support for SASL/SCRAM-SHA-256 and SCRAM-SHA-512 authentication
2. Support SASL/PLAIN for development environments
3. Enable SSL/TLS encryption for secure communication
4. Maintain backward compatibility with existing unauthenticated connections
5. Provide flexible configuration via environment variables and YAML
6. Follow security best practices for credential management
7. Provide comprehensive documentation and examples

## Implementation Plan

### Phase 1: Configuration Structure

**Objective**: Create type-safe configuration structures for Kafka authentication.

#### Task 1.1: Create Configuration Module

**File**: `src/infrastructure/messaging/config.rs`

**Components**:

```rust
// Core types
pub enum SecurityProtocol {
    Plaintext,
    Ssl,
    SaslPlaintext,
    SaslSsl,
}

pub enum SaslMechanism {
    Plain,
    ScramSha256,
    ScramSha512,
    Gssapi,
    OAuthBearer,
}

pub struct SaslConfig {
    pub mechanism: SaslMechanism,
    pub username: String,
    pub password: String,
}

pub struct KafkaAuthConfig {
    pub security_protocol: SecurityProtocol,
    pub sasl_config: Option<SaslConfig>,
    pub ssl_ca_location: Option<String>,
    pub ssl_certificate_location: Option<String>,
    pub ssl_key_location: Option<String>,
}
```

**Key Methods**:

- `KafkaAuthConfig::default()` - Returns plaintext (no auth) configuration
- `KafkaAuthConfig::scram_sha256(username, password)` - SCRAM-SHA-256 helper
- `KafkaAuthConfig::scram_sha512(username, password)` - SCRAM-SHA-512 helper
- `KafkaAuthConfig::sasl_plain(username, password)` - SASL/PLAIN helper
- `KafkaAuthConfig::from_env()` - Load configuration from environment variables
- `KafkaAuthConfig::apply_to_client_config(&self, config)` - Apply settings to rdkafka ClientConfig

**Environment Variables**:

- `KAFKA_SECURITY_PROTOCOL` - Security protocol (PLAINTEXT, SSL, SASL_PLAINTEXT, SASL_SSL)
- `KAFKA_SASL_MECHANISM` - SASL mechanism (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512)
- `KAFKA_SASL_USERNAME` - SASL username
- `KAFKA_SASL_PASSWORD` - SASL password
- `KAFKA_SSL_CA_LOCATION` - Path to CA certificate
- `KAFKA_SSL_CERT_LOCATION` - Path to client certificate (optional)
- `KAFKA_SSL_KEY_LOCATION` - Path to client key (optional)

**Estimated Lines**: ~350 lines (including documentation and tests)

#### Task 1.2: Update Module Exports

**File**: `src/infrastructure/messaging/mod.rs`

Add:
```rust
pub mod config;
```

**Estimated Lines**: 1 line change

---

### Phase 2: Update Producer and Topic Manager

**Objective**: Integrate authentication configuration into existing Kafka clients.

#### Task 2.1: Update KafkaEventPublisher

**File**: `src/infrastructure/messaging/producer.rs`

**Changes**:

1. Import `KafkaAuthConfig`
2. Add `with_auth()` constructor that accepts `Option<KafkaAuthConfig>`
3. Keep existing `new()` for backward compatibility (calls `with_auth(None)`)
4. Apply auth config to `ClientConfig` before creating producer

**Signature Changes**:

```rust
// New method
pub fn with_auth(
    brokers: &str,
    topic: &str,
    auth_config: Option<KafkaAuthConfig>,
) -> Result<Self>

// Existing method (backward compatible)
pub fn new(brokers: &str, topic: &str) -> Result<Self> {
    Self::with_auth(brokers, topic, None)
}
```

**Implementation Pattern**:

```rust
let mut config = ClientConfig::new();
config
    .set("bootstrap.servers", brokers)
    .set("message.timeout.ms", "5000")
    .set("client.id", "xzepr-event-publisher");

// Apply authentication if provided
if let Some(auth) = auth_config {
    auth.apply_to_client_config(&mut config);
}

let producer: FutureProducer = config.create()?;
```

**Estimated Lines**: ~30 lines changed, ~50 lines added (with documentation)

#### Task 2.2: Update TopicManager

**File**: `src/infrastructure/messaging/topics.rs`

**Changes**: Same pattern as `KafkaEventPublisher`

1. Add `with_auth()` constructor
2. Keep existing `new()` for backward compatibility
3. Apply auth config to admin client

**Estimated Lines**: ~25 lines changed, ~40 lines added (with documentation)

---

### Phase 3: Configuration Loading

**Objective**: Enable loading authentication configuration from multiple sources.

#### Task 3.1: Environment Variable Loading

**File**: `src/infrastructure/messaging/config.rs` (extend)

Add `KafkaAuthConfig::from_env()` method:

```rust
pub fn from_env() -> Result<Option<Self>> {
    // Read KAFKA_SECURITY_PROTOCOL
    // If not set or PLAINTEXT, return None
    // Parse SASL mechanism and credentials
    // Parse SSL certificate paths
    // Return configured KafkaAuthConfig
}
```

**Error Handling**:

- Missing required variables (username/password when SASL enabled) → return Error
- Missing optional variables (SSL certs) → continue with None
- Invalid enum values → return Error with descriptive message

**Estimated Lines**: ~80 lines (including error handling and validation)

#### Task 3.2: YAML Configuration Support

**Files**:
- `config/development.yaml`
- `config/production.yaml`
- `config/test.yaml`

Add `kafka.auth` section:

```yaml
kafka:
  brokers: "kafka.example.com:9093"
  topic: "xzepr.prod.events"
  auth:
    security_protocol: "SASL_SSL"
    sasl:
      mechanism: "SCRAM-SHA-256"
      username: "${KAFKA_USERNAME}"  # Environment variable reference
      password: "${KAFKA_PASSWORD}"
    ssl:
      ca_location: "/etc/kafka/certs/ca-cert.pem"
```

**Estimated Lines**: ~15 lines per config file

#### Task 3.3: Update Main Configuration Structure

**File**: `src/infrastructure/config.rs` (if exists) or relevant config loading code

Add authentication config to main application configuration:

```rust
pub struct KafkaConfig {
    pub brokers: String,
    pub topic: String,
    pub auth: Option<KafkaAuthConfig>,
}
```

**Estimated Lines**: ~20 lines

---

### Phase 4: Testing

**Objective**: Ensure authentication works correctly and existing functionality is preserved.

#### Task 4.1: Unit Tests for Configuration

**File**: `src/infrastructure/messaging/config.rs` (test module)

**Test Cases**:

1. `test_default_is_plaintext()` - Verify default is no auth
2. `test_scram_sha256_config()` - Verify SCRAM-SHA-256 helper
3. `test_scram_sha512_config()` - Verify SCRAM-SHA-512 helper
4. `test_sasl_plain_config()` - Verify SASL/PLAIN helper
5. `test_apply_to_client_config()` - Verify config application doesn't panic
6. `test_from_env_with_scram()` - Verify env var loading (requires env setup)
7. `test_from_env_missing_password()` - Verify error on missing credentials
8. `test_security_protocol_to_string()` - Verify enum mappings
9. `test_sasl_mechanism_to_string()` - Verify mechanism mappings

**Estimated Lines**: ~200 lines

#### Task 4.2: Unit Tests for Producer Updates

**File**: `src/infrastructure/messaging/producer.rs` (test module)

**Test Cases**:

1. `test_kafka_publisher_creation()` - Existing test (ensure still passes)
2. `test_kafka_publisher_with_auth()` - Create with auth config
3. `test_kafka_publisher_with_scram_sha256()` - SCRAM-SHA-256 specific
4. `test_kafka_publisher_with_scram_sha512()` - SCRAM-SHA-512 specific
5. `test_backward_compatibility()` - Verify `new()` still works

**Estimated Lines**: ~80 lines

#### Task 4.3: Unit Tests for TopicManager Updates

**File**: `src/infrastructure/messaging/topics.rs` (test module)

**Test Cases**: Same pattern as producer tests

**Estimated Lines**: ~70 lines

#### Task 4.4: Integration Tests

**File**: `tests/kafka_auth_integration.rs` (new)

**Test Cases** (require running Kafka with SASL):

1. `test_producer_connection_with_scram()` - Connect and send message
2. `test_admin_client_with_scram()` - Create topic with auth
3. `test_invalid_credentials()` - Verify proper error on bad credentials

**Feature Flag**: `#[cfg(feature = "integration_tests")]`

**Estimated Lines**: ~150 lines

---

### Phase 5: Documentation

**Objective**: Provide comprehensive documentation for users and operators.

#### Task 5.1: How-To Guide

**File**: `docs/how_to/configure_kafka_authentication.md`

**Sections**:

1. Overview of supported mechanisms
2. Environment variable configuration
3. YAML configuration
4. Security best practices
5. Verification steps
6. Troubleshooting common issues

**Estimated Lines**: ~250 lines

#### Task 5.2: Update Existing Documentation

**Files**:
- `docs/explanation/event_publication_implementation.md`
- `README.md`

**Changes**:

- Add authentication section
- Update examples to show both authenticated and unauthenticated usage
- Add security considerations
- Link to new authentication guide

**Estimated Lines**: ~50 lines added across files

#### Task 5.3: Example Code

**File**: `examples/kafka_with_auth.rs`

**Content**:

```rust
// Demonstrates:
// 1. Creating publisher with SCRAM-SHA-256
// 2. Publishing events
// 3. Error handling
// 4. Loading config from environment
```

**Estimated Lines**: ~120 lines

---

### Phase 6: Security Considerations

**Objective**: Ensure implementation follows security best practices.

#### Task 6.1: Credential Protection

**Requirements**:

1. Never log passwords or credentials
2. Clear sensitive data from memory when possible
3. Use secure string types if available
4. Document credential rotation procedures

**Implementation**:

- Implement `Debug` for `SaslConfig` that redacts password
- Add `zeroize` dependency for clearing sensitive data
- Add security warnings to documentation

**Estimated Lines**: ~30 lines

#### Task 6.2: Security Documentation

**File**: `docs/explanation/kafka_security_best_practices.md`

**Sections**:

1. Credential management
2. Network security (SSL/TLS)
3. ACL configuration in Kafka
4. Monitoring and auditing
5. Incident response procedures

**Estimated Lines**: ~200 lines

---

### Phase 7: Migration and Rollout

**Objective**: Enable safe migration from unauthenticated to authenticated connections.

#### Task 7.1: Migration Guide

**File**: `docs/how_to/migrate_to_kafka_authentication.md`

**Sections**:

1. Pre-migration checklist
2. Step-by-step migration procedure
3. Rollback procedures
4. Verification steps
5. Common migration issues

**Estimated Lines**: ~150 lines

#### Task 7.2: Feature Flag (Optional)

If gradual rollout is needed:

**File**: `src/infrastructure/messaging/mod.rs`

Add feature flag:
```rust
#[cfg(feature = "kafka_auth")]
pub mod config;
```

Update `Cargo.toml`:
```toml
[features]
kafka_auth = []
```

**Estimated Lines**: ~10 lines

---

## Implementation Checklist

### Phase 1: Configuration Structure
- [ ] Create `src/infrastructure/messaging/config.rs`
- [ ] Implement `SecurityProtocol` enum
- [ ] Implement `SaslMechanism` enum
- [ ] Implement `SaslConfig` struct
- [ ] Implement `KafkaAuthConfig` struct
- [ ] Implement `KafkaAuthConfig::default()`
- [ ] Implement `KafkaAuthConfig::scram_sha256()`
- [ ] Implement `KafkaAuthConfig::scram_sha512()`
- [ ] Implement `KafkaAuthConfig::sasl_plain()`
- [ ] Implement `KafkaAuthConfig::apply_to_client_config()`
- [ ] Update `src/infrastructure/messaging/mod.rs`
- [ ] Add unit tests for configuration
- [ ] Run `cargo fmt --all`
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo test --all-features`

### Phase 2: Update Producer and Topic Manager
- [ ] Update `KafkaEventPublisher::new()` (backward compatible)
- [ ] Add `KafkaEventPublisher::with_auth()`
- [ ] Update `TopicManager::new()` (backward compatible)
- [ ] Add `TopicManager::with_auth()`
- [ ] Update doc comments with authentication examples
- [ ] Add unit tests for producer with auth
- [ ] Add unit tests for topic manager with auth
- [ ] Run `cargo fmt --all`
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo test --all-features`

### Phase 3: Configuration Loading
- [ ] Implement `KafkaAuthConfig::from_env()`
- [ ] Add environment variable parsing
- [ ] Add error handling for missing required variables
- [ ] Update `config/development.yaml`
- [ ] Update `config/production.yaml`
- [ ] Update `config/test.yaml`
- [ ] Update main application config structure
- [ ] Add unit tests for environment loading
- [ ] Run `cargo fmt --all`
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo test --all-features`

### Phase 4: Testing
- [ ] Write unit tests for `config.rs` (9 tests)
- [ ] Write unit tests for producer updates (5 tests)
- [ ] Write unit tests for topic manager updates (5 tests)
- [ ] Create integration tests (optional, requires Kafka)
- [ ] Verify all existing tests still pass
- [ ] Achieve >80% code coverage for new code
- [ ] Run `cargo test --all-features -- --nocapture`
- [ ] Verify test count increased

### Phase 5: Documentation
- [ ] Create `docs/how_to/configure_kafka_authentication.md`
- [ ] Update `docs/explanation/event_publication_implementation.md`
- [ ] Update `README.md` with authentication section
- [ ] Create `examples/kafka_with_auth.rs`
- [ ] Add inline code documentation
- [ ] Verify no emojis in documentation
- [ ] Verify all filenames are lowercase with underscores

### Phase 6: Security
- [ ] Implement `Debug` for `SaslConfig` (redact password)
- [ ] Add security warnings to documentation
- [ ] Create `docs/explanation/kafka_security_best_practices.md`
- [ ] Document credential rotation procedures
- [ ] Add security checklist to documentation

### Phase 7: Migration and Rollout
- [ ] Create `docs/how_to/migrate_to_kafka_authentication.md`
- [ ] Document rollback procedures
- [ ] Create deployment checklist
- [ ] Plan staged rollout strategy

### Final Validation
- [ ] Run `cargo fmt --all`
- [ ] Run `cargo check --all-targets --all-features`
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings` (zero warnings)
- [ ] Run `cargo test --all-features` (>80% coverage)
- [ ] Verify all documentation created
- [ ] Verify all filenames follow conventions
- [ ] Create implementation summary document

---

## File Summary

### New Files

1. `src/infrastructure/messaging/config.rs` (~350 lines)
2. `examples/kafka_with_auth.rs` (~120 lines)
3. `tests/kafka_auth_integration.rs` (~150 lines)
4. `docs/how_to/configure_kafka_authentication.md` (~250 lines)
5. `docs/how_to/migrate_to_kafka_authentication.md` (~150 lines)
6. `docs/explanation/kafka_security_best_practices.md` (~200 lines)
7. `docs/explanation/kafka_sasl_scram_authentication_plan.md` (this document, ~500 lines)

**Total New Lines**: ~1,720 lines

### Modified Files

1. `src/infrastructure/messaging/mod.rs` (~1 line)
2. `src/infrastructure/messaging/producer.rs` (~80 lines changed/added)
3. `src/infrastructure/messaging/topics.rs` (~65 lines changed/added)
4. `config/development.yaml` (~15 lines)
5. `config/production.yaml` (~15 lines)
6. `config/test.yaml` (~15 lines)
7. `docs/explanation/event_publication_implementation.md` (~30 lines)
8. `README.md` (~20 lines)

**Total Modified Lines**: ~241 lines

### Grand Total: ~1,961 lines

---

## Estimated Effort

### By Phase

- **Phase 1**: Configuration Structure - 4 hours
- **Phase 2**: Update Producer/Manager - 3 hours
- **Phase 3**: Configuration Loading - 2 hours
- **Phase 4**: Testing - 4 hours
- **Phase 5**: Documentation - 3 hours
- **Phase 6**: Security - 2 hours
- **Phase 7**: Migration - 1 hour

**Total Estimated Time**: 19 hours

### By Task Type

- Implementation: 9 hours
- Testing: 4 hours
- Documentation: 6 hours

---

## Dependencies

### Crates Required

Current dependencies (no new crates required):
- `rdkafka` - Already included, supports SASL/SCRAM
- `serde` - Already included, for configuration
- `thiserror` - Already included, for error handling

Optional (for enhanced security):
- `zeroize` - For clearing sensitive data from memory
- `secrecy` - For protecting secret strings

### Kafka Version Compatibility

SASL/SCRAM support requires:
- Kafka 0.10.0+ for SCRAM-SHA-256
- Kafka 2.0.0+ for SCRAM-SHA-512
- Redpanda (all versions) - Fully supports SCRAM

---

## Risk Assessment

### Low Risk

- Backward compatibility maintained (existing code continues to work)
- Configuration is optional (defaults to no auth)
- Well-tested rdkafka library handles authentication

### Medium Risk

- Environment variable typos could cause runtime failures
- Incorrect SASL configuration may lock out all connections
- Password exposure if logging is misconfigured

### Mitigation Strategies

1. **Configuration Validation**: Validate auth config on startup, fail fast
2. **Comprehensive Testing**: Unit tests + integration tests + manual verification
3. **Secure Defaults**: Redact passwords in Debug output
4. **Clear Documentation**: Step-by-step guides with examples
5. **Staged Rollout**: Test in dev → staging → production
6. **Monitoring**: Add metrics for authentication failures

---

## Success Criteria

### Functional Requirements

- [ ] Producer can connect to Kafka with SCRAM-SHA-256 authentication
- [ ] Producer can connect to Kafka with SCRAM-SHA-512 authentication
- [ ] Topic manager can connect with authentication
- [ ] Configuration can be loaded from environment variables
- [ ] Configuration can be loaded from YAML files
- [ ] Existing unauthenticated connections still work

### Quality Requirements

- [ ] All code passes `cargo fmt --all`
- [ ] All code passes `cargo clippy -- -D warnings` (zero warnings)
- [ ] All tests pass with >80% coverage
- [ ] Documentation complete for all new features
- [ ] Examples demonstrate common use cases

### Security Requirements

- [ ] Passwords never logged or exposed in debug output
- [ ] SSL/TLS supported for encrypted connections
- [ ] Clear documentation on credential management
- [ ] Security best practices documented

---

## Future Enhancements

Not in scope for this phase but documented for future consideration:

1. **OAuth/OIDC Authentication**: Support for OAUTHBEARER mechanism
2. **Kerberos (GSSAPI)**: Support for enterprise authentication
3. **Certificate-Based Authentication**: Client certificates for mTLS
4. **Secret Manager Integration**: Fetch credentials from Vault/AWS Secrets Manager
5. **Dynamic Credential Rotation**: Support for rotating credentials without restart
6. **Metrics and Monitoring**: Expose authentication success/failure metrics
7. **Connection Pooling**: Reuse authenticated connections across requests

---

## References

### External Documentation

- [Apache Kafka SASL/SCRAM Documentation](https://kafka.apache.org/documentation/#security_sasl_scram)
- [rdkafka Configuration Reference](https://github.com/edenhill/librdkafka/blob/master/CONFIGURATION.md)
- [CloudEvents Security](https://github.com/cloudevents/spec/blob/main/cloudevents/primer.md#security)

### Internal Documentation

- `docs/explanation/architecture.md` - XZepr architecture overview
- `docs/explanation/event_publication_implementation.md` - Event publishing
- `docs/how_to/setup_monitoring.md` - Monitoring and observability
- `AGENTS.md` - Development guidelines and standards

---

## Conclusion

This implementation plan provides a comprehensive roadmap for adding SASL/SCRAM authentication to XZepr's Kafka integration. The phased approach ensures:

- Backward compatibility with existing deployments
- Security best practices from the start
- Comprehensive testing and documentation
- Clear migration path for existing users
- Foundation for future authentication enhancements

Total estimated effort is 19 hours, with the majority focused on implementation and testing. The modular design allows for incremental implementation and testing of each phase.

---

**Document Status**: Ready for Implementation
**Last Updated**: 2024
**Prepared By**: AI Agent
**Approved By**: Pending Review
