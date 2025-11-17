# OPA RBAC Expansion Implementation Plan

## Overview

This plan outlines the phased implementation to expand the current RBAC system with Open Policy Agent (OPA) integration, adding fine-grained resource ownership and group-based access control. The implementation will enable resource owners to control who can access their resources while maintaining the existing role-based permissions system.

## Current State Analysis

### Existing Infrastructure

#### Authentication and Authorization

- **JWT Authentication**: Implemented in `src/api/middleware/jwt.rs` with token validation and user claims extraction
- **RBAC Foundation**: Basic roles and permissions defined in `src/auth/rbac/`
  - `roles.rs`: Four roles (Admin, EventManager, EventViewer, User)
  - `permissions.rs`: CRUD permissions for events, receivers, and groups
- **GraphQL Guards**: Authorization checks in `src/api/graphql/guards.rs`
- **REST Middleware**: JWT middleware with role and permission checks

#### Domain Entities

- **EventReceiver**: Domain entity in `src/domain/entities/event_receiver.rs`
  - No owner tracking
  - No group membership tracking
- **EventReceiverGroup**: Collection of event receivers in `src/domain/entities/event_receiver_group.rs`
  - Contains list of receiver IDs
  - No owner or membership tracking
- **Event**: Event entity with receiver_id reference
  - No owner tracking
- **User**: User entity with roles and permissions in `src/domain/entities/user.rs`
  - UserId, username, email, roles
  - No resource ownership tracking

### Identified Issues

1. **No Resource Ownership**: Entities lack owner_id field to track creators
2. **No Group-Based Access**: No mechanism to grant group members access to resources
3. **No Fine-Grained Authorization**: Current RBAC is role-based only, no resource-level checks
4. **No Policy Engine**: Authorization logic embedded in application code
5. **No Audit Trail**: No tracking of authorization decisions
6. **Database Schema Gap**: No ownership or membership tables
7. **No OPA Integration**: No policy evaluation infrastructure

## Implementation Phases

### Phase 1: Domain Model Extension and Database Schema

#### Task 1.1: Extend Domain Entities with Ownership

**Objective**: Add owner tracking to domain entities

**Changes Required**:

- Update `src/domain/entities/event_receiver.rs`

  - Add `owner_id: UserId` field to `EventReceiver` struct
  - Add `owner_id` parameter to `new()` method
  - Update `EventReceiverData` struct
  - Add `owner_id()` getter method
  - Update validation and tests

- Update `src/domain/entities/event_receiver_group.rs`

  - Add `owner_id: UserId` field to `EventReceiverGroup` struct
  - Add `members: Vec<UserId>` field for group membership
  - Add methods: `add_member()`, `remove_member()`, `is_member()`
  - Update `EventReceiverGroupData` struct
  - Update tests for ownership and membership

- Update `src/domain/entities/event.rs`
  - Add `owner_id: UserId` field to `Event` struct
  - Update `CreateEventParams` to include `owner_id`
  - Add `owner_id()` getter method

**Files to Create**:

- None (modifications only)

**Files to Modify**:

- `src/domain/entities/event_receiver.rs`
- `src/domain/entities/event_receiver_group.rs`
- `src/domain/entities/event.rs`

#### Task 1.2: Create Database Migration for Ownership

**Objective**: Add ownership and membership tables to database schema

**Changes Required**:

Create migration file `migrations/YYYYMMDD_add_ownership_and_membership.sql`:

```sql
-- Add owner_id to event_receivers
ALTER TABLE event_receivers ADD COLUMN owner_id TEXT NOT NULL DEFAULT 'system';

-- Add owner_id to event_receiver_groups
ALTER TABLE event_receiver_groups ADD COLUMN owner_id TEXT NOT NULL DEFAULT 'system';

-- Add owner_id to events
ALTER TABLE events ADD COLUMN owner_id TEXT NOT NULL DEFAULT 'system';

-- Create event_receiver_group_members table
CREATE TABLE event_receiver_group_members (
    group_id TEXT NOT NULL REFERENCES event_receiver_groups(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    added_by TEXT NOT NULL REFERENCES users(id),
    PRIMARY KEY (group_id, user_id)
);

-- Create indexes for performance
CREATE INDEX idx_event_receivers_owner ON event_receivers(owner_id);
CREATE INDEX idx_event_receiver_groups_owner ON event_receiver_groups(owner_id);
CREATE INDEX idx_events_owner ON events(owner_id);
CREATE INDEX idx_group_members_user ON event_receiver_group_members(user_id);
CREATE INDEX idx_group_members_group ON event_receiver_group_members(group_id);
```

**Files to Create**:

- `migrations/YYYYMMDD_add_ownership_and_membership.sql`

#### Task 1.3: Update Repository Interfaces

**Objective**: Add ownership queries to repository traits

**Changes Required**:

- Update `src/domain/repositories/event_receiver_repo.rs`

  - Add `find_by_owner(owner_id: UserId)` method
  - Add `is_owner(receiver_id: EventReceiverId, user_id: UserId)` method

- Update `src/domain/repositories/event_receiver_group_repo.rs`

  - Add `find_by_owner(owner_id: UserId)` method
  - Add `is_owner(group_id: EventReceiverGroupId, user_id: UserId)` method
  - Add `is_member(group_id: EventReceiverGroupId, user_id: UserId)` method
  - Add `add_member(group_id, user_id, added_by)` method
  - Add `remove_member(group_id, user_id)` method
  - Add `get_members(group_id)` method

- Update `src/domain/repositories/event_repo.rs`
  - Add `find_by_owner(owner_id: UserId)` method
  - Add `is_owner(event_id: EventId, user_id: UserId)` method

**Files to Modify**:

- `src/domain/repositories/event_receiver_repo.rs`
- `src/domain/repositories/event_receiver_group_repo.rs`
- `src/domain/repositories/event_repo.rs`

#### Task 1.4: Implement PostgreSQL Repository Updates

**Objective**: Implement ownership queries in PostgreSQL repositories

**Changes Required**:

Update `src/infrastructure/persistence/postgres/event_receiver_repo.rs`:

- Implement new ownership query methods
- Update `save()` to include owner_id
- Update SQL queries to include owner_id column
- Add version column for cache invalidation tracking
- Emit cache invalidation events on updates

- Update `src/infrastructure/persistence/postgres/event_receiver_group_repo.rs`

  - Implement ownership and membership methods
  - Add queries for group_members table
  - Update save/update operations

- Update `src/infrastructure/persistence/postgres/event_repo.rs`
  - Implement ownership queries
  - Update save operations

Create cache invalidation hook in repositories:

