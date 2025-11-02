# How to Implement Custom Metrics

## Overview

This guide shows you how to add custom application-specific metrics to XZepr beyond the automatic HTTP instrumentation. You'll learn how to record business metrics, custom counters, gauges, and histograms.

## Prerequisites

- Basic understanding of Prometheus metrics
- Familiarity with Rust and Axum
- XZepr development environment set up

## Automatic vs Custom Metrics

**Automatic Metrics** (already provided):
- HTTP request counts and durations
- Active connections
- Security events (auth, rate limits, CORS)
- Validation errors

**Custom Metrics** (what this guide covers):
- Business-specific counters (events created, users registered)
- Application-specific gauges (queue depth, cache size)
- Custom histograms (processing time, batch sizes)

## Pattern 1: Using Existing PrometheusMetrics

### Step 1: Access Metrics from State

Extract the metrics from your handler state:

```rust
use axum::{extract::State, Json};
use std::sync::Arc;
use crate::infrastructure::PrometheusMetrics;

async fn create_event(
    State(metrics): State<Arc<PrometheusMetrics>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<Json<Event>, StatusCode> {
    // Your handler logic here
    let event = service.create_event(payload).await?;

    // Record custom metric
    metrics.record_validation_error("/api/v1/events", "custom_field");

    Ok(Json(event))
}
```

### Step 2: Use Existing Metric Methods

The PrometheusMetrics provides these methods:

```rust
// Security metrics
metrics.record_auth_failure("invalid_token", "client123");
metrics.record_auth_success("jwt", "user123");
metrics.record_rate_limit_rejection("/api/events", "client123");
metrics.record_cors_violation("https://evil.com", "/api/events");
metrics.record_validation_error("/api/events", "name");
metrics.record_complexity_violation("client123");

// Application metrics
metrics.record_http_request("GET", "/api/events", 200, 0.045);
metrics.increment_active_connections();
metrics.decrement_active_connections();
metrics.set_active_connections(42);

// System metrics
metrics.update_uptime(3600);
```

## Pattern 2: Adding New Metrics to PrometheusMetrics

For application-specific metrics, extend the PrometheusMetrics struct.

### Step 1: Add Metric Fields

Edit `src/infrastructure/metrics.rs`:

```rust
use prometheus::{CounterVec, GaugeVec, HistogramVec};

#[derive(Clone)]
pub struct PrometheusMetrics {
    // ... existing fields ...

    // Add your new metrics
    events_created_total: CounterVec,
    queue_depth: GaugeVec,
    batch_processing_duration_seconds: HistogramVec,
}
```

### Step 2: Register Metrics in Constructor

```rust
impl PrometheusMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // ... existing metrics registration ...

        // Register your new metrics
        let events_created_total = CounterVec::new(
            Opts::new(
                "xzepr_events_created_total",
                "Total number of events created"
            ),
            &["event_type", "source"],
        )?;
        registry.register(Box::new(events_created_total.clone()))?;

        let queue_depth = GaugeVec::new(
            Opts::new(
                "xzepr_queue_depth",
                "Current depth of processing queues"
            ),
            &["queue_name"],
        )?;
        registry.register(Box::new(queue_depth.clone()))?;

        let batch_processing_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "xzepr_batch_processing_duration_seconds",
                "Time to process event batches"
            ).buckets(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]),
            &["batch_type"],
        )?;
        registry.register(Box::new(batch_processing_duration_seconds.clone()))?;

        Ok(Self {
            // ... existing fields ...
            events_created_total,
            queue_depth,
            batch_processing_duration_seconds,
        })
    }
}
```

### Step 3: Add Public Methods

```rust
impl PrometheusMetrics {
    /// Records an event creation
    pub fn record_event_created(&self, event_type: &str, source: &str) {
        self.events_created_total
            .with_label_values(&[event_type, source])
            .inc();
    }

    /// Sets the current queue depth
    pub fn set_queue_depth(&self, queue_name: &str, depth: i64) {
        self.queue_depth
            .with_label_values(&[queue_name])
            .set(depth as f64);
    }

    /// Records batch processing time
    pub fn record_batch_processing(&self, batch_type: &str, duration_secs: f64) {
        self.batch_processing_duration_seconds
            .with_label_values(&[batch_type])
            .observe(duration_secs);
    }
}
```

