# XZEPR Documentation

Welcome to the XZEPR event tracking server documentation.

## About XZEPR

XZEPR is a high-performance event tracking server built in Rust, featuring
real-time event streaming with Redpanda, comprehensive authentication,
role-based access control, and observability. Perfect for tracking CI/CD events,
deployments, builds, and other system activities.

## Documentation Structure

This documentation follows the [Diataxis Framework](https://diataxis.fr/),
organizing content into four categories based on your needs:

### Tutorials

**Learning-oriented** - Step-by-step lessons to help you learn by doing.

- [Getting Started](tutorials/getting_started.md) - Your first steps with XZEPR
- [Building Your First Event Tracker](tutorials/first_event_tracker.md) - Create
  a simple event tracking application

Start here if you're new to XZEPR and want to learn the basics through hands-on
practice.

### How-to Guides

**Task-oriented** - Practical guides to solve specific problems.

- [Running the Server](how_to/running_server.md) - Start, stop, and manage the
  XZEPR server
- [Setting Up Authentication](how_to/authentication.md) - Configure local, OIDC,
  and API key authentication
- [Deploying with Docker](how_to/deployment.md) - Deploy XZEPR in containers
- [Database Management](how_to/database.md) - Manage migrations and backups

Use these guides when you need to accomplish a specific task or solve a
particular problem.

### Explanations

**Understanding-oriented** - Conceptual discussions to deepen your knowledge.

- [Architecture Overview](explanations/architecture.md) - System design and
  component interaction
- [Authentication and Authorization](explanations/auth_concepts.md) - How RBAC
  and multi-provider auth works
- [Event Streaming Model](explanations/event_streaming.md) - Understanding
  real-time event processing
- [Design Decisions](explanations/design_decisions.md) - Why things are built
  the way they are

Read these to understand the concepts and reasoning behind XZEPR's design.

### Reference

**Information-oriented** - Technical specifications and details.

- [API Reference](reference/api.md) - Complete REST API documentation
- [Configuration Reference](reference/configuration.md) - All configuration
  options
- [Database Schema](reference/database_schema.md) - Table structures and
  relationships
- [Makefile Commands](reference/makefile.md) - All available Make targets
- [Environment Variables](reference/environment_variables.md) - Configuration
  via environment

Consult these when you need to look up specific information or details.

## Quick Navigation

### I want to

**Learn XZEPR from scratch** → Start with
[Getting Started Tutorial](tutorials/getting_started.md)

**Set up and run the server** → Follow
[Running the Server](how_to/running_server.md)

**Integrate with the API** → Check [API Reference](reference/api.md)

**Understand how it works** → Read
[Architecture Overview](explanations/architecture.md)

**Deploy to production** → See [Deploying with Docker](how_to/deployment.md)

**Look up a configuration option** → Browse
[Configuration Reference](reference/configuration.md)

## Getting Help

Each documentation page includes:

- Prerequisites clearly listed
- Step-by-step instructions with verification
- Troubleshooting sections for common issues
- Links to related documentation

If you can't find what you need:

1. Check the relevant category's README for more documents
2. Look at the troubleshooting sections in how-to guides
3. Review the project's main [README.md](../README.md)

## Contributing to Documentation

When adding or updating documentation:

1. **Choose the right category:**

   - Tutorials: Teaching through hands-on examples
   - How-to: Solving specific problems
   - Explanations: Clarifying concepts
   - Reference: Technical specifications

2. **Follow naming conventions:**

   - Use lowercase with underscores: `getting_started.md`
   - Exception: `README.md` files use uppercase

3. **Include standard sections:**

   - Clear title and introduction
   - Prerequisites (for how-to and tutorials)
   - Code examples with explanations
   - Troubleshooting (for how-to guides)
   - Related links

4. **Follow markdown standards:**
   - Use `.markdownlint.json` rules
   - No emojis
   - Specify language for code blocks
   - Test all commands and examples

See [AGENTS.md](../AGENTS.md) for detailed documentation guidelines.

## External Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [Redpanda Documentation](https://docs.redpanda.com/)
- [Axum Web Framework](https://docs.rs/axum/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [Diataxis Framework](https://diataxis.fr/)