```rust
impl PostgresEventReceiverRepository {
    async fn save_with_cache_invalidation(
        &self,
        receiver: &EventReceiver,
    ) -> Result<()> {
        // Save to database (increments version)
        self.save(receiver).await?;

        // Invalidate cache for this resource
        if let Some(cache) = &self.cache {
            cache.invalidate_resource(
                "event_receiver",
                &receiver.id().to_string()
            ).await;
        }

        Ok(())
    }
}
```

**Files to Modify**:

- `src/infrastructure/persistence/postgres/event_receiver_repo.rs`
- `src/infrastructure/persistence/postgres/event_receiver_group_repo.rs`
- `src/infrastructure/persistence/postgres/event_repo.rs`

#### Task 1.5: Testing Requirements

- Unit tests for entity ownership validation
- Unit tests for group membership operations
- Repository tests for ownership queries
- Integration tests for database operations
- Migration rollback tests

#### Task 1.6: Deliverables

- Domain entities with owner_id fields
- Database migration with ownership tables
- Repository interfaces with ownership methods
- PostgreSQL implementation with ownership queries
- Comprehensive test coverage greater than 80 percent

#### Task 1.7: Success Criteria

- All entities track ownership
- Group membership can be managed
- Repositories support ownership queries
- Database migration applies cleanly
- All tests pass with cargo test
- Zero clippy warnings

### Phase 2: OPA Infrastructure Setup

#### Task 2.1: Add OPA Dependencies

**Objective**: Add OPA client library to project

**Changes Required**:

Update `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
```

**Files to Modify**:

- `Cargo.toml`

#### Task 2.2: Create OPA Client Module

**Objective**: Implement OPA HTTP client for policy evaluation

**Changes Required**:

Create `src/auth/opa/mod.rs`:

```rust
pub mod client;
pub mod policy;
pub mod types;

pub use client::OpaClient;
pub use policy::{AuthorizationRequest, AuthorizationResponse};
pub use types::{OpaConfig, OpaError};
```

Create `src/auth/opa/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct OpaConfig {
    pub enabled: bool,
    pub url: String,
    pub timeout_seconds: u64,
    pub policy_path: String,
    pub bundle_url: Option<String>,
    pub cache_ttl_seconds: i64,
}

#[derive(Error, Debug)]
pub enum OpaError {
    #[error("OPA request failed: {0}")]
    RequestFailed(String),

    #[error("OPA response invalid: {0}")]
    InvalidResponse(String),

    #[error("Policy evaluation error: {0}")]
    EvaluationError(String),

    #[error("Connection timeout")]
    Timeout,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpaRequest {
    pub input: OpaInput,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpaInput {
    pub user: UserContext,
    pub action: String,
    pub resource: ResourceContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpaResponse {
    pub result: bool,
}
```

Create `src/auth/opa/client.rs`:

- Implement `OpaClient` struct
- Implement `evaluate_policy()` method
- Implement `check_permission()` helper method
- Add connection pooling and retry logic
- Add health check method

Create `src/auth/opa/policy.rs`:

- Define `AuthorizationRequest` struct (user, action, resource)
- Define `AuthorizationResponse` struct (allow: bool, reason: Option<String>)
- Define helper builders for common scenarios

**Files to Create**:

- `src/auth/opa/mod.rs`
- `src/auth/opa/types.rs`
- `src/auth/opa/client.rs`
- `src/auth/opa/policy.rs`

#### Task 2.3: Create Rego Policy Files

**Objective**: Write OPA policies for resource authorization

**Changes Required**:

Create `config/opa/policies/rbac.rego`:

```rego
package xzepr.authz

import future.keywords.if
import future.keywords.in

# Default deny
default allow = false

# Admin can do everything
allow if {
    "admin" in input.user.roles
}

# Resource ownership checks
is_owner if {
    input.resource.owner_id == input.user.user_id
}

# Group membership checks
is_group_member if {
    input.resource.group_id
    input.user.user_id in input.resource.members
}

# Event receiver POST permissions
allow if {
    input.action == "event:create"
    input.resource.type == "event_receiver"
    is_group_member
}

# Owner can modify their own resources
allow if {
    input.action in ["update", "delete"]
    is_owner
}

# Read permissions based on role
allow if {
    input.action == "read"
    has_read_permission
}

has_read_permission if {
    "event_viewer" in input.user.roles
}

has_read_permission if {
    "event_manager" in input.user.roles
}
```

Create `config/opa/policies/event_receiver.rego`:

```rego
package xzepr.authz.event_receiver

import future.keywords.if
import data.xzepr.authz as authz

# Event receiver specific rules
allow if {
    input.action == "create"
    "event_manager" in input.user.roles
}

allow if {
    input.action == "read"
    authz.has_read_permission
}

allow if {
    input.action == "update"
    authz.is_owner
}

allow if {
    input.action == "delete"
    authz.is_owner
}
```

Create `config/opa/policies/event.rego`:

```rego
package xzepr.authz.event

import future.keywords.if
import data.xzepr.authz as authz

# Event specific rules
allow if {
    input.action == "create"
    input.resource.receiver_group_id
    authz.is_group_member
}

allow if {
    input.action == "read"
    authz.has_read_permission
}

allow if {
    input.action == "update"
    authz.is_owner
}

allow if {
    input.action == "delete"
    authz.is_owner
}
```

**Files to Create**:

- `config/opa/policies/rbac.rego`
- `config/opa/policies/event_receiver.rego`
- `config/opa/policies/event.rego`
- `config/opa/policies/event_receiver_group.rego`

#### Task 2.4: OPA Configuration Management

**Objective**: Add OPA configuration to application config

**Changes Required**:

Update `config/development.yaml`:

```yaml
opa:
  enabled: true
  url: "http://localhost:8181"
  timeout_seconds: 5
  policy_path: "/v1/data/xzepr/authz/allow"
  cache_ttl_seconds: 300
```

Update `config/production.yaml`:

```yaml
opa:
  enabled: true
  url: "${OPA_URL:-http://opa:8181}"
  timeout_seconds: 10
  policy_path: "/v1/data/xzepr/authz/allow"
  bundle_url: "${OPA_BUNDLE_URL}"
  cache_ttl_seconds: 300
```

Create `src/infrastructure/config/opa.rs`:

```rust
use serde::{Deserialize, Serialize};
use crate::error::ConfigError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpaConfig {
    pub enabled: bool,
    pub url: String,
    pub timeout_seconds: u64,
    pub policy_path: String,
    #[serde(default)]
    pub bundle_url: Option<String>,
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: i64,
}

fn default_cache_ttl() -> i64 {
    300 // 5 minutes
}

impl OpaConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.enabled {
            if self.url.is_empty() {
                return Err(ConfigError::ValidationError {
                    field: "opa.url".to_string(),
                    message: "OPA URL cannot be empty when enabled".to_string(),
                });
            }

            if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
                return Err(ConfigError::ValidationError {
                    field: "opa.url".to_string(),
                    message: "OPA URL must start with http:// or https://".to_string(),
                });
            }

            if self.cache_ttl_seconds < 0 {
                return Err(ConfigError::ValidationError {
                    field: "opa.cache_ttl_seconds".to_string(),
                    message: "Cache TTL must be non-negative".to_string(),
                });
            }
        }
        Ok(())
    }
}
```

**Files to Create**:

- `src/infrastructure/config/opa.rs`

**Files to Modify**:

- `config/development.yaml`
- `config/production.yaml`
- `src/infrastructure/config/mod.rs`

#### Task 2.5: Docker Compose OPA Service

**Objective**: Add OPA service to docker-compose for local development

**Changes Required**:

Update `docker-compose.yaml`:

```yaml
opa:
  image: openpolicyagent/opa:latest
  container_name: xzepr-opa
  command:
    - "run"
    - "--server"
    - "--log-level=debug"
    - "/policies"
  ports:
    - "8181:8181"
  volumes:
    - ./config/opa/policies:/policies:ro
  networks:
    - xzepr-network
  healthcheck:
    test: ["CMD", "curl", "-f", "http://localhost:8181/health"]
    interval: 10s
    timeout: 5s
    retries: 5
```

**Files to Modify**:

- `docker-compose.yaml`

#### Task 2.6: Implement Authorization Cache with Invalidation

**Objective**: Build caching layer for OPA decisions with resource-based invalidation

**Changes Required**:

Create `src/auth/opa/cache.rs`:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

pub struct AuthorizationCache {
    cache: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,
    default_ttl: Duration,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct CacheKey {
    pub user_id: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub resource_version: i64,
}

pub struct CacheEntry {
    pub decision: bool,
    pub expires_at: DateTime<Utc>,
    pub cached_at: DateTime<Utc>,
}

impl AuthorizationCache {
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::seconds(ttl_seconds),
        }
    }

    pub async fn get(&self, key: &CacheKey) -> Option<bool> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Utc::now() {
                return Some(entry.decision);
            }
        }
        None
    }

    pub async fn set(&self, key: CacheKey, decision: bool) {
        let entry = CacheEntry {
            decision,
            expires_at: Utc::now() + self.default_ttl,
            cached_at: Utc::now(),
        };
        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }

    pub async fn invalidate_resource(&self, resource_type: &str, resource_id: &str) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, _| {
            !(key.resource_type == resource_type && key.resource_id == resource_id)
        });
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn evict_expired(&self) {
        let now = Utc::now();
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| entry.expires_at > now);
    }
}
```

Create `src/domain/events/resource_updated.rs`:

```rust
// Domain event for cache invalidation
pub enum ResourceUpdatedEvent {
    EventReceiverUpdated {
        receiver_id: EventReceiverId,
        version: i64,
    },
    EventReceiverGroupUpdated {
        group_id: EventReceiverGroupId,
        version: i64,
    },
    EventUpdated {
        event_id: EventId,
        version: i64,
    },
}
```

Update `src/auth/opa/client.rs` to integrate caching:

```rust
pub struct OpaClient {
    http_client: reqwest::Client,
    config: OpaConfig,
    cache: AuthorizationCache,
    metrics: Arc<PrometheusMetrics>,
}

impl OpaClient {
    pub async fn evaluate_policy_cached(
        &self,
        request: AuthorizationRequest,
    ) -> Result<bool, OpaError> {
        // Build cache key with resource version
        let cache_key = CacheKey {
            user_id: request.user.user_id.clone(),
            action: request.action.clone(),
            resource_type: request.resource.resource_type.clone(),
            resource_id: request.resource.resource_id.clone(),
            resource_version: request.resource.version,
        };

        // Check cache first
        if let Some(decision) = self.cache.get(&cache_key).await {
            self.metrics.record_cache_hit();
            return Ok(decision);
        }

        // Cache miss - evaluate with OPA
        self.metrics.record_cache_miss();
        let decision = self.evaluate_policy_uncached(request).await?;

        // Store in cache
        self.cache.set(cache_key, decision).await;

        Ok(decision)
    }
}
```

**Files to Create**:

- `src/auth/opa/cache.rs`
- `src/domain/events/resource_updated.rs`

**Files to Modify**:

- `src/auth/opa/client.rs`
- `src/auth/opa/mod.rs`

#### Task 2.7: Implement Circuit Breaker for OPA Fallback

**Objective**: Add circuit breaker to fallback to legacy RBAC when OPA fails

**Changes Required**:

Create `src/auth/opa/circuit_breaker.rs`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_threshold: usize,
    timeout_duration: Duration,
}

#[derive(Debug, Clone)]
enum CircuitState {
    Closed { consecutive_failures: usize },
    Open { opened_at: DateTime<Utc> },
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, timeout_seconds: i64) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed {
                consecutive_failures: 0,
            })),
            failure_threshold,
            timeout_duration: Duration::seconds(timeout_seconds),
        }
    }

    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: Future<Output = Result<T, E>>,
    {
        // Check circuit state
        let should_attempt = self.should_attempt().await;
        if !should_attempt {
            return Err(CircuitBreakerError::CircuitOpen);
        }

        // Attempt call
        match f.await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(CircuitBreakerError::CallFailed(e))
            }
        }
    }

    async fn should_attempt(&self) -> bool {
        let mut state = self.state.write().await;
        match *state {
            CircuitState::Closed { .. } => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open { opened_at } => {
                if Utc::now() > opened_at + self.timeout_duration {
                    *state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
        }
    }

    async fn record_success(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed {
            consecutive_failures: 0,
        };
    }

    async fn record_failure(&self) {
        let mut state = self.state.write().await;
        match *state {
            CircuitState::Closed { consecutive_failures } => {
                let failures = consecutive_failures + 1;
                if failures >= self.failure_threshold {
                    *state = CircuitState::Open {
                        opened_at: Utc::now(),
                    };
                } else {
                    *state = CircuitState::Closed {
                        consecutive_failures: failures,
                    };
                }
            }
            CircuitState::HalfOpen => {
                *state = CircuitState::Open {
                    opened_at: Utc::now(),
                };
            }
            CircuitState::Open { .. } => {}
        }
    }

    pub async fn is_open(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, CircuitState::Open { .. })
    }
}

pub enum CircuitBreakerError<E> {
    CircuitOpen,
    CallFailed(E),
}
```

Update `src/auth/opa/client.rs` with circuit breaker:

