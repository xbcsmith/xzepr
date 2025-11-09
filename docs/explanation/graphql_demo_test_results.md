# GraphQL Playground Demo Test Results

## Overview

This document summarizes the results of testing the Docker-based GraphQL
Playground demo tutorial for XZepr. The test was conducted on October 18, 2025,
following the steps outlined in `docs/tutorials/docker_demo.md`.

## Test Environment

- **Operating System:** Linux
- **Docker Version:** 28.5.1
- **Docker Compose Version:** v2.24.5
- **Project Directory:** `/home/bsmith/go/src/github.com/xbcsmith/xzepr`

## Test Results Summary

| Component | Status | Notes |
|-----------|--------|-------|
| Docker Prerequisites | ✅ Pass | Docker and Docker Compose available |
| TLS Certificate Generation | ✅ Pass | Certificates created successfully |
| Backend Services Startup | ✅ Pass | PostgreSQL, Keycloak, Redpanda started |
| XZepr Docker Image Build | ✅ Pass | Image built successfully (1 warning) |
| Server Configuration | ⚠️ Partial | Environment variable format issue |
| Database Connection | ✅ Pass | Connected after config fix |
| Server Health Check | ✅ Pass | HTTP health endpoint working |
| GraphQL Endpoints | ❌ Fail | Routes not registered in main.rs |
| Admin CLI | ⚠️ Partial | Available but requires server running |

## Detailed Test Execution

### Step 1: Prerequisites Check

**Command:**

```bash
docker --version && docker compose version
```

**Result:** ✅ Pass

```text
Docker version 28.5.1, build e180ab8
Docker Compose version v2.24.5
```

### Step 2: TLS Certificate Creation

**Command:**

```bash
mkdir -p certs && cd certs
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem \
  -days 365 -nodes -subj "/CN=localhost"
```

**Result:** ✅ Pass

- Created `certs/cert.pem` (1805 bytes)
- Created `certs/key.pem` (3272 bytes)

### Step 3: Backend Services Startup

**Command:**

```bash
docker compose -f docker-compose.services.yaml up -d
```

**Result:** ✅ Pass

All services started successfully:

- `xzepr-postgres-1` - PostgreSQL 16 (healthy)
- `xzepr-keycloak-1` - Keycloak 24.0
- `redpanda-0` - Redpanda (Kafka-compatible)
- `redpanda-console` - Redpanda Console UI

**Ports Mapped:**

- PostgreSQL: `5432` (host and container)
- Keycloak: `8080` (host and container)
- Redpanda Kafka: `19092` (host) → `9092` (container)
- Redpanda Console: `8081` (host) → `8080` (container)

### Step 4: Docker Image Build

**Command:**

```bash
docker build -t xzepr:latest .
```

**Result:** ✅ Pass (with warning)

- Build completed in ~43 seconds
- Release binaries created: `xzepr` and `admin`
- Image size: ~19MB (runtime stage)

**Warning Found:**

```text
SecretsUsedInArgOrEnv: Do not use ARG or ENV instructions for sensitive data
(ENV "XZEPR__TLS__KEY_PATH") (line 84)
```

**Analysis:** Acceptable for demo purposes; production deployments should use
Docker secrets or mounted configuration files.

### Step 5: Server Configuration Discovery

**Issue Identified:** Environment variable naming mismatch

The tutorial documentation specified:

```bash
-e DATABASE_URL="postgres://xzepr:xzepr@xzepr-postgres-1:5432/xzepr"
```

However, the configuration system in `src/infrastructure/config.rs` uses the
`XZEPR__` prefix with double underscore separators:

```rust
builder = builder.add_source(Environment::with_prefix("XZEPR").separator("__"));
```

**Correct Format:**

```bash
-e XZEPR__DATABASE__URL="postgres://xzepr:password@xzepr-postgres-1:5432/xzepr"
-e XZEPR__KAFKA__BROKERS="redpanda-0:9092"
-e XZEPR__SERVER__ENABLE_HTTPS="false"
-e XZEPR__SERVER__PORT="8443"
```

**Additional Issue:** Database password mismatch

- Tutorial used password: `xzepr`
- Actual password in `docker-compose.services.yaml`: `password`

### Step 6: Server Startup

