# RBAC Completion Implementation Plan

## Overview

This plan outlines the phased approach to complete the Role-Based Access Control (RBAC) implementation in XZepr. The core RBAC system is ~80% complete with all domain logic, roles, permissions, and JWT integration fully tested. The remaining work focuses on integrating existing RBAC middleware with REST API routes, adding OIDC authentication with Keycloak, and implementing structured audit logging.

**Estimated Total Effort**: 5-7 days
**Risk Level**: Low (integration work, core logic complete)
**Dependencies**: Existing JWT middleware, RBAC domain logic, error types
**Implementation Order**: Phase 1 (REST Protection) → Phase 2 (OIDC) → Phase 3 (Hardening)

### User Decisions Incorporated

This plan reflects the following decisions:

1. **OIDC Implementation**: Will be completed in Phase 2 after REST API protection
2. **OIDC Provider**: Focus on Keycloak only; other providers can be added later
3. **Audit Logging**: Use structured logging (JSON) only, feeding into ELK stack or Datadog; no database storage

## Current State Analysis

### Existing Infrastructure

**Completed Components** (Production-Ready):

- `src/auth/rbac/roles.rs` - 4 roles (Admin, EventManager, EventViewer, User) with permission mappings, 20 tests passing
- `src/auth/rbac/permissions.rs` - 14 granular permissions across events/receivers/groups/admin, 24 tests passing
- `src/domain/entities/user.rs` - User entity with `has_role()` and `has_permission()` methods, 30+ tests passing
- `src/auth/jwt/` - Complete JWT service with role/permission claims, 40+ tests passing
- `src/api/middleware/jwt.rs` - JWT middleware with `AuthenticatedUser` extraction and role/permission guards, 15+ tests passing
- `src/api/graphql/guards.rs` - GraphQL RBAC guards fully functional, 10+ tests passing
- `src/error.rs` - `AuthError` and `AuthorizationError` types defined

**Infrastructure Ready for Use**:

- `src/api/rest/events.rs` - `AppState` with handlers
- `src/api/rest/routes.rs` - `build_protected_router()` with commented middleware
- `src/api/router.rs` - Main router with security middleware layers

### Identified Issues

**Critical Issues** (Blocking Production):

1. **RBAC Middleware Broken** (`src/auth/rbac/middleware.rs`):

   - References undefined types: `UserId`, `ApiError`, `AuthService`
   - Duplicates functionality already in `src/api/middleware/jwt.rs`
   - Not imported anywhere in module hierarchy
   - Module path comment says `src/api/middleware/rbac.rs` but lives in `src/auth/rbac/`

2. **REST Routes Unprotected** (`src/api/rest/routes.rs`):

   - All routes are public (no authentication required)
   - Middleware is commented out in `build_protected_router()`
   - No permission checks on sensitive operations

3. **OIDC Module Incomplete**:
   - `src/auth/oidc/` commented out in `src/auth/mod.rs`
   - Keycloak integration not implemented
   - Dependencies (`oauth2`, `openidconnect`) installed but unused

**Non-Critical Issues**:

- No integration tests for protected REST endpoints
- No audit logging for permission checks
- API key authentication exists but RBAC integration unclear

## Implementation Phases

### Phase 1: Fix RBAC Middleware and Enable REST Protection

**Goal**: Secure REST API endpoints with existing JWT middleware
**Duration**: 1-2 days
**Priority**: CRITICAL (blocks production deployment)

#### Task 1.1: Remove Broken RBAC Middleware Module

**Rationale**: The `src/auth/rbac/middleware.rs` file duplicates functionality already present in `src/api/middleware/jwt.rs` and has compilation errors. The JWT middleware already provides `AuthenticatedUser` extraction and role/permission checking.

**Actions**:

1. Delete `src/auth/rbac/middleware.rs` (broken, unused file)
2. Update `src/auth/rbac/mod.rs` to remove middleware export
3. Verify JWT middleware in `src/api/middleware/jwt.rs` has all needed functions:
   - `AuthenticatedUser` extractor (exists)
   - `require_roles()` middleware factory (exists)
   - `require_permissions()` middleware factory (exists)
4. Run `cargo check` to verify no references remain