```rust
pub struct OpaClient {
    http_client: reqwest::Client,
    config: OpaConfig,
    cache: AuthorizationCache,
    circuit_breaker: CircuitBreaker,
    metrics: Arc<PrometheusMetrics>,
}

impl OpaClient {
    pub fn new(config: OpaConfig, metrics: Arc<PrometheusMetrics>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            config,
            cache: AuthorizationCache::new(300), // 5 minute TTL
            circuit_breaker: CircuitBreaker::new(5, 30), // 5 failures, 30 sec timeout
            metrics,
        }
    }

    pub async fn evaluate_with_fallback(
        &self,
        request: AuthorizationRequest,
        fallback_fn: impl FnOnce() -> bool,
    ) -> Result<bool, OpaError> {
        match self.circuit_breaker.call(|| self.evaluate_policy_cached(request.clone())).await {
            Ok(decision) => Ok(decision),
            Err(CircuitBreakerError::CircuitOpen) => {
                warn!("OPA circuit breaker is open, falling back to legacy RBAC");
                self.metrics.record_fallback();
                Ok(fallback_fn())
            }
            Err(CircuitBreakerError::CallFailed(e)) => {
                warn!("OPA evaluation failed: {}, falling back to legacy RBAC", e);
                self.metrics.record_fallback();
                Ok(fallback_fn())
            }
        }
    }
}
```

**Files to Create**:

- `src/auth/opa/circuit_breaker.rs`

**Files to Modify**:

- `src/auth/opa/client.rs`
- `src/auth/opa/mod.rs`

#### Task 2.8: Testing Requirements

- Unit tests for OPA client
- Mock OPA server for integration tests
- Policy evaluation tests with various scenarios
- Error handling tests (OPA unavailable, timeout)
- Configuration loading tests
- Cache hit/miss tests
- Cache invalidation tests
- Circuit breaker state transition tests
- Fallback to legacy RBAC tests

#### Task 2.9: Deliverables

- OPA client implementation in `src/auth/opa/`
- Rego policy files in `config/opa/policies/`
- OPA configuration integration
- Docker Compose with OPA service
- Authorization cache with invalidation
- Circuit breaker for OPA fallback
- Comprehensive test suite

#### Task 2.10: Success Criteria

- OPA client can communicate with OPA server
- Policy evaluation returns correct allow/deny decisions
- Configuration loads correctly from YAML
- Docker Compose starts OPA successfully
- Cache hit rate greater than 70 percent in testing
- Circuit breaker opens after 5 consecutive failures
- Fallback to legacy RBAC works correctly
- Cache invalidation works on resource updates
- All tests pass
- Zero clippy warnings

### Phase 3: Authorization Middleware Integration

#### Task 3.1: Create OPA Authorization Middleware

**Objective**: Build middleware to intercept requests and evaluate policies with caching and fallback

**Changes Required**:

Create `src/api/middleware/opa.rs`:

```rust
use std::sync::Arc;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use http::StatusCode;

use crate::auth::opa::{AuthorizationRequest, OpaClient};
use crate::auth::rbac::{Permission, Role};
use crate::api::middleware::jwt::AuthenticatedUser;
use crate::infrastructure::{AuditLogger, PrometheusMetrics};

// OPA authorization middleware state
pub struct OpaMiddlewareState {
    opa_client: Arc<OpaClient>,
    audit_logger: Arc<AuditLogger>,
    metrics: Arc<PrometheusMetrics>,
}

pub async fn opa_authorize_middleware(
    State(state): State<OpaMiddlewareState>,
    user: AuthenticatedUser,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = std::time::Instant::now();

    // Extract resource information from request path
    let resource_info = extract_resource_info(&request)?;

    // Build authorization request
    let auth_req = AuthorizationRequest {
        user: user.to_opa_user(),
        action: resource_info.action.clone(),
        resource: resource_info.context,
    };

    // Define fallback using legacy RBAC
    let fallback_fn = || {
        legacy_rbac_check(&user, &resource_info.action, &resource_info.resource_type)
    };

    // Call OPA with fallback to legacy RBAC
    match state.opa_client.evaluate_with_fallback(auth_req, fallback_fn).await {
        Ok(true) => {
            // Log successful authorization
            state.audit_logger.log_authorization(
                &user.user_id(),
                &resource_info.action,
                &resource_info.resource_id,
                true,
                None,
            ).await;

            state.metrics.record_authorization_decision(
                &resource_info.action,
                &resource_info.resource_type,
                true,
                start_time.elapsed(),
            );

            Ok(next.run(request).await)
        }
        Ok(false) => {
            // Log denied authorization
            state.audit_logger.log_authorization(
                &user.user_id(),
                &resource_info.action,
                &resource_info.resource_id,
                false,
                Some("OPA policy denied access".to_string()),
            ).await;

            state.metrics.record_authorization_decision(
                &resource_info.action,
                &resource_info.resource_type,
                false,
                start_time.elapsed(),
            );

            Err(StatusCode::FORBIDDEN)
        }
        Err(e) => {
            error!("Authorization error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Legacy RBAC check for fallback
fn legacy_rbac_check(user: &AuthenticatedUser, action: &str, resource_type: &str) -> bool {
    let permission = Permission::from_action(resource_type, action);

    match permission {
        Some(perm) => user.roles.iter().any(|role| role.has_permission(&perm)),
        None => {
            // Unknown permission, check if user is admin
            user.roles.contains(&Role::Admin)
        }
    }
}

// Resource extractors for different entity types
async fn extract_event_receiver_context(
    receiver_id: EventReceiverId,
    repo: &dyn EventReceiverRepository,
) -> Result<ResourceContext, AuthError> {
    // Fetch receiver from DB with version
    let receiver = repo.find_by_id(receiver_id).await?
        .ok_or(AuthError::ResourceNotFound)?;

    // Get group membership if receiver belongs to groups
    let groups = repo.find_groups_for_receiver(receiver_id).await?;
    let members = if let Some(group) = groups.first() {
        repo.get_group_members(group.id()).await?
    } else {
        vec![]
    };

    // Build context with version for cache key
    Ok(ResourceContext {
        resource_type: "event_receiver".to_string(),
        resource_id: receiver.id().to_string(),
        owner_id: Some(receiver.owner_id()),
        group_id: groups.first().map(|g| g.id()),
        members,
        version: receiver.version(),
    })
}
```

**Files to Create**:

- `src/api/middleware/opa.rs`

**Files to Modify**:

- `src/api/middleware/mod.rs` (add opa module)

#### Task 3.2: Create Resource Context Builders

**Objective**: Build context objects with ownership and membership data

**Changes Required**:

Create `src/auth/opa/context.rs`:

