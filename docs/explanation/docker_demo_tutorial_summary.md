# Docker Demo Tutorial Summary

## Overview

This document summarizes the Docker-based demonstration tutorial for the XZepr event tracking server. The tutorial provides a complete, production-like deployment experience using Docker and Docker Compose, without relying on Makefile commands.

## Purpose and Goals

The Docker demo tutorial was created to provide:

- A self-contained, reproducible development and testing environment
- Step-by-step instructions using only Docker and shell commands
- Hands-on experience with all major XZepr features
- Integration with backend services (PostgreSQL, Redpanda, Keycloak)
- Practical examples of API usage with multiple tools

## Architecture

The tutorial implements a multi-container architecture:

```text
┌─────────────────────────────────────────────────────────┐
│  XZepr Server Container                                 │
│  - REST API endpoints                                   │
│  - GraphQL API with Playground                          │
│  - Event processing                                     │
└─────────────────────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  PostgreSQL  │ │   Redpanda   │ │   Keycloak   │
│  Database    │ │   Streaming  │ │   Auth       │
└──────────────┘ └──────────────┘ └──────────────┘
        │               │               │
        └───────────────┴───────────────┘
                        │
                ┌───────┴────────┐
                ▼                ▼
        ┌──────────────┐  ┌──────────────┐
        │   Redpanda   │  │     User     │
        │   Console    │  │   Browser    │
        └──────────────┘  └──────────────┘
```

## Key Components

### Backend Services

**PostgreSQL Database**
- Stores user accounts, roles, and permissions
- Maintains event receiver definitions
- Persists event data and metadata
- Handles transactional consistency

**Redpanda Streaming Platform**
- Kafka-compatible event streaming
- Real-time event distribution
- Message persistence and replay
- Schema registry integration

**Keycloak Identity Provider**
- OpenID Connect authentication
- Single Sign-On support
- User federation capabilities
- Optional for demo (local auth available)

**Redpanda Console**
- Web-based Kafka UI
- Topic and message inspection
- Consumer group monitoring
- Schema registry management

### XZepr Server

**REST API**
- CRUD operations for event receivers
- Event creation and querying
- Event receiver group management
- Health check endpoints

**GraphQL API**
- Flexible query language
- Interactive Playground IDE
- Schema introspection
- Type-safe operations

**Admin CLI**
- User management
- Role assignment
- API key generation
- System administration

## Tutorial Structure

The tutorial follows a logical progression through 20 steps:

### Phase 1: Environment Setup (Steps 1-3)

1. Repository cloning
2. TLS certificate generation
3. Backend services startup

This phase establishes the foundation by preparing the environment and starting dependencies.

### Phase 2: Application Deployment (Steps 4-6)

4. Docker image building
5. Database initialization
6. Admin user creation

This phase builds XZepr and prepares the database with initial data.

### Phase 3: User Management (Steps 7-9)

7. Creating test users with different roles
8. Listing all users
9. API key generation

This phase demonstrates the authentication and authorization system.

### Phase 4: Server Operations (Steps 10-11)

10. Starting XZepr server
11. Health check verification

This phase launches the main application and verifies it is operational.

### Phase 5: Interactive Tools (Steps 12-13)

12. Redpanda Console exploration
13. GraphQL Playground usage

This phase introduces the web-based management and development tools.

### Phase 6: API Testing (Steps 14-15)

14. REST API testing with curl
15. GraphQL API testing with curl

This phase demonstrates programmatic API access.

### Phase 7: Monitoring and Administration (Steps 16-18)

16. Event monitoring in Redpanda Console
17. Container log viewing
18. Advanced admin operations

This phase covers operational monitoring and management.

### Phase 8: Integration Testing (Step 19)

19. Complete workflow testing

This phase demonstrates end-to-end functionality with a test script.

### Phase 9: Cleanup (Step 20)

20. Resource cleanup and data management

This phase explains how to properly shut down and clean up resources.

## Technical Decisions

### Docker-Only Approach

**Decision:** Use only Docker and Docker Compose commands, avoiding Makefile dependencies.

**Rationale:**
- Universal compatibility across platforms
- No additional build tool requirements
- Clear, explicit command syntax
- Easy to adapt for CI/CD pipelines
- Better for learning and understanding

### Separate Backend Services

**Decision:** Run backend services via docker-compose.services.yaml, XZepr in standalone container.

**Rationale:**
- Services can persist between XZepr restarts
- Easier debugging and log inspection
- Mirrors production deployment patterns
- Allows testing multiple XZepr configurations
- Better resource management

### Development vs Production Configuration

**Decision:** Use simplified configuration for demo (HTTP instead of HTTPS, embedded secrets).

**Rationale:**
- Reduces setup complexity
- Focuses on functionality over security
- Faster iteration during learning
- Clear distinction from production setup
- Includes production recommendations in notes

### Self-Signed Certificates

**Decision:** Generate self-signed TLS certificates with openssl.

**Rationale:**
- No external CA required
- Works offline
- Standard tooling available everywhere
- Good for development and testing
- Production guide recommends proper certificates

## Learning Outcomes

After completing the tutorial, users will understand:

1. **Container Orchestration**
   - Docker networking
   - Service dependencies
   - Volume management
   - Port mapping

2. **Database Operations**
   - Schema migrations
   - Connection configuration
   - Data persistence
   - Backup considerations

3. **Event Streaming**
   - Kafka topics
   - Message production
   - Consumer groups
   - Schema registry

4. **Authentication and Authorization**
   - User management
   - Role-based access control
   - API key authentication
   - Token management