**Files Modified**:

- `src/auth/rbac/mod.rs` - Remove `pub mod middleware;`
- Delete `src/auth/rbac/middleware.rs`

#### Task 1.2: Create Permission Mapping Helper

**Rationale**: REST handlers need to map HTTP routes to RBAC permissions. Create helper to convert route patterns to required permissions.

**Actions**:

1. Create `src/api/middleware/rbac_helpers.rs`
2. Implement `PermissionMapper` struct with methods:
   - `permission_for_route(method: &str, path: &str) -> Option<Permission>`
   - Map POST /api/v1/events to `Permission::EventCreate`
   - Map GET /api/v1/events/:id to `Permission::EventRead`
   - Map PUT /api/v1/receivers/:id to `Permission::ReceiverUpdate`
   - Map DELETE /api/v1/receivers/:id to `Permission::ReceiverDelete`
   - Similar mappings for all routes
3. Add unit tests for all route-to-permission mappings
4. Export in `src/api/middleware/mod.rs`

**Files Created**:

- `src/api/middleware/rbac_helpers.rs` (~150 lines with tests)

**Files Modified**:

- `src/api/middleware/mod.rs` - Add `pub mod rbac_helpers;`

#### Task 1.3: Wire Up JWT Middleware to REST Routes

**Rationale**: Apply existing JWT authentication middleware to REST API routes using the already-tested `jwt_auth_middleware` function.

**Actions**:

1. In `src/api/rest/routes.rs`:
   - Import `JwtMiddlewareState` and `jwt_auth_middleware` from `src/api/middleware/jwt.rs`
   - Uncomment middleware in `build_protected_router()`
   - Add JWT state to function parameters
   - Apply `jwt_auth_middleware` layer to protected routes
2. Update `build_protected_router()` signature to accept `JwtMiddlewareState`
3. Keep health check and GraphQL playground routes public
4. Apply authentication to all `/api/v1/` routes

**Files Modified**:

- `src/api/rest/routes.rs` - Uncomment and configure middleware (~20 line changes)

**Example Change**:

```rust
// Before
pub fn build_protected_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/events", post(create_event))
        .with_state(state)
        // .route_layer(middleware::from_fn_with_state(...))
}

// After
pub fn build_protected_router(
    state: AppState,
    jwt_state: JwtMiddlewareState,
) -> Router {
    Router::new()
        .route("/api/v1/events", post(create_event))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            jwt_state,
            jwt_auth_middleware,
        ))
}
```

#### Task 1.4: Add Permission Guards to Specific Routes

**Rationale**: Some routes require specific permissions beyond authentication. Use `require_permissions()` middleware for fine-grained control.

**Actions**:

1. Group routes by permission requirements:
   - Read-only routes: Apply `Permission::EventRead`, etc.
   - Create routes: Apply `Permission::EventCreate`, etc.
   - Delete routes: Require Admin role or specific delete permissions
2. Use `route_layer()` to apply permission guards to route groups
3. Structure router with nested groups for different permission levels

**Files Modified**:

- `src/api/rest/routes.rs` - Add permission guards to route groups (~40 line changes)

**Example Structure**:

```rust
Router::new()
    // Public routes
    .route("/health", get(health_check))
    // Authenticated routes with read permission
    .route("/api/v1/events/:id", get(get_event))
    .route_layer(middleware::from_fn(
        require_permissions(vec!["event:read".to_string()])
    ))
    // Authenticated routes with create permission
    .route("/api/v1/events", post(create_event))
    .route_layer(middleware::from_fn(
        require_permissions(vec!["event:create".to_string()])
    ))
    // Admin-only routes
    .route("/api/v1/receivers/:id", delete(delete_event_receiver))
    .route_layer(middleware::from_fn(
        require_roles(vec!["admin".to_string()])
    ))
    // Base authentication for all /api/v1/* routes
    .layer(middleware::from_fn_with_state(jwt_state, jwt_auth_middleware))
```

#### Task 1.5: Update Main Router to Use Protected Routes

**Rationale**: Integrate protected router into main application initialization.

**Actions**:

1. In `src/api/router.rs`:
   - Import `JwtService` and `JwtMiddlewareState`
   - Create JWT middleware state from configuration
   - Pass JWT state to `build_protected_router()`
2. In `src/bin/server.rs`:
   - Load JWT configuration from environment
   - Initialize `JwtService`
   - Pass to router builder
3. Update `RouterConfig` if needed to include JWT configuration

**Files Modified**:

- `src/api/router.rs` - Add JWT initialization (~30 lines)
- `src/bin/server.rs` - Wire up JWT service (~20 lines)

#### Task 1.6: Testing Requirements

**Test Coverage Goal**: >80% for new code

**Unit Tests**:

1. `src/api/middleware/rbac_helpers.rs`:
   - Test each route-to-permission mapping
   - Test unknown routes return None
   - Test method sensitivity (GET vs POST)
2. `src/api/rest/routes.rs`:
   - Mock tests for middleware application
   - Verify public routes remain public
   - Verify protected routes require auth

**Integration Tests** (new file: `tests/rbac_rest_integration.rs`):

1. Test authenticated request succeeds
2. Test unauthenticated request returns 401
3. Test insufficient permissions returns 403
4. Test admin can access all routes
5. Test EventViewer cannot create events
6. Test role-based access control for each role
7. Test token expiration handling

**Files Created**:

- `tests/rbac_rest_integration.rs` (~200-300 lines)

#### Task 1.7: Deliverables

**Code**:

- ✅ JWT middleware wired to REST routes
- ✅ Permission guards applied to sensitive operations
- ✅ RBAC helper utilities for route mapping
- ✅ Integration tests for protected endpoints

**Documentation**:

- Updated `docs/how_to/use_rbac.md` with REST API examples
- Updated `docs/explanation/rbac_status_summary.md` to reflect completion
- API endpoint documentation showing required permissions

**Validation**:

- All existing tests still pass (433+ tests)
- New integration tests pass (8+ new tests)
- `cargo clippy` shows zero warnings
- `cargo fmt --check` passes

#### Task 1.8: Success Criteria

**Functional**:

- [ ] Unauthenticated requests to `/api/v1/*` return 401 Unauthorized
- [ ] Valid JWT token allows authenticated access
- [ ] Users with insufficient permissions receive 403 Forbidden
- [ ] Admin role can access all endpoints
- [ ] EventViewer role can only read, not create/update/delete
- [ ] Health check remains public (no auth required)
- [ ] GraphQL continues to work with existing guards

**Quality**:

- [ ] Test coverage >80% on new code
- [ ] Zero clippy warnings
- [ ] All documentation updated
- [ ] No breaking changes to existing GraphQL API

### Phase 2: OIDC Integration with Keycloak

**Goal**: Enable OpenID Connect authentication with Keycloak
**Duration**: 2-3 days
**Priority**: HIGH (implements after Phase 1 per user requirements)
**Note**: Focus on Keycloak support only; other OIDC providers can be added later

#### Task 2.1: Implement OIDC Client

**Rationale**: Enable federated authentication with external identity providers like Keycloak.

**Actions**:

1. Uncomment OIDC module in `src/auth/mod.rs`
2. Create OIDC module structure:
   - `src/auth/oidc/mod.rs` - Public exports
   - `src/auth/oidc/client.rs` - OIDC client implementation
   - `src/auth/oidc/config.rs` - Configuration structures
   - `src/auth/oidc/callback.rs` - Callback handler
3. Implement `OidcClient` struct:
   - `new(issuer_url, client_id, client_secret, redirect_url)`
   - `authorization_url()` - Generate auth URL with state
   - `exchange_code(code)` - Exchange authorization code for tokens
   - `verify_id_token(token)` - Validate ID token
   - `get_user_info(access_token)` - Fetch user details
4. Use `openidconnect` crate for standard OIDC flows
5. Focus implementation on Keycloak-specific features and requirements
6. Add comprehensive unit tests with mock Keycloak responses

**Files Created**:

- `src/auth/oidc/mod.rs` (~50 lines)
- `src/auth/oidc/client.rs` (~250 lines)
- `src/auth/oidc/config.rs` (~100 lines)
- `src/auth/oidc/callback.rs` (~150 lines)

