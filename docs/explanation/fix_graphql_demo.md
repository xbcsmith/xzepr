# How to Fix GraphQL Demo Issues

## Overview

This guide provides step-by-step instructions for fixing the issues identified
during GraphQL Playground demo testing. These fixes will enable the GraphQL
endpoints and update the documentation to match the actual system behavior.

## Issue Summary

The demo has three critical issues preventing GraphQL functionality:

1. GraphQL routes are not registered in the main server binary
2. Environment variable documentation uses incorrect format
3. Database password in documentation doesn't match actual configuration

## Prerequisites

- Basic understanding of Rust and Axum web framework
- Text editor or IDE
- Docker and Docker Compose installed
- Access to project source code

## Fix 1: Integrate GraphQL Routes

### Problem

The main server binary (`src/main.rs`) uses a simple router without GraphQL
routes, while a comprehensive router with GraphQL support exists in
`src/api/rest/routes.rs` but is never used.

### Solution Option A: Use Existing GraphQL Router (Recommended)

**Step 1:** Open `src/main.rs` and locate the `build_router` function (around
line 131).

**Step 2:** Check if the function imports are present. If not, add these imports
at the top of the file:

```rust
use xzepr::api::rest::routes::build_router as build_api_router;
use xzepr::application::handlers::{
    EventHandler, EventReceiverGroupHandler, EventReceiverHandler
};
```

**Step 3:** Modify the `main` function to create the application handlers and
use the comprehensive router. Replace the current state creation section:

```rust
// Create application state
let state = AppState {
    db_pool: db_pool.clone(),
    user_repo,
    api_key_repo,
};

// Build router with all routes and middleware
let app = build_router(state);
```

With:

```rust
// Initialize repositories for application handlers
let event_repo = Arc::new(xzepr::infrastructure::repository::PostgresEventRepository::new(db_pool.clone()));
let receiver_repo = Arc::new(xzepr::infrastructure::repository::PostgresEventReceiverRepository::new(db_pool.clone()));
let group_repo = Arc::new(xzepr::infrastructure::repository::PostgresEventReceiverGroupRepository::new(db_pool.clone()));

// Create application handlers
let event_handler = EventHandler::new(event_repo, receiver_repo.clone());
let receiver_handler = EventReceiverHandler::new(receiver_repo.clone());
let group_handler = EventReceiverGroupHandler::new(group_repo, receiver_repo.clone());

// Create application state for API router
let api_state = xzepr::api::rest::AppState {
    event_handler,
    event_receiver_handler: receiver_handler,
    event_receiver_group_handler: group_handler,
};

// Use the comprehensive router with GraphQL support
let app = build_api_router(api_state);
```

**Step 4:** Build and test:

```bash
cargo build --release
```

### Solution Option B: Add GraphQL Routes to Main Router

If Option A causes conflicts, manually add GraphQL routes to the existing
router.

**Step 1:** Add imports at the top of `src/main.rs`:

```rust
use xzepr::api::graphql::{graphql_handler, graphql_playground, graphql_health_check};
use xzepr::api::graphql::schema::create_schema;
```

**Step 2:** Modify `build_router` to include GraphQL routes:

```rust
fn build_router(state: AppState) -> Router {
    // Create GraphQL schema
    let schema = create_schema(
        state.event_receiver_handler.clone(),
        state.event_receiver_group_handler.clone(),
    );

    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    // Create tracing layer
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(LatencyUnit::Millis),
        );

    Router::new()
        .route("/health", get(health_check))
        .route("/", get(root_handler))
        .route("/api/v1/status", get(api_status))
        .route("/api/v1/auth/login", post(login))
        // GraphQL routes
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        .route("/graphql/health", get(graphql_health_check))
        .layer(ServiceBuilder::new().layer(trace_layer).layer(cors))
        .with_state((state, schema))
}
```

**Step 3:** Build and test:

```bash
cargo build --release
```

## Fix 2: Update Environment Variable Documentation

### Problem

Tutorial uses `DATABASE_URL` but the configuration system requires
`XZEPR__DATABASE__URL` format.

### Solution

**Step 1:** Open `docs/tutorials/docker_demo.md`

**Step 2:** Find all Docker run commands with environment variables.

**Step 3:** Update environment variable format:

**Before:**