**Final Working Command:**

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
  -e XZEPR__AUTH__JWT_SECRET="your-secret-key-change-in-production" \
  -e RUST_LOG="info,xzepr=debug" \
  xzepr:latest
```

**Result:** ✅ Pass

Server logs confirmed successful startup:

```text
INFO src/main.rs:45: Starting XZEPR Event Tracking Server
INFO src/main.rs:49: Configuration loaded successfully
INFO src/main.rs:52: Connecting to database at postgres://xzepr:password@xzepr-postgres-1:5432/xzepr
INFO src/main.rs:56: Database connection established
INFO src/main.rs:63: Database health check passed
INFO src/main.rs:89: Server listening on 0.0.0.0:8443
INFO src/main.rs:90: Health check available at: http://0.0.0.0:8443/health
INFO src/main.rs:91: API endpoints available at: http://0.0.0.0:8443/api/v1/*
INFO src/main.rs:112: TLS/HTTPS disabled - running in HTTP mode
INFO src/main.rs:113: Starting HTTP server on http://0.0.0.0:8443
```

### Step 7: Health Check Verification

**Command:**

```bash
curl http://localhost:8042/health | jq .
```

**Result:** ✅ Pass

```json
{
  "components": {
    "database": "healthy"
  },
  "service": "xzepr",
  "status": "healthy",
  "version": "0.1.0"
}
```

### Step 8: GraphQL Endpoints Testing

**Commands Tested:**

```bash
curl http://localhost:8042/graphql/health
curl http://localhost:8042/graphql/playground
```

**Result:** ❌ Fail

Both endpoints returned:

```text
HTTP/1.1 404 Not Found
```

**Root Cause Analysis:**

The project has two different router implementations:

1. **GraphQL-enabled router** in `src/api/rest/routes.rs`:
   - Includes GraphQL playground route
   - Includes GraphQL query endpoint
   - Includes GraphQL health check
   - Creates GraphQL schema with handlers

2. **Simple router** in `src/main.rs`:
   - Only includes basic health check
   - Only includes root handler and auth login
   - Does NOT include GraphQL routes
   - This is the router actually used by the server

**Code Reference:**

`src/main.rs` line 149:

```rust
Router::new()
    .route("/health", get(health_check))
    .route("/", get(root_handler))
    .route("/api/v1/status", get(api_status))
    .route("/api/v1/auth/login", post(login))
    // TODO: Add event routes
    .layer(ServiceBuilder::new().layer(trace_layer).layer(cors))
    .with_state(state)