**Files Modified**:

- `src/auth/mod.rs` - Uncomment `pub mod oidc;`

#### Task 2.2: Add OIDC Routes

**Rationale**: Provide HTTP endpoints for OIDC authentication flow.

**Actions**:

1. Create `src/api/rest/auth.rs` for authentication endpoints
2. Implement handlers:
   - `POST /api/v1/auth/login` - Local authentication
   - `GET /api/v1/auth/oidc/login` - Initiate OIDC flow
   - `GET /api/v1/auth/oidc/callback` - Handle OIDC callback
   - `POST /api/v1/auth/refresh` - Refresh token
   - `POST /api/v1/auth/logout` - Logout (blacklist token)
3. Add routes to `build_router()` (public, no auth required)
4. Handle state parameter validation for CSRF protection

**Files Created**:

- `src/api/rest/auth.rs` (~300 lines)

**Files Modified**:

- `src/api/rest/mod.rs` - Add `pub mod auth;`
- `src/api/rest/routes.rs` - Add auth routes

#### Task 2.3: User Provisioning from OIDC Claims

**Rationale**: Automatically create/update users from OIDC authentication.

**Actions**:

1. Extend `User::new_oidc()` constructor if needed
2. Create user repository method `find_or_create_oidc_user()`
3. Map OIDC claims to user attributes:
   - `sub` claim to user ID
   - `email` claim to user email
   - `preferred_username` to username
   - Keycloak-specific claims to roles
4. Handle role mapping from Keycloak:
   - Read roles from `realm_access.roles` claim (Keycloak standard)
   - Map Keycloak role names to internal `Role` enum
   - Support custom role claim path via configuration
   - Default to `Role::User` if no roles present
5. Store OIDC subject ID for future authentication

**Files Modified**:

- `src/domain/entities/user.rs` - Enhance OIDC support if needed (~50 lines)
- `src/domain/repositories/user_repository.rs` - Add OIDC methods

#### Task 2.4: Configuration and Environment Variables

**Rationale**: Make OIDC configuration externalized and environment-specific.

**Actions**:

1. Add OIDC configuration to environment variables:
   - `OIDC_ENABLED=true/false`
   - `OIDC_ISSUER_URL=https://keycloak.example.com/realms/xzepr`
   - `OIDC_CLIENT_ID=xzepr-client`
   - `OIDC_CLIENT_SECRET=secret`
   - `OIDC_REDIRECT_URL=https://app.example.com/auth/oidc/callback`
   - `OIDC_ROLE_CLAIM=roles` (claim name containing roles)
2. Update configuration loading in `src/lib.rs` or config module
3. Add validation for OIDC configuration
4. Create example `.env` file with OIDC settings

**Files Modified**:

- `.env.example` - Add OIDC variables
- Configuration loading code - Parse OIDC config

#### Task 2.5: Testing Requirements

**Unit Tests**:

1. OIDC client methods with mock Keycloak responses
2. Token validation with mock Keycloak JWKs
3. User provisioning logic
4. Role mapping from Keycloak realm_access.roles claim
5. Keycloak-specific claim handling

**Integration Tests** (new file: `tests/oidc_keycloak_integration.rs`):

1. Mock Keycloak provider (using testcontainers-keycloak if available)
2. Test full Keycloak authentication flow
3. Test user creation from Keycloak claims
4. Test role assignment from Keycloak realm_access
5. Test token refresh with Keycloak
6. Test error cases (invalid code, expired token, invalid realm)

**Manual Testing Checklist**:

1. Set up Keycloak instance (Docker or local)
2. Create XZepr realm with client configuration
3. Configure realm roles matching internal Role enum
4. Test login flow end-to-end with Keycloak
5. Verify user appears in system with correct attributes
6. Verify Keycloak roles map to internal roles correctly
7. Test with multiple users and role combinations

**Files Created**:

- `tests/oidc_keycloak_integration.rs` (~250 lines)
- `docs/how_to/setup_keycloak.md` - Keycloak setup and testing guide
- `docker/keycloak/realm-export.json` - Sample realm configuration

#### Task 2.6: Deliverables

**Code**:

- ✅ OIDC client implementation
- ✅ Authentication routes (login, callback, refresh, logout)
- ✅ User provisioning from OIDC claims
- ✅ Role mapping configuration

**Documentation**:

- `docs/how_to/setup_keycloak.md` - Keycloak configuration guide
- `docs/explanation/oidc_implementation.md` - Technical details
- Updated `docs/explanation/architecture.md` - OIDC architecture section
- API documentation for auth endpoints

**Configuration**:

- `.env.example` updated with Keycloak OIDC variables
- Sample Keycloak realm export in `docker/keycloak/realm-export.json`
- Docker Compose configuration for local Keycloak testing

#### Task 2.7: Success Criteria

**Functional**:

- [ ] User can initiate OIDC login
- [ ] User is redirected to Keycloak (or other provider)
- [ ] Successful authentication creates/updates user
- [ ] OIDC roles map to internal roles correctly
- [ ] User receives valid JWT after OIDC authentication
- [ ] Subsequent requests use JWT (not OIDC every time)
- [ ] Refresh token flow works

**Quality**:

- [ ] Unit tests cover OIDC client (>80% coverage)
- [ ] Integration tests pass with mock provider
- [ ] Manual testing with real Keycloak succeeds
- [ ] Error handling for all OIDC failure cases
- [ ] Security: State parameter prevents CSRF
- [ ] Security: Token validation includes signature check

### Phase 3: Production Hardening

**Goal**: Add audit logging, monitoring, and production-ready features
**Duration**: 1-2 days
**Priority**: MEDIUM (improves observability and security)

#### Task 3.1: Audit Logging for Authentication Events

**Rationale**: Track security-relevant events for compliance and forensics using structured logging for ingestion into ELK stack or Datadog.

**Actions**:

1. Create `src/infrastructure/audit/mod.rs` module
2. Define `AuditEvent` struct with fields:
   - `timestamp`, `user_id`, `action`, `resource`, `outcome`, `metadata`
   - `ip_address`, `user_agent`, `session_id`
   - All fields structured for JSON serialization
3. Implement `AuditLogger` struct:
   - `log_event(event: AuditEvent)` - Emit structured log at INFO level
   - Use `tracing` crate with JSON formatting
   - Include correlation IDs for request tracing
   - No database storage - logs go to stdout/stderr for collection
4. Add audit logging to:
   - JWT middleware (successful/failed auth)
   - Permission checks (granted/denied)
   - OIDC callback (new user, existing user)
   - User management operations
5. Configure JSON log format in tracing subscriber for ELK/Datadog ingestion

**Files Created**:

- `src/infrastructure/audit/mod.rs` (~150 lines, structured logging only)
- Example log parsing configurations for ELK/Datadog (optional)

#### Task 3.2: Metrics for RBAC Operations

**Rationale**: Monitor authentication/authorization performance and patterns.

**Actions**:

1. Add Prometheus metrics (using existing `PrometheusMetrics`):
   - Counter: `auth_attempts_total{result="success|failure"}`
   - Counter: `permission_checks_total{result="granted|denied", permission}`
   - Histogram: `auth_duration_seconds`
   - Gauge: `active_sessions_total`
2. Instrument JWT middleware with metrics
3. Instrument permission check middleware
4. Add metrics endpoint documentation

**Files Modified**:

- `src/infrastructure/metrics.rs` - Add auth metrics
- `src/api/middleware/jwt.rs` - Record auth metrics
- Permission check code - Record permission metrics

#### Task 3.3: Rate Limiting for Authentication Endpoints

**Rationale**: Prevent brute force attacks on login endpoints.

**Actions**:

1. Apply stricter rate limits to authentication routes:
   - `/api/v1/auth/login` - 5 requests per minute per IP
   - `/api/v1/auth/oidc/callback` - 10 requests per minute per IP
2. Use existing rate limiting middleware from `src/api/middleware/rate_limit.rs`
3. Return 429 Too Many Requests with retry-after header
4. Log rate limit violations for security monitoring

**Files Modified**:

- `src/api/rest/routes.rs` - Apply rate limits to auth routes
- `src/api/middleware/rate_limit.rs` - Add auth-specific limits if needed

#### Task 3.4: Security Headers and CORS Configuration

