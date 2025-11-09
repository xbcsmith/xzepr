# Distributed Tracing Architecture

## Overview

XZepr implements a comprehensive distributed tracing infrastructure built on the `tracing` ecosystem, providing structured logging, span creation, request correlation, and readiness for OpenTelemetry/Jaeger integration.

## Architecture Principles

### 1. Structured Logging First

All logging uses structured fields instead of string interpolation, enabling:
- Machine-parseable logs
- Efficient log aggregation
- Correlation across services
- Performance monitoring

### 2. Zero-Configuration Instrumentation

Tracing middleware automatically instruments all HTTP requests without requiring manual span creation in handlers.

### 3. Environment-Aware Configuration

Different configurations for development, staging, and production:
- **Development**: Verbose logging, human-readable format, file/line numbers
- **Staging**: Balanced sampling, JSON logs, moderate verbosity
- **Production**: Optimized sampling, JSON logs, minimal overhead

### 4. OpenTelemetry Ready

The architecture is designed to easily integrate OpenTelemetry and Jaeger when needed, with minimal code changes.

## Components

### Core Tracing Infrastructure

**File:** `src/infrastructure/tracing.rs`

Provides the foundational tracing capabilities:

#### TracingConfig

Configuration for the tracing system:

```rust
pub struct TracingConfig {
    pub service_name: String,
    pub service_version: String,
    pub jaeger_endpoint: Option<String>,
    pub sample_rate: f64,
    pub environment: String,
    pub enabled: bool,
    pub log_level: String,
    pub json_logs: bool,
    pub show_target: bool,
    pub show_thread_ids: bool,
    pub show_thread_names: bool,
    pub show_file_line: bool,
}
```

**Environment-Specific Configurations:**

```rust
// Development
TracingConfig {
    log_level: "debug",
    json_logs: false,
    show_file_line: true,
    sample_rate: 1.0,
    ..
}

// Production
TracingConfig {
    log_level: "info",
    json_logs: true,
    show_thread_ids: true,
    sample_rate: 0.1,
    ..
}
```

#### Initialization

```rust
pub fn init_tracing(config: TracingConfig) -> Result<(), Box<dyn std::error::Error>>
```

Sets up the tracing subscriber with appropriate formatting and filtering.

**Features:**
- Environment-based log filtering
- JSON or human-readable output
- Span event tracking (NEW and CLOSE events)
- Service metadata injection
- Configurable detail levels

### Tracing Middleware

**File:** `src/api/middleware/tracing_middleware.rs`

Automatic HTTP request instrumentation with multiple variants:

#### Basic Tracing Middleware

```rust
pub async fn tracing_middleware(request: Request, next: Next) -> Response
```

Automatically creates spans for all HTTP requests with:
- Request method and path
- Request ID tracking
- Response status code
- Duration measurement
- Structured event logging

**Span Fields:**
- `http.method` - HTTP method (GET, POST, etc.)
- `http.route` - Matched route pattern
- `http.target` - Full URI
- `http.request_id` - Request correlation ID
- `http.trace_id` - Distributed trace ID
- `http.status_code` - Response status
- `http.duration_ms` - Request duration

#### Enhanced Tracing Middleware

```rust
pub async fn enhanced_tracing_middleware(request: Request, next: Next) -> Response
```

Extended version that includes:
- User authentication information
- Response size tracking
- Custom span attributes
- Request ID generation and injection

**Additional Span Fields:**
- `http.user_id` - Authenticated user ID
- `http.response_size` - Response body size

#### Request ID Middleware

```rust
pub async fn request_id_middleware(request: Request, next: Next) -> Response
```

Manages request ID lifecycle:
- Extracts existing request ID from headers
- Generates new ID if not present
- Stores ID in request extensions
- Injects ID into response headers

### Trace Context Propagation

#### Extract Trace Context

```rust
pub fn extract_trace_context(headers: &HeaderMap) -> Option<String>
```

Extracts trace identifiers from HTTP headers:
- `traceparent` - W3C Trace Context standard
- `x-request-id` - Fallback correlation ID

#### Inject Trace Context

```rust
pub fn inject_trace_context(headers: &mut HeaderMap, trace_id: &str)
```

Injects trace identifiers into outgoing requests:
- Adds `x-request-id` header
- Prepared for W3C Trace Context format

#### Generate Trace ID

```rust
pub fn generate_trace_id() -> String
```

Creates unique trace identifiers:
- 32-character hexadecimal format
- Timestamp-based prefix for ordering
- Random suffix for uniqueness
- Compatible with OpenTelemetry format

