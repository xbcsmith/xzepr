# OPA RBAC Expansion Implementation Summary

## Overview

This document summarizes the phased implementation plan for expanding XZepr's RBAC system with Open Policy Agent (OPA) integration. The implementation adds fine-grained resource ownership and group-based access control while maintaining backward compatibility with the existing role-based permissions system.

## Problem Statement

The current RBAC system provides role-based permissions but lacks:

- Resource ownership tracking (who created the resource)
- Group-based access control (team-level permissions)
- Fine-grained authorization (resource-level access control)
- Centralized policy management
- Audit trail for authorization decisions

## Use Case Example

Joe creates an event receiver. The system should enforce:

- Only members of the joe_receiver group can POST events to Joe's event receiver
- Only Joe himself can modify or delete Joe's event receiver
- Same pattern applies to events and event_receiver_groups

## Solution Architecture

### Core Components

1. **OPA Integration**: External policy engine for authorization decisions
2. **Resource Ownership**: Domain entities track owner_id field
3. **Group Membership**: Users can be members of event_receiver_groups
4. **Authorization Cache**: Cache with resource version-based invalidation
5. **Circuit Breaker**: Fallback to legacy RBAC when OPA unavailable
6. **Bundle Server**: Versioned policy distribution system

### Technology Stack

- **OPA**: Open Policy Agent 0.50.0 or later
- **Rego**: Policy definition language
- **PostgreSQL**: Ownership and membership tables
- **Rust**: OPA client, cache, circuit breaker implementation

## Implementation Phases

### Phase 1: Domain Model Extension (1-2 weeks)

**Objective**: Add ownership tracking to domain entities and database schema

**Key Deliverables**:

- `owner_id: UserId` field added to EventReceiver, EventReceiverGroup, Event
- `members: Vec<UserId>` field added to EventReceiverGroup
- Database migration with ownership and membership tables
- Repository methods for ownership queries
- PostgreSQL implementation with indexes

**Success Metrics**:

- All entities track ownership from creation
- Group membership can be managed via repository methods
- Test coverage greater than 80 percent

### Phase 2: OPA Infrastructure Setup (1-2 weeks)

**Objective**: Deploy OPA service and implement client library

**Key Deliverables**:

- OPA client in `src/auth/opa/` with HTTP communication
- Rego policy files in `config/opa/policies/`
- Docker Compose service for local development
- Authorization cache with 5-minute TTL
- Circuit breaker with fallback to legacy RBAC
- Configuration management for OPA settings

**Success Metrics**:

- OPA evaluates policies correctly
- Cache hit rate greater than 70 percent
- Circuit breaker opens after 5 consecutive failures
- Fallback to legacy RBAC works seamlessly

### Phase 3: Authorization Middleware Integration (2-3 weeks)

**Objective**: Integrate OPA authorization into API layer

**Key Deliverables**:

- OPA authorization middleware in `src/api/middleware/opa.rs`
- Resource context builders with ownership and membership data
- REST endpoint protection with OPA checks
- GraphQL resolver protection with OPA guards
- Application handlers updated with owner_id parameters

**Success Metrics**:

- All REST and GraphQL endpoints enforce OPA policies
- Owner-only operations restricted correctly
- Group members can POST to receivers
- Non-members denied access appropriately

### Phase 4: Group Management APIs (1 week)

**Objective**: Build APIs for managing group membership

**Key Deliverables**:

- REST endpoints: `POST /api/v1/groups/:id/members`
- REST endpoints: `DELETE /api/v1/groups/:id/members/:user_id`
- REST endpoints: `GET /api/v1/groups/:id/members`
- GraphQL mutations: `addGroupMember`, `removeGroupMember`
- GraphQL query: `groupMembers`

**Success Metrics**:

- Group owners can add/remove members
- Non-owners cannot modify membership
- Members list queryable via API

### Phase 5: Audit Logging and Monitoring (1 week)

**Objective**: Add observability for authorization decisions

**Key Deliverables**:

- Audit logger enhanced for authorization events
- Prometheus metrics for OPA decisions
- OpenTelemetry spans in authorization flow
- Grafana dashboard queries documented

**Metrics Tracked**:

- Authorization request rate
- Authorization denial rate
- Authorization latency P95
- Cache hit/miss rate
- Fallback to legacy RBAC count
- Circuit breaker state

### Phase 6: Documentation and Deployment (1 week)

**Objective**: Document system and prepare for deployment

**Key Deliverables**:

- API reference documentation
- Group management how-to guide
- OPA bundle server setup guide
- Policy development guide
- Updated architecture documentation
- OpenAPI specification updates

## Key Design Decisions

### 1. Caching Strategy: Cache with Invalidation

- Cache OPA decisions with resource version tracking
- 5-minute TTL as safety net
- Invalidate cache entries when resources are modified
- Cache invalidation hooks in repository update methods

**Rationale**: Balances performance with consistency. Resource version prevents stale decisions.

### 2. OPA Deployment: Single Instance

- Start with single OPA instance for simplicity
- Monitor availability and performance metrics
- Plan migration to HA deployment as usage grows

**Rationale**: Avoid premature complexity. Single instance sufficient for initial deployment.

### 3. Migration Timeline: No Migration Required

