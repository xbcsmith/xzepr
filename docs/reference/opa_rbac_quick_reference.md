# OPA RBAC Quick Reference Guide

## Overview

Quick reference for developers implementing OPA-based authorization in XZepr.

## Core Concepts

### Resource Ownership

Every resource has an `owner_id` field:

```rust
pub struct EventReceiver {
    id: EventReceiverId,
    owner_id: UserId,  // Creator of the resource
    // ... other fields
}
```

### Group Membership

EventReceiverGroups have members who can POST events:

```rust
pub struct EventReceiverGroup {
    id: EventReceiverGroupId,
    owner_id: UserId,
    members: Vec<UserId>,  // Users who can POST to receivers in this group
    // ... other fields
}
```

### Authorization Flow

1. User authenticated via JWT
2. Request intercepted by OPA middleware
3. Resource context built (owner, members, version)
4. OPA evaluates policy (with cache check)
5. Decision logged to audit trail
6. Request allowed or denied

## Common Patterns

### Pattern 1: Check Ownership

```rust
use crate::auth::opa::{AuthorizationRequest, OpaClient};

// In your handler
let auth_req = AuthorizationRequest {
    user: user.to_opa_user(),
    action: "update".to_string(),
    resource: ResourceContext {
        resource_type: "event_receiver".to_string(),
        resource_id: receiver.id().to_string(),
        owner_id: Some(receiver.owner_id()),
        group_id: None,
        members: vec![],
        version: receiver.version(),
    },
};

let decision = opa_client.evaluate_policy_cached(auth_req).await?;
if !decision {
    return Err(StatusCode::FORBIDDEN);
}
```

### Pattern 2: Check Group Membership

```rust
// Get group members
let members = group_repo.get_members(group_id).await?;

let auth_req = AuthorizationRequest {
    user: user.to_opa_user(),
    action: "event:create".to_string(),
    resource: ResourceContext {
        resource_type: "event_receiver".to_string(),
        resource_id: receiver.id().to_string(),
        owner_id: Some(receiver.owner_id()),
        group_id: Some(group.id()),
        members,  // OPA checks if user in members list
        version: receiver.version(),
    },
};
```

### Pattern 3: Fallback to Legacy RBAC

```rust
// Define fallback using existing role-based permissions
let fallback_fn = || {
    user.roles.iter().any(|r| r.has_permission(&Permission::EventCreate))
};

// Evaluate with fallback
let decision = opa_client
    .evaluate_with_fallback(auth_req, fallback_fn)
    .await?;
```

### Pattern 4: Invalidate Cache on Update

```rust
// In repository update method
async fn update(&self, receiver: &EventReceiver) -> Result<()> {
    // Update database (this increments version)
    sqlx::query("UPDATE event_receivers SET ... WHERE id = $1")
        .bind(receiver.id())
        .execute(&self.pool)
        .await?;

    // Invalidate cache
    if let Some(cache) = &self.cache {
        cache.invalidate_resource("event_receiver", &receiver.id().to_string()).await;
    }

    Ok(())
}
```

## OPA Policy Structure

### Main Authorization Policy

Location: `config/opa/policies/rbac.rego`

```rego
package xzepr.authz

# Default deny
default allow = false

# Admin can do everything
allow if {
    "admin" in input.user.roles
}

# Owner can modify their resources
allow if {
    input.action in ["update", "delete"]
    input.resource.owner_id == input.user.user_id
}

# Group members can create events
allow if {
    input.action == "event:create"
    input.user.user_id in input.resource.members
}
```

### Policy Input Format

```json
{
  "user": {
    "user_id": "01HXXX...",
    "username": "joe",
    "roles": ["user"],
    "permissions": []
  },
  "action": "event:create",
  "resource": {
    "resource_type": "event_receiver",
    "resource_id": "01HYYY...",
    "owner_id": "01HXXX...",
    "group_id": "01HZZZ...",
    "members": ["01HXXX...", "01HAAA..."],
    "version": 5
  }
}
```

## REST API Endpoints

### Group Membership Management

```bash
# Add member to group
POST /api/v1/groups/:group_id/members
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "user_id": "01HXXX..."
}

# Remove member from group
DELETE /api/v1/groups/:group_id/members/:user_id
Authorization: Bearer <jwt_token>

# List group members
GET /api/v1/groups/:group_id/members
Authorization: Bearer <jwt_token>
```

### Creating Resources with Ownership

```bash
# Create event receiver (owner_id set from JWT claims)
POST /api/v1/receivers
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "name": "Joe's Receiver",
  "type": "webhook",
  "version": "1.0.0",
  "description": "My event receiver",
  "schema": {}
}

# POST event (checks group membership)
POST /api/v1/events
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "name": "deployment.success",
  "event_receiver_id": "01HYYY...",
  "payload": {},
  ...
}
```

## GraphQL Operations

### Query Group Members

```graphql
query GetGroupMembers($groupId: ID!) {
  groupMembers(groupId: $groupId) {
    id
    username
    email
  }
}
```

### Add Group Member

```graphql
mutation AddMember($groupId: ID!, $userId: ID!) {
  addGroupMember(groupId: $groupId, userId: $userId) {
    success
    message
    group {
      id
      name
      members
    }
  }
}
```

## Configuration

### Development Configuration

File: `config/development.yaml`