## Span Lifecycle

### 1. Request Arrival

```
Client Request
     ↓
Extract trace_id from headers (or generate)
     ↓
Create root span with request metadata
```

### 2. Request Processing

```
Root Span
     ↓
middleware::tracing_middleware
     ├── CORS middleware
     ├── Metrics middleware
     ├── Rate limiting
     ├── Authentication
     └── Handler execution
```

### 3. Response Completion

```
Handler completes
     ↓
Record response status and duration
     ↓
Log completion event
     ↓
Close span
```

## Logging Patterns

### Structured Logging

**Good - Structured fields:**
```rust
tracing::info!(
    method = %request.method(),
    path = %request.uri(),
    status = %response.status(),
    duration_ms = duration.as_millis(),
    "HTTP request completed"
);
```

**Bad - String interpolation:**
```rust
tracing::info!("Request {} {} completed with {}",
    method, path, status);
```

### Log Levels

**ERROR** - System errors requiring immediate attention:
```rust
tracing::error!(
    error = %err,
    "Database connection failed"
);
```

**WARN** - Degraded operation or recoverable issues:
```rust
tracing::warn!(
    status = %status,
    "HTTP request client error"
);
```

**INFO** - Normal operation events:
```rust
tracing::info!(
    service = %config.service_name,
    version = %config.service_version,
    "Tracing initialized"
);
```

**DEBUG** - Detailed diagnostic information:
```rust
tracing::debug!(
    user_id = %user.id,
    "User authenticated"
);
```

**TRACE** - Very detailed execution traces:
```rust
tracing::trace!(
    query = %sql,
    "Executing database query"
);
```

## Request ID Tracking

### Request ID Format

Generated IDs follow the pattern: `req-{timestamp}-{counter}`

Example: `req-1704123456789-42`

### Request ID Flow

```
1. Client sends request (with or without x-request-id)
   ↓
2. request_id_middleware extracts or generates ID
   ↓
3. ID stored in request extensions
   ↓
4. ID available to all handlers and middleware
   ↓
5. ID injected into response headers
```

### Using Request ID in Handlers

```rust
use xzepr::api::middleware::tracing_middleware::RequestId;

async fn handler(
    Extension(request_id): Extension<RequestId>,
) -> Response {
    tracing::info!(
        request_id = %request_id.0,
        "Processing request"
    );
    // Handler logic
}
```

## Error Handling with Tracing

### TracedError

Automatic error logging with trace context:

```rust
use xzepr::api::middleware::tracing_middleware::TracedError;

async fn handler() -> Result<Response, TracedError> {
    if validation_failed {
        return Err(TracedError::new(
            StatusCode::BAD_REQUEST,
            "Validation failed"
        ).with_request_id(request_id));
    }
    Ok(response)
}
```

**Features:**
- Automatic error logging
- Request ID correlation
- Structured error responses
- JSON error format

## Log Output Formats

### Development Format

Human-readable output with colors:

```
2024-01-01T12:00:00.123Z  INFO xzepr: Tracing initialized
    service: xzepr
    version: 0.1.0
    environment: development

2024-01-01T12:00:01.456Z  INFO http_request: HTTP request completed
    http.method: GET
    http.route: /api/v1/events
    http.status_code: 200
    http.duration_ms: 45
```

### Production Format

JSON output for log aggregation:

```json
{
  "timestamp": "2024-01-01T12:00:00.123Z",
  "level": "INFO",
  "target": "xzepr",
  "fields": {
    "message": "Tracing initialized",
    "service": "xzepr",
    "version": "0.1.0",
    "environment": "production"
  },
  "span": {
    "name": "http_request",
    "http.method": "GET",
    "http.route": "/api/v1/events",
    "http.status_code": 200,
    "http.duration_ms": 45
  }
}
```

## Configuration

### Environment Variables

```bash
# Environment (development, staging, production)
XZEPR__ENVIRONMENT=production

# Log level (trace, debug, info, warn, error)
XZEPR__LOG_LEVEL=info

# JSON formatted logs
XZEPR__JSON_LOGS=true

# Jaeger endpoint (future OpenTelemetry integration)
XZEPR__JAEGER_ENDPOINT=http://jaeger:4317
```

### Application Configuration

```rust
use xzepr::infrastructure::tracing::{init_tracing, TracingConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = TracingConfig::from_env();

    // Initialize tracing
    init_tracing(config)?;

    tracing::info!("Application started");

    // Application code...

    Ok(())
}
```

## OpenTelemetry Integration (Future)

The current implementation is designed to easily integrate OpenTelemetry when needed.

