# OPA Authorization Architecture

## Overview

This document describes the architecture and design of the Open Policy Agent (OPA) based authorization system in XZepr. The system provides fine-grained, policy-based access control for event receivers, events, and group memberships with support for ownership, group-based permissions, and role-based access control (RBAC).

## Architecture Goals

The OPA authorization architecture is designed to achieve:

1. **Separation of Concerns**: Authorization logic is decoupled from application code and stored as declarative policies
2. **Flexibility**: Policies can be updated without code changes or redeployment
3. **Performance**: Caching and circuit breaker patterns minimize latency impact
4. **Auditability**: All authorization decisions are logged for compliance and security analysis
5. **Resilience**: Fallback mechanisms ensure availability during OPA outages
6. **Scalability**: Stateless design allows horizontal scaling

## High-Level Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                         API Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   REST API   │  │  GraphQL API │  │   gRPC API   │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                 │                  │                   │
│         └─────────────────┼──────────────────┘                   │
│                           ▼                                      │
│                  ┌─────────────────┐                            │
│                  │ OPA Middleware  │                            │
│                  └────────┬────────┘                            │
└───────────────────────────┼─────────────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Authorization Layer                           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   OPA Client                              │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────┐ │  │
│  │  │   Cache    │  │  Circuit   │  │   Metrics &        │ │  │
│  │  │            │  │  Breaker   │  │   Tracing          │ │  │
│  │  └────────────┘  └────────────┘  └────────────────────┘ │  │
│  └──────────────────────┬───────────────────────────────────┘  │
│                         ▼                                       │
│              ┌──────────────────────┐                          │
│              │  Fallback Handler    │                          │
│              │  (Legacy RBAC)       │                          │
│              └──────────────────────┘                          │
└───────────────────────┬─────────────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                   OPA Server (External)                         │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  Policy Engine                            │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────┐ │  │
│  │  │   Rego     │  │   Bundle   │  │   Decision Logs    │ │  │
│  │  │  Policies  │  │   Loader   │  │                    │ │  │
│  │  └────────────┘  └────────────┘  └────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Infrastructure Layer                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │  PostgreSQL  │  │  Audit Log   │  │   Prometheus         │ │
│  │  (Context)   │  │  Storage     │  │   Metrics            │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. OPA Middleware

The OPA middleware intercepts all API requests and enforces authorization before handlers execute.

**Location**: `src/api/middleware/opa_authorization.rs`

**Responsibilities**:

- Extract user identity from JWT token
- Build resource context from request path and body
- Call OPA client to evaluate authorization
- Log authorization decisions
- Return 403 Forbidden for denied requests
- Record metrics for monitoring

**Flow**:

```text
Request → Extract User → Build Context → OPA Evaluate → Allow/Deny → Handler
                                              ↓
                                        Audit Log + Metrics
```

### 2. OPA Client

The OPA client manages communication with the OPA server and implements resilience patterns.

**Location**: `src/infrastructure/opa/client.rs`

**Responsibilities**:

- Send authorization requests to OPA via HTTP
- Implement caching with TTL and invalidation
- Implement circuit breaker for fault tolerance
- Fallback to legacy RBAC on failure
- Record metrics and traces
- Handle OPA response parsing

**Key Features**:

- **Request Caching**: Authorization decisions are cached to reduce OPA load
- **Circuit Breaker**: Automatically fails fast during OPA outages
- **Graceful Degradation**: Falls back to legacy RBAC when OPA is unavailable
- **Timeout Handling**: Configurable request timeouts prevent hanging requests

### 3. Resource Context Builders

Context builders extract resource information needed for authorization decisions.

**Location**: `src/api/middleware/resource_context.rs`

**Responsibilities**:

- Load resource details from database (owner, group, members)
- Build structured context for OPA input
- Handle missing or invalid resources
- Cache resource lookups when possible

**Supported Resources**:

- Event Receivers
- Event Receiver Groups
- Events

### 4. Authorization Cache

The authorization cache stores recent authorization decisions to improve performance.

**Location**: `src/infrastructure/opa/cache.rs`

**Responsibilities**:

- Cache authorization decisions with TTL
- Invalidate cache on resource updates
- Track cache hit/miss metrics
- Evict expired entries periodically

**Cache Key Structure**:

```rust
CacheKey {
    user_id: UserId,
    action: String,
    resource_type: String,
    resource_id: String,
    resource_version: Option<i64>,
}
```

**Invalidation Strategy**:

- Invalidate on resource ownership change
- Invalidate on group membership change
- Invalidate on resource deletion
- Automatic TTL expiration (default: 5 minutes)

### 5. Circuit Breaker

The circuit breaker protects the system from cascading failures when OPA is unavailable.

**Location**: `src/infrastructure/opa/circuit_breaker.rs`

**States**:

1. **Closed**: Normal operation, requests flow to OPA
2. **Open**: Too many failures, all requests fail fast
3. **Half-Open**: Testing recovery, limited requests allowed

**Configuration**:

- Failure threshold: 5 consecutive failures
- Timeout duration: 30 seconds
- Success threshold to close: 2 consecutive successes

**Behavior**:

```text
Closed ──(5 failures)──> Open ──(30s timeout)──> Half-Open
  ↑                                                   │
  └──────────────(2 successes)─────────────────────┘
```

### 6. Fallback Handler

The fallback handler provides authorization when OPA is unavailable.

**Location**: `src/api/middleware/opa_authorization.rs::legacy_rbac_check()`

**Fallback Rules**:

- Admins have full access to all resources
- Owners have full access to their resources
- Other users are denied by default

**Important**: Fallback is intentionally conservative to maintain security during outages.

## Authorization Flow

### Standard Authorization Flow

```text
1. API Request arrives
   ↓
2. Middleware extracts user from JWT
   ↓
3. Middleware identifies resource from path/body
   ↓
4. Context builder loads resource details
   ↓
5. Check authorization cache
   ├─ Cache Hit → Return cached decision
   └─ Cache Miss → Continue
   ↓
6. Call OPA client
   ↓
7. Circuit breaker checks state
   ├─ Open → Fallback to legacy RBAC
   └─ Closed/Half-Open → Continue
   ↓
8. Send HTTP request to OPA
   ↓
9. OPA evaluates Rego policies
   ↓
10. OPA returns decision (allow/deny)
    ↓
11. Cache decision
    ↓
12. Log audit event
    ↓
13. Record metrics
    ↓
14. Return result to middleware
    ↓
15. Allow request or return 403
```

### Fallback Flow (OPA Unavailable)

```text
1. OPA request fails (timeout, connection error, etc.)
   ↓
2. Circuit breaker records failure
   ↓
3. Check failure threshold
   ├─ Threshold exceeded → Open circuit
   └─ Below threshold → Continue
   ↓
4. Invoke fallback handler
   ↓
5. Check if user is admin
   ├─ Yes → Allow
   └─ No → Continue
   ↓
6. Check if user is resource owner
   ├─ Yes → Allow
   └─ No → Deny
   ↓
7. Log fallback decision
   ↓
8. Record fallback metric
   ↓
9. Return result
```

## Policy Structure

### Policy Organization

Policies are organized by resource type:

- `authz.rego` - Main authorization entry point
- `event_receiver.rego` - Event receiver permissions
- `event_receiver_group.rego` - Group permissions
- `event.rego` - Event permissions
- `helpers.rego` - Shared utility functions

### Policy Decision Point

All authorization requests evaluate:

```rego
allow {
    # Entry point for all authorization decisions
    input.user
    input.action
    input.resource

    # Delegate to resource-specific rules
    resource_allows
}
```

### Input Structure

OPA receives authorization requests in this format:

```json
{
  "input": {
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "roles": ["owner"],
      "permissions": ["event_receiver:read"]
    },
    "action": "event_receiver:read",
    "resource": {
      "type": "event_receiver",
      "id": "770e8400-e29b-41d4-a716-446655440000",
      "owner_id": "550e8400-e29b-41d4-a716-446655440000",
      "group_id": "880e8400-e29b-41d4-a716-446655440000",
      "members": [
        "550e8400-e29b-41d4-a716-446655440000",
        "990e8400-e29b-41d4-a716-446655440000"
      ]
    }
  }
}
```

### Policy Rules

#### Admin Rule

Admins have access to all resources:

```rego
allow {
    input.user.roles[_] == "admin"
}
```

#### Ownership Rule

Owners have full access to their resources:

```rego
allow {
    input.user.id == input.resource.owner_id
    startswith(input.action, input.resource.type)
}
```

#### Group Membership Rule

Group members have read and create access:

```rego
allow {
    input.resource.members[_] == input.user.id
    input.action in ["event_receiver:read", "event:read", "event:create"]
}
```

#### Permission-Based Rule

Check user has required permission:

```rego
allow {
    required := required_permissions[input.action]
    input.user.permissions[_] == required
}
```

## Cache Invalidation Strategy

### Cache Invalidation Events

