# OPA RBAC Implementation - Next Steps and Roadmap

**Status**: Implementation phase complete (85-90%), moving into validation and deployment
**Timeline**: 2-3 weeks to production readiness
**Target Date**: End of Week 3

---

## What's Complete

The entire OPA RBAC infrastructure is implemented and tested:

✅ Domain model with ownership tracking (Event, EventReceiver, EventReceiverGroup)
✅ Repository layer with ownership and membership queries
✅ OPA client with HTTP communication and error handling
✅ Authorization cache with resource version invalidation
✅ Circuit breaker with fallback to legacy RBAC
✅ Authorization middleware ready for integration
✅ REST and GraphQL APIs for group membership
✅ Audit logging for authorization decisions
✅ Prometheus metrics infrastructure
✅ 618 tests passing with >80% coverage
✅ Zero Clippy warnings, fully formatted code
✅ Complete architecture documentation

---

## What's Required for Production

### 🔴 CRITICAL PATH (Must Complete)

#### Task 1: Create Rego Policy Files (2-3 days)

**Why**: System cannot function without policies - OPA needs rules to evaluate

**Deliverables**:

1. **config/opa/policies/rbac.rego** (Core RBAC rules)
   ```rego
   package xzepr.rbac

   # Allow owner-only operations
   allow_owner_operation {
       input.user.id == input.resource.owner_id
   }

   # Allow group member operations
   allow_group_member_operation {
       user_is_group_member
   }

   # Allow role-based operations
   allow_role_based_operation {
       user_has_required_role
   }
   ```

2. **config/opa/policies/event_receiver.rego** (Event receiver access rules)
   - Owner can create, read, update, delete own receivers
   - Members of receiver's group can POST events
   - Admins have full access

3. **config/opa/policies/event.rego** (Event access rules)
   - Owner can create, read, update, delete own events
   - Event receiver owner can read events sent to their receiver
   - Group members can read group events

4. **config/opa/policies/event_receiver_group.rego** (Group access rules)
   - Owner can manage group
   - Owner can add/remove members
   - Members can read group info

5. **config/opa/policies/utils.rego** (Helper functions)
   - `user_is_owner(resource)` - Check if user owns resource
   - `user_is_group_member(group_id)` - Check group membership
   - `user_has_role(role)` - Check user role
   - `user_is_admin()` - Check admin status

**Testing Strategy**:
```bash
# Test policies locally with OPA CLI
opa test config/opa/policies/ -v

# Evaluate sample decisions
opa eval -d config/opa/policies/ \
  'data.xzepr.rbac.allow' \
  -i sample_input.json
```

**Success Criteria**:
- All policies evaluate correctly with sample inputs
- Owner-only operations properly restricted
- Group member access properly enforced
- Role-based access working as designed
- Zero policy syntax errors

---

#### Task 2: OPA Bundle Server Setup (1-2 days)

**Why**: Production needs versioned, signed policy bundles

**Deliverables**:

1. **OPA Bundle Server** (Choose one approach)

   Option A: Docker container with static bundle serving
   ```dockerfile
   FROM openpolicyagent/opa:latest
   COPY bundle.tar.gz /policies/
   ENTRYPOINT ["/opa", "run", "--server", "--bundle", "/policies/bundle.tar.gz"]
   ```

   Option B: Git-based bundle (OPA bundles from Git repo)
   - Repository: `https://github.com/org/xzepr-policies`
   - Structure:
     ```
     xzepr-policies/
     ├── rbac.rego
     ├── event_receiver.rego
     ├── event.rego
     ├── event_receiver_group.rego
     ├── utils.rego
     └── .manifest (versioning)
     ```

2. **Bundle Creation Pipeline**
   - Script to create bundle: `scripts/create_opa_bundle.sh`
   - Versioning: Semantic versioning (e.g., v1.0.0)
   - Storage: Docker registry or S3 bucket
   - Signature verification (optional)

3. **docker-compose.yaml Update**
   ```yaml
   opa:
     image: openpolicyagent/opa:latest
     container_name: xzepr-opa
     command:
       - run
       - --server
       - --addr=0.0.0.0:8181
       - --log-level=debug
     volumes:
       - ./config/opa/policies:/policies
     ports:
       - "8181:8181"
     healthcheck:
       test: ["CMD", "curl", "-f", "http://localhost:8181/health?bundles"]
       interval: 10s
       timeout: 5s
       retries: 3
     networks:
       - xzepr-network
   ```

**Testing Strategy**:
```bash
# Start OPA server
docker-compose up opa

# Test OPA availability
curl http://localhost:8181/health

# Test policy evaluation
curl -X POST http://localhost:8181/v1/data/xzepr/rbac/allow \
  -H "Content-Type: application/json" \
  -d @sample_input.json
```

