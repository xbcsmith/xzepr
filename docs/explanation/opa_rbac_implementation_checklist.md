# OPA RBAC Implementation Checklist

## Overview

This checklist tracks progress through the OPA RBAC expansion implementation. Update status as tasks are completed.

**Legend**:
- `[ ]` Not started
- `[~]` In progress
- `[x]` Complete
- `[!]` Blocked

---

## Phase 1: Domain Model Extension and Database Schema

**Target Duration**: 1-2 weeks

### Task 1.1: Extend Domain Entities with Ownership

- [ ] Update `EventReceiver` entity with `owner_id` field
- [ ] Update `EventReceiverGroup` entity with `owner_id` and `members` fields
- [ ] Update `Event` entity with `owner_id` field
- [ ] Add `owner_id()` getter methods
- [ ] Add `add_member()`, `remove_member()`, `is_member()` methods to EventReceiverGroup
- [ ] Update entity validation logic
- [ ] Update entity unit tests

**Files to Modify**:
- `src/domain/entities/event_receiver.rs`
- `src/domain/entities/event_receiver_group.rs`
- `src/domain/entities/event.rs`

### Task 1.2: Create Database Migration

- [ ] Create migration file for ownership columns
- [ ] Add `owner_id` to `event_receivers` table
- [ ] Add `owner_id` to `event_receiver_groups` table
- [ ] Add `owner_id` to `events` table
- [ ] Create `event_receiver_group_members` table
- [ ] Create indexes for performance
- [ ] Test migration rollback

**Files to Create**:
- `migrations/YYYYMMDD_add_ownership_and_membership.sql`

### Task 1.3: Update Repository Interfaces

- [ ] Add `find_by_owner()` to EventReceiverRepository
- [ ] Add `is_owner()` to EventReceiverRepository
- [ ] Add `find_by_owner()` to EventReceiverGroupRepository
- [ ] Add `is_owner()` to EventReceiverGroupRepository
- [ ] Add `is_member()` to EventReceiverGroupRepository
- [ ] Add `add_member()` to EventReceiverGroupRepository
- [ ] Add `remove_member()` to EventReceiverGroupRepository
- [ ] Add `get_members()` to EventReceiverGroupRepository
- [ ] Add ownership methods to EventRepository

**Files to Modify**:
- `src/domain/repositories/event_receiver_repo.rs`
- `src/domain/repositories/event_receiver_group_repo.rs`
- `src/domain/repositories/event_repo.rs`

### Task 1.4: Implement PostgreSQL Repository Updates

- [ ] Implement ownership queries in PostgresEventReceiverRepository
- [ ] Update `save()` to include owner_id
- [ ] Add version column for cache invalidation
- [ ] Implement membership methods in PostgresEventReceiverGroupRepository
- [ ] Add queries for `event_receiver_group_members` table
- [ ] Implement ownership queries in PostgresEventRepository
- [ ] Add cache invalidation hooks

**Files to Modify**:
- `src/infrastructure/persistence/postgres/event_receiver_repo.rs`
- `src/infrastructure/persistence/postgres/event_receiver_group_repo.rs`
- `src/infrastructure/persistence/postgres/event_repo.rs`

### Task 1.5: Testing

- [ ] Unit tests for entity ownership validation
- [ ] Unit tests for group membership operations
- [ ] Repository tests for ownership queries
- [ ] Integration tests for database operations
- [ ] Migration rollback tests
- [ ] Verify test coverage greater than 80 percent

### Phase 1 Validation

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes
- [ ] Test coverage greater than 80 percent
- [ ] All entities track ownership
- [ ] Group membership can be managed
- [ ] Database migration applies cleanly

---

## Phase 2: OPA Infrastructure Setup

**Target Duration**: 1-2 weeks

### Task 2.1: Add OPA Dependencies

- [ ] Add `reqwest` to Cargo.toml
- [ ] Verify dependencies compile

### Task 2.2: Create OPA Client Module

