# OpenTelemetry OTLP Quick Reference

## Quick Start

```bash
# 1. Start Jaeger
docker run -d --name jaeger \
  -p 4317:4317 -p 16686:16686 \
  -e COLLECTOR_OTLP_ENABLED=true \
  jaegertracing/all-in-one:latest

# 2. Enable OTLP in XZepr
export XZEPR__ENABLE_OTLP=true
export XZEPR__OTLP_ENDPOINT=http://localhost:4317

# 3. Start XZepr
cargo run --bin server

# 4. View traces
open http://localhost:16686
```

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `XZEPR__ENABLE_OTLP` | Yes | `false` | Enable OTLP exporter |
| `XZEPR__OTLP_ENDPOINT` | Yes* | None | OTLP collector endpoint |
| `XZEPR__ENVIRONMENT` | No | `development` | Environment (affects sampling) |
| `XZEPR__LOG_LEVEL` | No | `info` | Log level filter |
| `XZEPR__JSON_LOGS` | No | `false` | JSON log format |

*Required when OTLP is enabled

## Environment Defaults

| Environment | OTLP | Sample Rate | Log Format |
|-------------|------|-------------|------------|
| Development | Off  | 100%        | Human      |
| Staging     | On   | 50%         | JSON       |
| Production  | On   | 10%         | JSON       |

## OTLP Endpoints

| Environment | Endpoint |
|-------------|----------|
| Local Docker | `http://localhost:4317` |
| Kubernetes | `http://jaeger.observability.svc.cluster.local:4317` |
| Docker Compose | `http://jaeger:4317` |

## Jaeger Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 4317 | gRPC | OTLP collector |
| 4318 | HTTP | OTLP HTTP (not used) |
| 16686 | HTTP | Jaeger UI |
| 14268 | HTTP | Jaeger collector |

## Common Commands

### Docker

```bash
# Start Jaeger
docker run -d --name jaeger \
  -p 4317:4317 -p 16686:16686 \
  -e COLLECTOR_OTLP_ENABLED=true \
  jaegertracing/all-in-one:latest

# Check Jaeger status
docker ps | grep jaeger

# View Jaeger logs
docker logs jaeger

# Stop Jaeger
docker stop jaeger

# Remove Jaeger
docker rm jaeger
```

### Kubernetes

```bash
# Deploy Jaeger
kubectl apply -f jaeger-deployment.yaml

# Check status
kubectl get pods -n observability | grep jaeger

# Port forward to access UI
kubectl port-forward -n observability svc/jaeger 16686:16686

# View logs
kubectl logs -n observability -l app=jaeger
```

### Application

```bash
# Enable OTLP
export XZEPR__ENABLE_OTLP=true
export XZEPR__OTLP_ENDPOINT=http://localhost:4317

# Start application
cargo run --bin server

# Check OTLP status in logs
cargo run --bin server 2>&1 | grep OTLP

# Expected output:
# INFO Initializing OTLP exporter
# INFO OTLP exporter initialized successfully
# INFO OTLP exporter configured and active
```

## Code Examples

### Application Setup

```rust
use xzepr::infrastructure::tracing::{init_tracing, shutdown_tracing, TracingConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with OTLP
    let config = TracingConfig::from_env();
    init_tracing(config)?;

    // Application code
    run_server().await?;

    // Shutdown and flush traces
    shutdown_tracing();

    Ok(())
}
```

### Creating Spans

```rust
use tracing::{info, instrument};

#[instrument(name = "process_request")]
async fn process_request(id: String) -> Result<Response> {
    info!(request_id = %id, "Processing request");

    // Work happens here
    let result = fetch_data(&id).await?;

    info!("Request completed");
    Ok(result)
}
```

### Adding Attributes

```rust
use tracing::Span;

let span = Span::current();
span.record("http.status_code", 200);
span.record("user.id", &user_id);
span.record("cache.hit", true);
```

## Troubleshooting

### No traces in Jaeger

