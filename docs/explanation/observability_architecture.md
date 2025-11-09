# Observability Architecture

## Overview

XZepr implements a comprehensive observability stack built around the three pillars of observability: metrics, logs, and traces. This document explains the architecture, design decisions, and integration patterns used to provide production-grade visibility into system behavior.

## Architecture Principles

### 1. Zero-Configuration Instrumentation

All HTTP requests are automatically instrumented through middleware without requiring manual instrumentation in handlers. This ensures consistent metrics across all endpoints and reduces the likelihood of missing instrumentation.

### 2. Low Overhead

The observability stack is designed for minimal performance impact:

- Metrics use efficient atomic operations
- Sampling strategies reduce trace volume
- Asynchronous logging prevents blocking
- Cardinality is controlled to prevent metric explosion

### 3. Production-First Design

The system is designed for production observability needs:

- Prometheus-compatible metrics format
- Structured logging with correlation IDs
- Distributed tracing support
- Alert-ready metric naming and labeling

## Components

### Metrics Layer (Prometheus)

#### PrometheusMetrics

The core metrics implementation provides standardized metrics across the application:

**Security Metrics:**
- `xzepr_auth_failures_total` - Authentication failures by reason and client
- `xzepr_auth_success_total` - Successful authentications by method and user
- `xzepr_rate_limit_rejections_total` - Rate limit rejections by endpoint and client
- `xzepr_cors_violations_total` - CORS violations by origin and endpoint
- `xzepr_validation_errors_total` - Input validation errors by endpoint and field
- `xzepr_graphql_complexity_violations_total` - GraphQL complexity limit violations

**Application Metrics:**
- `xzepr_http_requests_total` - HTTP request count by method, path, and status
- `xzepr_http_request_duration_seconds` - Request duration histogram with percentiles
- `xzepr_active_connections` - Current number of active HTTP connections

**System Metrics:**
- `xzepr_uptime_seconds` - Server uptime
- `xzepr_info` - Server version information

#### Metrics Middleware

Automatic HTTP request instrumentation:

```rust
pub async fn metrics_middleware(
    State(state): State<MetricsMiddlewareState>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    state.metrics.increment_active_connections();

    let response = next.run(request).await;

    state.metrics.decrement_active_connections();
    let duration = start.elapsed().as_secs_f64();
    state.metrics.record_http_request(&method, &path, status, duration);

    response
}
```

**Features:**
- Automatic path extraction using MatchedPath for consistent labels
- Active connection tracking
- Duration histograms with configurable buckets
- Status code tracking

#### Histogram Buckets

Request duration buckets are optimized for typical API response times:

```
[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
```

This provides percentile calculations (p50, p95, p99) for:
- Fast responses: 1ms - 100ms (cache hits, health checks)
- Normal responses: 100ms - 500ms (database queries)
- Slow responses: 500ms - 5s (complex operations)

### Logging Layer (Tracing)

#### Structured Logging

All logs use structured fields for machine parsing:

```rust
tracing::info!(
    method = %request.method(),
    path = %request.uri(),
    status = %response.status(),
    duration_ms = duration.as_millis(),
    "HTTP request completed"
);
```

#### Log Levels

- **ERROR** - System errors requiring immediate attention
- **WARN** - Degraded operation or configuration issues
- **INFO** - Normal operation events (requests, configuration changes)
- **DEBUG** - Detailed diagnostic information
- **TRACE** - Very detailed execution traces

#### Security Event Logging

The SecurityMonitor component logs all security-relevant events:

```rust
pub fn record_auth_failure(&self, reason: &str, details: &str) {
    tracing::warn!(
        security_event = "auth_failure",
        reason = reason,
        details = details,
        "Authentication failed"
    );

    if let Some(metrics) = &self.metrics {
        metrics.record_auth_failure(reason, details);
    }
}
```

### Tracing Layer (Jaeger/OpenTelemetry)

#### Distributed Tracing

Integration with Jaeger for distributed request tracing:

- Request ID propagation via headers
- Span creation for each middleware layer
- Database query tracing
- External service call tracking

#### TraceLayer Integration

```rust
.layer(TraceLayer::new_for_http()
    .make_span_with(|request: &Request<_>| {
        tracing::info_span!(
            "http_request",
            method = %request.method(),
            path = %request.uri().path(),
            request_id = %request.headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
        )
    })
)
```