- [ ] Create `src/auth/opa/mod.rs`
- [ ] Create `src/auth/opa/types.rs` with OpaConfig, OpaError
- [ ] Create `src/auth/opa/client.rs` with OpaClient implementation
- [ ] Create `src/auth/opa/policy.rs` with request/response types
- [ ] Implement `evaluate_policy()` method
- [ ] Add connection pooling
- [ ] Add health check method

**Files to Create**:
- `src/auth/opa/mod.rs`
- `src/auth/opa/types.rs`
- `src/auth/opa/client.rs`
- `src/auth/opa/policy.rs`

### Task 2.3: Create Rego Policy Files

- [ ] Create `config/opa/policies/rbac.rego`
- [ ] Create `config/opa/policies/event_receiver.rego`
- [ ] Create `config/opa/policies/event.rego`
- [ ] Create `config/opa/policies/event_receiver_group.rego`
- [ ] Test policies with OPA CLI

**Files to Create**:
- `config/opa/policies/rbac.rego`
- `config/opa/policies/event_receiver.rego`
- `config/opa/policies/event.rego`
- `config/opa/policies/event_receiver_group.rego`

### Task 2.4: OPA Configuration Management

- [ ] Update `config/development.yaml` with OPA settings
- [ ] Update `config/production.yaml` with OPA settings
- [ ] Create `src/infrastructure/config/opa.rs`
- [ ] Add OpaConfig to main Config struct
- [ ] Add configuration validation

**Files to Create**:
- `src/infrastructure/config/opa.rs`

**Files to Modify**:
- `config/development.yaml`
- `config/production.yaml`
- `src/infrastructure/config/mod.rs`

### Task 2.5: Docker Compose OPA Service

- [ ] Add OPA service to `docker-compose.yaml`
- [ ] Configure volume mounts for policies
- [ ] Add health check
- [ ] Test OPA service starts correctly

**Files to Modify**:
- `docker-compose.yaml`

### Task 2.6: Implement Authorization Cache

- [ ] Create `src/auth/opa/cache.rs`
- [ ] Implement `AuthorizationCache` with HashMap
- [ ] Implement cache key with resource version
- [ ] Implement `get()`, `set()`, `invalidate_resource()` methods
- [ ] Implement `evict_expired()` for TTL cleanup
- [ ] Create `src/domain/events/resource_updated.rs` for cache invalidation events
- [ ] Integrate cache into OpaClient

**Files to Create**:
- `src/auth/opa/cache.rs`
- `src/domain/events/resource_updated.rs`

**Files to Modify**:
- `src/auth/opa/client.rs`
- `src/auth/opa/mod.rs`

### Task 2.7: Implement Circuit Breaker

- [ ] Create `src/auth/opa/circuit_breaker.rs`
- [ ] Implement CircuitBreaker with state machine
- [ ] Implement failure threshold logic (5 failures)
- [ ] Implement timeout and half-open state
- [ ] Add circuit breaker to OpaClient
- [ ] Implement `evaluate_with_fallback()` method

**Files to Create**:
- `src/auth/opa/circuit_breaker.rs`

**Files to Modify**:
- `src/auth/opa/client.rs`
- `src/auth/opa/mod.rs`

### Task 2.8: Testing

- [ ] Unit tests for OPA client
- [ ] Mock OPA server for integration tests
- [ ] Policy evaluation tests
- [ ] Error handling tests
- [ ] Configuration loading tests
- [ ] Cache hit/miss tests
- [ ] Cache invalidation tests
- [ ] Circuit breaker state transition tests
- [ ] Fallback to legacy RBAC tests

### Phase 2 Validation

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes
- [ ] OPA client communicates with OPA server
- [ ] Policy evaluation returns correct decisions
- [ ] Docker Compose starts OPA successfully
- [ ] Cache hit rate greater than 70 percent in testing
- [ ] Circuit breaker opens after 5 consecutive failures
- [ ] Fallback to legacy RBAC works

---

## Phase 3: Authorization Middleware Integration

**Target Duration**: 2-3 weeks

### Task 3.1: Create OPA Authorization Middleware

