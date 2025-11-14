# Audit Logs Reference

## Overview

XZepr emits structured audit logs for all security-relevant events. These logs are designed for compliance, forensics, and security monitoring. Logs are emitted in JSON format to stdout/stderr for collection by centralized logging systems.

## Log Format

### Standard Fields

All audit events contain the following fields:

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `level` | string | Log level (INFO, WARN, ERROR) | `"INFO"` |
| `event_type` | string | Always "audit" for audit events | `"audit"` |
| `app` | string | Application name | `"xzepr"` |
| `env` | string | Environment (production, staging, development) | `"production"` |
| `timestamp` | ISO 8601 | Event timestamp in UTC | `"2025-01-15T10:30:45.123Z"` |
| `user_id` | string | User identifier (null for anonymous) | `"user_abc123"` |
| `action` | string | Action performed (see Action Types) | `"login"` |
| `resource` | string | Resource accessed or modified | `"/api/v1/events"` |
| `outcome` | string | Result of the action (see Outcomes) | `"success"` |

### Optional Fields

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `ip_address` | string | Client IP address | `"192.168.1.100"` |
| `user_agent` | string | HTTP User-Agent header | `"Mozilla/5.0..."` |
| `session_id` | string | Session identifier | `"sess_xyz789"` |
| `request_id` | string | Request trace ID | `"req_abc123"` |
| `error_message` | string | Error description (for failures) | `"Invalid credentials"` |
| `duration_ms` | integer | Operation duration in milliseconds | `50` |
| `metadata` | object | Additional context (key-value pairs) | `{"permission": "event:read"}` |

## Action Types

| Action | Description | Common Resources |
|--------|-------------|------------------|
| `login` | Local authentication attempt | `/auth/login` |
| `logout` | User logout | `/auth/logout` |
| `token_refresh` | JWT token refresh | `/auth/refresh` |
| `token_validation` | JWT token validation | `/api/v1/*` |
| `permission_check` | Authorization check | `/api/v1/*` |
| `user_create` | User account creation | `/api/v1/users` |
| `user_update` | User account modification | `/api/v1/users/:id` |
| `user_delete` | User account deletion | `/api/v1/users/:id` |
| `role_assign` | Role assignment to user | `/api/v1/users/:id/roles` |
| `role_remove` | Role removal from user | `/api/v1/users/:id/roles` |
| `oidc_auth` | OIDC authentication | `/auth/oidc/callback` |
| `oidc_callback` | OIDC callback processing | `/auth/oidc/callback` |
| `api_access` | General API access | `/api/v1/*` |
| `resource_create` | Resource creation | `/api/v1/events` |
| `resource_read` | Resource retrieval | `/api/v1/events/:id` |
| `resource_update` | Resource modification | `/api/v1/events/:id` |
| `resource_delete` | Resource deletion | `/api/v1/events/:id` |
| `config_change` | Configuration change | `/admin/config` |
| `security_policy_change` | Security policy modification | `/admin/security` |

## Outcomes

| Outcome | Description | Log Level | Use Cases |
|---------|-------------|-----------|-----------|
| `success` | Action completed successfully | INFO | Normal operations |
| `failure` | Action failed (invalid input, wrong credentials) | WARN | Failed login attempts |
| `denied` | Action denied by authorization | WARN | Permission violations |
| `rate_limited` | Action blocked by rate limiter | WARN | Brute force attempts |
| `error` | System error occurred | ERROR | Internal failures |

## Example Log Entries

### Successful Login

```json
{
  "level": "INFO",
  "event_type": "audit",
  "app": "xzepr",
  "env": "production",
  "timestamp": "2025-01-15T10:30:45.123Z",
  "user_id": "user_abc123",
  "action": "login",
  "resource": "/auth/login",
  "outcome": "success",
  "ip_address": "192.168.1.100",
  "user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
  "session_id": "sess_xyz789",
  "request_id": "req_abc123",
  "duration_ms": 45
}
```

### Failed Login