### Required Dependencies

Add to `Cargo.toml`:

```toml
opentelemetry = "0.26"
opentelemetry_sdk = { version = "0.26", features = ["rt-tokio"] }
opentelemetry-otlp = "0.26"
tracing-opentelemetry = "0.26"
```

### Integration Steps

1. **Update `init_tracing` function:**

```rust
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry_otlp::WithExportConfig;

pub fn init_tracing(config: TracingConfig) -> Result<()> {
    // Create OTLP exporter
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.jaeger_endpoint.unwrap())
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Create OpenTelemetry layer
    let telemetry_layer = OpenTelemetryLayer::new(tracer);

    // Add to subscriber
    Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .with(telemetry_layer)  // Add this layer
        .init();

    Ok(())
}
```

2. **Update trace context functions to use W3C format**
3. **Enable global trace propagation**

## Jaeger Deployment

### Docker Compose

```yaml
services:
  jaeger:
    image: jaegertracing/all-in-one:latest
    environment:
      - COLLECTOR_OTLP_ENABLED=true
    ports:
      - "16686:16686"  # Jaeger UI
      - "4317:4317"    # OTLP gRPC
      - "4318:4318"    # OTLP HTTP
```

### Kubernetes

```yaml
apiVersion: v1
kind: Service
metadata:
  name: jaeger
spec:
  ports:
    - name: otlp-grpc
      port: 4317
      targetPort: 4317
    - name: ui
      port: 16686
      targetPort: 16686
  selector:
    app: jaeger
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: jaeger
spec:
  template:
    spec:
      containers:
        - name: jaeger
          image: jaegertracing/all-in-one:latest
          env:
            - name: COLLECTOR_OTLP_ENABLED
              value: "true"
          ports:
            - containerPort: 4317
            - containerPort: 16686
```

## Performance Considerations

### Overhead Analysis

**Per-Request Tracing:**
- Span creation: ~200ns
- Field recording: ~50ns per field
- Event logging: ~100ns
- Total overhead: ~500-1000ns (<0.001% for typical 50ms request)

**Memory Usage:**
- Active span: ~1KB
- Buffered events: ~100 bytes each
- Minimal impact on typical applications

### Optimization Strategies

1. **Sampling** - Reduce trace volume in production:
```rust
TracingConfig {
    sample_rate: 0.1,  // Sample 10% of requests
    ..
}
```

2. **Log Level Filtering** - Limit verbosity:
```rust
EnvFilter::new("xzepr=info,tower_http=warn")
```

3. **Conditional Tracing** - Skip traces for health checks:
```rust
if !path.starts_with("/health") {
    tracing::info_span!("http_request").in_scope(|| {
        // Process request
    });
}
```

## Best Practices

### DO

1. **Use structured fields** for all log statements
2. **Create spans** for logical operations
3. **Record duration** for performance-sensitive operations
4. **Propagate trace context** across service boundaries
5. **Use appropriate log levels** (ERROR for failures, INFO for normal operations)
6. **Include request ID** in all logs

### DON'T

1. **Don't log sensitive data** (passwords, tokens, PII)
2. **Don't create excessive spans** (adds overhead)
3. **Don't use string interpolation** in log messages
4. **Don't log in hot loops** without sampling
5. **Don't block on logging** operations
6. **Don't log raw SQL** with user data

## Testing

### Unit Tests

```rust
#[test]
fn test_trace_id_generation() {
    let id1 = generate_trace_id();
    let id2 = generate_trace_id();

    assert_ne!(id1, id2);
    assert_eq!(id1.len(), 32);
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_request_id_propagation() {
    let app = test_app();

    let response = app
        .get("/api/events")
        .header("x-request-id", "test-123")
        .await;

    assert_eq!(
        response.headers().get("x-request-id").unwrap(),
        "test-123"
    );
}
```

## Troubleshooting

### Logs Not Appearing

**Check:**
1. Tracing initialized: `init_tracing()` called
2. Log level: `XZEPR__LOG_LEVEL=debug`
3. Filter configuration: Check `EnvFilter` settings

### Missing Request IDs

**Ensure:**
1. `request_id_middleware` is applied
2. Middleware order is correct
3. Extensions are accessible in handlers

### Performance Issues

**Solutions:**
1. Reduce log level in production
2. Enable sampling for high-volume endpoints
3. Use async logging backends
4. Filter noisy components

## References

- [Tracing Ecosystem](https://docs.rs/tracing/)
- [Tracing Subscriber](https://docs.rs/tracing-subscriber/)
- [OpenTelemetry](https://opentelemetry.io/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