- [ ] Create `src/api/middleware/opa.rs`
- [ ] Implement `OpaMiddlewareState` struct
- [ ] Implement `opa_authorize_middleware` function
- [ ] Implement resource extraction from request
- [ ] Implement legacy RBAC fallback function
- [ ] Add audit logging for decisions
- [ ] Add metrics recording

**Files to Create**:
- `src/api/middleware/opa.rs`

**Files to Modify**:
- `src/api/middleware/mod.rs`

### Task 3.2: Create Resource Context Builders

- [ ] Create `src/auth/opa/context.rs`
- [ ] Define `ResourceContext` struct
- [ ] Define `ResourceContextBuilder` trait
- [ ] Implement `EventReceiverContextBuilder`
- [ ] Implement `EventContextBuilder`
- [ ] Implement `EventReceiverGroupContextBuilder`

**Files to Create**:
- `src/auth/opa/context.rs`

### Task 3.3: Update REST Handlers with Authorization

- [ ] Update `create_event` with OPA check
- [ ] Update `update_event` with ownership check
- [ ] Update `delete_event` with ownership check
- [ ] Update `get_event` with read permission check
- [ ] Update `create_event_receiver` with OPA check
- [ ] Update `update_event_receiver` with ownership check
- [ ] Update `delete_event_receiver` with ownership check
- [ ] Add OPA middleware to routes

**Files to Modify**:
- `src/api/rest/events.rs`
- `src/api/rest/routes.rs`

### Task 3.4: Update GraphQL Resolvers with Authorization

- [ ] Update `src/api/graphql/guards.rs` with OPA helper
- [ ] Create `src/api/graphql/opa_guards.rs`
- [ ] Implement `check_resource_permission` function
- [ ] Implement `require_event_receiver_owner` function
- [ ] Update event receiver mutation resolvers
- [ ] Update event mutation resolvers
- [ ] Update event receiver group mutation resolvers

**Files to Create**:
- `src/api/graphql/opa_guards.rs`

**Files to Modify**:
- `src/api/graphql/guards.rs`
- `src/api/graphql/resolvers/event_receiver.rs`
- `src/api/graphql/resolvers/event.rs`
- `src/api/graphql/resolvers/event_receiver_group.rs`

### Task 3.5: Update Application Handlers

- [ ] Add `user_id` parameter to `create_event_receiver()`
- [ ] Add `user_id` parameter to `create_event()`
- [ ] Add `user_id` parameter to `create_group()`
- [ ] Add `add_member()` method to EventReceiverGroupHandler
- [ ] Add `remove_member()` method to EventReceiverGroupHandler
- [ ] Add ownership checks in update methods
- [ ] Add ownership checks in delete methods

**Files to Modify**:
- `src/application/handlers/event_receiver_handler.rs`
- `src/application/handlers/event_handler.rs`
- `src/application/handlers/event_receiver_group_handler.rs`

### Task 3.6: Testing

- [ ] Integration tests for authorized operations
- [ ] Integration tests for denied operations
- [ ] Test ownership checks
- [ ] Test group membership checks
- [ ] Test role-based access
- [ ] Test error messages and status codes
- [ ] Mock OPA responses for unit tests

### Phase 3 Validation

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes
- [ ] Test coverage greater than 80 percent
- [ ] REST endpoints enforce OPA policies
- [ ] GraphQL resolvers enforce OPA policies
- [ ] Authorization decisions logged
- [ ] Owner-only operations restricted
- [ ] Group-based POST to receivers works

---

## Phase 4: Group Management and Membership APIs

**Target Duration**: 1 week

### Task 4.1: Create Group Membership REST Endpoints

- [ ] Create `src/api/rest/groups.rs`
- [ ] Implement `add_group_member` handler
- [ ] Implement `remove_group_member` handler
- [ ] Implement `list_group_members` handler
- [ ] Add OPA authorization checks
- [ ] Add routes to router

**Files to Create**:
- `src/api/rest/groups.rs`

### Task 4.2: Create Group Membership GraphQL Mutations

- [ ] Update `src/api/graphql/schema.graphql`
- [ ] Create `src/api/graphql/resolvers/group_membership.rs`
- [ ] Implement `add_group_member` mutation resolver
- [ ] Implement `remove_group_member` mutation resolver
- [ ] Implement `group_members` query resolver
- [ ] Add OPA authorization checks