```bash
-e DATABASE_URL="postgres://xzepr:xzepr@xzepr-postgres-1:5432/xzepr"
-e KAFKA_BROKERS="redpanda-0:9092"
-e ENABLE_HTTPS="false"
-e HTTP_PORT="8443"
```

**After:**

```bash
-e XZEPR__DATABASE__URL="postgres://xzepr:password@xzepr-postgres-1:5432/xzepr"
-e XZEPR__KAFKA__BROKERS="redpanda-0:9092"
-e XZEPR__SERVER__ENABLE_HTTPS="false"
-e XZEPR__SERVER__PORT="8443"
-e XZEPR__AUTH__JWT_SECRET="your-secret-key-change-in-production"
```

**Step 4:** Update all admin CLI examples:

**Before:**

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e DATABASE_URL="postgres://xzepr:xzepr@xzepr-postgres-1:5432/xzepr" \
  xzepr:latest \
  /app/admin create-user ...
```

**After:**

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL="postgres://xzepr:password@xzepr-postgres-1:5432/xzepr" \
  xzepr:latest \
  /app/admin create-user --role admin ...
```

**Step 5:** Add required `--role` flag to all `create-user` examples.

## Fix 3: Database Password Consistency

### Problem

Tutorial uses password `xzepr` but `docker-compose.services.yaml` sets password
to `password`.

### Solution Option A: Update Documentation (Recommended)

**Step 1:** Open `docs/tutorials/docker_demo.md`

**Step 2:** Find all database connection strings with password `xzepr`

**Step 3:** Replace with `password`:

```bash
# Old
postgres://xzepr:xzepr@xzepr-postgres-1:5432/xzepr

# New
postgres://xzepr:password@xzepr-postgres-1:5432/xzepr
```

### Solution Option B: Update Docker Compose

**Step 1:** Open `docker-compose.services.yaml`

**Step 2:** Change the PostgreSQL password:

```yaml
environment:
  - POSTGRES_USER=xzepr
  - POSTGRES_PASSWORD=xzepr  # Changed from 'password'
  - POSTGRES_DB=xzepr
```

**Step 3:** Update documentation to match.

## Fix 4: Remove Non-Existent Commands

### Problem

Tutorial references `admin db migrate` command that doesn't exist.

### Solution

**Step 1:** Open `docs/tutorials/docker_demo.md`

**Step 2:** Find Step 5: Initialize the Database

**Step 3:** Replace the entire step with:

```markdown
## Step 5: Verify Database Schema

The XZepr server automatically creates database tables on first startup using
SQLx migrations. No manual migration command is needed.

To verify the database is accessible:

```bash
docker exec xzepr-postgres-1 psql -U xzepr -d xzepr -c "\dt"
```

This will show "Did not find any relations" until the server has started once.
```

## Fix 5: Add Configuration Troubleshooting

Add a new troubleshooting section to the tutorial.

**Step 1:** Open `docs/tutorials/docker_demo.md`

**Step 2:** Add to the Troubleshooting section:

```markdown
### Environment Variables Not Taking Effect

**Symptom:** Server logs show default values instead of your configured values

**Cause:** Environment variables must use the `XZEPR__` prefix with double
underscore separators

**Solution:**

Verify your environment variables follow this pattern:

```bash
XZEPR__SECTION__KEY=value
```

Examples:

```bash
XZEPR__DATABASE__URL="postgres://..."
XZEPR__SERVER__PORT=8443
XZEPR__SERVER__ENABLE_HTTPS=false
XZEPR__AUTH__JWT_SECRET="secret"
```

To verify configuration, check the server startup logs for the actual values
being used.
```

## Verification Steps

After applying all fixes, verify the functionality:

### Step 1: Rebuild Docker Image

```bash
cd /path/to/xzepr
docker build -t xzepr:latest .
```

### Step 2: Start Services

```bash
docker compose -f docker-compose.services.yaml up -d
```

### Step 3: Start XZepr Server

```bash
docker run -d \
  --name xzepr-server \
  --network xzepr_redpanda_network \
  -p 8042:8443 \
  -v $(pwd)/certs:/app/certs:ro \
  -e XZEPR__DATABASE__URL="postgres://xzepr:password@xzepr-postgres-1:5432/xzepr" \
  -e XZEPR__KAFKA__BROKERS="redpanda-0:9092" \
  -e XZEPR__SERVER__ENABLE_HTTPS="false" \
  -e XZEPR__SERVER__PORT="8443" \
  -e XZEPR__AUTH__JWT_SECRET="test-secret-key" \
  -e RUST_LOG="info,xzepr=debug" \
  xzepr:latest
```

