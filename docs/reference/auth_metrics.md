# Authentication Metrics Reference

## Overview

XZepr exposes Prometheus metrics for authentication and authorization operations. These metrics enable monitoring of security events, performance tracking, and alerting on suspicious activity.

## Metrics Endpoint

Metrics are exposed at:

```
GET /metrics
```

This endpoint returns metrics in Prometheus text format. It should be protected and only accessible from internal monitoring systems.

## Authentication Metrics

### xzepr_auth_attempts_total

**Type**: Counter

**Description**: Total number of authentication attempts

**Labels**:
- `result`: `success` or `failure`

**Usage**:

```promql
# Total authentication attempts
sum(xzepr_auth_attempts_total)

# Authentication success rate
rate(xzepr_auth_attempts_total{result="success"}[5m]) /
rate(xzepr_auth_attempts_total[5m]) * 100

# Failed authentication attempts per minute
rate(xzepr_auth_attempts_total{result="failure"}[1m]) * 60
```

### xzepr_auth_failures_total

**Type**: Counter

**Description**: Total number of authentication failures with reason

**Labels**:
- `reason`: `invalid_token`, `missing_token`, `expired_token`, `invalid_credentials`
- `client_id`: Identifier for the client (IP address or user ID)

**Usage**:

```promql
# Failures by reason
sum by (reason) (xzepr_auth_failures_total)

# Failed attempts from specific IP
xzepr_auth_failures_total{client_id="192.168.1.100"}

# Top 10 IPs with most failures
topk(10, sum by (client_id) (xzepr_auth_failures_total))
```

### xzepr_auth_success_total

**Type**: Counter

**Description**: Successful authentications

**Labels**:
- `method`: `jwt`, `oidc`, `local`
- `user_id`: User identifier

**Usage**:

```promql
# Authentications by method
sum by (method) (xzepr_auth_success_total)

# User login frequency
topk(20, sum by (user_id) (xzepr_auth_success_total))

# OIDC authentication rate
rate(xzepr_auth_success_total{method="oidc"}[5m])
```

### xzepr_auth_duration_seconds

**Type**: Histogram

**Description**: Duration of authentication operations

**Labels**:
- `operation`: `jwt_validation`, `oidc_callback`, `local_login`, `token_refresh`

**Buckets**: 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0 seconds

**Usage**:

```promql
# 95th percentile auth latency
histogram_quantile(0.95,
  rate(xzepr_auth_duration_seconds_bucket[5m]))

# Average JWT validation time
rate(xzepr_auth_duration_seconds_sum{operation="jwt_validation"}[5m]) /
rate(xzepr_auth_duration_seconds_count{operation="jwt_validation"}[5m])

# Slow authentication operations (>500ms)
histogram_quantile(0.99,
  rate(xzepr_auth_duration_seconds_bucket{operation="jwt_validation"}[5m])) > 0.5
```

## Authorization Metrics

### xzepr_permission_checks_total

**Type**: Counter

**Description**: Total number of permission checks performed

**Labels**:
- `result`: `granted` or `denied`
- `permission`: The permission checked (e.g., `EventRead`, `EventCreate`, `AdminWrite`)

**Usage**:

```promql
# Permission check success rate
rate(xzepr_permission_checks_total{result="granted"}[5m]) /
rate(xzepr_permission_checks_total[5m]) * 100

# Most denied permissions
topk(10, sum by (permission) (xzepr_permission_checks_total{result="denied"}))

# Permission denials per minute
rate(xzepr_permission_checks_total{result="denied"}[1m]) * 60

# Specific permission grant rate
rate(xzepr_permission_checks_total{permission="EventCreate",result="granted"}[5m])
```

## Session Metrics

### xzepr_active_sessions_total

**Type**: Gauge

**Description**: Current number of active user sessions

**Usage**:

```promql
# Current active sessions
xzepr_active_sessions_total

# Session growth rate
deriv(xzepr_active_sessions_total[5m])

# Alert: Too many active sessions
xzepr_active_sessions_total > 10000
```

## Rate Limiting Metrics

### xzepr_rate_limit_rejections_total

**Type**: Counter

**Description**: Number of requests rejected due to rate limiting

**Labels**:
- `endpoint`: API endpoint path
- `client_id`: Client identifier (IP or user)

**Usage**:

