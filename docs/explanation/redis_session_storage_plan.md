# Redis Session Storage Implementation Plan

## Overview

This document outlines the plan for implementing Redis-based session storage for OIDC authentication flows in XZepr. The current implementation uses an in-memory HashMap which is not suitable for production as it:

- Does not scale across multiple instances
- Loses sessions on server restart
- Has no automatic expiration mechanism
- Cannot be shared across load-balanced services

## Current State

The OIDC authentication flow currently stores session data in-memory:

```rust
pub struct AuthState {
    pub jwt_service: Arc<JwtService>,
    pub oidc_client: Option<Arc<OidcClient>>,
    pub oidc_callback_handler: Option<Arc<OidcCallbackHandler>>,
    pub session_store: Arc<std::sync::RwLock<std::collections::HashMap<String, OidcSession>>>,
}
```

Session data includes:
- State parameter (CSRF protection)
- PKCE verifier
- Nonce (for ID token validation)
- Original redirect URL

## Requirements

### Functional Requirements

1. Store OIDC session data with automatic expiration
2. Support distributed deployment (multiple service instances)
3. Persist sessions across server restarts
4. Automatic cleanup of expired sessions
5. Support for JWT token blacklist (future requirement)

### Non-Functional Requirements

1. Low latency (sub-10ms for read/write operations)
2. High availability (Redis cluster or sentinel)
3. Secure connection (TLS in production)
4. Configurable TTL for sessions
5. Connection pooling for performance

## Architecture

### Components

#### 1. Redis Session Store

```rust
pub struct RedisSessionStore {
    client: redis::Client,
    connection_manager: redis::aio::ConnectionManager,
    key_prefix: String,
    default_ttl: Duration,
}
```

Responsibilities:
- Serialize/deserialize session data
- Set TTL on session keys
- Handle Redis connection management
- Provide error handling and retry logic

#### 2. Session Store Trait

Define a trait to abstract storage implementation:

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn store(&self, key: &str, session: OidcSession, ttl: Duration) -> Result<(), SessionError>;
    async fn retrieve(&self, key: &str) -> Result<Option<OidcSession>, SessionError>;
    async fn remove(&self, key: &str) -> Result<(), SessionError>;
    async fn exists(&self, key: &str) -> Result<bool, SessionError>;
}
```

This allows for:
- In-memory implementation (development/testing)
- Redis implementation (production)
- Mock implementation (unit tests)

#### 3. Updated AuthState

```rust
pub struct AuthState {
    pub jwt_service: Arc<JwtService>,
    pub oidc_client: Option<Arc<OidcClient>>,
    pub oidc_callback_handler: Option<Arc<OidcCallbackHandler>>,
    pub session_store: Arc<dyn SessionStore>,
}
```

### Redis Key Structure

Use namespaced keys for organization:

```
xzepr:session:{state_value}     # OIDC session data
xzepr:blacklist:{jti}            # JWT token blacklist (future)
xzepr:refresh:{user_id}:{jti}   # Refresh token metadata (future)
```

### Data Serialization

Use JSON for session data:
- Human-readable for debugging
- Easy to inspect with Redis CLI
- Compatible with existing OidcSession struct (derives Serialize/Deserialize)

Alternative: MessagePack for better performance and smaller size

## Implementation Plan

### Phase 1: Infrastructure Setup

1. Add Redis configuration to `src/infrastructure/config.rs`
   - Redis URL/host/port
   - TLS settings
   - Connection pool size
   - Default TTL values

2. Create `src/infrastructure/redis/mod.rs`
   - Redis client initialization
   - Connection pool management
   - Health check endpoint

3. Create `src/infrastructure/redis/session_store.rs`
   - Implement RedisSessionStore struct
   - Connection management
   - Error handling

### Phase 2: Session Store Abstraction

1. Create `src/auth/session/mod.rs`
   - Define SessionStore trait
   - Define SessionError types

2. Create `src/auth/session/memory.rs`
   - Implement InMemorySessionStore
   - Use for development/testing

3. Create `src/auth/session/redis.rs`
   - Implement RedisSessionStore
   - Use SessionStore trait

### Phase 3: Integration

1. Update `src/api/rest/auth.rs`
   - Replace HashMap with SessionStore trait
   - Update oidc_login to use session_store.store()
   - Update oidc_callback to use session_store.retrieve() and session_store.remove()

2. Update application bootstrap
   - Initialize Redis client based on configuration
   - Create appropriate SessionStore implementation
   - Pass to AuthState

3. Add session cleanup background task
   - Optional: Redis handles TTL automatically
   - For in-memory: periodic cleanup of expired sessions

### Phase 4: JWT Blacklist Integration

1. Extend SessionStore trait for blacklist operations
2. Implement blacklist methods in RedisSessionStore
3. Update JwtService to use Redis-backed blacklist
4. Migrate from in-memory TokenBlacklist

## Configuration

### Environment Variables

```bash
REDIS_URL=redis://localhost:6379
REDIS_TLS_ENABLED=false
REDIS_PASSWORD=secret
REDIS_DATABASE=0
REDIS_POOL_SIZE=10
REDIS_TIMEOUT_SECONDS=5
SESSION_TTL_SECONDS=600
```

### YAML Configuration

```yaml
redis:
  url: "redis://localhost:6379"
  tls:
    enabled: false
  password: null
  database: 0
  pool:
    size: 10
    timeout_seconds: 5

