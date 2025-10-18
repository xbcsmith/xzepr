# Production Implementation Guide

This document provides a comprehensive overview of the production-readiness implementation structure for XZepr. It serves as the master guide for implementing the remaining features needed for production deployment.

## Document Purpose

This guide consolidates all production implementation requirements into a single reference document. Use this as your starting point, then dive into the detailed guides in the `docs/` directory for specific implementation steps.

## Current Status

### What's Complete

- **Architecture**: Clean layered architecture (domain/application/infrastructure/api)
- **APIs**: REST and GraphQL endpoints with basic functionality
- **Authentication**: Multi-provider auth (local, Keycloak, API keys) with RBAC
- **Database**: PostgreSQL with migrations for users and API keys
- **Messaging**: Redpanda/Kafka integration for event streaming
- **TLS**: HTTPS support with certificate management
- **Docker**: Containerized deployment with docker-compose
- **GraphQL**: Playground and query/mutation support

### What's Needed for Production

1. **PostgreSQL Repository Implementations**
   - Events, Event Receivers, Event Receiver Groups
   - Replace in-memory mocks with persistent storage

2. **Proper JWT Authentication**
   - Secure key management (RS256)
   - Token refresh flow
   - Token blacklisting/revocation

3. **Security Hardening**
   - CORS configuration (specific origins only)
   - Rate limiting (per-user and per-endpoint)
   - GraphQL authentication guards
   - Input validation and sanitization

4. **Observability**
   - Prometheus metrics export
   - Jaeger distributed tracing
   - Structured JSON logging
   - Comprehensive health checks

5. **Load Testing**
   - k6 test scenarios
   - Performance baselines
   - Continuous testing in CI/CD

## Implementation Structure

### Documentation Organization (Diataxis Framework)

All documentation follows the Diataxis framework:

```text
docs/
├── explanations/           # Understanding-oriented
│   ├── production_readiness_roadmap.md
│   └── architecture.md
├── how_to/                # Task-oriented
│   ├── implement_postgres_repositories.md
│   ├── implement_jwt_authentication.md
│   ├── setup_load_testing.md
│   └── security_hardening.md
├── tutorials/             # Learning-oriented
│   └── getting_started.md
└── reference/             # Information-oriented
    └── api.md
```

### Source Code Structure

```text
src/
├── api/
│   ├── graphql/           # GraphQL schema and resolvers
│   ├── middleware/        # CORS, rate limiting, JWT (NEW)
│   │   ├── mod.rs
│   │   ├── cors.rs       ✅ Implemented
│   │   ├── rate_limit.rs ✅ Implemented
│   │   └── jwt.rs        ⏳ TODO
│   └── rest/             # REST endpoints
├── auth/
│   ├── jwt/              ⏳ TODO: Implement JWT service
│   │   ├── mod.rs
│   │   ├── config.rs
│   │   ├── service.rs
│   │   ├── claims.rs
│   │   └── blacklist.rs
│   ├── api_key.rs
│   ├── oauth.rs
│   └── rbac/
├── domain/
│   ├── entities/
│   ├── repositories/      # Traits defined
│   └── value_objects/
├── infrastructure/
│   ├── database/
│   │   ├── postgres.rs           # User/ApiKey repos ✅
│   │   ├── postgres_event_repo.rs ⏳ TODO
│   │   ├── postgres_receiver_repo.rs ⏳ TODO
│   │   └── postgres_receiver_group_repo.rs ⏳ TODO
│   ├── messaging/
│   │   └── kafka.rs
│   └── telemetry/        ⏳ TODO: Implement observability
│       ├── metrics.rs
│       ├── tracing.rs
│       └── logging.rs
└── application/
    └── handlers/
```

### Test Structure

```text
tests/
├── integration/          # Integration tests
├── load/                # Load testing ⏳ TODO
│   ├── scenarios/
│   │   ├── baseline.js
│   │   ├── stress.js
│   │   ├── spike.js
│   │   └── soak.js
│   ├── data/
│   └── results/
└── security/            # Security tests
```

## Implementation Roadmap