```rust
pub struct ResourceContext {
    pub resource_type: String,
    pub resource_id: String,
    pub owner_id: Option<UserId>,
    pub group_id: Option<EventReceiverGroupId>,
    pub members: Vec<UserId>,
}

pub trait ResourceContextBuilder {
    async fn build_context(
        &self,
        resource_id: &str,
    ) -> Result<ResourceContext, AuthError>;
}

// Implementations for each resource type
pub struct EventReceiverContextBuilder {
    repo: Arc<dyn EventReceiverRepository>,
    group_repo: Arc<dyn EventReceiverGroupRepository>,
}

pub struct EventContextBuilder {
    repo: Arc<dyn EventRepository>,
    receiver_repo: Arc<dyn EventReceiverRepository>,
}
```

**Files to Create**:

- `src/auth/opa/context.rs`

#### Task 3.3: Update REST Handlers with Authorization

**Objective**: Add OPA authorization checks to REST endpoints

**Changes Required**:

Update `src/api/rest/events.rs`:

- Add OPA middleware to route handlers
- Update `create_event` to check group membership
- Update `update_event` to check ownership
- Update `delete_event` to check ownership
- Update `get_event` to check read permissions

Example pattern with caching and fallback:

```rust
pub async fn create_event(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateEventRequest>,
) -> Result<Json<CreateEventResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Build resource context with version
    let context = build_event_receiver_context(
        &request.event_receiver_id,
        &state.event_receiver_repo,
    ).await?;

    // Build authorization request
    let auth_req = AuthorizationRequest {
        user: user.to_opa_user(),
        action: "event:create".to_string(),
        resource: context,
    };

    // Define legacy RBAC fallback
    let fallback_fn = || {
        user.roles.iter().any(|r| r.has_permission(&Permission::EventCreate))
    };

    // Check authorization with fallback
    let decision = state.opa_client
        .evaluate_with_fallback(auth_req, fallback_fn)
        .await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(
                "authorization_error".to_string(),
                format!("Failed to evaluate authorization: {}", e),
            )))
        })?;

    if !decision {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse::new(
            "authorization_failed".to_string(),
            "You do not have permission to create events for this receiver".to_string(),
        ))));
    }

    // Existing creation logic...
}
```

**Files to Modify**:

- `src/api/rest/events.rs`
- `src/api/rest/routes.rs`

#### Task 3.4: Update GraphQL Resolvers with Authorization

**Objective**: Add OPA authorization to GraphQL operations

**Changes Required**:

Update `src/api/graphql/guards.rs`:

- Add `require_opa_permission` guard function
- Add helper to build OPA requests from GraphQL context

Create `src/api/graphql/opa_guards.rs`:

```rust
pub async fn check_resource_permission(
    ctx: &Context<'_>,
    resource_type: &str,
    resource_id: &str,
    action: &str,
) -> Result<()> {
    // Get user claims from context
    // Build resource context
    // Call OPA
    // Return Ok or Error
}

pub fn require_event_receiver_owner<'a>(
    ctx: &Context<'a>,
    receiver_id: &str,
) -> impl Future<Output = Result<&'a Claims>> {
    // Check ownership via OPA
}
```

Update GraphQL mutation resolvers in `src/api/graphql/resolvers/`:

- Add OPA checks to create/update/delete mutations
- Add ownership validation for modifications

**Files to Create**:

- `src/api/graphql/opa_guards.rs`

**Files to Modify**:

- `src/api/graphql/guards.rs`
- `src/api/graphql/resolvers/event_receiver.rs`
- `src/api/graphql/resolvers/event.rs`
- `src/api/graphql/resolvers/event_receiver_group.rs`

#### Task 3.5: Update Application Handlers

**Objective**: Add owner_id to handler operations

**Changes Required**:

Update `src/application/handlers/event_receiver_handler.rs`:

- Add `user_id: UserId` parameter to `create_event_receiver()`
- Pass owner_id when creating entity
- Update error messages for authorization failures

Update `src/application/handlers/event_handler.rs`:

- Add `user_id: UserId` parameter to `create_event()`
- Validate group membership before creation
- Pass owner_id to event entity

Update `src/application/handlers/event_receiver_group_handler.rs`:

- Add `user_id: UserId` parameter to `create_group()`
- Add `add_member()` and `remove_member()` methods
- Add ownership checks in update/delete methods

**Files to Modify**:

- `src/application/handlers/event_receiver_handler.rs`
- `src/application/handlers/event_handler.rs`
- `src/application/handlers/event_receiver_group_handler.rs`

#### Task 3.6: Testing Requirements

- Integration tests for authorized operations
- Integration tests for denied operations
- Test ownership checks (owner can modify, non-owner cannot)
- Test group membership (member can POST, non-member cannot)
- Test role-based access (admin can do everything)
- Test error messages and HTTP status codes
- Mock OPA responses for unit tests

#### Task 3.7: Deliverables

- OPA authorization middleware
- Resource context builders
- Updated REST endpoints with OPA checks
- Updated GraphQL resolvers with OPA checks
- Updated application handlers with ownership
- Comprehensive integration test suite

#### Task 3.8: Success Criteria

- REST and GraphQL endpoints enforce OPA policies
- Authorization decisions logged to audit trail
- Owner-only operations restricted correctly
- Group-based POST to event receivers works
- All integration tests pass
- Zero clippy warnings
- Test coverage greater than 80 percent

### Phase 4: Group Management and Membership APIs

#### Task 4.1: Create Group Membership REST Endpoints

**Objective**: Add API endpoints for managing group membership

**Changes Required**:

Create `src/api/rest/groups.rs`:

```rust
// POST /api/v1/groups/:id/members
pub async fn add_group_member(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(group_id): Path<String>,
    Json(request): Json<AddMemberRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if user is group owner
    // Add member to group
}

// DELETE /api/v1/groups/:id/members/:user_id
pub async fn remove_group_member(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((group_id, member_id)): Path<(String, String)>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if user is group owner
    // Remove member from group
}

// GET /api/v1/groups/:id/members
pub async fn list_group_members(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(group_id): Path<String>,
) -> Result<Json<Vec<UserResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Check read permission
    // Return member list
}
```

**Files to Create**:

- `src/api/rest/groups.rs`

#### Task 4.2: Create Group Membership GraphQL Mutations

**Objective**: Add GraphQL operations for group management

**Changes Required**:

Update `src/api/graphql/schema.graphql`:

```graphql
type Mutation {
  addGroupMember(groupId: ID!, userId: ID!): GroupMembershipResult!
  removeGroupMember(groupId: ID!, userId: ID!): GroupMembershipResult!
}

type Query {
  groupMembers(groupId: ID!): [User!]!
}

type GroupMembershipResult {
  success: Boolean!
  message: String
  group: EventReceiverGroup
}
```