session:
  ttl_seconds: 600  # 10 minutes
  key_prefix: "xzepr:session:"

jwt_blacklist:
  ttl_seconds: 86400  # 24 hours
  key_prefix: "xzepr:blacklist:"
```

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Redis connection error: {0}")]
    ConnectionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Session not found")]
    NotFound,

    #[error("Session expired")]
    Expired,

    #[error("Invalid session data: {0}")]
    InvalidData(String),
}
```

### Retry Strategy

- Connection errors: Retry up to 3 times with exponential backoff
- Timeout errors: Log and return error (don't retry to avoid cascading delays)
- Serialization errors: Don't retry (indicates bug)

## Testing Strategy

### Unit Tests

1. Test SessionStore implementations (in-memory and Redis)
   - Store and retrieve operations
   - TTL expiration
   - Key deletion
   - Error handling

2. Test serialization/deserialization
   - Valid session data
   - Invalid/corrupted data
   - Edge cases (empty fields, long strings)

### Integration Tests

1. Redis integration tests
   - Use testcontainers-rs for Redis instance
   - Test full OIDC flow with Redis sessions
   - Test session expiration
   - Test concurrent access

2. End-to-end tests
   - Complete auth flow with Redis
   - Multiple concurrent logins
   - Session cleanup

### Performance Tests

1. Benchmark session operations
   - Store: target < 5ms
   - Retrieve: target < 2ms
   - Delete: target < 2ms

2. Load testing
   - 1000 concurrent sessions
   - Measure latency percentiles (p50, p95, p99)

## Security Considerations

### Data Protection

1. Use TLS for Redis connections in production
2. Set strong Redis password
3. Use Redis ACLs to limit permissions
4. Encrypt sensitive session data fields (optional)

### Session Security

1. Use cryptographically secure random values for state/nonce
2. Set appropriate TTL (10 minutes recommended)
3. Validate state parameter before session retrieval
4. Delete session immediately after successful callback

### Production Deployment

1. Use Redis Sentinel or Redis Cluster for high availability
2. Enable Redis persistence (AOF or RDB)
3. Monitor Redis memory usage
4. Set maxmemory-policy to allkeys-lru
5. Enable Redis slowlog for debugging

## Monitoring and Observability

### Metrics

Track the following Prometheus metrics:

```
xzepr_session_store_operations_total{operation="store|retrieve|delete", status="success|error"}
xzepr_session_store_operation_duration_seconds{operation="store|retrieve|delete"}
xzepr_redis_connection_errors_total
xzepr_redis_pool_size{state="active|idle"}
xzepr_sessions_active_count
```

### Logging

Log the following events:

- Session creation (debug level)
- Session retrieval success/failure (debug level)
- Redis connection errors (error level)
- Session expiration (info level)
- Serialization errors (error level)

### Alerts

Set up alerts for:

- Redis connection failures (critical)
- High session operation latency (warning)
- Redis memory usage > 80% (warning)
- Session store error rate > 1% (warning)

## Migration Path

### Phase 1: Parallel Run (Optional)

Run both in-memory and Redis stores in parallel:
- Write to both stores
- Read from Redis, fall back to memory
- Compare results for validation

### Phase 2: Redis Primary

Switch to Redis as primary store:
- Write only to Redis
- Remove in-memory store

### Phase 3: Cleanup

Remove in-memory session store code

## Rollback Plan

If Redis integration causes issues:

1. Feature flag to switch back to in-memory store
2. Configuration option to disable Redis
3. Keep in-memory implementation as fallback

## Dependencies

### Rust Crates

Already in Cargo.toml:
- `redis = { version = "0.32.7", features = ["tokio-comp", "connection-manager"] }`

May need to add:
- `redis = { version = "0.32.7", features = ["tokio-comp", "connection-manager", "tls-rustls"] }` (for TLS support)

### Infrastructure

Production deployment requires:
- Redis 6.0+ (for ACL support)
- Redis Sentinel or Redis Cluster (for HA)
- TLS certificates (for secure connection)

## Timeline Estimate

- Phase 1 (Infrastructure Setup): 2-3 days
- Phase 2 (Session Store Abstraction): 2-3 days
- Phase 3 (Integration): 2-3 days
- Phase 4 (JWT Blacklist): 2-3 days
- Testing and Documentation: 2-3 days

Total: 10-15 days

## References

- Redis official documentation: https://redis.io/docs
- redis-rs crate: https://docs.rs/redis
- OIDC specification: https://openid.net/specs/openid-connect-core-1_0.html
- XZepr architecture: `docs/explanation/architecture.md`

## Success Criteria

Implementation is considered successful when:

1. All unit tests pass with >80% coverage
2. Integration tests pass with real Redis instance
3. Session operations have <10ms latency at p95
4. No session data loss during normal operation
5. Clean failover during Redis restart
6. Documentation complete with examples
7. All quality checks pass (fmt, clippy, test)

## Future Enhancements

1. Redis Streams for real-time session monitoring
2. Session analytics (login patterns, geographic distribution)
3. Rate limiting using Redis counters
4. Distributed locking for concurrent operations
5. Multi-region session replication