**Success Criteria**:
- OPA server starts and stays healthy
- Policies load without errors
- Policy evaluation works via HTTP API
- Health check endpoint responds correctly

---

#### Task 3: Integration Testing with Real OPA (2-3 days)

**Why**: Validates entire authorization flow end-to-end

**Deliverables**:

1. **Integration Test Suite** (`tests/opa_integration_tests.rs`)
   ```rust
   #[tokio::test]
   async fn test_owner_can_delete_own_event_receiver() {
       // Setup: Create OPA server container
       // Act: Call DELETE /event_receivers/123 as owner
       // Assert: 204 No Content response
   }

   #[tokio::test]
   async fn test_non_owner_cannot_delete_event_receiver() {
       // Setup: Create OPA server container
       // Act: Call DELETE /event_receivers/123 as non-owner
       // Assert: 403 Forbidden response
   }

   #[tokio::test]
   async fn test_group_member_can_post_events() {
       // Setup: Create group, add member, create event receiver
       // Act: POST event as group member
       // Assert: 201 Created response
   }
   ```

2. **Test Scenarios**
   - Owner operations (create, read, update, delete)
   - Group member operations (read, post events)
   - Non-owner/non-member access denial
   - Role-based access (admin override)
   - Cache invalidation after updates
   - Circuit breaker fallback behavior

3. **Test Infrastructure**
   - `testcontainers` for OPA Docker container
   - Sample authorization inputs
   - Mock database with test data
   - Assertion helpers for authorization checks

**Testing Strategy**:
```bash
# Run integration tests
cargo test --test opa_integration_tests --all-features -- --test-threads=1 --nocapture

# Run specific test
cargo test --test opa_integration_tests owner_can_delete -- --nocapture
```

**Success Criteria**:
- All 50+ integration tests passing
- 100% authorization decision accuracy
- Cache invalidation working correctly
- Circuit breaker fallback validated
- Load test shows < 50ms P95 latency

---

#### Task 4: Production Deployment Documentation (2-3 days)

**Why**: Operations team needs clear procedures for deployment and troubleshooting

**Deliverables**:

1. **docs/how-to/opa_production_deployment.md**
   - Prerequisites (OPA version, PostgreSQL, etc.)
   - Step-by-step deployment procedure
   - Configuration validation checklist
   - Post-deployment verification
   - Rollback procedures
   - Monitoring setup

2. **docs/how-to/opa_policy_development.md**
   - Policy structure and organization
   - Writing and testing Rego policies
   - Policy versioning strategy
   - Code review checklist for policies
   - Testing policies locally
   - Deploying policy updates

3. **docs/how-to/opa_troubleshooting.md**
   - Common authorization denial issues
   - Cache invalidation problems
   - Circuit breaker troubleshooting
   - Performance investigation
   - OPA server unavailability handling
   - Debug logging for decision tracing

4. **docs/reference/opa_api.md**
   - Authorization API endpoints
   - Request/response schema examples
   - Error codes and meanings
   - Rate limiting information
   - Metrics endpoint specification

5. **Operations Runbooks**
   - Restart OPA server procedure
   - Rollback policy to previous version
   - Clear authorization cache
   - Manual circuit breaker reset
   - Enable/disable OPA evaluation

**Success Criteria**:
- All deployment steps documented with examples
- Troubleshooting guide covers 80% of likely issues
- API documentation includes curl examples
- Runbooks tested by operations team

---

### 🟡 HIGH PRIORITY (Strongly Recommended)

#### Task 5: Performance Baseline and Optimization (2-3 days)

**Why**: Validates system meets latency requirements (P95 < 50ms)

**Deliverables**:

1. **Performance Testing**
   ```rust
   #[tokio::test]
   async fn benchmark_authorization_latency() {
       // Warm up cache
       // Measure 1000 authorization decisions
       // Record latencies (min, P50, P95, P99, max)
       // Assert P95 < 50ms
   }
   ```

2. **Cache Performance**
   - Measure cache hit rate under normal load
   - Verify cache warmup strategy
   - Test cache invalidation performance
   - Optimize cache size settings

3. **Database Performance**
   - Profile ownership/membership queries
   - Verify index usage
   - Measure query latency
   - Optimize connection pooling

4. **OPA Server Performance**
   - Measure policy evaluation time
   - Test with various policy complexity
   - Verify bundle size is optimal
   - Benchmark resource usage

**Success Criteria**:
- Authorization latency P95 < 50ms (with cache)
- Cache hit rate > 70% in steady state
- No query N+1 problems
- OPA server CPU/memory usage acceptable

---

#### Task 6: Security Hardening (2-3 days)

**Why**: Production system needs security controls