Create `src/api/graphql/resolvers/group_membership.rs`:

- Implement `add_group_member` mutation resolver
- Implement `remove_group_member` mutation resolver
- Implement `group_members` query resolver
- Add OPA authorization checks

**Files to Create**:

- `src/api/graphql/resolvers/group_membership.rs`

**Files to Modify**:

- `src/api/graphql/schema.rs`
- `src/api/graphql/mod.rs`

#### Task 4.3: Create DTOs for Group Membership

**Objective**: Define request/response types for membership operations

**Changes Required**:

Update `src/api/rest/dtos.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct GroupMemberResponse {
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub added_at: DateTime<Utc>,
    pub added_by: String,
}

#[derive(Debug, Serialize)]
pub struct GroupMembersResponse {
    pub group_id: String,
    pub members: Vec<GroupMemberResponse>,
}
```

**Files to Modify**:

- `src/api/rest/dtos.rs`

#### Task 4.4: Testing Requirements

- REST endpoint tests for add/remove members
- GraphQL mutation tests for membership
- Authorization tests (only owner can modify)
- Validation tests (invalid user_id, group_id)
- Integration tests with OPA policies
- Test group member queries

#### Task 4.5: Deliverables

- REST endpoints for group membership management
- GraphQL mutations and queries for membership
- DTOs for membership operations
- Integration tests for all operations

#### Task 4.6: Success Criteria

- Group owners can add/remove members
- Non-owners cannot modify membership
- Members list is queryable
- All endpoints enforce OPA policies
- Tests pass with greater than 80 percent coverage
- Zero clippy warnings

### Phase 5: Audit Logging and Monitoring

#### Task 5.1: Enhance Audit Logger for Authorization Events

**Objective**: Log all authorization decisions for security audit

**Changes Required**:

Update `src/infrastructure/audit/mod.rs`:

- Add `AuthorizationDecision` audit action
- Add fields for policy name, decision, and reason
- Log OPA request/response details

Create `src/infrastructure/audit/opa.rs`:

```rust
pub async fn log_authorization_decision(
    audit_logger: &AuditLogger,
    user_id: &str,
    action: &str,
    resource: &str,
    decision: bool,
    reason: Option<String>,
) {
    audit_logger.log(AuditEvent {
        timestamp: Utc::now(),
        user_id: Some(user_id.to_string()),
        action: AuditAction::AuthorizationDecision,
        resource: resource.to_string(),
        outcome: if decision {
            AuditOutcome::Success
        } else {
            AuditOutcome::Denied
        },
        details: Some(json!({
            "action": action,
            "decision": decision,
            "reason": reason,
        })),
    });
}
```

**Files to Create**:

- `src/infrastructure/audit/opa.rs`

**Files to Modify**:

- `src/infrastructure/audit/mod.rs`
- `src/api/middleware/opa.rs` (add audit calls)

#### Task 5.2: Add Prometheus Metrics for Authorization

**Objective**: Track authorization metrics for monitoring

**Changes Required**:

Update `src/infrastructure/telemetry/metrics.rs`:

```rust
// Add metrics constants
pub const OPA_AUTHORIZATION_REQUESTS: &str = "xzepr_opa_authorization_requests_total";
pub const OPA_AUTHORIZATION_DURATION: &str = "xzepr_opa_authorization_duration_seconds";
pub const OPA_AUTHORIZATION_DENIALS: &str = "xzepr_opa_authorization_denials_total";
pub const OPA_CACHE_HITS: &str = "xzepr_opa_cache_hits_total";
pub const OPA_CACHE_MISSES: &str = "xzepr_opa_cache_misses_total";
pub const OPA_FALLBACK_COUNT: &str = "xzepr_opa_fallback_to_rbac_total";
pub const OPA_CIRCUIT_BREAKER_STATE: &str = "xzepr_opa_circuit_breaker_state";

impl PrometheusMetrics {
    pub fn record_authorization_decision(
        &self,
        action: &str,
        resource_type: &str,
        decision: bool,
        duration: Duration,
    ) {
        // Record request count
        self.counter_with_labels(
            OPA_AUTHORIZATION_REQUESTS,
            &[("action", action), ("resource_type", resource_type), ("decision", &decision.to_string())],
        );

        // Record decision time
        self.histogram(OPA_AUTHORIZATION_DURATION, duration.as_secs_f64());

        // Record denials counter
        if !decision {
            self.counter_with_labels(
                OPA_AUTHORIZATION_DENIALS,
                &[("action", action), ("resource_type", resource_type)],
            );
        }
    }

    pub fn record_cache_hit(&self) {
        self.counter(OPA_CACHE_HITS);
    }

    pub fn record_cache_miss(&self) {
        self.counter(OPA_CACHE_MISSES);
    }

    pub fn record_fallback(&self) {
        self.counter(OPA_FALLBACK_COUNT);
    }

    pub fn set_circuit_breaker_state(&self, is_open: bool) {
        self.gauge(OPA_CIRCUIT_BREAKER_STATE, if is_open { 1.0 } else { 0.0 });
    }
}
```

**Files to Modify**:

- `src/infrastructure/telemetry/metrics.rs`
- `src/api/middleware/opa.rs` (add metrics recording)

#### Task 5.3: Add OpenTelemetry Spans for Authorization

**Objective**: Add tracing spans for authorization flow

**Changes Required**:

Update OPA middleware to add spans:

```rust
#[tracing::instrument(
    name = "opa_authorization",
    skip(opa_client),
    fields(
        user_id = %user.user_id(),
        action = %action,
        resource = %resource_id,
        decision = tracing::field::Empty,
    )
)]
async fn evaluate_authorization(
    opa_client: &OpaClient,
    user: &AuthenticatedUser,
    action: &str,
    resource_id: &str,
) -> Result<bool, AuthError> {
    // OPA evaluation logic
    // Record decision in span
}
```

**Files to Modify**:

- `src/api/middleware/opa.rs`
- `src/auth/opa/client.rs`

#### Task 5.4: Create Authorization Dashboard

**Objective**: Document Grafana dashboard queries for authorization monitoring

**Changes Required**:

Create `docs/reference/monitoring_dashboards.md`:

````markdown
## Authorization Metrics

### Authorization Request Rate

```promql
rate(xzepr_opa_authorization_requests_total[5m])
```

### Authorization Denial Rate

```promql
rate(xzepr_opa_authorization_denials_total[5m])
```

### Authorization Latency P95

```promql
histogram_quantile(0.95, rate(xzepr_opa_authorization_duration_seconds_bucket[5m]))
```