The system invalidates caches when resources change:

1. **Event Receiver Updated**: Invalidate all decisions for that receiver
2. **Group Membership Changed**: Invalidate all decisions for group members
3. **Resource Deleted**: Invalidate all decisions for that resource
4. **Ownership Transferred**: Invalidate decisions for old and new owner

### Invalidation Implementation

```rust
pub enum ResourceUpdatedEvent {
    EventReceiverUpdated {
        receiver_id: UserId,
        version: i64,
    },
    EventReceiverGroupUpdated {
        group_id: GroupId,
        version: i64,
    },
    EventUpdated {
        event_id: EventId,
        version: i64,
    },
}

impl AuthorizationCache {
    pub async fn invalidate_resource(&self, event: ResourceUpdatedEvent) {
        match event {
            ResourceUpdatedEvent::EventReceiverUpdated { receiver_id, .. } => {
                self.cache.retain(|k, _| {
                    k.resource_id != receiver_id.to_string()
                }).await;
            }
            // ... other cases
        }
    }
}
```

## Observability

### Metrics

The system exposes Prometheus metrics:

- `opa_authorization_requests_total` - Total authorization requests by result
- `opa_authorization_duration_seconds` - Authorization latency histogram
- `opa_authorization_denials_total` - Denied requests by resource type
- `opa_cache_hits_total` - Cache hit counter
- `opa_cache_misses_total` - Cache miss counter
- `opa_fallback_total` - Fallback invocations by reason
- `opa_circuit_breaker_state` - Circuit breaker state gauge

### Audit Logging

All authorization decisions are logged:

```rust
audit_logger.log_authorization_decision(
    user_id,
    action,
    resource_type,
    resource_id,
    decision,  // "allow" or "deny"
    reason,    // "opa", "cache", "fallback"
    latency_ms,
).await;
```

Audit logs include:

- Timestamp
- User ID
- Action attempted
- Resource type and ID
- Decision (allow/deny)
- Reason (OPA, cache, fallback)
- Latency
- Request ID for correlation

### Tracing

OpenTelemetry spans track authorization flow:

```rust
let span = create_authorization_span(user_id, action, resource_type);
span.set_attribute("cache_hit", cache_hit);
span.set_attribute("decision", decision);
span.set_attribute("latency_ms", latency);
```

Traces enable:

- Identifying slow authorization requests
- Correlating authorization with business logic
- Debugging authorization failures
- Understanding cache effectiveness

## Security Considerations

### Defense in Depth

Multiple security layers protect the system:

1. **Authentication**: JWT token validation before authorization
2. **Authorization**: OPA policy evaluation
3. **Fallback**: Conservative deny-by-default fallback
4. **Audit**: Comprehensive logging of all decisions
5. **Rate Limiting**: Prevent abuse of authorization endpoints

### Policy Security

Policy security best practices:

- **Principle of Least Privilege**: Grant minimum necessary permissions
- **Default Deny**: Deny access unless explicitly allowed
- **Input Validation**: Validate all policy inputs
- **Policy Testing**: Comprehensive test coverage for all rules
- **Policy Review**: Peer review all policy changes

### Cache Security

Cache security measures:

- **Version Tracking**: Include resource version in cache keys
- **TTL Limits**: Aggressive TTL prevents stale decisions
- **Invalidation**: Immediate invalidation on resource changes
- **No Sensitive Data**: Cache only decisions, not resource details

## Performance Optimization

### Caching Strategy

The caching strategy balances performance and consistency:

- **Default TTL**: 5 minutes for most decisions
- **Aggressive Invalidation**: Invalidate immediately on changes
- **Version-Based Keys**: Include resource version to prevent stale data
- **Selective Caching**: Cache only successful decisions

### Batching

For bulk operations, consider batching authorization checks:

```rust
let decisions = opa_client
    .evaluate_batch(user_id, actions, resources)
    .await?;
```

Benefits:

- Reduced network roundtrips
- Lower latency for bulk operations
- Better resource utilization

### Database Optimization

Context building is optimized:

- **Connection Pooling**: Reuse database connections
- **Selective Loading**: Load only needed resource fields
- **Query Optimization**: Use indexes on owner_id and group_id
- **Caching**: Cache frequently accessed resource contexts

## Configuration

### OPA Client Configuration

```yaml
opa:
  enabled: true
  url: "http://localhost:8181"
  timeout_seconds: 5
  policy_path: "/v1/data/xzepr/authz/allow"
  bundle_url: "http://bundle-server:8888/xzepr-policies.tar.gz"
  cache_ttl_seconds: 300
```