### Phase 1: Database Persistence (Priority: HIGH)

**Estimated Time**: 2 weeks

**Goal**: Replace all in-memory repositories with PostgreSQL implementations

**Components**:
- `src/infrastructure/database/postgres_event_repo.rs` ⏳
- `src/infrastructure/database/postgres_receiver_repo.rs` ⏳
- `src/infrastructure/database/postgres_receiver_group_repo.rs` ⏳

**Success Criteria**:
- All events persist to database
- Data survives server restarts
- All repository tests pass
- Migration scripts complete

**Documentation**:
- See: `docs/how_to/implement_postgres_repositories.md`

**Key Tasks**:
1. Review existing migrations in `migrations/`
2. Implement `postgres_event_repo.rs` (structure exists)
3. Implement receiver repositories
4. Add integration tests with testcontainers
5. Update dependency injection in `main.rs`
6. Remove mock implementations

### Phase 2: JWT Authentication (Priority: HIGH)

**Estimated Time**: 2 weeks

**Goal**: Implement secure JWT authentication with proper key management

**Components**:
- `src/auth/jwt/` directory (create)
- `src/api/middleware/jwt.rs` (create)
- RSA key pair generation
- Token refresh endpoints

**Success Criteria**:
- JWT tokens use RS256 signing
- Access tokens expire in 15 minutes
- Refresh token flow works
- Token blacklist implemented
- All tests pass

**Documentation**:
- See: `docs/how_to/implement_jwt_authentication.md`

**Key Tasks**:
1. Generate RSA key pair (store securely)
2. Create JWT configuration module
3. Implement token generation/validation
4. Create JWT middleware
5. Add refresh token endpoint
6. Implement token blacklist
7. Update login/logout flows
8. Add comprehensive tests

### Phase 3: Security Hardening (Priority: HIGH)

**Estimated Time**: 1 week

**Goal**: Implement production-grade security controls

**Components**:
- CORS configuration ✅ (implemented)
- Rate limiting ✅ (implemented)
- GraphQL authentication guards ⏳
- Input validation ⏳
- Security headers ⏳

**Success Criteria**:
- CORS uses specific origins only
- Rate limiting active on all endpoints
- GraphQL requires authentication
- All inputs validated
- Security headers present

**Key Tasks**:
1. Configure production CORS (no wildcards)
2. Enable rate limiting middleware
3. Add authentication to GraphQL schema
4. Implement role-based GraphQL guards
5. Add input validation to all DTOs
6. Add security headers middleware
7. Test with security scanner

### Phase 4: Observability (Priority: MEDIUM)

**Estimated Time**: 1 week

**Goal**: Implement comprehensive monitoring and tracing

**Components**:
- `src/infrastructure/telemetry/metrics.rs` ⏳
- `src/infrastructure/telemetry/tracing.rs` ⏳
- `src/infrastructure/telemetry/logging.rs` ⏳
- Prometheus exporter
- Jaeger integration

**Success Criteria**:
- Metrics exported at `/metrics`
- Traces visible in Jaeger UI
- JSON structured logging in production
- Health checks include all components

**Key Tasks**:
1. Add Prometheus metrics crate
2. Instrument key operations
3. Set up Jaeger tracing
4. Add trace context propagation
5. Configure structured logging
6. Enhance health check endpoint
7. Create Grafana dashboards

### Phase 5: Load Testing (Priority: MEDIUM)

**Estimated Time**: 1 week

**Goal**: Establish performance baselines and validate capacity

**Components**:
- `tests/load/scenarios/*.js` ⏳
- CI/CD integration ⏳
- Performance dashboards ⏳

**Success Criteria**:
- All test scenarios pass
- Performance meets SLA targets
- Load tests run in CI/CD
- Results tracked over time

**Key Tasks**:
1. Install k6
2. Create test scenarios (baseline, stress, spike, soak)
3. Define performance targets
4. Run baseline tests
5. Add to CI/CD pipeline
6. Document results
7. Set up alerts for regressions

## Quick Start

### For PostgreSQL Repositories