### Step 4: Use New Metrics in Handlers

```rust
async fn create_event(
    State(metrics): State<Arc<PrometheusMetrics>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<Json<Event>, StatusCode> {
    let event = service.create_event(payload).await?;

    // Record custom metric
    metrics.record_event_created(&event.event_type, &event.source);

    Ok(Json(event))
}
```

## Pattern 3: Separate Business Metrics Module

For complex applications, create a separate metrics module.

### Step 1: Create Business Metrics Module

Create `src/infrastructure/business_metrics.rs`:

```rust
use prometheus::{CounterVec, GaugeVec, HistogramVec, Registry, Opts, HistogramOpts};
use std::sync::Arc;

#[derive(Clone)]
pub struct BusinessMetrics {
    registry: Arc<Registry>,

    // Business metrics
    users_registered_total: CounterVec,
    subscription_revenue_total: CounterVec,
    active_sessions: GaugeVec,
    api_usage_by_tier: CounterVec,
}

impl BusinessMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let users_registered_total = CounterVec::new(
            Opts::new(
                "xzepr_users_registered_total",
                "Total users registered"
            ),
            &["tier", "source"],
        )?;
        registry.register(Box::new(users_registered_total.clone()))?;

        let subscription_revenue_total = CounterVec::new(
            Opts::new(
                "xzepr_subscription_revenue_total",
                "Total subscription revenue in cents"
            ),
            &["tier", "interval"],
        )?;
        registry.register(Box::new(subscription_revenue_total.clone()))?;

        let active_sessions = GaugeVec::new(
            Opts::new(
                "xzepr_active_sessions",
                "Number of active user sessions"
            ),
            &["tier"],
        )?;
        registry.register(Box::new(active_sessions.clone()))?;

        let api_usage_by_tier = CounterVec::new(
            Opts::new(
                "xzepr_api_usage_by_tier_total",
                "API calls by subscription tier"
            ),
            &["tier", "endpoint"],
        )?;
        registry.register(Box::new(api_usage_by_tier.clone()))?;

        Ok(Self {
            registry: Arc::new(registry),
            users_registered_total,
            subscription_revenue_total,
            active_sessions,
            api_usage_by_tier,
        })
    }

    pub fn record_user_registration(&self, tier: &str, source: &str) {
        self.users_registered_total
            .with_label_values(&[tier, source])
            .inc();
    }

    pub fn record_subscription_revenue(&self, tier: &str, interval: &str, cents: u64) {
        self.subscription_revenue_total
            .with_label_values(&[tier, interval])
            .inc_by(cents as f64);
    }

    pub fn set_active_sessions(&self, tier: &str, count: i64) {
        self.active_sessions
            .with_label_values(&[tier])
            .set(count as f64);
    }

    pub fn record_api_usage(&self, tier: &str, endpoint: &str) {
        self.api_usage_by_tier
            .with_label_values(&[tier, endpoint])
            .inc();
    }

    pub fn gather(&self) -> Result<String, prometheus::Error> {
        use prometheus::{Encoder, TextEncoder};

        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer).map_err(|e| {
            prometheus::Error::Msg(format!("Failed to encode metrics: {}", e))
        })
    }
}
```

### Step 2: Integrate with Router

Update `src/api/router.rs`:

```rust
use crate::infrastructure::{PrometheusMetrics, BusinessMetrics};

pub struct RouterConfig {
    pub security: SecurityConfig,
    pub monitor: Arc<SecurityMonitor>,
    pub metrics: Option<Arc<PrometheusMetrics>>,
    pub business_metrics: Option<Arc<BusinessMetrics>>,
}

impl RouterConfig {
    pub fn with_business_metrics(mut self, metrics: Arc<BusinessMetrics>) -> Self {
        self.business_metrics = Some(metrics);
        self
    }
}

pub async fn build_router(state: AppState, config: RouterConfig) -> Router {
    // Create business metrics endpoint
    let business_metrics = config.business_metrics.clone().unwrap_or_else(|| {
        Arc::new(BusinessMetrics::new().expect("Failed to create business metrics"))
    });

    Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/metrics/business", get(business_metrics_handler))
        .with_state((metrics_state, business_metrics))
        // ... rest of router ...
}

async fn business_metrics_handler(
    State(metrics): State<Arc<BusinessMetrics>>
) -> String {
    metrics.gather().unwrap_or_else(|e| {
        format!("# Error gathering business metrics: {}\n", e)
    })
}
```