```promql
# Rate limit hits per minute
rate(xzepr_rate_limit_rejections_total[1m]) * 60

# Most rate-limited endpoints
topk(10, sum by (endpoint) (xzepr_rate_limit_rejections_total))

# Clients hitting rate limits
topk(10, sum by (client_id) (xzepr_rate_limit_rejections_total))

# Rate limit effectiveness
rate(xzepr_rate_limit_rejections_total{endpoint="/api/v1/auth/login"}[5m])
```

## Security Metrics

### xzepr_cors_violations_total

**Type**: Counter

**Description**: CORS policy violations

**Labels**:
- `origin`: Origin header value
- `endpoint`: Endpoint accessed

**Usage**:

```promql
# CORS violations
sum(xzepr_cors_violations_total)

# Origins causing violations
topk(10, sum by (origin) (xzepr_cors_violations_total))
```

### xzepr_validation_errors_total

**Type**: Counter

**Description**: Input validation errors

**Labels**:
- `endpoint`: API endpoint
- `field`: Field that failed validation

**Usage**:

```promql
# Validation errors by endpoint
sum by (endpoint) (xzepr_validation_errors_total)

# Most problematic fields
topk(10, sum by (field) (xzepr_validation_errors_total))
```

## Prometheus Alert Rules

### Critical Alerts

```yaml
groups:
- name: xzepr_auth_critical
  interval: 30s
  rules:
  - alert: HighAuthenticationFailureRate
    expr: rate(xzepr_auth_failures_total[5m]) > 10
    for: 2m
    labels:
      severity: critical
      component: authentication
    annotations:
      summary: "High authentication failure rate"
      description: "{{ $value }} authentication failures per second"

  - alert: BruteForceAttack
    expr: sum by (client_id) (rate(xzepr_auth_failures_total[5m])) > 5
    for: 1m
    labels:
      severity: critical
      component: security
    annotations:
      summary: "Possible brute force attack from {{ $labels.client_id }}"
      description: "{{ $value }} failures per second from single client"

  - alert: HighPermissionDenialRate
    expr: rate(xzepr_permission_checks_total{result="denied"}[5m]) > 5
    for: 5m
    labels:
      severity: warning
      component: authorization
    annotations:
      summary: "High permission denial rate"
      description: "{{ $value }} permission denials per second"
```

### Warning Alerts

```yaml
  - alert: SlowAuthenticationOperations
    expr: histogram_quantile(0.99, rate(xzepr_auth_duration_seconds_bucket[5m])) > 1.0
    for: 5m
    labels:
      severity: warning
      component: performance
    annotations:
      summary: "Slow authentication operations"
      description: "99th percentile auth latency is {{ $value }}s"

  - alert: RateLimitingActive
    expr: rate(xzepr_rate_limit_rejections_total{endpoint="/api/v1/auth/login"}[5m]) > 1
    for: 2m
    labels:
      severity: warning
      component: security
    annotations:
      summary: "Rate limiting active on login endpoint"
      description: "{{ $value }} rate limit rejections per second"

  - alert: UnusualPermissionDenials
    expr: sum by (permission) (rate(xzepr_permission_checks_total{result="denied"}[10m])) > 0.5
    for: 10m
    labels:
      severity: info
      component: authorization
    annotations:
      summary: "Unusual permission denials for {{ $labels.permission }}"
      description: "{{ $value }} denials per second"
```

## Grafana Dashboard

### Dashboard JSON Template

```json
{
  "dashboard": {
    "title": "XZepr Authentication & Authorization",
    "panels": [
      {
        "title": "Authentication Success Rate",
        "targets": [
          {
            "expr": "rate(xzepr_auth_attempts_total{result=\"success\"}[5m]) / rate(xzepr_auth_attempts_total[5m]) * 100"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Permission Checks by Result",
        "targets": [
          {
            "expr": "sum by (result) (xzepr_permission_checks_total)",
            "legendFormat": "{{ result }}"
          }
        ],
        "type": "piechart"
      },
      {
        "title": "Auth Latency Percentiles",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(xzepr_auth_duration_seconds_bucket[5m]))",
            "legendFormat": "p50"
          },
          {
            "expr": "histogram_quantile(0.95, rate(xzepr_auth_duration_seconds_bucket[5m]))",
            "legendFormat": "p95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(xzepr_auth_duration_seconds_bucket[5m]))",
            "legendFormat": "p99"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Active Sessions",
        "targets": [
          {
            "expr": "xzepr_active_sessions_total"
          }
        ],
        "type": "stat"
      },
      {
        "title": "Top Denied Permissions",
        "targets": [
          {
            "expr": "topk(10, sum by (permission) (xzepr_permission_checks_total{result=\"denied\"}))"
          }
        ],
        "type": "table"
      }
    ]
  }
}
```

