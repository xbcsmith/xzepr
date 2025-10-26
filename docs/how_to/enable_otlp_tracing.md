# How to Enable OpenTelemetry OTLP Tracing

## Overview

This guide walks you through enabling and using OpenTelemetry OTLP (OpenTelemetry Protocol) tracing in XZepr to export distributed traces to Jaeger or other OpenTelemetry-compatible collectors.

## Prerequisites

- XZepr application built and ready to run
- Docker (for local Jaeger) or access to existing Jaeger/OTLP collector
- Basic understanding of distributed tracing concepts

## Quick Start

### Step 1: Start Jaeger Collector

**Local Development (Docker):**

```bash
docker run -d --name jaeger \
  -p 4317:4317 \
  -p 4318:4318 \
  -p 16686:16686 \
  -e COLLECTOR_OTLP_ENABLED=true \
  jaegertracing/all-in-one:latest
```

**Verify Jaeger is running:**

```bash
# Check container status
docker ps | grep jaeger

# Access Jaeger UI
open http://localhost:16686
```

### Step 2: Enable OTLP in XZepr

**Set environment variables:**

```bash
export XZEPR__ENABLE_OTLP=true
export XZEPR__OTLP_ENDPOINT=http://localhost:4317
export XZEPR__ENVIRONMENT=development
```

### Step 3: Start XZepr

```bash
cargo run --bin server
```

**Look for these log messages:**

```
INFO Initializing OTLP exporter
INFO OTLP exporter initialized successfully
INFO OTLP exporter configured and active
```

### Step 4: Generate Traffic

```bash
# Make some requests to generate traces
curl http://localhost:8080/health
curl http://localhost:8080/api/events
curl -X POST http://localhost:8080/api/events \
  -H "Content-Type: application/json" \
  -d '{"name": "test.event", "version": "1.0.0"}'
```

### Step 5: View Traces in Jaeger

1. Open Jaeger UI: http://localhost:16686
2. Select service: **xzepr**
3. Click **Find Traces**
4. Explore your traces!

## Detailed Configuration

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `XZEPR__ENABLE_OTLP` | Yes | `false` | Enable OTLP exporter |
| `XZEPR__OTLP_ENDPOINT` | Yes* | None | OTLP collector endpoint (gRPC) |
| `XZEPR__ENVIRONMENT` | No | `development` | Environment name (affects sampling) |
| `XZEPR__LOG_LEVEL` | No | `info` | Log level filter |
| `XZEPR__JSON_LOGS` | No | `false` | Use JSON log format |

*Required when `XZEPR__ENABLE_OTLP=true`

### Sampling Configuration

Sampling is automatically configured based on environment:

**Development:**
- Sample rate: 100% (all traces collected)
- OTLP: Disabled by default
- Logs: Human-readable format

**Staging:**
- Sample rate: 50% (half of traces collected)
- OTLP: Enabled by default
- Logs: JSON format

**Production:**
- Sample rate: 10% (1 in 10 traces collected)
- OTLP: Enabled by default
- Logs: JSON format

**Override sampling:**

```bash
# Not supported yet - uses environment defaults
# Future: XZEPR__SAMPLE_RATE=0.25
```

## Deployment Scenarios

### Docker Compose (Local Development)

**docker-compose.yaml:**

```yaml
version: '3.8'

services:
  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "4317:4317"    # OTLP gRPC
      - "16686:16686"  # Jaeger UI
    environment:
      - COLLECTOR_OTLP_ENABLED=true

  xzepr:
    build: .
    depends_on:
      - jaeger
      - postgres
    environment:
      - XZEPR__ENABLE_OTLP=true
      - XZEPR__OTLP_ENDPOINT=http://jaeger:4317
      - XZEPR__ENVIRONMENT=development
      - XZEPR__LOG_LEVEL=debug
    ports:
      - "8080:8080"
```

**Start services:**

```bash
docker-compose up -d
```

### Kubernetes (Staging/Production)

**jaeger-deployment.yaml:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: jaeger
  namespace: observability
spec:
  replicas: 1
  selector:
    matchLabels:
      app: jaeger
  template:
    metadata:
      labels:
        app: jaeger
    spec:
      containers:
      - name: jaeger
        image: jaegertracing/all-in-one:1.50
        ports:
        - containerPort: 4317
          name: otlp-grpc
        - containerPort: 16686
          name: ui
        env:
        - name: COLLECTOR_OTLP_ENABLED
          value: "true"
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
---
apiVersion: v1
kind: Service
metadata:
  name: jaeger
  namespace: observability
spec:
  selector:
    app: jaeger
  ports:
  - port: 4317
    targetPort: 4317
    name: otlp-grpc
  - port: 16686
    targetPort: 16686
    name: ui
---
apiVersion: v1
kind: Service
metadata:
  name: jaeger-ui
  namespace: observability