**Files to Create**:
- `src/api/graphql/resolvers/group_membership.rs`

**Files to Modify**:
- `src/api/graphql/schema.rs`
- `src/api/graphql/mod.rs`

### Task 4.3: Create DTOs for Group Membership

- [ ] Add `AddMemberRequest` to dtos
- [ ] Add `GroupMemberResponse` to dtos
- [ ] Add `GroupMembersResponse` to dtos
- [ ] Add validation methods

**Files to Modify**:
- `src/api/rest/dtos.rs`

### Task 4.4: Testing

- [ ] REST endpoint tests for add member
- [ ] REST endpoint tests for remove member
- [ ] REST endpoint tests for list members
- [ ] GraphQL mutation tests
- [ ] Authorization tests (only owner can modify)
- [ ] Validation tests

### Phase 4 Validation

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes
- [ ] Test coverage greater than 80 percent
- [ ] Group owners can add/remove members
- [ ] Non-owners cannot modify membership
- [ ] Members list is queryable

---

## Phase 5: Audit Logging and Monitoring

**Target Duration**: 1 week

### Task 5.1: Enhance Audit Logger

- [ ] Update `src/infrastructure/audit/mod.rs` with AuthorizationDecision action
- [ ] Create `src/infrastructure/audit/opa.rs`
- [ ] Implement `log_authorization_decision` function
- [ ] Add audit calls to OPA middleware

**Files to Create**:
- `src/infrastructure/audit/opa.rs`

**Files to Modify**:
- `src/infrastructure/audit/mod.rs`
- `src/api/middleware/opa.rs`

### Task 5.2: Add Prometheus Metrics

- [ ] Add authorization metrics constants
- [ ] Implement `record_authorization_decision` method
- [ ] Implement `record_cache_hit` method
- [ ] Implement `record_cache_miss` method
- [ ] Implement `record_fallback` method
- [ ] Implement `set_circuit_breaker_state` method
- [ ] Add metrics recording to OPA middleware

**Files to Modify**:
- `src/infrastructure/telemetry/metrics.rs`
- `src/api/middleware/opa.rs`

### Task 5.3: Add OpenTelemetry Spans

- [ ] Add tracing span to `evaluate_authorization`
- [ ] Add span attributes for user, action, resource
- [ ] Record decision in span

**Files to Modify**:
- `src/api/middleware/opa.rs`
- `src/auth/opa/client.rs`

### Task 5.4: Create Monitoring Dashboard Documentation

- [ ] Create `docs/reference/monitoring_dashboards.md`
- [ ] Document authorization request rate query
- [ ] Document denial rate query
- [ ] Document latency P95 query
- [ ] Document cache hit rate query
- [ ] Document fallback rate query
- [ ] Document circuit breaker state query

**Files to Create**:
- `docs/reference/monitoring_dashboards.md`

### Task 5.5: Testing

- [ ] Test audit log entries
- [ ] Test metrics recording
- [ ] Test tracing spans
- [ ] Validate audit log format
- [ ] Test metrics queries

### Phase 5 Validation