### Panel Descriptions

1. **Authentication Success Rate** (Graph)
   - Shows percentage of successful vs failed authentication attempts over time
   - Helps identify authentication issues or attacks

2. **Permission Checks by Result** (Pie Chart)
   - Breakdown of granted vs denied permission checks
   - Quick view of authorization effectiveness

3. **Auth Latency Percentiles** (Graph)
   - p50, p95, p99 authentication latencies
   - Identifies performance degradation

4. **Active Sessions** (Stat)
   - Current number of active sessions
   - Gauge for system load

5. **Top Denied Permissions** (Table)
   - Most frequently denied permissions
   - Helps identify misconfigured roles or attack patterns

6. **Failed Logins by Reason** (Table)
   - Breakdown of authentication failures
   - Identifies specific issues (expired tokens, invalid credentials, etc.)

7. **Rate Limit Hits** (Graph)
   - Rate limit rejections over time
   - Shows effectiveness of rate limiting

## Query Examples

### Security Analysis

```promql
# Detect potential account compromise
(rate(xzepr_auth_failures_total[10m]) > 0.5) and
(rate(xzepr_auth_success_total[10m]) > 0)

# Unusual access patterns
sum by (user_id) (rate(xzepr_permission_checks_total{result="denied"}[1h])) > 5

# Off-hours authentication
hour(xzepr_auth_success_total) < 6 or hour(xzepr_auth_success_total) > 22
```

### Performance Analysis

```promql
# Slow authentication operations by method
avg by (operation) (rate(xzepr_auth_duration_seconds_sum[5m]) /
                    rate(xzepr_auth_duration_seconds_count[5m]))

# Authentication throughput
sum(rate(xzepr_auth_success_total[1m])) * 60

# Percentage of slow authentications (>100ms)
sum(rate(xzepr_auth_duration_seconds_bucket{le="0.1"}[5m])) /
sum(rate(xzepr_auth_duration_seconds_count[5m])) * 100
```

### Capacity Planning

```promql
# Session growth rate per hour
rate(xzepr_active_sessions_total[1h]) * 3600

# Peak authentication load
max_over_time(rate(xzepr_auth_success_total[5m])[24h:])

# Average session count
avg_over_time(xzepr_active_sessions_total[24h])
```

## Best Practices

1. **Monitor continuously**: Set up dashboards and alerts before production deployment
2. **Baseline metrics**: Establish normal ranges during load testing
3. **Alert tuning**: Adjust thresholds based on actual traffic patterns
4. **Correlation**: Combine metrics with audit logs for complete picture
5. **Regular review**: Weekly review of metrics to identify trends
6. **Capacity planning**: Use metrics to predict scaling needs
7. **Incident response**: Pre-defined runbooks based on metric alerts

## Integration

### Prometheus Configuration

```yaml
scrape_configs:
  - job_name: 'xzepr'
    scrape_interval: 15s
    scrape_timeout: 10s
    static_configs:
      - targets: ['xzepr-1:9090', 'xzepr-2:9090', 'xzepr-3:9090']
        labels:
          environment: 'production'
          service: 'xzepr'
```

### Grafana Data Source

```yaml
apiVersion: 1
datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    jsonData:
      timeInterval: "15s"
```

## Troubleshooting

### Metrics Not Appearing

1. Check `/metrics` endpoint is accessible
2. Verify Prometheus is scraping the target
3. Check for metric registration errors in logs
4. Ensure PrometheusMetrics is initialized at startup

### Incorrect Values

1. Verify metric labels match query
2. Check for clock skew between systems
3. Ensure metrics are being recorded in code
4. Review metric type (counter vs gauge vs histogram)

### High Cardinality

If metrics have too many label combinations:

1. Limit client_id cardinality (hash IPs if needed)
2. Aggregate low-frequency permissions
3. Use recording rules to pre-aggregate
4. Consider metric sampling for high-volume events

## References

- Implementation: `docs/explanation/phase3_production_hardening_implementation.md`
- Audit Logs: `docs/reference/audit_logs.md`
- Security Architecture: `docs/explanation/security_architecture.md`
- Prometheus Documentation: https://prometheus.io/docs/
- Grafana Documentation: https://grafana.com/docs/