spec:
  type: LoadBalancer
  selector:
    app: jaeger
  ports:
  - port: 80
    targetPort: 16686
    name: ui
```

**xzepr-deployment.yaml:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: xzepr
  namespace: default
spec:
  replicas: 3
  selector:
    matchLabels:
      app: xzepr
  template:
    metadata:
      labels:
        app: xzepr
    spec:
      containers:
      - name: xzepr
        image: xzepr:latest
        ports:
        - containerPort: 8080
        env:
        - name: XZEPR__ENABLE_OTLP
          value: "true"
        - name: XZEPR__OTLP_ENDPOINT
          value: "http://jaeger.observability.svc.cluster.local:4317"
        - name: XZEPR__ENVIRONMENT
          value: "production"
        - name: XZEPR__LOG_LEVEL
          value: "info"
        - name: XZEPR__JSON_LOGS
          value: "true"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

**Deploy:**

```bash
kubectl apply -f jaeger-deployment.yaml
kubectl apply -f xzepr-deployment.yaml
```

### Cloud Environments

**AWS (using AWS Distro for OpenTelemetry):**

```bash
export XZEPR__ENABLE_OTLP=true
export XZEPR__OTLP_ENDPOINT=http://aws-otel-collector:4317
export XZEPR__ENVIRONMENT=production
```

**GCP (using Cloud Trace):**

```bash
export XZEPR__ENABLE_OTLP=true
export XZEPR__OTLP_ENDPOINT=http://otel-collector:4317
export XZEPR__ENVIRONMENT=production
```

**Azure (using Azure Monitor):**

```bash
export XZEPR__ENABLE_OTLP=true
export XZEPR__OTLP_ENDPOINT=http://otel-collector:4317
export XZEPR__ENVIRONMENT=production
```

## Verifying OTLP is Working

### Check Application Logs

**Look for initialization messages:**

```bash
# Docker
docker logs xzepr | grep OTLP

# Kubernetes
kubectl logs -l app=xzepr | grep OTLP
```

**Expected output:**

```
INFO Initializing OTLP exporter otlp_endpoint="http://jaeger:4317" sample_rate=0.1
INFO OTLP exporter initialized successfully
INFO Tracing initialized service="xzepr" environment="production" otlp_enabled=true
INFO OTLP exporter configured and active
```

### Check Jaeger UI

**Access Jaeger:**

```bash
# Local
open http://localhost:16686

# Kubernetes (port-forward)
kubectl port-forward -n observability svc/jaeger 16686:16686
open http://localhost:16686
```

**Verify traces:**

1. Service dropdown should show **xzepr**
2. Click **Find Traces**
3. Traces should appear in the list
4. Click a trace to see span details

### Validate Trace Quality

**Check for these span attributes:**

- `service.name`: xzepr
- `service.version`: (application version)
- `deployment.environment`: production/staging/development
- `http.method`: GET/POST/etc
- `http.route`: /api/events
- `http.status_code`: 200/404/etc

**Check span hierarchy:**

```
Request Span (root)
├── Database Query Span
├── Kafka Publish Span
└── Authentication Span
```

## Troubleshooting

### Problem: No traces appearing in Jaeger

**Check 1: Is OTLP enabled?**

```bash
echo $XZEPR__ENABLE_OTLP
# Should output: true
```

**Check 2: Is Jaeger running?**

```bash
# Docker
docker ps | grep jaeger

# Kubernetes
kubectl get pods -n observability | grep jaeger
```

**Check 3: Can application reach Jaeger?**

```bash
# From application container/pod
curl -v http://jaeger:4317
# Or
nc -zv jaeger 4317
```

**Check 4: Check application logs for errors**

```bash
docker logs xzepr 2>&1 | grep -i "error\|failed"
```

### Problem: OTLP initialization fails

**Symptom:** Warning in logs: "Failed to initialize OTLP exporter"

**Solution 1: Verify endpoint format**

```bash
# Correct (no trailing slash)
XZEPR__OTLP_ENDPOINT=http://jaeger:4317

# Incorrect
XZEPR__OTLP_ENDPOINT=http://jaeger:4317/
```

**Solution 2: Check network connectivity**

```bash
# Ping Jaeger host
ping jaeger