### Step 4: Verify Health

```bash
curl http://localhost:8042/health | jq .
```

Expected output:

```json
{
  "status": "healthy",
  "service": "xzepr",
  "version": "0.1.0",
  "components": {
    "database": "healthy"
  }
}
```

### Step 5: Verify GraphQL Health

```bash
curl http://localhost:8042/graphql/health | jq .
```

Expected output:

```json
{
  "status": "healthy",
  "service": "graphql",
  "endpoint": "/graphql"
}
```

### Step 6: Test GraphQL Playground

Open browser to:

```text
http://localhost:8042/graphql/playground
```

You should see the GraphQL Playground IDE interface.

### Step 7: Execute Test Query

In the playground, run:

```graphql
query {
  eventReceivers {
    id
    name
    type
    version
  }
}
```

Expected: Empty array `[]` (no receivers created yet)

### Step 8: Cleanup

```bash
docker stop xzepr-server && docker rm xzepr-server
docker compose -f docker-compose.services.yaml down
```

## Additional Improvements

### Add Integration Tests

Create `tests/integration/graphql_endpoints_test.rs`:

```rust
#[tokio::test]
async fn test_graphql_playground_accessible() {
    let response = reqwest::get("http://localhost:8042/graphql/playground")
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("GraphQL Playground"));
}

#[tokio::test]
async fn test_graphql_health_endpoint() {
    let response = reqwest::get("http://localhost:8042/graphql/health")
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["status"], "healthy");
}

#[tokio::test]
async fn test_graphql_query_endpoint() {
    let client = reqwest::Client::new();
    let query = r#"{"query": "{ eventReceivers { id } }"}"#;

    let response = client
        .post("http://localhost:8042/graphql")
        .header("Content-Type", "application/json")
        .body(query)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), StatusCode::OK);
}
```

### Update CI Pipeline

Add to `.github/workflows/ci.yaml`:

```yaml
- name: Run integration tests
  run: |
    docker compose -f docker-compose.services.yaml up -d
    sleep 10
    docker run -d --name xzepr-test --network xzepr_redpanda_network \
      -p 8042:8443 \
      -e XZEPR__DATABASE__URL="postgres://xzepr:password@xzepr-postgres-1:5432/xzepr" \
      -e XZEPR__SERVER__ENABLE_HTTPS="false" \
      xzepr:latest
    sleep 5
    curl -f http://localhost:8042/health
    curl -f http://localhost:8042/graphql/health
    docker stop xzepr-test
    docker compose -f docker-compose.services.yaml down
```

## Common Pitfalls

### Router Module Conflicts

If you get compilation errors about duplicate route definitions, ensure you're
only using ONE router implementation:

- Either use `xzepr::api::rest::routes::build_router`
- Or use the `build_router` in `src/main.rs`
- Do NOT try to merge both

### Database Connection Pool Exhaustion

If admin CLI commands fail with "PoolTimedOut", the server might be holding all
connections. Increase pool size in configuration:

```rust
let db_pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(&settings.database.url)
    .await?;
```

### Certificate Volume Mount Issues

If running without HTTPS, you may still need the certs directory to exist:

```bash
mkdir -p certs
touch certs/cert.pem certs/key.pem
```

Or remove the volume mount entirely when `ENABLE_HTTPS=false`.

## Summary

After applying these fixes:

1. ✅ GraphQL Playground will be accessible
2. ✅ GraphQL query endpoint will work
3. ✅ Documentation will match actual system behavior
4. ✅ Environment variables will work as documented
5. ✅ Admin CLI examples will succeed

Total time to fix: Approximately 2-3 hours including testing.

## References

- Configuration System: `src/infrastructure/config.rs`
- Main Server Binary: `src/main.rs`
- GraphQL Routes: `src/api/rest/routes.rs`
- GraphQL Handlers: `src/api/graphql/handlers.rs`
- Tutorial: `docs/tutorials/docker_demo.md`
- Test Results: `docs/explanation/graphql_demo_test_results.md`