```bash
# 1. Review the implementation guide
cat docs/how_to/implement_postgres_repositories.md

# 2. Check existing migrations
ls -la migrations/

# 3. Implement the event repository
vi src/infrastructure/database/postgres_event_repo.rs

# 4. Run tests
cargo test postgres_event_repo

# 5. Update main.rs dependency injection
vi src/main.rs
```

### For JWT Authentication

```bash
# 1. Read the implementation guide
cat docs/how_to/implement_jwt_authentication.md

# 2. Generate RSA keys
mkdir -p secrets
openssl genrsa -out secrets/jwt-private.pem 4096
openssl rsa -in secrets/jwt-private.pem -pubout -out secrets/jwt-public.pem

# 3. Create JWT module structure
mkdir -p src/auth/jwt
touch src/auth/jwt/{mod.rs,config.rs,service.rs,claims.rs,blacklist.rs}

# 4. Implement JWT service
vi src/auth/jwt/service.rs

# 5. Add middleware
vi src/api/middleware/jwt.rs
```

### For Load Testing

```bash
# 1. Install k6
brew install k6  # macOS
# or use Docker

# 2. Create test structure
mkdir -p tests/load/{scenarios,data,results}

# 3. Copy baseline test
cp docs/how_to/setup_load_testing.md tests/load/README.md

# 4. Create your first test
vi tests/load/scenarios/baseline.js

# 5. Run the test
export BASE_URL="http://localhost:8042"
export API_KEY="your-api-key"
k6 run tests/load/scenarios/baseline.js
```

## Configuration

### Environment Variables

All configuration uses the `XZEPR__` prefix with `__` (double underscore) separators:

```bash
# Database
export XZEPR__DATABASE__URL="postgresql://user:pass@localhost:5432/xzepr"
export XZEPR__DATABASE__MAX_CONNECTIONS=20

# JWT
export XZEPR__JWT__PRIVATE_KEY_PATH="secrets/jwt-private.pem"
export XZEPR__JWT__PUBLIC_KEY_PATH="secrets/jwt-public.pem"
export XZEPR__JWT__ACCESS_TOKEN_EXPIRATION=900
export XZEPR__JWT__REFRESH_TOKEN_EXPIRATION=604800

# Security
export XZEPR__SECURITY__CORS__ALLOWED_ORIGINS="https://app.example.com,https://admin.example.com"
export XZEPR__SECURITY__RATE_LIMIT__ANONYMOUS_RPM=10
export XZEPR__SECURITY__RATE_LIMIT__AUTHENTICATED_RPM=100

# Observability
export XZEPR__TELEMETRY__JAEGER_ENDPOINT="http://localhost:14268/api/traces"
export XZEPR__TELEMETRY__LOG_LEVEL="info"
export XZEPR__TELEMETRY__JSON_LOGS=true
```

### Configuration Files

```yaml
# config/production.yaml
server:
  host: "0.0.0.0"
  port: 8042
  enable_https: true

database:
  max_connections: 20
  min_connections: 5
  connection_timeout_seconds: 30

jwt:
  issuer: "xzepr.io"
  audience: "xzepr-api"
  access_token_expiration: 900
  refresh_token_expiration: 604800

security:
  cors:
    allow_credentials: true
    max_age_seconds: 3600
  rate_limit:
    anonymous_rpm: 10
    authenticated_rpm: 100
    admin_rpm: 1000

telemetry:
  metrics_enabled: true
  tracing_enabled: true
  log_level: "info"
  json_logs: true
```

## Testing Strategy

### Unit Tests

```bash
# Run all unit tests
cargo test

# Run specific module tests
cargo test postgres_event_repo
cargo test jwt_service
cargo test rate_limit
```

### Integration Tests

```bash
# Run integration tests (requires Docker)
cargo test --test integration

# Run with testcontainers
cargo test postgres_integration
```

### Load Tests

```bash
# Run baseline test
k6 run tests/load/scenarios/baseline.js

# Run stress test
k6 run tests/load/scenarios/stress.js

# Run all tests
./tests/load/run-all.sh
```

### Security Tests