## Middleware Stack Integration

The observability stack is integrated at multiple layers in the middleware stack:

### Layer Order (Outer to Inner)

1. **Security Headers** - First layer, no instrumentation needed
2. **CORS** - Records violations via SecurityMonitor
3. **Metrics Middleware** - Records all requests passing CORS
4. **Rate Limiting** - Records rejections to metrics
5. **Body Size Limits** - Logs oversized requests
6. **Tracing** - Creates request spans
7. **Authentication** - Records auth events to metrics
8. **Routes** - Handler execution is fully instrumented

This ordering ensures:
- All legitimate requests are counted
- Security events are captured before rejection
- Trace spans include full request lifecycle
- Authentication metrics are accurate

## Metrics Endpoint

The `/metrics` endpoint exposes Prometheus-compatible metrics:

```rust
async fn metrics_handler(
    State(metrics): State<Arc<PrometheusMetrics>>
) -> String {
    metrics.gather().unwrap_or_else(|e| {
        format!("# Error gathering metrics: {}\n", e)
    })
}
```

**Configuration:**
- Public endpoint (no authentication required)
- Separate state to avoid circular dependencies
- Text format compatible with Prometheus scraping
- Error handling to prevent metrics endpoint failures

## Cardinality Management

### Label Strategy

Cardinality is carefully controlled to prevent metric explosion:

**High Cardinality Labels (Avoided):**
- User IDs in most metrics (use aggregated counts)
- Full request paths (use matched route patterns)
- Timestamps (use Prometheus time-series)

**Controlled Cardinality Labels (Used):**
- HTTP methods (GET, POST, PUT, DELETE, PATCH)
- HTTP status codes (200, 400, 500, etc.)
- Route patterns (/api/v1/events/:id)
- Client tiers (anonymous, authenticated, admin)

**Example:**
```rust
// BAD - Unbounded cardinality
metrics.with_label_values(&[&user_id, &path])

// GOOD - Bounded cardinality
metrics.with_label_values(&["authenticated", &matched_path])
```

### Client ID Handling

Rate limiting and auth metrics use hashed client IDs to limit cardinality:

```rust
let client_id_hash = format!("{:x}", md5::compute(client_id));
metrics.record_rate_limit_rejection(&endpoint, &client_id_hash[..8]);
```

## Integration Patterns

### Handler-Level Metrics

For custom handler metrics:

```rust
async fn create_event(
    State(metrics): State<Arc<PrometheusMetrics>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<Json<Event>, StatusCode> {
    let start = Instant::now();

    let result = service.create_event(payload).await;

    match &result {
        Ok(_) => metrics.record_auth_success("api_key", "event_created"),
        Err(e) => metrics.record_validation_error("/api/v1/events", &e.field),
    }

    result
}
```

### SecurityMonitor Integration

The SecurityMonitor provides dual logging and metrics:

```rust
let metrics = Arc::new(PrometheusMetrics::new()?);
let monitor = Arc::new(SecurityMonitor::new_with_metrics(metrics.clone()));

// Automatically logs AND records metrics
monitor.record_auth_failure("invalid_token", "client123");
```

### Error Recording

Use MetricsError for automatic error metrics:

```rust
use crate::api::middleware::metrics::MetricsError;

async fn handler() -> Result<Response, MetricsError> {
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

## Prometheus Configuration

### Scrape Configuration

```yaml
scrape_configs:
  - job_name: 'xzepr'
    scrape_interval: 15s
    scrape_timeout: 10s
    static_configs:
      - targets: ['xzepr:8080']
    metrics_path: /metrics