```json
{
  "level": "WARN",
  "event_type": "audit",
  "app": "xzepr",
  "env": "production",
  "timestamp": "2025-01-15T10:32:18.456Z",
  "user_id": null,
  "action": "login",
  "resource": "/auth/login",
  "outcome": "failure",
  "ip_address": "192.168.1.100",
  "error_message": "Invalid credentials",
  "request_id": "req_def456",
  "duration_ms": 20
}
```

### Permission Denied

```json
{
  "level": "WARN",
  "event_type": "audit",
  "app": "xzepr",
  "env": "production",
  "timestamp": "2025-01-15T10:35:22.789Z",
  "user_id": "user_abc123",
  "action": "permission_check",
  "resource": "/api/v1/admin/settings",
  "outcome": "denied",
  "ip_address": "192.168.1.100",
  "session_id": "sess_xyz789",
  "request_id": "req_ghi789",
  "metadata": {
    "permission": "admin:write",
    "user_roles": "user,event_viewer"
  },
  "duration_ms": 2
}
```

### OIDC Authentication

```json
{
  "level": "INFO",
  "event_type": "audit",
  "app": "xzepr",
  "env": "production",
  "timestamp": "2025-01-15T10:40:15.234Z",
  "user_id": "user_xyz456",
  "action": "oidc_auth",
  "resource": "/auth/oidc/callback",
  "outcome": "success",
  "ip_address": "10.0.0.5",
  "metadata": {
    "provider": "keycloak",
    "new_user": "false"
  },
  "request_id": "req_jkl012",
  "duration_ms": 150
}
```

### Token Validation

```json
{
  "level": "INFO",
  "event_type": "audit",
  "app": "xzepr",
  "env": "production",
  "timestamp": "2025-01-15T10:45:30.567Z",
  "user_id": "user_abc123",
  "action": "token_validation",
  "resource": "/api/v1/events",
  "outcome": "success",
  "ip_address": "192.168.1.100",
  "session_id": "sess_xyz789",
  "request_id": "req_mno345",
  "duration_ms": 5
}
```

## Query Examples

### Elasticsearch / ELK Stack

#### All failed login attempts in last 24 hours

```json
{
  "query": {
    "bool": {
      "must": [
        {"match": {"event_type": "audit"}},
        {"match": {"action": "login"}},
        {"match": {"outcome": "failure"}},
        {"range": {"timestamp": {"gte": "now-24h"}}}
      ]
    }
  }
}
```

#### Permission denials for specific user

```json
{
  "query": {
    "bool": {
      "must": [
        {"match": {"event_type": "audit"}},
        {"match": {"action": "permission_check"}},
        {"match": {"outcome": "denied"}},
        {"match": {"user_id": "user_abc123"}}
      ]
    }
  }
}
```

#### Suspicious activity from IP address

```json
{
  "query": {
    "bool": {
      "must": [
        {"match": {"event_type": "audit"}},
        {"match": {"ip_address": "192.168.1.100"}},
        {"terms": {"outcome": ["failure", "denied", "rate_limited"]}}
      ]
    }
  },
  "aggs": {
    "by_action": {
      "terms": {"field": "action"}
    }
  }
}
```

### Datadog

#### Failed login attempts by IP

```
event_type:audit action:login outcome:failure
| top ip_address
```

#### Permission denials over time

```
event_type:audit action:permission_check outcome:denied
| timeseries count() by user_id
```

#### OIDC authentication success rate

```
event_type:audit action:oidc_auth
| pct(outcome:success)
```

### Splunk

#### Failed authentications in last hour

```
index=xzepr event_type="audit" action="login" outcome="failure" earliest=-1h
| stats count by ip_address, error_message
| sort -count
```

#### User activity timeline

```
index=xzepr event_type="audit" user_id="user_abc123"
| table timestamp, action, resource, outcome
| sort timestamp
```

#### High-value events (admin actions)

```
index=xzepr event_type="audit"
  (action="user_create" OR action="user_delete" OR action="role_assign" OR action="security_policy_change")
| table timestamp, user_id, action, resource, outcome
```

## Compliance and Retention

### Recommended Retention Policies