- Fresh implementation, no existing users
- All new resources have owners from creation
- No legacy data to migrate

**Rationale**: System not yet in production use.

### 4. Fallback Behavior: Fallback to Legacy RBAC

- Circuit breaker pattern with 5 failure threshold
- Automatic fallback to role-based permissions
- Log all fallback events for monitoring
- Automatic recovery when OPA available

**Rationale**: Maintains availability during OPA outages while preserving security.

### 5. Policy Updates: Bundle Server with Versioning

- OPA bundle server for policy distribution
- Policies versioned in Git repository
- Automated bundle building in CI/CD
- Rolling updates with version verification

**Rationale**: Production-grade policy management with audit trail.

## Security Considerations

### Authentication and Authorization

- JWT authentication remains unchanged
- OPA evaluates authorization after authentication
- All authorization decisions logged to audit trail
- Failed authorization attempts tracked in metrics

### Data Protection

- Never log sensitive data in authorization decisions
- Cache only authorization decisions, not resource data
- HTTPS required for OPA communication in production
- Bundle server requires authentication tokens

### Attack Surface

- Rate limiting for OPA requests prevents DoS
- Input validation before OPA evaluation prevents injection
- Circuit breaker prevents cascade failures
- Fallback provides defense in depth

## Performance Characteristics

### Expected Performance

- Authorization decision latency: less than 50ms P95 (with cache)
- Cache hit rate: greater than 70 percent after warmup
- OPA evaluation time: 10-30ms without cache
- Database ownership query: 5-15ms

### Optimization Strategies

- Aggressive caching with invalidation
- Database indexes on owner_id columns
- Connection pooling for OPA HTTP client
- Batch membership queries where possible
- Bundle server reduces policy download overhead

## Rollout Strategy

### Development Environment

1. Enable OPA in docker-compose
2. Run database migrations
3. Test with sample data
4. Validate policies locally

### Staging Environment

1. Deploy OPA service with bundle server
2. Run database migrations
3. Enable OPA with monitoring
4. Run integration test suite
5. Performance testing
6. Validate cache behavior

### Production Environment

1. Deploy single OPA instance with bundle server
2. Run database migrations
3. Enable OPA with feature flag
4. Monitor authorization metrics closely
5. Verify fallback to legacy RBAC works
6. Plan HA deployment based on metrics

## Success Metrics

- Authorization decision latency less than 50ms P95
- Cache hit rate greater than 70 percent
- Zero unauthorized access to resources
- 100 percent of new resources have assigned owners
- Audit logs capture all authorization decisions
- Policy bundle deployment time less than 2 minutes
- Fallback occurs within 1 second of OPA failure

## Testing Strategy

### Unit Tests

- Domain entity ownership validation
- Repository ownership queries
- OPA client request building
- Cache hit/miss logic
- Circuit breaker state transitions

### Integration Tests

- End-to-end authorization flows
- REST endpoint authorization
- GraphQL mutation authorization
- Group membership operations
- Cache invalidation on updates

### Performance Tests

- OPA response time under load
- Authorization middleware latency
- Cache hit rate measurement
- Database query performance

### Security Tests

- Unauthorized access attempts
- Permission escalation attempts
- Invalid token handling
- OPA unavailability scenarios

## Risk Mitigation

### Risk: OPA Single Point of Failure

**Mitigation**: Circuit breaker with fallback to legacy RBAC. Monitor OPA availability. Plan HA deployment when justified.

### Risk: Policy Bugs Cause Outages

**Mitigation**: Comprehensive policy testing. Code review for policy changes. Gradual rollout with quick rollback.

### Risk: Performance Degradation

**Mitigation**: Multi-layer caching. Continuous P95 monitoring. Load testing before deployment.

### Risk: Cache Invalidation Bugs

**Mitigation**: Comprehensive invalidation path testing. TTL safety net. Manual cache clear endpoint.

## Open Items

- [ ] Determine bundle server hosting solution
- [ ] Define policy versioning scheme
- [ ] Establish policy change approval process
- [ ] Create runbooks for OPA outage scenarios
- [ ] Define SLOs for authorization latency
- [ ] Plan capacity for OPA scaling

## References

- **Detailed Plan**: `docs/explanation/opa_rbac_expansion_plan.md`
- **Current RBAC**: `docs/explanation/rbac_opa.md`
- **Architecture**: `docs/explanation/architecture.md`
- **Security**: `docs/explanation/security_architecture.md`
- **OPA Documentation**: https://www.openpolicyagent.org/docs/latest/

## Timeline Estimate

Total estimated time: 7-10 weeks

- Phase 1: 1-2 weeks
- Phase 2: 1-2 weeks
- Phase 3: 2-3 weeks
- Phase 4: 1 week
- Phase 5: 1 week
- Phase 6: 1 week

## Next Steps

1. Review and approve implementation plan
2. Set up development environment with OPA
3. Begin Phase 1: Domain model extension
4. Establish weekly progress reviews
5. Track metrics and adjust timeline as needed

---

**Document Metadata**:

- Created: 2025-01-XX
- Author: AI Agent
- Status: Draft
- Version: 1.0
- Related: `opa_rbac_expansion_plan.md`