## Pattern 4: Middleware-Level Metrics

For metrics that need to be recorded across all requests.

### Step 1: Create Custom Middleware

```rust
use axum::middleware::Next;
use axum::extract::{Request, State};
use axum::response::Response;

pub async fn business_metrics_middleware(
    State(metrics): State<Arc<BusinessMetrics>>,
    request: Request,
    next: Next,
) -> Response {
    // Extract tier from auth headers
    let tier = request
        .headers()
        .get("x-subscription-tier")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("free");

    let path = request.uri().path().to_string();

    // Record API usage
    metrics.record_api_usage(tier, &path);

    next.run(request).await
}
```

### Step 2: Apply Middleware

```rust
.layer(middleware::from_fn_with_state(
    business_metrics,
    business_metrics_middleware
))
```

## Best Practices

### Metric Naming

Follow Prometheus naming conventions:

```rust
// GOOD
"xzepr_events_created_total"      // Counter with _total suffix
"xzepr_queue_depth"                // Gauge, no suffix
"xzepr_processing_duration_seconds" // Histogram with _seconds suffix

// BAD
"EventsCreated"                    // Not snake_case
"xzepr_events"                     // Ambiguous
"xzepr_duration_ms"                // Use seconds, not milliseconds
```

### Label Cardinality

Keep label cardinality bounded:

```rust
// GOOD - Bounded cardinality
metrics.with_label_values(&["premium", "api"])  // ~10 tiers × ~50 endpoints = 500 series

// BAD - Unbounded cardinality
metrics.with_label_values(&[&user_id, &full_path])  // Millions of series!
```

### Label Naming

Use clear, consistent label names:

```rust
// GOOD
&["tier", "endpoint", "status"]

// BAD
&["t", "e", "s"]  // Too terse
&["subscription_tier_name", "api_endpoint_path"]  // Too verbose
```

### Histogram Buckets

Choose buckets appropriate for your measurements:

```rust
// API response times (milliseconds)
vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]

// Batch processing (seconds)
vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0]

// Queue sizes (count)
vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]
```