| Log Category | Retention Period | Rationale |
|--------------|------------------|-----------|
| Authentication events | 90 days | Security investigations |
| Authorization denials | 90 days | Security investigations |
| User management | 1 year | Compliance requirements |
| Admin actions | 2 years | Audit trail |
| General API access | 30 days | Performance analysis |

### GDPR Considerations

When user data is deleted:

1. User ID remains in logs for audit trail
2. PII (email, name) is not logged in audit events
3. Logs can be filtered/anonymized by user ID if required
4. IP addresses should be considered PII and handled accordingly

### SOC 2 Compliance

Audit logs satisfy SOC 2 requirements for:

- Access logging (who accessed what and when)
- Authentication logging (login attempts and outcomes)
- Authorization logging (permission checks)
- Change tracking (user and configuration changes)
- Anomaly detection (failed attempts, rate limiting)

## Integration with SIEM

### Log Shipping Configuration

#### Fluentd

```yaml
<source>
  @type tail
  path /var/log/xzepr/audit.log
  pos_file /var/log/td-agent/xzepr-audit.pos
  tag xzepr.audit
  <parse>
    @type json
    time_key timestamp
    time_format %Y-%m-%dT%H:%M:%S.%LZ
  </parse>
</source>

<filter xzepr.audit>
  @type record_transformer
  <record>
    service xzepr
    environment ${ENV["XZEPR_ENVIRONMENT"]}
  </record>
</filter>

<match xzepr.audit>
  @type elasticsearch
  host elasticsearch.internal
  port 9200
  index_name xzepr-audit-%Y.%m.%d
  type_name _doc
</match>
```

#### Filebeat

```yaml
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/xzepr/audit.log
  json.keys_under_root: true
  json.add_error_key: true
  fields:
    service: xzepr
    environment: production

output.elasticsearch:
  hosts: ["elasticsearch.internal:9200"]
  index: "xzepr-audit-%{+yyyy.MM.dd}"
```

## Monitoring and Alerting

### Critical Alerts

1. **Multiple failed logins from same IP**
   - Threshold: >5 failures in 5 minutes
   - Action: Block IP, notify security team

2. **Unusual permission denials**
   - Threshold: >10 denials for same user in 10 minutes
   - Action: Review user permissions, potential compromise

3. **Admin action outside business hours**
   - Threshold: Any user_create, role_assign, security_policy_change between 10pm-6am
   - Action: Verify with admin, potential unauthorized access

4. **OIDC authentication failures**
   - Threshold: >3 failures in 5 minutes
   - Action: Check OIDC provider status

### Informational Alerts

1. New user login from unknown IP
2. Permission denial for previously granted permission
3. High authentication latency (>500ms)

## Troubleshooting

### Missing Logs

If audit logs are not appearing:

1. Check application logs for errors initializing AuditLogger
2. Verify RUST_LOG environment variable includes audit module
3. Check log shipping agent is running and configured
4. Verify centralized logging system is reachable

### Incomplete Log Entries

If logs are missing expected fields:

1. Ensure latest XZepr version is deployed
2. Check for middleware ordering (JWT before RBAC)
3. Verify request headers contain IP forwarding headers

### High Log Volume

If log volume is excessive:

1. Filter out noisy endpoints (health checks) at shipper level
2. Sample high-frequency, low-value events (token_validation)
3. Increase log retention compression
4. Review whether all events require audit logging

## Best Practices

1. **Never log sensitive data**: Passwords, tokens, API keys should never appear in logs
2. **Use correlation IDs**: Include request_id and session_id for tracing
3. **Structured logging**: Always use JSON format for machine parsing
4. **Index key fields**: Create indexes on user_id, action, outcome, timestamp
5. **Regular review**: Review audit logs weekly for anomalies
6. **Automated analysis**: Use SIEM rules to detect patterns
7. **Retention compliance**: Follow organizational and regulatory requirements
8. **Access control**: Restrict audit log access to security and compliance teams

## References

- Implementation: `docs/explanation/phase3_production_hardening_implementation.md`
- Metrics Reference: `docs/reference/auth_metrics.md`
- Security Architecture: `docs/explanation/security_architecture.md`