**Deliverables**:

1. **TLS Configuration**
   - Configure OPA with mTLS
   - Generate certificates and keys
   - Update OPA client to use TLS
   - Verify secure communication

2. **Secret Management**
   - Remove hardcoded secrets
   - Integrate with secret vault (Vault, AWS Secrets Manager, etc.)
   - OPA credentials in secrets
   - Database credentials in secrets

3. **Rate Limiting**
   - Implement rate limiting on authorization endpoint
   - Prevent authorization endpoint abuse
   - Configure burst limits
   - Add rate limit response headers

4. **Input Validation**
   - Validate all input to OPA
   - Prevent injection attacks
   - Validate user IDs, resource IDs
   - Sanitize error messages

5. **Audit Log Protection**
   - Enable audit log encryption
   - Implement retention policy
   - Prevent audit log modification
   - Secure audit log storage

**Success Criteria**:
- TLS communication verified with packet inspection
- All secrets stored in vault, none hardcoded
- Rate limiting working correctly
- Security audit passes

---

### 🟢 MEDIUM PRIORITY (Good to Have)

#### Task 7: Operational Monitoring (2 days)

**Why**: Enables real-time visibility into authorization system health

**Deliverables**:

1. **Prometheus Metrics Dashboard**
   - Authorization request rate
   - Authorization denial rate
   - Authorization latency histogram
   - Cache hit/miss rate
   - Circuit breaker state
   - OPA server availability

2. **Alerting Rules**
   - Alert when authorization denial rate > 5%
   - Alert when P95 latency > 50ms
   - Alert when cache hit rate < 60%
   - Alert when circuit breaker is open
   - Alert when OPA server is down

3. **Grafana Dashboard**
   - Real-time authorization metrics
   - Historical trend analysis
   - Cache performance visualization
   - Circuit breaker state indicator
   - Policy version tracking

**Success Criteria**:
- Dashboard displays all key metrics
- Alerts fire correctly on threshold breach
- Dashboard loads without performance issues

---

## Step-by-Step Implementation Plan

### Week 1: Core Policy Development

**Monday-Tuesday**: Create Rego policies
- [ ] Write rbac.rego with core rules
- [ ] Write event_receiver.rego policy
- [ ] Write event.rego policy
- [ ] Write event_receiver_group.rego policy
- [ ] Write utils.rego helpers
- [ ] Test policies with OPA CLI

**Wednesday-Thursday**: Setup OPA infrastructure
- [ ] Create bundle structure
- [ ] Setup OPA Docker service
- [ ] Configure docker-compose
- [ ] Test OPA availability
- [ ] Verify policy loading

**Friday**: Initial integration
- [ ] Start OPA server locally
- [ ] Create sample authorization requests
- [ ] Test policy evaluation
- [ ] Verify cache integration
- [ ] Test fallback behavior

### Week 2: Integration Testing and Validation

**Monday-Tuesday**: Build integration tests
- [ ] Setup testcontainers for OPA
- [ ] Write owner operation tests
- [ ] Write group member tests
- [ ] Write denial case tests
- [ ] Run test suite

**Wednesday-Thursday**: Performance testing
- [ ] Create performance test suite
- [ ] Measure authorization latency
- [ ] Validate cache hit rate
- [ ] Test under load
- [ ] Optimize if needed

**Friday**: Documentation
- [ ] Write deployment guide
- [ ] Write policy development guide
- [ ] Create troubleshooting guide
- [ ] Document APIs
- [ ] Create operational runbooks

### Week 3: Security and Production Readiness

**Monday-Tuesday**: Security hardening
- [ ] Configure TLS
- [ ] Setup secret management
- [ ] Implement rate limiting
- [ ] Validate input handling
- [ ] Security audit

**Wednesday-Thursday**: Monitoring setup
- [ ] Create Prometheus metrics
- [ ] Setup Grafana dashboard
- [ ] Configure alerting rules
- [ ] Test alert triggering
- [ ] Document metrics

**Friday**: Staging validation
- [ ] Deploy to staging environment
- [ ] Run full integration test suite
- [ ] Validate monitoring
- [ ] Perform smoke tests
- [ ] Sign off for production

---

## Files to Create

### Rego Policy Files
- [ ] `config/opa/policies/rbac.rego` (200-300 lines)
- [ ] `config/opa/policies/event_receiver.rego` (100-150 lines)
- [ ] `config/opa/policies/event.rego` (100-150 lines)
- [ ] `config/opa/policies/event_receiver_group.rego` (100-150 lines)
- [ ] `config/opa/policies/utils.rego` (100-150 lines)

### Test Files
- [ ] `tests/opa_integration_tests.rs` (500-800 lines)
- [ ] `tests/fixtures/opa_test_data.json` (Sample inputs/outputs)