### Cache Hit Rate

```promql
rate(xzepr_opa_cache_hits_total[5m]) / (rate(xzepr_opa_cache_hits_total[5m]) + rate(xzepr_opa_cache_misses_total[5m]))
```

### Fallback to Legacy RBAC Rate

```promql
rate(xzepr_opa_fallback_to_rbac_total[5m])
```

### Circuit Breaker State

```promql
xzepr_opa_circuit_breaker_state
```
````

**Files to Create**:

- `docs/reference/monitoring_dashboards.md`

#### Task 5.5: Testing Requirements

- Test audit log entries for authorization events
- Test metrics recording for decisions
- Test tracing span creation
- Validate audit log format and content
- Test metrics aggregation queries

#### Task 5.6: Deliverables

- Audit logging for authorization decisions
- Prometheus metrics for authorization
- OpenTelemetry spans in authorization flow
- Monitoring dashboard documentation

#### Task 5.7: Success Criteria

- All authorization decisions logged to audit trail
- Metrics available in Prometheus
- Spans visible in Jaeger
- Dashboard queries documented
- Tests pass with greater than 80 percent coverage

### Phase 6: Documentation and Deployment Guide

#### Task 6.1: Create API Documentation

**Objective**: Document new authorization model and APIs

**Changes Required**:

Create `docs/reference/authorization_api.md`:

- Document OPA policy structure
- Document resource ownership model
- Document group membership model
- Document API endpoints for membership management
- Provide examples for each use case

Create `docs/how-to/manage_group_membership.md`:

- Step-by-step guide for creating groups
- How to add/remove members
- How to grant access to event receivers
- Troubleshooting common issues

**Files to Create**:

- `docs/reference/authorization_api.md`
- `docs/how-to/manage_group_membership.md`

#### Task 6.2: Create Policy Bundle Server Setup

**Objective**: Configure OPA bundle server for policy distribution

**Changes Required**:

Create `docs/how-to/setup_opa_bundle_server.md`:

```markdown
## OPA Bundle Server Setup

### Overview

Configure OPA to load policies from bundle server with versioning.

### Bundle Structure
```

bundles/
├── .manifest
├── policies/
│ ├── rbac.rego
│ ├── event_receiver.rego
│ ├── event.rego
│ └── event_receiver_group.rego
└── data.json

````

### CI/CD Integration

1. Build bundle on policy changes
2. Version bundle with Git SHA
3. Upload to bundle server
4. OPA polls for updates

### Configuration

```yaml
services:
  bundle_server:
    url: http://bundle-server:8080
    credentials:
      bearer:
        token: "${BUNDLE_SERVER_TOKEN}"

bundles:
  xzepr_authz:
    service: bundle_server
    resource: bundles/xzepr-authz-latest.tar.gz
    polling:
      min_delay_seconds: 60
      max_delay_seconds: 120
````

````

Create `scripts/build_opa_bundle.sh`:

```bash
#!/bin/bash
# Script to build OPA policy bundle
# Validates policies and creates versioned bundle
````

**Files to Create**:

- `docs/how-to/setup_opa_bundle_server.md`
- `scripts/build_opa_bundle.sh`
- `scripts/validate_opa_policies.sh`
- `.github/workflows/opa_bundle_deploy.yaml`

Create `scripts/validate_opa_policies.sh`:

```bash
#!/bin/bash
# Validate OPA policies before deployment
set -e

echo "Validating OPA policies..."

# Check OPA CLI is installed
if ! command -v opa &> /dev/null; then
    echo "Error: OPA CLI not found. Install from https://www.openpolicyagent.org/docs/latest/#running-opa"
    exit 1
fi

# Validate policy syntax
opa check config/opa/policies/*.rego

# Run policy tests if they exist
if [ -f "config/opa/policies/test_rbac.rego" ]; then
    opa test config/opa/policies/
fi

echo "All policies validated successfully!"
```

#### Task 6.3: Update Architecture Documentation

**Objective**: Document OPA integration in architecture docs

**Changes Required**:

Update `docs/explanation/architecture.md`:

- Add OPA component to architecture diagram
- Explain authorization flow
- Document policy evaluation process
- Show interaction between middleware and OPA

Update `docs/explanation/security_architecture.md`:

- Document OPA security considerations
- Explain policy management
- Document authorization decision caching strategy

**Files to Modify**:

- `docs/explanation/architecture.md`
- `docs/explanation/security_architecture.md`

#### Task 6.4: Create OpenAPI Specification Updates

**Objective**: Update OpenAPI docs with new endpoints and authorization

**Changes Required**:

Update OpenAPI specification files:

- Add group membership endpoints
- Add authorization error responses (403 Forbidden)
- Document required permissions for each endpoint
- Add examples with authorization headers

**Files to Modify**:

- API specification files in `docs/reference/`

#### Task 6.5: Create Policy Development Guide

**Objective**: Guide for writing and testing OPA policies

**Changes Required**:

Create `docs/how-to/develop_opa_policies.md`:

- How to write Rego policies
- How to test policies locally
- How to deploy policies to OPA
- Best practices for policy organization
- Common patterns and examples

**Files to Create**:

- `docs/how-to/develop_opa_policies.md`

#### Task 6.6: Testing Requirements

- Review all documentation for accuracy
- Test migration script with sample data
- Validate OpenAPI examples
- Test code examples in documentation

#### Task 6.7: Deliverables

- API reference documentation
- How-to guides for group management
- Migration guide and scripts
- Updated architecture documentation
- Policy development guide

#### Task 6.8: Success Criteria

- All documentation follows Diataxis framework
- Documentation placed in correct categories
- All file names use lowercase_with_underscores.md
- Code examples are tested and working
- Migration guide tested with sample data
- OpenAPI specification validates successfully

## Implementation Guidelines

### Code Quality Requirements

All phases must meet these requirements:

- Run `cargo fmt --all` before committing
- Zero warnings from `cargo clippy --all-targets --all-features -- -D warnings`
- Test coverage greater than 80 percent
- All public functions have doc comments with examples
- Integration tests for all API endpoints
- Unit tests for all business logic

### Security Considerations

- Never log sensitive data (passwords, tokens) in authorization decisions
- Implement rate limiting for OPA requests
- Cache policy decisions with invalidation on resource updates (5 minute TTL)
- Use HTTPS for OPA communication in production
- Validate all user input before OPA evaluation
- Implement circuit breaker for OPA unavailability (5 failures trigger fallback to legacy RBAC)
- Secure bundle server with authentication tokens
- Version and sign policy bundles

### Performance Considerations