**Rationale**: Ensure secure browser communication and proper CORS setup.

**Actions**:

1. Verify existing security headers middleware is applied
2. Configure CORS for authentication endpoints:
   - Allow credentials for OIDC callback
   - Restrict allowed origins in production
   - Set proper preflight caching
3. Add Content-Security-Policy for auth pages if frontend exists
4. Ensure SameSite cookie attributes for session cookies

**Files Modified**:

- `src/api/router.rs` - Verify security middleware configuration
- `src/api/middleware/cors.rs` - Add auth-specific CORS rules if needed
- `src/infrastructure/tracing.rs` - Configure JSON output for audit logs

#### Task 3.5: Testing Requirements

**Security Tests**:

1. Test brute force protection on login endpoint
2. Test CSRF protection on OIDC callback
3. Test token tampering detection
4. Test expired token rejection
5. Test blacklisted token rejection
6. Test permission escalation attempts

**Performance Tests** (optional):

1. Benchmark JWT validation overhead
2. Benchmark permission check performance
3. Load test authentication endpoints

**Files Created**:

- `tests/security_tests.rs` (~200 lines)

#### Task 3.6: Deliverables

**Code**:

- ✅ Audit logging for security events
- ✅ Prometheus metrics for auth/authz
- ✅ Rate limiting on auth endpoints
- ✅ Security headers configuration

**Documentation**:

- `docs/explanation/security_architecture.md` - Security features
- `docs/reference/audit_logs.md` - Audit log format and queries
- `docs/reference/metrics.md` - Authentication metrics

**Monitoring**:

- Grafana dashboard JSON for auth metrics
- Alert rules for suspicious patterns

#### Task 3.7: Success Criteria

**Functional**:

- [ ] All authentication events logged to audit table
- [ ] Permission denials logged with context
- [ ] Metrics exported via `/metrics` endpoint
- [ ] Rate limiting prevents brute force attacks
- [ ] Security headers present in all responses

**Observability**:

- [ ] Audit events emitted as structured JSON logs
- [ ] Logs include all required fields for ELK/Datadog parsing
- [ ] Can graph authentication success/failure rates via metrics
- [ ] Can monitor active sessions count via metrics
- [ ] Can query logs in external system (ELK/Datadog) for user activity
- [ ] Can alert on suspicious patterns via external monitoring

**Security**:

- [ ] CSRF protection on all state-changing operations
- [ ] XSS protection via CSP headers
- [ ] Clickjacking protection via X-Frame-Options
- [ ] Rate limiting prevents abuse

## Validation Checklist

### Phase 1 Validation (REST API Protection)

**Before Starting**:

- [ ] Review current JWT middleware implementation
- [ ] Understand existing error types
- [ ] Read REST route structure

**After Completion**:

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes with >433 tests
- [ ] New integration tests pass
- [ ] Manual testing: unauthenticated request returns 401
- [ ] Manual testing: authenticated request succeeds
- [ ] Manual testing: insufficient permissions returns 403
- [ ] Documentation updated in `docs/explanation/`

### Phase 2 Validation (OIDC)

**Before Starting**:

- [ ] Set up Keycloak test instance
- [ ] Configure test realm and client
- [ ] Review `oauth2` and `openidconnect` crate docs

**After Completion**:

- [ ] All Phase 1 checks still pass
- [ ] OIDC unit tests pass
- [ ] OIDC integration tests pass (with mock)
- [ ] Manual Keycloak test succeeds
- [ ] User created from OIDC claims
- [ ] Roles mapped correctly
- [ ] Documentation includes Keycloak setup guide

### Phase 3 Validation (Hardening)

**After Completion**:

- [ ] All previous phase checks still pass
- [ ] Audit logs appear in database/logs
- [ ] Metrics visible on `/metrics` endpoint
- [ ] Rate limiting blocks excessive requests
- [ ] Security headers present in responses
- [ ] Load testing shows acceptable performance
- [ ] Security documentation complete

## Risk Assessment and Mitigation

### Technical Risks

**Risk**: Breaking existing GraphQL API during REST integration
**Likelihood**: Low
**Impact**: High
**Mitigation**:

- Keep GraphQL and REST middleware separate
- Run full test suite after each change
- Test GraphQL endpoints manually after integration

**Risk**: JWT middleware performance overhead
**Likelihood**: Medium
**Impact**: Low
**Mitigation**:

- Benchmark token validation
- Cache validated tokens if needed
- Use short token expiration (15 minutes)

**Risk**: OIDC provider compatibility issues
**Likelihood**: Medium
**Impact**: Medium
**Mitigation**:

- Use standard `openidconnect` crate
- Test with multiple providers (Keycloak, Auth0)
- Implement comprehensive error handling

### Operational Risks

**Risk**: Production deployment without testing
**Likelihood**: Low (if plan followed)
**Impact**: Critical
**Mitigation**:

- Deploy to staging environment first
- Run smoke tests on staging
- Have rollback plan ready

**Risk**: Configuration errors in production
**Likelihood**: Medium
**Impact**: High
**Mitigation**:

- Validate all configuration at startup
- Fail fast with clear error messages
- Document all environment variables

## Timeline and Dependencies

### Phase 1: REST API Protection

**Days 1-2**:

- Task 1.1: Remove broken middleware (2 hours)
- Task 1.2: Create RBAC helpers (4 hours)
- Task 1.3: Wire up JWT middleware (3 hours)
- Task 1.4: Add permission guards (4 hours)
- Task 1.5: Update main router (2 hours)
- Task 1.6-1.8: Testing and validation (6 hours)

**Dependencies**: None (self-contained)

### Phase 2: OIDC Integration

**Days 3-5**:

- Task 2.1: Implement OIDC client (8 hours)
- Task 2.2: Add OIDC routes (4 hours)
- Task 2.3: User provisioning (4 hours)
- Task 2.4: Configuration (2 hours)
- Task 2.5-2.7: Testing and validation (8 hours)

**Dependencies**: Phase 1 complete (JWT infrastructure must work)

### Phase 3: Production Hardening

**Days 6-7**:

- Task 3.1: Audit logging (6 hours)
- Task 3.2: Metrics (3 hours)
- Task 3.3: Rate limiting (2 hours)
- Task 3.4: Security headers (2 hours)
- Task 3.5-3.7: Testing and validation (4 hours)

**Dependencies**: Phases 1 and 2 complete

**Total Timeline**: 5-7 working days

## Success Metrics

### Functional Completeness

- [ ] REST API requires authentication for all `/api/v1/*` routes
- [ ] Permission-based authorization enforced
- [ ] OIDC authentication working with Keycloak
- [ ] All 4 roles (Admin, EventManager, EventViewer, User) function correctly
- [ ] GraphQL API continues to work unchanged

### Quality Metrics

- [ ] Test coverage >80% on all new code
- [ ] Zero clippy warnings
- [ ] Zero compilation errors
- [ ] All documentation complete and accurate
- [ ] Code review passed (if applicable)

### Security Metrics

- [ ] Penetration testing passed (if applicable)
- [ ] No authentication bypass possible
- [ ] No permission escalation possible
- [ ] Rate limiting prevents brute force
- [ ] Audit logging captures all security events

### Performance Metrics

- [ ] JWT validation <5ms p99
- [ ] Authentication endpoint <100ms p99
- [ ] No memory leaks under load
- [ ] Graceful degradation under rate limiting

## Post-Implementation Tasks

### Documentation

1. Update `docs/explanation/architecture.md` with complete RBAC architecture
2. Update `docs/explanation/implementation.md` with production-ready status
3. Update `docs/how_to/use_rbac.md` with REST API examples
4. Create `docs/tutorials/authentication_setup.md` for new users
5. Update `README.md` with authentication requirements

### Deployment

1. Update deployment scripts with new environment variables
2. Create database migrations for audit logs
3. Update monitoring dashboards
4. Configure alerting rules
5. Document rollback procedure

### Communication

1. Notify team of API changes (authentication now required)
2. Update API documentation/OpenAPI spec
3. Provide migration guide for existing clients
4. Announce completion in team channels

## Rollback Plan

If issues arise during or after implementation:

**Phase 1 Rollback**:

1. Comment out middleware in `build_protected_router()`
2. Revert to public REST endpoints
3. GraphQL remains protected
4. Investigate issues, fix, redeploy

**Phase 2 Rollback** (OIDC):

1. Disable OIDC via `OIDC_ENABLED=false` environment variable
2. Local authentication continues to work
3. JWT authentication continues to work
4. Existing OIDC users can still authenticate with JWT tokens
5. No user data loss (OIDC users remain in database)

**Phase 3 Rollback** (Hardening):

1. Adjust log level to reduce audit log verbosity if needed
2. Reduce rate limits if too restrictive
3. Disable specific metrics if causing overhead
4. Core auth functionality unaffected by any changes

## References

- `docs/explanation/rbac_status_summary.md` - Current RBAC status
- `docs/explanation/architecture.md` - System architecture
- `docs/how_to/use_rbac.md` - RBAC usage guide
- `AGENTS.md` - Development guidelines
- `PLAN.md` - Planning template

## Appendix: File Changes Summary

### Files to Create

**Phase 1**:

- `src/api/middleware/rbac_helpers.rs` (~150 lines)
- `tests/rbac_rest_integration.rs` (~250 lines)

**Phase 2** (Keycloak OIDC):

- `src/auth/oidc/mod.rs` (~50 lines)
- `src/auth/oidc/client.rs` (~250 lines, Keycloak-focused)
- `src/auth/oidc/config.rs` (~100 lines)
- `src/auth/oidc/callback.rs` (~150 lines)
- `src/api/rest/auth.rs` (~300 lines)
- `tests/oidc_keycloak_integration.rs` (~250 lines)
- `docs/how_to/setup_keycloak.md` - Keycloak configuration guide
- `docs/explanation/oidc_keycloak_implementation.md` - Technical details
- `docker/keycloak/realm-export.json` - Sample Keycloak realm
- `docker/keycloak/docker-compose.yaml` - Local Keycloak setup

**Phase 3** (Hardening with Structured Logging):

- `src/infrastructure/audit/mod.rs` (~150 lines, structured logging)
- `tests/security_tests.rs` (~200 lines)
- `docs/explanation/security_architecture.md`
- `docs/reference/structured_audit_logs.md` - Log format and fields
- `docs/reference/metrics.md` - Authentication metrics
- `config/logstash/` (optional) - Example ELK parsing rules
- `config/datadog/` (optional) - Example Datadog log parsing

**Total New Files**: 18 files, ~2,100 lines of code

### Files to Modify

**Phase 1**:

- `src/auth/rbac/mod.rs` (remove middleware export)
- `src/api/middleware/mod.rs` (add rbac_helpers export)
- `src/api/rest/routes.rs` (wire up middleware, ~60 line changes)
- `src/api/router.rs` (JWT initialization, ~30 lines)
- `src/bin/server.rs` (wire up JWT service, ~20 lines)
- `docs/how_to/use_rbac.md` (add REST examples)
- `docs/explanation/rbac_status_summary.md` (update status)

**Phase 2** (Keycloak OIDC):

- `src/auth/mod.rs` (uncomment OIDC module)
- `src/api/rest/mod.rs` (add auth module)
- `src/domain/entities/user.rs` (enhance Keycloak OIDC support, ~50 lines)
- `.env.example` (add Keycloak OIDC variables)
- `docs/explanation/architecture.md` (add Keycloak OIDC section)
- `docker-compose.yaml` (add Keycloak service for local development)

**Phase 3** (Hardening):

- `src/infrastructure/metrics.rs` (add auth metrics)
- `src/infrastructure/tracing.rs` (configure JSON formatting for audit logs)
- `src/api/middleware/jwt.rs` (add metrics recording and audit logging)
- `src/api/rest/routes.rs` (add rate limits to auth routes)
- `src/api/router.rs` (verify security middleware)
- Logging configuration for ELK/Datadog integration

**Total Modified Files**: 16 files, ~350 line changes

### Files to Delete

- `src/auth/rbac/middleware.rs` (broken, unused)

---

**Plan Version**: 1.0
**Last Updated**: 2025-01-14
**Status**: Ready for Implementation
**Estimated Completion**: 5-7 working days