```bash
# Check if OTLP enabled
echo $XZEPR__ENABLE_OTLP  # Should be: true

# Check Jaeger running
docker ps | grep jaeger

# Check connectivity
curl -v http://localhost:4317

# Check application logs
cargo run 2>&1 | grep -i "otlp\|error"
```

### OTLP initialization fails

```bash
# Check endpoint format (no trailing slash)
echo $XZEPR__OTLP_ENDPOINT
# Correct: http://jaeger:4317
# Wrong: http://jaeger:4317/

# Test network connectivity
ping jaeger
nc -zv jaeger 4317
```

### Only some requests traced

This is normal! Check sampling rate for your environment:
- Production: 10% (1 in 10 requests)
- Staging: 50% (1 in 2 requests)
- Development: 100% (all requests)

To see all traces, use development environment:
```bash
export XZEPR__ENVIRONMENT=development
```

## Docker Compose Example

```yaml
version: '3.8'

services:
  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "4317:4317"
      - "16686:16686"
    environment:
      - COLLECTOR_OTLP_ENABLED=true

  xzepr:
    build: .
    depends_on:
      - jaeger
    environment:
      - XZEPR__ENABLE_OTLP=true
      - XZEPR__OTLP_ENDPOINT=http://jaeger:4317
      - XZEPR__ENVIRONMENT=development
    ports:
      - "8080:8080"
```

Start with:
```bash
docker-compose up -d
```

## Kubernetes Example

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: xzepr
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
        env:
        - name: XZEPR__ENABLE_OTLP
          value: "true"
        - name: XZEPR__OTLP_ENDPOINT
          value: "http://jaeger.observability.svc.cluster.local:4317"
        - name: XZEPR__ENVIRONMENT
          value: "production"
```

## Best Practices

### DO

- ✅ Always call `shutdown_tracing()` before exit
- ✅ Use descriptive span names
- ✅ Add relevant attributes (status code, user ID, etc.)
- ✅ Use `#[instrument]` macro for automatic instrumentation
- ✅ Log errors with span context

### DON'T

- ❌ Add high-cardinality attributes (timestamps, full bodies)
- ❌ Create spans for every loop iteration
- ❌ Use generic span names ("process", "handler")
- ❌ Forget to flush spans on shutdown
- ❌ Block on span export (it's already async)

## Performance Impact

| Metric | Impact |
|--------|--------|
| Request overhead | <1ms |
| CPU per span | <0.5ms |
| Memory (1000 spans) | ~5MB |
| Network | Batch export, minimal |

## Sampling Strategy

| Traffic Level | Recommended Sample Rate |
|---------------|------------------------|
| Low (<100 RPS) | 100% (AlwaysOn) |
| Medium (100-1000 RPS) | 50% (TraceIdRatio 0.5) |
| High (>1000 RPS) | 10% (TraceIdRatio 0.1) |

## Log Messages

### Success

```
INFO Initializing OTLP exporter otlp_endpoint="http://jaeger:4317" sample_rate=0.1
INFO OTLP exporter initialized successfully
INFO Tracing initialized service="xzepr" environment="production" otlp_enabled=true
INFO OTLP exporter configured and active
```

### Failure (with fallback)

```
INFO Initializing OTLP exporter otlp_endpoint="http://jaeger:4317"
WARN Failed to initialize OTLP exporter, continuing without it
INFO Tracing initialized service="xzepr" otlp_enabled=false
```

## Resources

### Documentation

- Implementation: `docs/explanations/otlp_exporter_implementation.md`
- How-to: `docs/how_to/enable_otlp_tracing.md`
- Validation: `docs/explanations/otlp_integration_validation.md`

### External

- Jaeger: https://www.jaegertracing.io/docs/
- OpenTelemetry: https://opentelemetry.io/docs/
- Tracing: https://docs.rs/tracing/

## Support

For issues:
1. Check application logs for error messages
2. Verify Jaeger is running and accessible
3. Review troubleshooting section above
4. Consult full documentation

---

**Last Updated:** 2024
**Version:** 1.0