- [ ] `cargo fmt --all` passes
- [ ] `cargo check --all-targets --all-features` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` shows zero warnings
- [ ] `cargo test --all-features` passes
- [ ] Authorization decisions logged
- [ ] Metrics available in Prometheus
- [ ] Spans visible in Jaeger

---

## Phase 6: Documentation and Deployment Guide

**Target Duration**: 1 week

### Task 6.1: Create API Documentation

- [ ] Create `docs/reference/authorization_api.md`
- [ ] Document OPA policy structure
- [ ] Document resource ownership model
- [ ] Document group membership model
- [ ] Create `docs/how_to/manage_group_membership.md`
- [ ] Provide step-by-step guide

**Files to Create**:
- `docs/reference/authorization_api.md`
- `docs/how_to/manage_group_membership.md`

### Task 6.2: Create Policy Bundle Server Setup

- [ ] Create `docs/how_to/setup_opa_bundle_server.md`
- [ ] Document bundle structure
- [ ] Document CI/CD integration
- [ ] Create `scripts/build_opa_bundle.sh`
- [ ] Create `scripts/validate_opa_policies.sh`
- [ ] Create `.github/workflows/opa_bundle_deploy.yaml`

**Files to Create**:
- `docs/how_to/setup_opa_bundle_server.md`
- `scripts/build_opa_bundle.sh`
- `scripts/validate_opa_policies.sh`
- `.github/workflows/opa_bundle_deploy.yaml`

### Task 6.3: Update Architecture Documentation

- [ ] Update `docs/explanation/architecture.md` with OPA component
- [ ] Update `docs/explanation/security_architecture.md`
- [ ] Document authorization flow
- [ ] Document policy evaluation process

**Files to Modify**:
- `docs/explanation/architecture.md`
- `docs/explanation/security_architecture.md`

### Task 6.4: Update OpenAPI Specification

- [ ] Add group membership endpoints
- [ ] Add 403 Forbidden responses
- [ ] Document required permissions
- [ ] Add authorization examples

### Task 6.5: Create Policy Development Guide

- [ ] Create `docs/how_to/develop_opa_policies.md`
- [ ] Document Rego policy writing
- [ ] Document policy testing
- [ ] Document policy deployment
- [ ] Document best practices

**Files to Create**:
- `docs/how_to/develop_opa_policies.md`

### Task 6.6: Testing

- [ ] Review all documentation for accuracy
- [ ] Validate OpenAPI examples
- [ ] Test code examples

### Phase 6 Validation

- [ ] All documentation follows Diataxis framework
- [ ] Documentation in correct categories
- [ ] File names use lowercase_with_underscores.md
- [ ] Code examples tested
- [ ] OpenAPI specification validates

---

## Final Validation

### Code Quality

- [ ] All files formatted with `cargo fmt --all`
- [ ] Zero warnings from `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Test coverage greater than 80 percent
- [ ] All public functions have doc comments with examples

### Functionality

- [ ] All domain entities track ownership
- [ ] Database migration applies and rolls back cleanly
- [ ] OPA client evaluates policies successfully
- [ ] Rego policies enforce ownership and membership rules
- [ ] REST endpoints enforce OPA authorization
- [ ] GraphQL operations enforce OPA authorization
- [ ] Group membership manageable via API

### Observability

- [ ] Authorization decisions logged to audit trail
- [ ] Metrics available in Prometheus
- [ ] Tracing spans visible in Jaeger
- [ ] Cache hit rate greater than 70 percent
- [ ] Circuit breaker functions correctly
- [ ] Fallback to legacy RBAC works

### Documentation

- [ ] Documentation complete and accurate
- [ ] All documentation files in correct Diataxis categories
- [ ] All file names use lowercase_with_underscores.md
- [ ] OpenAPI specification updated
- [ ] No emojis in documentation

### Performance

- [ ] Authorization decision latency less than 50ms P95
- [ ] Cache hit rate greater than 70 percent
- [ ] Performance meets SLO targets

### Security

- [ ] Security review completed
- [ ] No sensitive data in logs
- [ ] Rate limiting implemented
- [ ] Input validation in place

---

## Deployment Checklist

### Development Environment

- [ ] OPA service running in docker-compose
- [ ] Database migrations applied
- [ ] Sample data created
- [ ] Policies validated locally

### Staging Environment

- [ ] OPA service deployed with bundle server
- [ ] Database migrations applied
- [ ] Integration tests passing
- [ ] Performance tests passing
- [ ] Cache behavior validated

### Production Environment

- [ ] Single OPA instance deployed
- [ ] Bundle server configured
- [ ] Database migrations applied
- [ ] Monitoring alerts configured
- [ ] Runbooks created for incidents
- [ ] Rollback plan documented

---

## Notes

### Blockers

Document any blockers here:

### Decisions

Document key decisions made during implementation:

### Lessons Learned

Document lessons learned for future reference:

---

**Last Updated**: YYYY-MM-DD
**Current Phase**: Phase X
**Overall Progress**: X of 6 phases complete