```yaml
opa:
  enabled: true
  url: "http://localhost:8181"
  timeout_seconds: 5
  policy_path: "/v1/data/xzepr/authz/allow"
  cache_ttl_seconds: 300
```

### Production Configuration

File: `config/production.yaml`

```yaml
opa:
  enabled: true
  url: "${OPA_URL}"
  timeout_seconds: 10
  policy_path: "/v1/data/xzepr/authz/allow"
  bundle_url: "${OPA_BUNDLE_URL}"
  cache_ttl_seconds: 300
```

## Testing

### Unit Test: Check Authorization

```rust
#[tokio::test]
async fn test_owner_can_update_receiver() {
    let owner_id = UserId::new();
    let receiver = EventReceiver::new_with_owner(
        "Test".to_string(),
        "webhook".to_string(),
        owner_id,
    ).unwrap();

    let auth_req = AuthorizationRequest {
        user: UserContext {
            user_id: owner_id.to_string(),
            username: "owner".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
        },
        action: "update".to_string(),
        resource: ResourceContext {
            resource_type: "event_receiver".to_string(),
            resource_id: receiver.id().to_string(),
            owner_id: Some(owner_id),
            group_id: None,
            members: vec![],
            version: 1,
        },
    };

    let decision = opa_client.evaluate_policy(auth_req).await.unwrap();
    assert!(decision);
}
```

### Integration Test: API Authorization

```rust
#[tokio::test]
async fn test_non_member_cannot_post_event() {
    let app = create_test_app().await;

    let non_member_token = create_test_token("non_member", vec!["user"]);

    let response = app
        .post("/api/v1/events")
        .header("Authorization", format!("Bearer {}", non_member_token))
        .json(&create_event_request())
        .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
```

## Troubleshooting

### OPA Not Responding

**Symptom**: Requests failing with 500 Internal Server Error

**Check**:

```bash
# Verify OPA is running
curl http://localhost:8181/health

# Check circuit breaker state
curl http://localhost:9090/metrics | grep xzepr_opa_circuit_breaker_state
```

**Solution**: System should automatically fallback to legacy RBAC. Check logs:

```bash
docker logs xzepr-opa
docker logs xzepr-api | grep "fallback to legacy RBAC"
```

### Cache Not Invalidating

**Symptom**: Stale authorization decisions after resource updates

**Check**:

```bash
# Check cache metrics
curl http://localhost:9090/metrics | grep xzepr_opa_cache
```

**Solution**: Manually clear cache:

```bash
# Via API (admin only)
curl -X POST http://localhost:8080/api/v1/admin/cache/clear \
  -H "Authorization: Bearer <admin_token>"
```

### Policy Evaluation Errors

**Symptom**: OPA returns 500 or evaluation errors

**Check**: Validate policies locally:

```bash
cd config/opa/policies
opa check *.rego
opa test .
```

**Debug**: Query OPA directly:

```bash
curl -X POST http://localhost:8181/v1/data/xzepr/authz/allow \
  -H "Content-Type: application/json" \
  -d @test_input.json
```

## Performance Guidelines

### Expected Latencies

- **Cache Hit**: less than 1ms
- **Cache Miss + OPA Eval**: 10-30ms
- **Database Ownership Query**: 5-15ms
- **Total P95**: less than 50ms

### Optimization Tips

1. Always include resource version in cache key
2. Batch membership queries where possible
3. Use database indexes on owner_id columns
4. Monitor cache hit rate (target greater than 70 percent)
5. Set appropriate TTL (default 5 minutes)

### Monitoring Queries

```promql
# Authorization request rate
rate(xzepr_opa_authorization_requests_total[5m])

# Cache hit rate
rate(xzepr_opa_cache_hits_total[5m]) /
  (rate(xzepr_opa_cache_hits_total[5m]) + rate(xzepr_opa_cache_misses_total[5m]))

# Fallback rate (should be near zero)
rate(xzepr_opa_fallback_to_rbac_total[5m])
```

## Common Errors

### Error: "Resource not found"

User trying to access non-existent resource.

```rust
Err(AuthError::ResourceNotFound)
```

**Solution**: Return 404 Not Found before authorization check.

### Error: "Circuit breaker open"

OPA unavailable, system in fallback mode.

```rust
Err(CircuitBreakerError::CircuitOpen)
```

**Solution**: System automatically using legacy RBAC. Monitor OPA service.

### Error: "Invalid cache key"

Resource version missing or invalid.

```rust
Err(CacheError::InvalidKey)
```

**Solution**: Ensure all resources track version field.

## Security Best Practices

1. **Never log sensitive data** in authorization decisions
2. **Always validate input** before OPA evaluation
3. **Use HTTPS** for OPA communication in production
4. **Rotate bundle server tokens** regularly
5. **Monitor failed authorization attempts** for suspicious activity
6. **Implement rate limiting** on authorization endpoints
7. **Review policies** in code review process
8. **Test policies** before deployment

## References

- **Detailed Implementation Plan**: `docs/explanation/opa_rbac_expansion_plan.md`
- **Implementation Summary**: `docs/explanation/opa_rbac_expansion_summary.md`
- **Architecture Documentation**: `docs/explanation/architecture.md`
- **OPA Documentation**: https://www.openpolicyagent.org/docs/latest/
- **Rego Language Reference**: https://www.openpolicyagent.org/docs/latest/policy-language/

---

**Last Updated**: 2025-01-XX
**Version**: 1.0