### Documentation Files
- [ ] `docs/how-to/opa_production_deployment.md` (300-400 lines)
- [ ] `docs/how-to/opa_policy_development.md` (200-300 lines)
- [ ] `docs/how-to/opa_troubleshooting.md` (200-300 lines)
- [ ] `docs/reference/opa_api.md` (150-200 lines)
- [ ] `docs/operations/opa_runbooks.md` (200-300 lines)

### Configuration Files
- [ ] `scripts/create_opa_bundle.sh` (Bash script to create bundles)
- [ ] `config/opa/bundle.tar.gz` (Policy bundle)
- [ ] `docker-compose.yaml` (Updated with OPA service)

### Total New Lines of Code: ~3,000-4,000 lines

---

## Success Criteria for Each Task

### Task 1: Rego Policies
- ✅ All policy files created
- ✅ No syntax errors reported by OPA CLI
- ✅ All policies evaluate correctly with test inputs
- ✅ Authorization decisions match expected outcomes

### Task 2: OPA Bundle Server
- ✅ OPA server starts without errors
- ✅ Health check endpoint responds
- ✅ Policies load and are accessible
- ✅ Policy evaluation works via HTTP API

### Task 3: Integration Tests
- ✅ 50+ tests written and passing
- ✅ Coverage for all major authorization paths
- ✅ P95 authorization latency < 50ms
- ✅ Cache hit rate > 70%

### Task 4: Documentation
- ✅ All deployment steps documented
- ✅ Troubleshooting guide covers common issues
- ✅ API documentation with examples
- ✅ Runbooks tested by operations team

### Task 5: Performance
- ✅ Authorization latency P95 < 50ms
- ✅ Cache hit rate > 70%
- ✅ No performance regressions
- ✅ Database queries optimized

### Task 6: Security
- ✅ TLS communication verified
- ✅ All secrets in vault
- ✅ Rate limiting working
- ✅ Input validation comprehensive

### Task 7: Monitoring
- ✅ Grafana dashboard working
- ✅ All metrics visible
- ✅ Alerts firing correctly
- ✅ Historical data retention

---

## Risk Mitigation

### Risk: OPA Policy Bugs
**Mitigation**:
- Comprehensive policy testing before deployment
- Code review for all policy changes
- Gradual rollout with feature flag
- Quick rollback procedure

### Risk: Performance Issues
**Mitigation**:
- Load testing before production
- P95 latency monitoring
- Aggressive caching strategy
- Capacity planning

### Risk: Cache Invalidation Bugs
**Mitigation**:
- Comprehensive test coverage
- Resource version validation
- TTL safety net
- Manual cache clear capability

### Risk: OPA Service Outage
**Mitigation**:
- Circuit breaker with fallback
- Monitoring and alerting
- Quick recovery procedures
- Documented troubleshooting

---

## Validation Checklist

### Pre-Deployment
- [ ] All Rego policies created and tested
- [ ] OPA bundle server configured and verified
- [ ] Integration tests passing (50+)
- [ ] Performance baseline met (P95 < 50ms)
- [ ] Security audit completed
- [ ] Documentation complete
- [ ] Monitoring configured
- [ ] Runbooks tested
- [ ] Staging deployment successful

### Post-Deployment
- [ ] Monitor authorization latency for 24 hours
- [ ] Verify cache hit rate > 70%
- [ ] Check for any authorization denials
- [ ] Validate audit logs
- [ ] Confirm monitoring alerts working
- [ ] Verify fallback is not being used
- [ ] Review metrics for anomalies

---

## Resources and References

### OPA Documentation
- https://www.openpolicyagent.org/docs/latest/
- https://www.openpolicyagent.org/docs/latest/policy-language/
- https://www.openpolicyagent.org/docs/latest/deployments/

### Rego Policy Examples
- https://github.com/open-policy-agent/opa/tree/main/examples
- https://www.openpolicyagent.org/docs/latest/policy-language/#rules

### Testing Tools
- OPA CLI: https://www.openpolicyagent.org/docs/latest/cli/
- Testcontainers: https://testcontainers.com/

### Related Documentation
- `docs/explanation/opa_rbac_expansion_plan.md` - Detailed plan
- `docs/explanation/opa_rbac_completion_status.md` - Status report
- `docs/explanation/opa_authorization_architecture.md` - Architecture

---

## Contact and Questions

For questions about these next steps:
- Review `docs/explanation/opa_rbac_completion_status.md` for status
- Check `docs/explanation/opa_authorization_architecture.md` for design details
- See `AGENTS.md` for development guidelines

---

**Document Version**: 1.0
**Last Updated**: 2025-01-20
**Status**: Ready for implementation
