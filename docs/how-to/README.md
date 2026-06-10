# How-to Guides

Welcome to the XZEPR how-to guides section. These task-oriented guides help you
accomplish specific goals and solve real-world problems.

## What are How-to Guides?

How-to guides are practical, problem-solving instructions that assume you have
some basic knowledge and want to accomplish a specific task. They provide clear
steps to achieve a particular goal.

**Characteristics of how-to guides:**

- Task-oriented and goal-focused
- Assume basic knowledge
- Provide practical solutions
- Show you how to solve specific problems
- Include troubleshooting steps

## Available Guides

### Server Operations

**[Running the Server](running_server.md)**

Start, stop, and manage the XZEPR event tracking server.

Topics covered:

- Quick start and configuration
- Running in development vs production
- TLS/HTTPS setup
- Logging and monitoring
- Process management
- Graceful shutdown
- Troubleshooting common issues

**Level:** Intermediate

### Authentication and Security

**[JWT Authentication Setup](jwt_authentication_setup.md)**

Configure JWT-based authentication for XZEPR.

Topics covered:

- JWT configuration
- Token generation and validation
- Authentication middleware setup
- Security considerations
- Verification steps

**Level:** Intermediate

### Deployment

**[Deploying with Docker](deployment.md)**

Deploy XZEPR using Docker and Docker Compose.

Topics covered:

- Container configuration
- Production deployment
- Environment variables
- Health checks and monitoring
- Scaling considerations

**Level:** Intermediate to Advanced

### Database Management

**[Setup Monitoring](setup_monitoring.md)**

Configure monitoring and observability for XZEPR.

Topics covered:

- Prometheus setup
- Grafana integration
- Metrics verification
- Monitoring stack configuration
- Troubleshooting monitoring issues

**Level:** Intermediate

### Development Workflow

**[Use GraphQL Playground](use_graphql_playground.md)**

Access and use the GraphQL Playground for interactive API exploration.

Topics covered:

- Opening the playground
- Running queries and mutations
- Testing authentication
- Exploring the schema
- Troubleshooting common issues

**Level:** Intermediate

## Who Should Use How-to Guides?

How-to guides are ideal if you:

- Know what you want to accomplish
- Need step-by-step instructions for a specific task
- Want practical solutions to real problems
- Have basic familiarity with XZEPR

## How to Use These Guides

1. **Pick the guide** that matches your goal
2. **Check prerequisites** listed at the start
3. **Follow the steps** in order
4. **Verify each step** using provided commands
5. **Check troubleshooting** if issues arise

## Prerequisites

Most how-to guides assume you have:

- XZEPR installed or access to the source code
- Basic command line proficiency
- Docker and Docker Compose (for deployment guides)
- PostgreSQL running (for database guides)

Specific prerequisites are listed in each guide.

## If You're New to XZEPR

If you're completely new to XZEPR, we recommend:

1. Start with [Getting Started Tutorial](../tutorials/getting_started.md)
2. Then return to the how-to guides for specific tasks
3. Read [Architecture Overview](../explanations/architecture.md) to understand
   how things work

## Related Documentation

- [Tutorials](../tutorials/README.md) - Learn XZEPR from scratch
- [Explanations](../explanations/README.md) - Understand concepts
- [Reference](../reference/README.md) - Look up technical details

## Contributing

Found a problem or have suggestions? How-to guides should be:

- Focused on a single task or problem
- Include clear prerequisites
- Provide step-by-step instructions
- Include verification steps
- Have troubleshooting sections
- Test all commands and examples

See [AGENTS.md](../../AGENTS.md) for documentation standards.
