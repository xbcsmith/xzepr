# XZEPR Documentation

Welcome to the XZEPR event tracking server documentation. This directory contains comprehensive guides covering all aspects of building, deploying, and using XZEPR.

## Overview

XZEPR is a high-performance event tracking server built in Rust, featuring real-time event streaming with Redpanda, comprehensive authentication, role-based access control, and observability. These documents will help you get started quickly and provide detailed reference information for advanced usage.

## Documentation Structure

The documentation follows the Diataxis framework, organizing information by user needs and context:

| Document                                                 | Description                                                                                                 | Audience                      | Type         |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ----------------------------- | ------------ |
| [quickstart.md](quickstart.md)                           | Get XZEPR running in minutes with Docker Compose. Includes setup, configuration, and first steps.           | New users, DevOps             | Tutorial     |
| [api.md](api.md)                                         | Comprehensive REST API reference with authentication, event management, and administrative endpoints.       | Developers, Integrators       | Reference    |
| [docker.md](docker.md)                                   | Complete Docker deployment guide covering development and production scenarios with Red Hat UBI containers. | DevOps, Platform Engineers    | How-to Guide |
| [makefile.md](makefile.md)                               | Documentation for the comprehensive Makefile with 50+ commands for development, testing, and deployment.    | Developers, DevOps            | Reference    |
| [xzepr-architecture-plan.md](xzepr-architecture-plan.md) | Detailed architecture plan and implementation roadmap for the XZEPR system design.                          | Architects, Senior Developers | Explanation  |

## Quick Navigation

### Getting Started

- **New to XZEPR?** Start with [quickstart.md](quickstart.md)
- **Want to try the API?** See [api.md](api.md) for examples
- **Need to deploy?** Check [docker.md](docker.md)

### Development

- **Building the project?** Use [makefile.md](makefile.md) for all commands
- **Understanding the architecture?** Read [xzepr-architecture-plan.md](xzepr-architecture-plan.md)

### Operations

- **Production deployment?** See [docker.md](docker.md) production sections
- **API integration?** Reference [api.md](api.md) for endpoints
- **Troubleshooting?** All guides include troubleshooting sections

## Key Features Covered

### Event Streaming

- Redpanda integration for high-performance messaging
- Real-time event processing and consumption
- Topic management and consumer groups
- Event serialization and schema handling

### Authentication & Authorization

- Local username/password authentication
- OpenID Connect (OIDC) integration with Keycloak
- API key authentication for service-to-service
- Role-based access control (RBAC) with fine-grained permissions

### Deployment & Operations

- Docker containerization with Red Hat UBI base images
- Docker Compose for development and production
- Kubernetes deployment manifests
- Comprehensive monitoring and health checks
- Automated backup and recovery procedures

### Development Workflow

- Comprehensive Makefile with 50+ automation commands
- Hot reload development environment
- Code quality assurance (formatting, linting, testing)
- CI/CD integration with GitHub Actions
- Database migrations and management

## Documentation Standards

All documentation in this directory follows these standards:

- **Markdown format** with consistent structure
- **Code examples** that are tested and working
- **Step-by-step instructions** with verification steps
- **Troubleshooting sections** for common issues
- **Cross-references** between related documents
- **Table of contents** for longer documents

## Contributing to Documentation

When updating documentation:

1. Follow the existing structure and style
2. Test all code examples and commands
3. Update cross-references when adding new sections
4. Include troubleshooting information for new procedures
5. Follow the AGENTS.md guidelines for markdown standards

## Additional Resources

### External Documentation

- [Rust Documentation](https://doc.rust-lang.org/) - Rust language reference
- [Redpanda Documentation](https://docs.redpanda.com/) - Event streaming platform
- [Docker Documentation](https://docs.docker.com/) - Container platform
- [PostgreSQL Documentation](https://www.postgresql.org/docs/) - Database system

### Project Resources

- **Main README.md** - Project overview and quick setup
- **AGENTS.md** - Guidelines for AI agents and contributors
- **Cargo.toml** - Rust project dependencies and metadata
- **Makefile** - Development and deployment automation

## Support

For questions about the documentation:

1. Check the relevant guide's troubleshooting section
2. Review the [quickstart.md](quickstart.md) common issues
3. Verify your setup matches the prerequisites
4. Check the project's main README.md for general information

Each document includes specific troubleshooting and help sections tailored to its topic area.