## Testing Custom Metrics

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_event_created() {
        let metrics = PrometheusMetrics::new().unwrap();

        metrics.record_event_created("deployment", "jenkins");
        metrics.record_event_created("deployment", "github");
        metrics.record_event_created("build", "jenkins");

        let output = metrics.gather().unwrap();

        assert!(output.contains("xzepr_events_created_total"));
        assert!(output.contains("event_type=\"deployment\""));
        assert!(output.contains("source=\"jenkins\""));
    }

    #[test]
    fn test_queue_depth() {
        let metrics = PrometheusMetrics::new().unwrap();

        metrics.set_queue_depth("events", 42);
        metrics.set_queue_depth("notifications", 7);

        let output = metrics.gather().unwrap();

        assert!(output.contains("xzepr_queue_depth"));
        assert!(output.contains("queue_name=\"events\""));
        assert!(output.contains("42"));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_metrics_endpoint_includes_custom_metrics() {
    let app = test_app();

    // Create some events to generate metrics
    app.create_event(test_event()).await;
    app.create_event(test_event()).await;

    // Fetch metrics
    let response = app.get("/metrics").await;
    let body = response.text().await;

    assert!(body.contains("xzepr_events_created_total"));
    assert!(body.contains("2"));
}
```

## Monitoring Custom Metrics

### Prometheus Queries

```promql
# Event creation rate
rate(xzepr_events_created_total[5m])

# Events by type
sum by (event_type) (xzepr_events_created_total)

# Queue depth average
avg_over_time(xzepr_queue_depth[5m])

# Batch processing p99 latency
histogram_quantile(0.99,
  rate(xzepr_batch_processing_duration_seconds_bucket[5m])
)
```

### Grafana Panels

Create panels for your custom metrics:

```yaml
- title: Event Creation Rate
  type: graph
  targets:
    - expr: rate(xzepr_events_created_total[5m])
      legendFormat: "{{event_type}}"

- title: Queue Depth
  type: graph
  targets:
    - expr: xzepr_queue_depth
      legendFormat: "{{queue_name}}"

- title: Processing Time (p50, p95, p99)
  type: graph
  targets:
    - expr: histogram_quantile(0.50, rate(xzepr_batch_processing_duration_seconds_bucket[5m]))
      legendFormat: "p50"
    - expr: histogram_quantile(0.95, rate(xzepr_batch_processing_duration_seconds_bucket[5m]))
      legendFormat: "p95"
    - expr: histogram_quantile(0.99, rate(xzepr_batch_processing_duration_seconds_bucket[5m]))
      legendFormat: "p99"
```

### Alert Rules

```yaml
groups:
  - name: custom_metrics_alerts
    rules:
      - alert: HighQueueDepth
        expr: xzepr_queue_depth > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Queue {{$labels.queue_name}} is backed up"

      - alert: SlowBatchProcessing
        expr: |
          histogram_quantile(0.95,
            rate(xzepr_batch_processing_duration_seconds_bucket[5m])
          ) > 30
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Batch processing is slow (p95 > 30s)"
```

## Troubleshooting

### Metrics Not Appearing

1. Check metric is registered: `registry.register(Box::new(metric.clone()))?`
2. Verify metric is recorded: Add debug logging
3. Check `/metrics` endpoint: Search for metric name
4. Verify Prometheus scraping: Check Prometheus targets page

### High Cardinality

Symptoms:
- Prometheus running out of memory
- Slow query performance
- Large `/metrics` response

Solutions:

```rust
// Hash high-cardinality values
let user_hash = format!("{:x}", md5::compute(user_id));
metrics.with_label_values(&[&user_hash[..8]])

// Aggregate rare values
let tier = if is_common_tier(tier) { tier } else { "other" };
metrics.with_label_values(&[tier])

// Use fewer labels
// BAD: &[user_id, session_id, endpoint, method, status]
// GOOD: &[tier, endpoint]
```

### Metrics Reset on Restart

Counters reset to zero on restart - this is normal. Use `rate()` and `increase()` functions in Prometheus which handle resets:

```promql
# Handles counter resets correctly
rate(xzepr_events_created_total[5m])

# Don't use raw counter values
xzepr_events_created_total  # BAD
```

## Examples

### Example 1: Track User Registration

```rust
// In user service
pub async fn register_user(&self, req: RegisterRequest) -> Result<User> {
    let user = self.repository.create_user(req).await?;

    // Record metric
    self.metrics.record_user_registration(
        &user.subscription_tier,
        &req.registration_source
    );

    Ok(user)
}
```

### Example 2: Monitor Background Jobs

```rust
pub async fn process_events_batch(&self, events: Vec<Event>) -> Result<()> {
    let start = Instant::now();

    // Update queue depth
    self.metrics.set_queue_depth("events", self.queue.len() as i64);

    // Process batch
    self.process_batch(events).await?;

    // Record processing time
    let duration = start.elapsed().as_secs_f64();
    self.metrics.record_batch_processing("events", duration);

    Ok(())
}
```

### Example 3: Track API Usage by Tier

```rust
// In authentication middleware
pub async fn auth_middleware(
    State(metrics): State<Arc<BusinessMetrics>>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = extract_token(&request)?;
    let user = validate_token(token).await?;

    // Record API usage
    let endpoint = request.uri().path();
    metrics.record_api_usage(&user.tier, endpoint);

    request.extensions_mut().insert(user);
    next.run(request).await
}
```

## References

- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [Metric Types](https://prometheus.io/docs/concepts/metric_types/)
- [Writing Exporters](https://prometheus.io/docs/instrumenting/writing_exporters/)
- [PromQL Basics](https://prometheus.io/docs/prometheus/latest/querying/basics/)