### Circuit Breaker Configuration

```rust
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,      // Default: 5
    pub timeout_duration: Duration,  // Default: 30s
    pub half_open_max_calls: u32,    // Default: 3
}
```

### Cache Configuration

```rust
pub struct CacheConfig {
    pub default_ttl: Duration,       // Default: 5 minutes
    pub max_entries: usize,          // Default: 10,000
    pub eviction_interval: Duration, // Default: 1 minute
}
```

## Deployment Considerations

### High Availability

For production deployments:

1. **Multiple OPA Instances**: Deploy OPA behind a load balancer
2. **Bundle Replication**: Replicate bundle server for redundancy
3. **Cache Warming**: Pre-populate caches after deployments
4. **Graceful Degradation**: Ensure fallback mechanism is tested

### Scaling

Scaling strategies:

- **Horizontal Scaling**: Add more OPA instances as load increases
- **Cache Sizing**: Increase cache size for high-traffic applications
- **Database Optimization**: Optimize context queries for performance
- **CDN Distribution**: Use CDN for bundle distribution

### Monitoring

Monitor these key metrics:

- Authorization latency p50, p95, p99
- Cache hit rate (target: >80%)
- Fallback invocation rate (target: <1%)
- Circuit breaker state changes
- Policy evaluation errors

### Disaster Recovery

Disaster recovery procedures:

1. **OPA Outage**: Fallback activates automatically
2. **Bundle Server Outage**: OPA uses cached bundle
3. **Database Outage**: Authorization fails (cannot build context)
4. **Network Partition**: Circuit breaker opens, fallback activates

## Migration Strategy

### Phase 1: Parallel Operation

Run OPA alongside existing authorization:

- OPA evaluates but doesn't enforce
- Log differences between OPA and legacy
- Fix policy inconsistencies

### Phase 2: Shadow Mode

OPA enforces but with fallback:

- OPA becomes primary authorization
- Fallback to legacy on OPA failure
- Monitor closely for issues

### Phase 3: Full Deployment

OPA is the only authorization mechanism:

- Remove legacy authorization code
- Optimize for performance
- Full production monitoring

## Testing Strategy

### Unit Tests

Test individual components:

- OPA client request/response handling
- Cache operations and invalidation
- Circuit breaker state transitions
- Context builders with mock data

### Integration Tests

Test end-to-end flows:

- Authorization middleware with real OPA
- Cache invalidation on resource updates
- Fallback activation on OPA failure
- Audit logging and metrics recording

### Policy Tests

Test Rego policies:

```rego
test_owner_can_read {
    allow with input as {
        "user": {"id": "user1"},
        "action": "event_receiver:read",
        "resource": {"owner_id": "user1"}
    }
}

test_non_owner_cannot_delete {
    not allow with input as {
        "user": {"id": "user2"},
        "action": "event_receiver:delete",
        "resource": {"owner_id": "user1"}
    }
}
```

### Load Tests

Test system under load:

- Authorization throughput (target: >1000 req/s)
- Cache effectiveness under load
- OPA failure recovery time
- Database query performance

## Troubleshooting

### Authorization Denied Unexpectedly

1. Check audit logs for denial reason
2. Verify user has correct roles/permissions
3. Test policy with OPA CLI
4. Check resource ownership in database

### High Authorization Latency

1. Check OPA server health and latency
2. Verify cache hit rate is acceptable
3. Optimize context builder queries
4. Consider increasing cache TTL

### Cache Inconsistency

1. Check cache invalidation is triggered
2. Verify resource version tracking
3. Reduce cache TTL temporarily
4. Clear cache and rebuild

### Circuit Breaker Stuck Open

1. Check OPA server health
2. Verify network connectivity
3. Review failure threshold configuration
4. Manually close circuit if needed

## Related Documentation

- [Group Membership API Reference](../reference/group_membership_api.md)
- [OPA Bundle Server Setup Guide](../how-to/opa_bundle_server_setup.md)
- [OPA Policy Development Guide](../how-to/opa_policy_development.md)
- [Phase 5 Audit Monitoring Implementation](phase5_audit_monitoring_implementation.md)

## References

- Open Policy Agent Documentation: https://www.openpolicyagent.org/docs/latest/
- Rego Language Reference: https://www.openpolicyagent.org/docs/latest/policy-language/
- OPA REST API: https://www.openpolicyagent.org/docs/latest/rest-api/
- Circuit Breaker Pattern: https://martinfowler.com/bliki/CircuitBreaker.html