5. **API Design**
   - REST endpoint patterns
   - GraphQL queries and mutations
   - Error handling
   - API versioning

6. **Monitoring and Observability**
   - Health checks
   - Log aggregation
   - Event inspection
   - Performance metrics

## Practical Applications

The tutorial demonstrates real-world scenarios:

### CI/CD Integration

Creating event receivers for build systems:
- GitHub Actions webhooks
- Jenkins notifications
- GitLab pipeline events
- CircleCI status updates

### Deployment Tracking

Recording deployment events:
- Application versions
- Target environments
- Success/failure status
- Rollback triggers

### System Monitoring

Tracking system activities:
- Service health changes
- Resource utilization alerts
- Error rate thresholds
- Performance degradation

### Compliance and Audit

Event logging for compliance:
- User actions
- Data access patterns
- Configuration changes
- Security events

## Tool Integration Examples

### curl for Automation

The tutorial provides curl examples for:
- REST API calls
- GraphQL queries
- Authentication headers
- Request/response handling

These examples can be adapted for:
- Shell scripts
- CI/CD pipelines
- Monitoring systems
- Integration tests

### GraphQL Playground for Development

The Playground enables:
- API exploration
- Query development
- Schema documentation
- Interactive testing

Developers can use it to:
- Understand available operations
- Test query combinations
- Generate code examples
- Debug API issues

### Redpanda Console for Operations

The Console provides:
- Topic visualization
- Message inspection
- Consumer monitoring
- Performance metrics

Operations teams can:
- Verify event delivery
- Debug message formats
- Monitor lag
- Manage schemas

## Best Practices Demonstrated

### Security

- Non-root container users
- Environment variable configuration
- API key authentication
- Network isolation
- Volume mount permissions

### Reliability

- Health checks
- Graceful shutdown
- Connection retry logic
- Transaction handling
- Data persistence

### Observability

- Structured logging
- Health endpoints
- Metric collection
- Distributed tracing hooks
- Error reporting

### Maintainability

- Clear separation of concerns
- Configuration externalization
- Version tagging
- Documentation integration
- Reproducible builds

## Common Use Cases

### Development Environment

Start services for local development:
```bash
docker compose -f docker-compose.services.yaml up -d
cargo run --bin server
```

### Integration Testing

Run tests against containerized services:
```bash
docker compose -f docker-compose.services.yaml up -d
cargo test --test integration_tests
```

### Demo and Training

Quick setup for demonstrations:
```bash
./setup-demo.sh  # Based on tutorial steps
./run-demo.sh    # Automated demo script
```

### CI/CD Pipeline

Automated testing in CI:
```bash
docker compose -f docker-compose.services.yaml up -d
docker build -t xzepr:test .
docker run --network xzepr_network xzepr:test cargo test
```

## Troubleshooting Guide

The tutorial includes troubleshooting for:

- Port conflicts
- Container startup failures
- Database connection issues
- Network problems
- Authentication errors
- API key problems

Each issue includes:
- Symptoms
- Diagnosis commands
- Resolution steps
- Prevention tips

## Performance Considerations

### Resource Requirements

Minimum system requirements:
- 4GB RAM
- 2 CPU cores
- 10GB disk space
- Docker Engine 20.10+

Recommended for production:
- 8GB+ RAM
- 4+ CPU cores
- 50GB+ disk space
- Monitoring integration

### Scaling Considerations

The tutorial architecture can scale:
- Horizontal scaling with multiple XZepr instances
- Database read replicas
- Redpanda cluster expansion
- Load balancer integration

## Extension Points

The tutorial provides a foundation for:

### Custom Event Receivers

- Webhook endpoints
- Message queue consumers
- File system watchers
- Database triggers

### Additional Authentication

- OAuth2 providers
- SAML integration
- LDAP directory
- Custom authenticators

### Enhanced Monitoring

- Prometheus metrics
- Grafana dashboards
- ELK stack integration
- APM tools

### Advanced Streaming

- Event transformation
- Stream processing
- Complex event processing
- Analytics pipelines

## Documentation Standards Compliance

The tutorial follows the Diataxis framework:

**Tutorial Category**
- Learning-oriented
- Hands-on exercises
- Step-by-step instructions
- Expected outcomes at each step

**Supporting Documents**
- How-to guides for specific tasks
- Explanations for concepts
- Reference documentation for APIs

**Style Guidelines**
- Lowercase filenames (except README.md)
- No emojis
- Code blocks with language specifications
- Clear section headers
- Consistent formatting

## Future Enhancements

Planned tutorial additions:

### Advanced Scenarios

- Multi-region deployment
- High availability setup
- Disaster recovery
- Blue-green deployments

### Integration Examples

- Prometheus scraping
- Jaeger tracing
- ELK logging
- Custom dashboards

### Performance Testing

- Load generation
- Benchmark scripts
- Capacity planning
- Optimization techniques

### Security Hardening

- TLS configuration
- Network policies
- Secret management
- Audit logging

## Conclusion

The Docker demo tutorial provides a comprehensive, hands-on introduction to XZepr. By following the 20-step guide, users gain practical experience with:

- Container-based deployment
- Event streaming architecture
- GraphQL and REST APIs
- Authentication and authorization
- Operational monitoring

The tutorial serves as both a learning resource and a reference for deployment patterns. Its Docker-only approach ensures compatibility and reproducibility across different environments, making it suitable for development, testing, and demonstration purposes.

The clear separation between backend services and the application container mirrors production patterns, while the detailed troubleshooting section helps users overcome common obstacles. Combined with the supporting documentation (API references, how-to guides, and architectural explanations), the tutorial provides a complete foundation for understanding and using XZepr effectively.