```bash
# Run security scanner
cargo audit

# Check for vulnerabilities
cargo deny check advisories

# Run OWASP ZAP
docker run -t owasp/zap2docker-stable zap-baseline.py -t http://localhost:8042
```

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| P50 Response Time | < 100ms | < 200ms |
| P95 Response Time | < 500ms | < 1000ms |
| P99 Response Time | < 1000ms | < 2000ms |
| Error Rate | < 0.1% | < 1% |
| Throughput | > 1000 req/s | > 500 req/s |
| CPU Usage | < 70% | < 90% |
| Memory Usage | < 80% | < 95% |

## Production Checklist

Before deploying to production, verify:

- [ ] All PostgreSQL repositories implemented and tested
- [ ] JWT using RS256 with secure key management
- [ ] CORS configured with specific origins (no wildcards)
- [ ] Rate limiting enabled on all endpoints
- [ ] GraphQL requires authentication
- [ ] Input validation on all DTOs
- [ ] Metrics exported at `/metrics`
- [ ] Tracing configured and tested
- [ ] Structured logging enabled
- [ ] Health checks comprehensive
- [ ] Load tests passing performance targets
- [ ] Database backups configured
- [ ] TLS certificates valid and auto-renewing
- [ ] Secrets in secure storage (not env files)
- [ ] Database indexes created
- [ ] Documentation updated
- [ ] Runbooks created
- [ ] Monitoring alerts configured
- [ ] Incident response plan defined

## Troubleshooting

### Common Issues

**Database Connection Errors**
- Check `XZEPR__DATABASE__URL` format
- Verify PostgreSQL is running
- Check connection pool configuration
- Review database logs

**Authentication Failures**
- Verify JWT keys are readable
- Check token expiration
- Review blacklist entries
- Check CORS configuration

**Performance Issues**
- Run load tests to identify bottlenecks
- Check database query performance
- Review connection pool usage
- Monitor resource utilization

**Rate Limiting Issues**
- Check rate limit configuration
- Verify user tier detection
- Review rate limit headers
- Check for IP address extraction

## Support and Resources

### Documentation

- Production Roadmap: `docs/explanations/production_readiness_roadmap.md`
- PostgreSQL Implementation: `docs/how_to/implement_postgres_repositories.md`
- JWT Authentication: `docs/how_to/implement_jwt_authentication.md`
- Load Testing: `docs/how_to/setup_load_testing.md`

### External Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Tokio Documentation](https://tokio.rs/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [Axum Documentation](https://docs.rs/axum/)
- [k6 Documentation](https://k6.io/docs/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [OpenTelemetry Rust](https://opentelemetry.io/docs/instrumentation/rust/)

### Getting Help

1. Review relevant documentation in `docs/`
2. Check existing code examples in `src/`
3. Run diagnostics: `cargo clippy -- -D warnings`
4. Check logs: `docker-compose logs -f xzepr`
5. Review test failures: `cargo test --verbose`

## Next Steps

1. **Choose a starting phase** - Begin with Phase 1 (PostgreSQL repositories)
2. **Read the detailed guide** - Review `docs/how_to/` for your chosen phase
3. **Set up your environment** - Ensure all prerequisites are met
4. **Implement incrementally** - Complete one component at a time
5. **Test thoroughly** - Run tests after each implementation
6. **Document as you go** - Update docs with any findings
7. **Review and iterate** - Get feedback and refine

## Contributing

When implementing these features:

1. Follow the coding guidelines in `AGENTS.md`
2. Write comprehensive tests
3. Document all public APIs
4. Run `cargo fmt` and `cargo clippy`
5. Update relevant documentation
6. Create pull requests with clear descriptions
7. Use conventional commit messages

## License

This project is licensed under the terms specified in the LICENSE file.

## Conclusion

This guide provides the structure and roadmap for implementing all remaining production features. Each phase builds on the previous ones, creating a robust, secure, and observable production-ready system.

Start with Phase 1 (PostgreSQL repositories), as it provides the foundation for persistent storage. Then proceed through the phases in order, testing thoroughly at each step.

For detailed implementation instructions, refer to the individual guides in the `docs/how_to/` directory. Good luck!