```

### Alert Rules

Example alert rules for production:

```yaml
groups:
  - name: xzepr_alerts
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: |
          rate(xzepr_http_requests_total{status=~"5.."}[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: High error rate detected

      - alert: HighP99Latency
        expr: |
          histogram_quantile(0.99,
            rate(xzepr_http_request_duration_seconds_bucket[5m])
          ) > 1.0
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: P99 latency exceeds 1 second
```

## Grafana Dashboards

### Recommended Panels

**Overview Dashboard:**
1. Request rate (requests/sec)
2. Error rate (%)
3. P50/P95/P99 latency
4. Active connections

**Security Dashboard:**
1. Authentication failures by reason
2. Rate limit rejections by endpoint
3. CORS violations
4. Validation errors

**System Dashboard:**
1. Uptime
2. Memory usage
3. CPU usage
4. Database connection pool

### PromQL Examples

**Request Rate:**
```promql
rate(xzepr_http_requests_total[5m])
```

**Error Rate:**
```promql
rate(xzepr_http_requests_total{status=~"5.."}[5m])
/
rate(xzepr_http_requests_total[5m])
```

**P99 Latency:**
```promql
histogram_quantile(0.99,
  rate(xzepr_http_request_duration_seconds_bucket[5m])
)
```

## Performance Considerations

### Overhead Analysis

**Metrics Recording:**
- Counter increment: ~50ns
- Histogram observation: ~200ns
- Label resolution: ~100ns
- Total per request: ~500ns

**Memory Usage:**
- Base metrics: ~100KB
- Per time-series: ~3KB
- Estimated for 1000 unique series: ~3MB

### Optimization Strategies

1. **Sampling for High-Volume Endpoints:**
   ```rust
   if should_sample() {
       metrics.record_http_request(method, path, status, duration);
   }
   ```

2. **Batch Updates:**
   ```rust
   // Instead of per-request updates
   tokio::spawn(async move {
       let mut interval = tokio::time::interval(Duration::from_secs(60));
       loop {
           interval.tick().await;
           metrics.update_uptime(start_time.elapsed().as_secs());
       }
   });
   ```

3. **Metric Pruning:**
   - Remove stale time-series after 24h
   - Limit client ID cardinality to top 1000
   - Aggregate rare endpoints into "other"

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_metrics_middleware() {
    let metrics = Arc::new(PrometheusMetrics::new().unwrap());
    let state = MetricsMiddlewareState::new(metrics.clone());

    // Make request through middleware
    let app = Router::new()
        .route("/test", get(handler))
        .layer(middleware::from_fn_with_state(state, metrics_middleware));

    // Verify metrics were recorded
    let output = metrics.gather().unwrap();
    assert!(output.contains("xzepr_http_requests_total"));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_metrics_endpoint() {
    let app = test_app();

    let response = app
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    assert!(text.contains("xzepr_http_requests_total"));
}
```

## Production Deployment

### Environment Variables

```bash
XZEPR__MONITORING__METRICS_ENABLED=true
XZEPR__MONITORING__TRACING_ENABLED=true
XZEPR__MONITORING__LOG_LEVEL=info
XZEPR__MONITORING__JAEGER_ENDPOINT=http://jaeger:14268/api/traces
```

### Kubernetes Configuration

```yaml
apiVersion: v1
kind: Service
metadata:
  name: xzepr
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "8080"
    prometheus.io/path: "/metrics"
spec:
  selector:
    app: xzepr
  ports:
    - port: 8080
```

### Health Checks

The `/health` endpoint provides liveness and readiness probes:

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 30

readinessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
```

## Best Practices

### DO

1. Use matched route paths for consistent metrics
2. Record both successes and failures
3. Include relevant context in labels
4. Document custom metrics
5. Test metric recording in unit tests
6. Monitor metric cardinality

### DON'T

1. Add unbounded label values (user IDs, full paths)
2. Create metrics in hot code paths without sampling
3. Expose sensitive data in metric labels
4. Record PII in metrics or logs
5. Block on metric recording
6. Create duplicate metrics with different names

## Future Enhancements

### Planned Features

1. **OpenTelemetry Integration** - Native OTLP support
2. **Custom Metrics API** - Application-specific metrics
3. **Metric Aggregation** - Pre-aggregated rollups
4. **Cost Metrics** - Track cloud resource costs
5. **Business Metrics** - Event counts, revenue tracking

### Under Consideration

1. **Exemplars** - Link traces to metrics
2. **Service Mesh Integration** - Istio/Linkerd metrics
3. **Real User Monitoring** - Frontend performance
4. **Synthetic Monitoring** - Automated endpoint checks

## References

- [Prometheus Best Practices](https://prometheus.io/docs/practices/naming/)
- [OpenTelemetry Specification](https://opentelemetry.io/docs/reference/specification/)
- [Grafana Dashboard Design](https://grafana.com/docs/grafana/latest/best-practices/)
- [The Four Golden Signals](https://sre.google/sre-book/monitoring-distributed-systems/)
