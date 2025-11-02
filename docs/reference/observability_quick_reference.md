# Observability Quick Reference

## Overview

Quick reference for XZepr's observability features including metrics, logging, and tracing.

## Metrics Endpoint

**URL:** `http://localhost:8080/metrics`

**Format:** Prometheus text format

**Access:** Public (no authentication required)

## Available Metrics

### HTTP Metrics

```promql
# Total requests by method, path, and status
xzepr_http_requests_total{method="GET", path="/api/v1/events", status="200"}

# Request duration histogram (seconds)
xzepr_http_request_duration_seconds_bucket{method="POST", path="/api/v1/events", le="0.1"}

# Active HTTP connections
xzepr_active_connections
```

### Security Metrics

```promql
# Authentication failures
xzepr_auth_failures_total{reason="invalid_token", client_id="hash123"}

# Successful authentications
xzepr_auth_success_total{method="jwt", user_id="hash456"}

# Rate limit rejections
xzepr_rate_limit_rejections_total{endpoint="/api/v1/events", client_id="hash123"}

# CORS violations
xzepr_cors_violations_total{origin="https://example.com", endpoint="/api/v1/events"}

# Validation errors
xzepr_validation_errors_total{endpoint="/api/v1/events", field="name"}

# GraphQL complexity violations
xzepr_graphql_complexity_violations_total{client_id="hash123"}
```

### System Metrics

```promql
# Server uptime in seconds
xzepr_uptime_seconds

# Server version information
xzepr_info{version="0.1.0"}
```

## Common PromQL Queries

### Request Rate

```promql
# Requests per second (5-minute average)
rate(xzepr_http_requests_total[5m])

# By endpoint
sum by (path) (rate(xzepr_http_requests_total[5m]))

# By status code
sum by (status) (rate(xzepr_http_requests_total[5m]))
```

### Error Rate

```promql
# 5xx error rate
rate(xzepr_http_requests_total{status=~"5.."}[5m])

# Error percentage
rate(xzepr_http_requests_total{status=~"5.."}[5m])
/
rate(xzepr_http_requests_total[5m])
* 100
```

### Latency Percentiles

```promql
# P50 latency
histogram_quantile(0.50,
  rate(xzepr_http_request_duration_seconds_bucket[5m])
)

# P95 latency
histogram_quantile(0.95,
  rate(xzepr_http_request_duration_seconds_bucket[5m])
)

# P99 latency
histogram_quantile(0.99,
  rate(xzepr_http_request_duration_seconds_bucket[5m])
)

# By endpoint
histogram_quantile(0.99,
  sum by (path, le) (rate(xzepr_http_request_duration_seconds_bucket[5m]))
)
```

### Security Monitoring

```promql
# Auth failure rate
rate(xzepr_auth_failures_total[5m])

# Rate limit rejection rate
rate(xzepr_rate_limit_rejections_total[5m])

# CORS violations
rate(xzepr_cors_violations_total[5m])
```

## Alert Rules

### High Error Rate

```yaml
alert: HighErrorRate
expr: |
  rate(xzepr_http_requests_total{status=~"5.."}[5m])
  /
  rate(xzepr_http_requests_total[5m])
  > 0.05
for: 5m
severity: critical
```

### High Latency

```yaml
alert: HighP99Latency
expr: |
  histogram_quantile(0.99,
    rate(xzepr_http_request_duration_seconds_bucket[5m])
  ) > 1.0
for: 10m
severity: warning
```

### High Auth Failures

```yaml
alert: HighAuthFailureRate
expr: rate(xzepr_auth_failures_total[5m]) > 10
for: 5m
severity: warning
```

### Rate Limit Abuse

```yaml
alert: HighRateLimitRejections
expr: rate(xzepr_rate_limit_rejections_total[5m]) > 50
for: 5m
severity: warning
```

## Grafana Dashboard Panels

### Request Rate Panel

```json
{
  "title": "Request Rate",
  "targets": [
    {
      "expr": "sum(rate(xzepr_http_requests_total[5m]))",
      "legendFormat": "Total Requests/sec"
    }
  ]
}
```

### Error Rate Panel

```json
{
  "title": "Error Rate (%)",
  "targets": [
    {
      "expr": "rate(xzepr_http_requests_total{status=~\"5..\"}[5m]) / rate(xzepr_http_requests_total[5m]) * 100",
      "legendFormat": "5xx Error Rate"
    }
  ]
}
```

### Latency Panel

