# XZEPR Quick Start Guide

Get XZEPR event tracking server up and running in minutes with Docker Compose.
This guide uses Redpanda for high-performance event streaming and PostgreSQL for
reliable data persistence. The project includes production-ready Docker
containers and extensive documentation.

## Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+ (or `docker compose` command)
- Git
- At least 4GB RAM available
- OpenSSL (for certificate generation)

## Quick Start

### 1. Clone and Setup

```bash
# Clone the repository
git clone <repository-url>
cd xzepr

# Copy environment configuration (optional - defaults work for development)
cp .env.example .env

# Edit .env with your configuration if needed
# nano .env
```

### 2. Generate TLS Certificates

```bash
# Generate self-signed certificates for development
mkdir -p certs

# Generate certificate and key in one step
openssl req -x509 -newkey rsa:4096 \
  -keyout certs/key.pem \
  -out certs/cert.pem \
  -days 365 -nodes \
  -subj "/C=US/ST=State/L=City/O=XZEPR/CN=localhost"

# Verify certificates were created
ls -la certs/

# Optional: View certificate details
openssl x509 -in certs/cert.pem -text -noout | head -20
```

### 3. Build and Start All Services

```bash
# Option 1: Use Docker Compose directly
docker compose up -d --build

# Check all service status
docker compose ps

# View startup logs
docker compose logs -f
```

### 4. Verify Installation and Create Admin User

The Docker setup automatically creates the database during startup. Create the
default admin user explicitly with the bundled admin CLI before attempting
login.

```bash
# Check application health
curl -k https://localhost:8443/health

# Expected response:
# {
#   "status": "healthy",
#   "version": "0.1.0",
#   "components": {
#     "database": "healthy"
#   }
# }

# Create the default admin user
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  create-user \
  --username admin \
  --email admin@xzepr.local \
  --password admin123 \
  --role admin

# Verify admin user exists (optional)
docker compose exec -T postgres psql -U xzepr -d xzepr -c "SELECT username, email FROM users;"
```

### 5. Access Web Interfaces

```bash
# Access web interfaces in your browser
# XZEPR API: https://localhost:8443
# Redpanda Console: http://localhost:8081

# Check all services status
docker compose ps
```

## First Steps

### Create Your First Event

```bash
# Login to get authentication token
RESPONSE=$(curl -X POST https://localhost:8443/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "admin123"
  }' -k -s)

# Extract token from response (requires jq)
TOKEN=$(echo $RESPONSE | jq -r '.token')

# Or manually copy the token from this command:
echo $RESPONSE | jq '.token'

# Create an event receiver
curl -X POST https://localhost:8443/api/v1/receivers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Receiver",
    "type": "webhook",
    "version": "1.0.0",
    "description": "My first event receiver",
    "schema": {}
  }' -k

# Save the receiver ID from response
RECEIVER_ID="receiver-id-from-response"

# Create your first event
curl -X POST https://localhost:8443/api/v1/events \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-world",
    "version": "1.0.0",
    "release": "2024.12",
    "platform_id": "local-dev",
    "package": "manual",
    "description": "My first XZEPR event",
    "success": true,
    "event_receiver_id": "'$RECEIVER_ID'"
  }' -k
```

### Monitor Events in Real-time

```bash
# View events in Redpanda Console
# Open http://localhost:8081 in your browser

# Or consume events directly with rpk
docker exec redpanda-0 rpk topic consume xzepr.events.hello-world

# List all events via API
curl -X GET "https://localhost:8443/api/v1/events" \
  -H "Authorization: Bearer $TOKEN" -k
```

## Development Setup

### Prerequisites for Local Development

```bash
# Install Rust toolchain (if developing locally)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install required Rust components
rustup component add clippy rustfmt
```

### Hot Reload Development

```bash
# Start required external services
docker compose up -d postgres redpanda-0 console

# Run the application locally
RUST_LOG=debug cargo run

# Run the admin CLI locally
cargo run --bin admin -- list-users
```

## Monitoring and Management

### Service URLs

- **XZEPR API**: <https://localhost:8443>
  - Health endpoint: <https://localhost:8443/health>
  - API documentation: <https://localhost:8443/api/v1>
- **Redpanda Console**: <http://localhost:8081>
  - Topic management and message browsing
- **PostgreSQL**: localhost:5432
  - Database: `xzepr`
  - Username: `xzepr`
- **Optional Monitoring** (with `--profile monitoring`):
  - Grafana: <http://localhost:3000>
  - Prometheus: <http://localhost:9090>

### Essential Management Commands

```bash
# View service logs
docker compose logs -f xzepr
docker compose logs -f redpanda-0
docker compose logs -f postgres

# Check service health and status
curl -k https://localhost:8443/health
docker compose ps

# Redpanda operations
docker exec redpanda-0 rpk cluster info
docker exec redpanda-0 rpk topic list
docker exec redpanda-0 rpk topic consume <topic>

# Database operations
docker compose exec postgres pg_isready -U xzepr
docker compose exec postgres psql -U xzepr -d xzepr

# Container management
docker compose restart <service-name>
docker compose down --volumes
docker system prune -f
```