# Check DNS resolution
nslookup jaeger
```

**Solution 3: Verify Jaeger OTLP port**

```bash
# Should show port 4317 listening
docker port jaeger
```

### Problem: Only some requests traced

**This is normal!** Sampling is working as designed.

**Explanation:**

- Production: 10% sampling (1 in 10 requests)
- Staging: 50% sampling (1 in 2 requests)
- Development: 100% sampling (all requests)

**To increase sampling temporarily:**

Change environment to development:

```bash
export XZEPR__ENVIRONMENT=development
```

### Problem: High memory usage

**Symptom:** Application memory growing continuously

**Cause:** Traces not being exported successfully

**Solution 1: Check Jaeger health**

```bash
# Jaeger UI should load
curl http://jaeger:16686
```

**Solution 2: Restart Jaeger**

```bash
docker restart jaeger
```

**Solution 3: Check Jaeger logs**

```bash
docker logs jaeger | grep -i error
```

### Problem: Application won't start with OTLP enabled

**Symptom:** Application exits immediately or crashes on startup

**Cause:** OTLP endpoint unreachable during initialization

**Solution:** Application has graceful fallback - check logs for actual error

```bash
docker logs xzepr 2>&1 | tail -50
```

**Note:** Application should continue running with logging only if OTLP fails.

## Advanced Usage

### Custom Span Attributes

Add custom attributes to spans in your code:

```rust
use tracing::Span;

let span = Span::current();
span.record("user.id", &user_id);
span.record("request.size_bytes", request.len());
span.record("cache.hit", cache_hit);
```

### Manual Span Creation

Create spans manually for fine-grained control:

```rust
use tracing::{info_span, instrument};

let span = info_span!("database_query", query = %sql);
let _guard = span.enter();

// Query execution...

drop(_guard); // Span ends here
```

### Instrument Functions

Use the `#[instrument]` macro:

```rust
use tracing::instrument;

#[instrument(name = "fetch_user", skip(db))]
async fn fetch_user(db: &Database, user_id: String) -> Result<User> {
    // Function body is automatically wrapped in a span
    // user_id is automatically recorded as an attribute
    db.query_user(&user_id).await
}
```

### Error Tracking

Record errors in spans:

```rust
use tracing::{error, Span};

match risky_operation().await {
    Ok(result) => { /* success */ }
    Err(e) => {
        let span = Span::current();
        span.record("error", true);
        span.record("error.message", &e.to_string());
        error!(error = %e, "Operation failed");
    }
}
```

## Best Practices

### DO: Always call shutdown_tracing()

```rust
use xzepr::infrastructure::tracing::shutdown_tracing;

#[tokio::main]
async fn main() {
    // ... application code ...

    // Flush pending spans before exit
    shutdown_tracing();
}
```

### DO: Use descriptive span names

```rust
// Good
#[instrument(name = "process_payment")]

// Bad
#[instrument(name = "fn1")]
```

### DO: Add relevant attributes

```rust
span.record("http.status_code", 200);
span.record("db.statement", &query);
span.record("user.role", &role);
```

### DON'T: Add high-cardinality attributes

```rust
// Bad - unique for every request
span.record("timestamp", SystemTime::now());
span.record("request.body", &large_json);

// Good - bounded values
span.record("http.method", "GET");
span.record("http.status_code", 200);
```

### DON'T: Create too many spans

```rust
// Bad - span for every loop iteration
for item in items {
    let span = info_span!("process_item");
    // ...
}

// Good - single span for batch
let span = info_span!("process_batch", count = items.len());
```

### DO: Use appropriate log levels

```rust
tracing::trace!("Fine-grained debug info");
tracing::debug!("Debug information");
tracing::info!("Informational messages");
tracing::warn!("Warning conditions");
tracing::error!("Error conditions");
```

## Performance Tuning

### Reduce Overhead

**Lower sample rate in production:**

```bash
# Already configured automatically for production
XZEPR__ENVIRONMENT=production  # 10% sampling
```

**Limit span attributes:**

Only add attributes that provide value for debugging.

**Use async export:**

OTLP export is already async and non-blocking.

### Monitor OTLP Performance

**Check export latency:**

Look for these metrics in Jaeger:
- Span export duration
- Failed export count
- Buffered span count

**Check memory usage:**

```bash
# Docker
docker stats xzepr

# Kubernetes
kubectl top pod -l app=xzepr
```

## Next Steps

1. **Enable OTLP in staging environment**
2. **Generate test traffic and validate traces**
3. **Create custom dashboards in Jaeger/Grafana**
4. **Set up alerts for trace export failures**
5. **Train team on trace analysis**
6. **Deploy to production with monitoring**

## Resources

### Internal Documentation

- Implementation: `docs/explanations/otlp_exporter_implementation.md`
- Architecture: `docs/explanations/distributed_tracing_architecture.md`
- Observability: `docs/explanations/observability_architecture.md`

### External Resources

- Jaeger Documentation: https://www.jaegertracing.io/docs/
- OpenTelemetry: https://opentelemetry.io/docs/
- Tracing Crate: https://docs.rs/tracing/

## Support

For issues or questions:

1. Check application logs for error messages
2. Review troubleshooting section above
3. Consult internal documentation
4. Open an issue in the XZepr repository

---

**Last Updated:** 2024
**Status:** Production Ready