- Cache OPA decisions with resource version tracking
- Cache resource context lookups (5 minute TTL)
- Invalidate cache on resource updates
- Batch membership queries where possible
- Add database indexes for owner_id columns
- Monitor OPA response times
- Implement fallback to legacy RBAC on OPA failure
- Use connection pooling for OPA HTTP client
- Bundle server reduces policy download overhead

### Backward Compatibility

- Maintain existing role-based permissions as fallback when OPA unavailable
- Support OPA disable via config flag for testing
- Existing API contracts remain unchanged
- Add new endpoints, do not modify existing ones
- Legacy RBAC provides safety net during OPA outages

## Testing Strategy

### Unit Tests

- Domain entity ownership validation
- Repository ownership queries
- OPA client request building
- Policy evaluation logic
- Context builder functionality

### Integration Tests

- End-to-end authorization flows
- REST endpoint authorization
- GraphQL mutation authorization
- Group membership operations
- Database migration validation

### Performance Tests

- OPA response time under load
- Authorization middleware latency
- Cache hit rate measurement
- Database query performance with ownership indexes

### Security Tests

- Unauthorized access attempts
- Permission escalation attempts
- Invalid token handling
- OPA unavailability scenarios
- SQL injection in ownership queries

## Rollout Strategy

### Development Environment

1. Enable OPA in docker-compose
2. Run migrations
3. Test with sample data
4. Validate policies locally

### Staging Environment

1. Deploy OPA service with bundle server
2. Run database migrations
3. Enable OPA with monitoring
4. Run full integration test suite
5. Performance testing
6. Validate cache invalidation behavior

### Production Environment

1. Deploy single OPA instance with bundle server
2. Run database migrations (no legacy data to migrate)
3. Enable OPA with feature flag
4. Monitor authorization metrics closely
5. Verify fallback to legacy RBAC works correctly
6. Plan for HA deployment based on usage metrics

## Success Metrics

- Authorization decision latency less than 50ms P95 (including cache hits)
- Cache hit rate greater than 70 percent
- Zero unauthorized access to resources
- 100 percent of new resources have assigned owners
- Audit logs capture all authorization decisions
- Zero production incidents related to authorization
- Policy bundle deployment time less than 2 minutes
- Fallback to legacy RBAC occurs within 1 second of OPA failure

## Design Decisions

1. **Caching Strategy**: Cache with invalidation on resource updates

   - Cache OPA decisions with resource version tracking
   - Invalidate cache entries when resources are modified
   - Default TTL of 5 minutes for cache entries
   - Implement cache invalidation hooks in repository update methods

2. **OPA Deployment**: Single instance to begin with

   - Start with single OPA instance for simplicity
   - Plan for future HA deployment as usage grows
   - Monitor availability and performance metrics
   - Document scaling path for production needs

3. **Migration Timeline**: No migration needed

   - Fresh implementation, no existing users
   - All new resources will have owners from creation
   - No legacy data to migrate

4. **Fallback Behavior**: Fallback to legacy RBAC

   - When OPA is unavailable, use existing role-based permissions
   - Log fallback events for monitoring
   - Implement circuit breaker pattern (5 failures triggers fallback)
   - Automatic recovery when OPA becomes available

5. **Policy Update Process**: Bundle server with versioning
   - Use OPA bundle server for policy distribution
   - Version policies in Git repository
   - Automated bundle building in CI/CD pipeline
   - Rolling updates with version verification

## Dependencies

### External Services

- OPA server (version 0.50.0 or later)
- PostgreSQL 14 or later (for ownership tables)
- Prometheus (for metrics)
- Jaeger (for tracing)

### Rust Crates

- reqwest 0.11 (HTTP client for OPA)
- serde_json 1.0 (JSON serialization)
- async-trait 0.1 (async traits)

### Infrastructure

- Docker and Docker Compose for local development
- Kubernetes for production deployment (optional)
- Load balancer for OPA HA (production)

## Risk Mitigation

### Risk: OPA Single Point of Failure

**Mitigation**: Implement circuit breaker pattern with fallback to legacy RBAC. Monitor OPA availability continuously. Plan migration to HA deployment when usage justifies complexity. Use aggressive caching with invalidation to reduce OPA dependency.

### Risk: Policy Bugs Cause Outages

**Mitigation**: Implement comprehensive policy testing framework. Use OPA playground for policy development. Require code review for all policy changes. Deploy with gradual rollout and quick rollback capability.

### Risk: Performance Degradation

**Mitigation**: Implement caching at multiple layers. Monitor P95 latency continuously. Set SLO for authorization decisions. Load test before production deployment.

### Risk: Cache Invalidation Bugs

**Mitigation**: Comprehensive testing of cache invalidation paths. Monitor cache hit/miss rates. Implement cache TTL as safety net (5 minutes). Add manual cache clear endpoint for emergency use. Log all cache invalidation events.

### Risk: Security Vulnerabilities

**Mitigation**: Regular security audits of policies. Penetration testing of authorization logic. Follow principle of least privilege. Log all authorization decisions for forensic analysis.

## Validation Checklist

Before considering implementation complete, verify:

- [ ] All domain entities track ownership
- [ ] Database migration applies and rolls back cleanly
- [ ] OPA client can evaluate policies successfully
- [ ] Rego policies enforce ownership and membership rules
- [ ] REST endpoints enforce OPA authorization
- [ ] GraphQL operations enforce OPA authorization
- [ ] Group membership can be managed via API
- [ ] Authorization decisions are logged to audit trail
- [ ] Metrics are available in Prometheus
- [ ] Tracing spans visible in Jaeger
- [ ] Cache invalidation works correctly on resource updates
- [ ] Fallback to legacy RBAC works when OPA unavailable
- [ ] Circuit breaker triggers appropriately
- [ ] Bundle server configuration complete
- [ ] Policy bundle build and deployment automated
- [ ] Documentation complete and accurate
- [ ] All tests pass with greater than 80 percent coverage
- [ ] Zero clippy warnings
- [ ] Code formatted with cargo fmt
- [ ] OpenAPI specification updated
- [ ] Performance meets SLO targets (including cache performance)
- [ ] Security review completed

## References

- OPA Documentation: https://www.openpolicyagent.org/docs/latest/
- Rego Language: https://www.openpolicyagent.org/docs/latest/policy-language/
- OPA REST API: https://www.openpolicyagent.org/docs/latest/rest-api/
- XZepr Architecture: `docs/explanation/architecture.md`
- XZepr Security: `docs/explanation/security_architecture.md`
- RBAC Current State: `docs/explanation/rbac_opa.md`

---

**Document Metadata**:

- Created: 2025-01-XX
- Author: AI Agent
- Status: Draft
- Version: 1.0