## Security Notes

### Development Security

- The tutorial creates a development admin user with password `admin123` -
  **change this for production**
- Self-signed certificates are generated for development only
- Default JWT secret should be replaced in production
- The admin user is created explicitly with the bundled admin CLI during setup

### Production Security

- Use proper TLS certificates from a trusted CA or Let's Encrypt
- Implement secrets management (Docker secrets, Kubernetes secrets, etc.)
- Change all default passwords and use strong authentication
- Review the comprehensive security section in
  [deployment.md](../how_to/deployment.md)
- Enable additional security features like rate limiting and audit logging

## Troubleshooting

### Quick Diagnostics

```bash
# Check overall deployment health
curl -k https://localhost:8443/health
docker compose ps

# View all service logs
docker compose logs --tail=100

# Check system resources
docker system df
docker stats
```

### Common Issues

**Container won't start:**

```bash
# Check specific container logs
docker compose logs <service-name>

# Verify Docker resources
docker system df
free -h  # Check available memory

# Restart problematic service
docker compose restart <service-name>
```

**Certificate errors:**

```bash
# Clean and regenerate certificates
rm -rf certs
mkdir -p certs
openssl req -x509 -newkey rsa:4096 \
  -keyout certs/key.pem \
  -out certs/cert.pem \
  -days 365 -nodes \
  -subj "/C=US/ST=State/L=City/O=XZEPR/CN=localhost"

# Verify certificate creation
ls -la certs/
openssl x509 -in certs/cert.pem -text -noout | head -20
```

**Database connection failed:**

```bash
# Check PostgreSQL health
docker compose exec postgres pg_isready -U xzepr

# Reset database completely
docker compose down --volumes
docker compose up -d --build

# Manual connection test
docker compose exec postgres psql -U xzepr -d xzepr -c "SELECT 1;"
```

**Redpanda connection issues:**

```bash
# Check Redpanda cluster health
docker exec redpanda-0 rpk cluster health
docker exec redpanda-0 rpk cluster info

# Test connectivity from app container
docker compose exec xzepr nc -zv redpanda-0 9092

# Restart Redpanda services
docker compose restart redpanda-0
docker compose restart console
```

**Build failures:**

```bash
# Clean and rebuild containers
docker compose down
docker compose build --no-cache
docker compose up -d

# Update Rust dependencies
cargo update

# Check Rust installation
rustc --version
cargo --version
```

**Port conflicts:**

```bash
# Check what's using the ports
lsof -i :8443  # XZEPR API
lsof -i :8081  # Redpanda Console
lsof -i :5432  # PostgreSQL

# Stop conflicting services or change ports in docker-compose.yaml
```

## Next Steps

### Immediate Next Steps

- Explore the [API Documentation](../reference/api.md) for comprehensive
  endpoint details
- Try creating events and receivers using the API examples
- View real-time events in the Redpanda Console

### Development Workflow

- Set up your IDE with Rust tooling and formatting
- Run the test suite: `cargo test --all-features`
- Check compilation: `cargo check --all-targets --all-features`
- Run linting: `cargo clippy --all-targets --all-features -- -D warnings`
- Format code: `cargo fmt --all`

### Production Preparation

- Read [deployment.md](../how_to/deployment.md) for production deployment
  strategies
- Configure proper TLS certificates and secrets management
- Set up monitoring with Prometheus/Grafana:
  `docker compose --profile monitoring up -d`
- Configure OIDC authentication with Keycloak for enterprise auth

### Advanced Features

- Set up CI/CD using the included GitHub Actions workflow
- Configure event streaming patterns and consumer groups
- Implement custom event receivers and webhooks
- Explore the reference documentation for API and operational details

## Getting Help

### Self-Service Debugging

1. **Quick health check**: `curl -k https://localhost:8443/health`
2. **View all logs**: `docker compose logs --tail=100`
3. **Service status**: `docker compose ps`
4. **System resources**: `docker system df && free -h`
5. **Review troubleshooting**: See detailed section above

### Documentation Resources

- **Complete API Guide**: [API Reference](../reference/api.md)
- **Docker Deployment**: [Deployment Guide](../how_to/deployment.md)
- **Architecture Overview**: Check the main [README.md](../../README.md)

### Common Solutions

- **Port conflicts**: Change ports in docker-compose.yaml
- **Resource issues**: Ensure 4GB+ RAM available
- **Certificate problems**: Regenerate the files in `certs/` with `openssl`
- **Database issues**:
  `docker compose down --volumes && docker compose up -d --build`
- **Build problems**: `docker compose build --no-cache`

### Additional Help

- Review the project structure and documentation in `docs/`
- Use `docker compose ps` and `docker compose logs` to inspect the running stack
- All configuration is environment-variable based for easy customization