```

The GraphQL routes were implemented in `src/api/rest/routes.rs` but the main
server binary does not use that router module.

### Step 9: Admin CLI Testing

**Command Tested:**

```bash
docker run --rm xzepr:latest /app/admin --help
```

**Result:** ✅ Pass

Available commands confirmed:

- `create-user` - Create a new local user
- `list-users` - List all users
- `add-role` - Add role to user
- `remove-role` - Remove role from user
- `generate-api-key` - Generate API key for user
- `list-api-keys` - List user's API keys
- `revoke-api-key` - Revoke API key

**Note:** Tutorial mentioned `admin db migrate` but this command does not exist.
The database schema appears to be auto-created or managed through SQLx
migrations when the server starts.

## Issues Identified

### Critical Issues

1. **GraphQL Routes Not Registered**
   - **Severity:** High
   - **Impact:** GraphQL Playground completely non-functional
   - **Location:** `src/main.rs` build_router function
   - **Fix Required:** Import and merge GraphQL routes from
     `src/api/rest/routes.rs` or refactor to use the comprehensive router

2. **Environment Variable Documentation Mismatch**
   - **Severity:** High
   - **Impact:** Server cannot connect to database with documented commands
   - **Location:** Tutorial documentation
   - **Fix Required:** Update all tutorial examples to use `XZEPR__*__*` format

### Medium Issues

3. **Database Password Inconsistency**
   - **Severity:** Medium
   - **Impact:** Connection failures if following tutorial literally
   - **Tutorial Used:** `xzepr`
   - **Actual Password:** `password`
   - **Fix Required:** Document correct password or make consistent

4. **Docker Build Warning**
   - **Severity:** Low
   - **Impact:** Security best practice violation
   - **Fix Required:** Use Docker secrets or external config for sensitive paths

5. **Missing Database Migration Command**
   - **Severity:** Low
   - **Impact:** Tutorial references non-existent `admin db migrate`
   - **Fix Required:** Remove references or implement the command

### Documentation Issues

6. **Router Architecture Confusion**
   - Two separate router implementations exist
   - Not clear which is canonical
   - `src/api/rest/routes.rs` has full GraphQL support
   - `src/main.rs` has simple router without GraphQL
   - Suggests incomplete integration work

## Recommendations

### Immediate Actions Required

1. **Integrate GraphQL Routes into Main Server**

   Option A: Use the comprehensive router from `src/api/rest/routes.rs`:

   ```rust
   // In src/main.rs
   use xzepr::api::rest::routes::build_router;
   ```

   Option B: Merge GraphQL routes into main.rs router:

   ```rust
   Router::new()
       .route("/health", get(health_check))
       .route("/graphql", post(graphql_handler))
       .route("/graphql/playground", get(graphql_playground))
       .route("/graphql/health", get(graphql_health))
       // ... rest of routes
   ```

2. **Update Tutorial Documentation**

   - Fix all environment variable examples to use `XZEPR__` prefix format
   - Update database password to `password` or make configurable
   - Remove references to `admin db migrate` command
   - Add troubleshooting section for common environment variable issues

3. **Add Integration Tests**

   - Create automated test that verifies GraphQL endpoints are accessible
   - Add to CI pipeline to prevent regression

### Medium Priority Actions

4. **Consolidate Router Implementations**

   - Choose one canonical router location
   - Remove or clearly mark the other as deprecated
   - Update imports across the codebase

5. **Improve Configuration System**

   - Support both `DATABASE_URL` and `XZEPR__DATABASE__URL` for compatibility
   - Add configuration validation on startup
   - Print effective configuration values (redacting secrets)

6. **Enhance Docker Build**

   - Address secret handling warning
   - Use multi-stage build secrets feature
   - Add healthcheck that works with both HTTP and HTTPS modes

### Low Priority Actions

7. **Admin CLI Enhancements**

   - Add `db migrate` command for explicit schema management
   - Add `db status` command to check migration state
   - Improve error messages with troubleshooting hints

8. **Documentation Structure**

   - Add "Quick Start" that tests basic functionality first
   - Separate "Development Setup" from "Production Deployment"
   - Add architecture diagram showing component relationships

## Test Conclusion

**Overall Status:** ⚠️ Partial Pass

The XZepr server Docker demo partially works. The core server, database
connectivity, and health checks function correctly. However, the primary feature
being demonstrated—the GraphQL Playground—is completely non-functional due to
missing route registration in the main server binary.

**Can Be Demonstrated:**

- Docker containerization
- Multi-service orchestration
- Database connectivity
- Basic health checks
- Admin CLI functionality

**Cannot Be Demonstrated:**

- GraphQL Playground IDE
- GraphQL query endpoint
- GraphQL mutations
- Event receiver GraphQL operations
- Any GraphQL functionality

**Effort to Fix:** Medium

The GraphQL implementation exists and appears complete in
`src/api/rest/routes.rs`. The issue is purely architectural—the wrong router is
being used. This could be fixed in 1-2 hours with proper testing.

**Blocker for Production:** Yes

The GraphQL feature is advertised in documentation and server startup logs but
does not work. This would be a critical issue for any user attempting to use the
documented GraphQL functionality.

## Next Steps

1. **Immediate:** Fix router integration to enable GraphQL endpoints
2. **Short-term:** Update all tutorial documentation with correct environment
   variables
3. **Medium-term:** Add integration tests to CI pipeline
4. **Long-term:** Architectural cleanup and consolidation

## Test Artifacts

**Server Logs:** Available in container `xzepr-server`

**Database State:** Tables created, no data

**Docker Images:**

- `xzepr:latest` - 19MB runtime image
- Base services pulled from official registries

**Network:** `xzepr_redpanda_network` (bridge network)

**Volumes:**

- `xzepr_postgres_data` - PostgreSQL data
- `xzepr_redpanda_data` - Redpanda data

## References

- Tutorial Document: `docs/tutorials/docker_demo.md`
- GraphQL Routes Implementation: `src/api/rest/routes.rs`
- Main Server Binary: `src/main.rs`
- Configuration System: `src/infrastructure/config.rs`
- Docker Compose: `docker-compose.services.yaml`