```json
{
  "title": "Response Time",
  "targets": [
    {
      "expr": "histogram_quantile(0.50, rate(xzepr_http_request_duration_seconds_bucket[5m]))",
      "legendFormat": "p50"
    },
    {
      "expr": "histogram_quantile(0.95, rate(xzepr_http_request_duration_seconds_bucket[5m]))",
      "legendFormat": "p95"
    },
    {
      "expr": "histogram_quantile(0.99, rate(xzepr_http_request_duration_seconds_bucket[5m]))",
      "legendFormat": "p99"
    }
  ]
}
```

### Active Connections Panel

```json
{
  "title": "Active Connections",
  "targets": [
    {
      "expr": "xzepr_active_connections",
      "legendFormat": "Active Connections"
    }
  ]
}
```

## Configuration

### Environment Variables

```bash
# Enable metrics
XZEPR__MONITORING__METRICS_ENABLED=true

# Enable tracing
XZEPR__MONITORING__TRACING_ENABLED=true

# Log level
XZEPR__MONITORING__LOG_LEVEL=info

# Jaeger endpoint
XZEPR__MONITORING__JAEGER_ENDPOINT=http://jaeger:14268/api/traces
```

### Prometheus Scrape Config

```yaml
scrape_configs:
  - job_name: 'xzepr'
    scrape_interval: 15s
    scrape_timeout: 10s
    static_configs:
      - targets: ['xzepr:8080']
    metrics_path: /metrics
```

### Kubernetes Service Annotations

```yaml
apiVersion: v1
kind: Service
metadata:
  name: xzepr
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "8080"
    prometheus.io/path: "/metrics"
```

## Using Metrics in Code

### Access Metrics from State

```rust
use axum::extract::State;
use std::sync::Arc;
use crate::infrastructure::PrometheusMetrics;

async fn handler(
    State(metrics): State<Arc<PrometheusMetrics>>,
) -> Result<Response, StatusCode> {
    metrics.record_auth_success("jwt", "user123");
    Ok(response)
}
```

### Record Custom Events

```rust
// Record authentication failure
metrics.record_auth_failure("invalid_token", "client123");

// Record rate limit rejection
metrics.record_rate_limit_rejection("/api/events", "client123");

// Record validation error
metrics.record_validation_error("/api/events", "name");

// Update active connections
metrics.increment_active_connections();
metrics.decrement_active_connections();

// Record HTTP request manually
metrics.record_http_request("POST", "/api/events", 201, 0.045);
```

### Using MetricsError

```rust
use crate::api::middleware::metrics::MetricsError;

async fn handler(
    State(metrics): State<Arc<PrometheusMetrics>>,
) -> Result<Response, MetricsError> {
    if validation_failed {
        return Err(MetricsError::new(
            StatusCode::BAD_REQUEST,
            "Validation failed",
            metrics,
            "/api/v1/events"
        ));
    }
    Ok(response)
}
```

## Troubleshooting

### Metrics Not Appearing

1. Check metrics endpoint: `curl http://localhost:8080/metrics`
2. Verify Prometheus scraping: Check Prometheus targets page
3. Check configuration: `XZEPR__MONITORING__METRICS_ENABLED=true`

### High Cardinality Issues

Symptoms:
- Prometheus running out of memory
- Slow queries
- Large `/metrics` response

Solutions:
- Hash user IDs: `format!("{:x}", md5::compute(user_id))[..8]`
- Aggregate rare values: `if is_common(x) { x } else { "other" }`
- Reduce label count

### Metrics Reset After Restart

Normal behavior - use `rate()` or `increase()` functions:

```promql
# Good - handles resets
rate(xzepr_http_requests_total[5m])

# Bad - affected by resets
xzepr_http_requests_total
```

## Health Check

**Endpoint:** `GET /health`

**Response:**
```json
{
  "status": "healthy"
}
```

## Histogram Buckets

Request duration buckets (seconds):

```
[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
```

Covers:
- Fast responses: 1ms - 100ms
- Normal responses: 100ms - 500ms
- Slow responses: 500ms - 5s

## Performance

**Overhead per request:** ~500ns

**Memory usage:**
- Base metrics: ~100KB
- Per time-series: ~3KB
- 1000 series: ~3MB

## References

- Architecture: `docs/explanations/observability_architecture.md`
- Implementation: `docs/explanations/phase4_observability_implementation.md`
- Custom Metrics: `docs/how_to/implement_custom_metrics.md`
- Prometheus: https://prometheus.io/docs/
- Grafana: https://grafana.com/docs/
