# XZEPR Quick Start Guide

Get XZEPR event tracking server up and running in minutes with Docker Compose.
This guide uses Redpanda for high-performance event streaming and PostgreSQL for
reliable data persistence. The project includes comprehensive tooling with a
feature-rich Makefile, production-ready Docker containers, and extensive
documentation.

## Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+ (or `docker compose` command)
- Git
- At least 4GB RAM available
- OpenSSL (for certificate generation)
- Make (optional, for using Makefile commands)

## 🚀 Quick Start

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

# Option 2: Use the comprehensive Makefile (recommended)
make deploy-dev

# Option 3: For production deployment
make deploy-prod

# Check all service status
docker compose ps

# View startup logs
docker compose logs -f
```

### 4. Verify Installation and Admin User

The Docker setup automatically creates the database and an admin user during startup.

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

# Verify admin user exists (optional)
docker compose -f docker-compose.prod.yaml exec -T postgres psql -U xzepr -d xzepr -c "SELECT username, email FROM users;"
```

### 5. Access Web Interfaces

```bash
# Access web interfaces
open https://localhost:8443  # XZEPR API (accept certificate warning)
open http://localhost:8081   # Redpanda Console

# Check all services status
docker compose -f docker-compose.prod.yaml ps
```

## 🎯 First Steps

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
curl -X POST https://localhost:8443/api/v1/event-receivers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Receiver",
    "description": "My first event receiver"
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
open http://localhost:8081

# Or consume events directly with rpk
docker exec xzepr-redpanda rpk topic consume xzepr.events.hello-world

# List all events via API
curl -X GET "https://localhost:8443/api/v1/events" \
  -H "Authorization: Bearer $TOKEN" -k
```

## 🔧 Development Setup

### Prerequisites for Local Development

```bash
# Install Rust toolchain (if developing locally)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Setup development environment
make setup-dev
```

### Hot Reload Development

```bash
# Option 1: Container-based development (recommended)
make dev-watch

# Option 2: Local development with external services
docker compose up -d postgres redpanda-0 console
RUST_LOG=debug cargo run

# Option 3: Full local development
make dev
```

### Using the Comprehensive Makefile

The project includes an extensive Makefile with 50+ commands for all aspects of
development:

```bash
# Show all available commands with descriptions
make help

# Development workflow
make setup-dev     # Install tools and setup environment
make build         # Build the project
make test          # Run all tests
make fmt           # Format code
make clippy        # Run Rust linter
make check         # Run all quality checks
make pre-commit    # Complete pre-commit workflow

# Docker operations
make docker-build  # Build container image
make docker-run    # Run container
make docker-logs   # View container logs

# Database management
make db-setup      # Initialize database with migrations
make db-migrate    # Run migrations
make db-reset      # Reset database completely
make db-status     # Check database connection

# Deployment
make deploy-dev    # Start development stack
make deploy-prod   # Start production stack
make deploy-logs   # View deployment logs
make deploy-health # Check all service health

# Utility commands
make info          # Show project information
make certs-generate # Generate TLS certificates
make clean         # Clean build artifacts
```

## 📊 Monitoring and Management

### Service URLs

- **XZEPR API**: https://localhost:8443
  - Health endpoint: https://localhost:8443/health
  - API documentation: https://localhost:8443/api/v1
- **Redpanda Console**: http://localhost:8081
  - Topic management and message browsing
- **PostgreSQL**: localhost:5432
  - Database: `xzepr`
  - Username: `xzepr`
- **Optional Monitoring** (with `--profile monitoring`):
  - Grafana: http://localhost:3000
  - Prometheus: http://localhost:9090

### Essential Management Commands

```bash
# Comprehensive service management
make deploy-logs        # View all service logs
make deploy-health      # Check health of all services
make deploy-stop        # Stop all services gracefully

# Individual service management
docker compose logs -f xzepr           # XZEPR application logs
docker compose logs -f redpanda-0      # Redpanda logs
docker compose logs -f postgres        # PostgreSQL logs

# Redpanda operations
docker exec xzepr-redpanda rpk cluster info          # Cluster status
docker exec xzepr-redpanda rpk topic list            # List topics
docker exec xzepr-redpanda rpk topic consume <topic> # Consume messages

# Database operations
make db-status                                        # Connection test
docker compose exec postgres psql -U xzepr -d xzepr  # Direct SQL access

# Container management
docker compose restart <service-name>  # Restart specific service
docker compose down --volumes         # Stop and remove volumes
docker system prune -f               # Clean up Docker resources
```

## 🔐 Security Notes

### Development Security

- Default admin password is `admin123` - **change this for production**
- Self-signed certificates are generated for development only
- Default JWT secret should be replaced in production
- Admin user is automatically created during startup with username `admin`

### Production Security

- Use proper TLS certificates from a trusted CA or Let's Encrypt
- Implement secrets management (Docker secrets, Kubernetes secrets, etc.)
- Change all default passwords and use strong authentication
- Review the comprehensive security section in [docker.md](docker.md)
- Enable additional security features like rate limiting and audit logging

## 🚨 Troubleshooting

### Quick Diagnostics

```bash
# Check overall deployment health
make deploy-health

# View all service logs
make deploy-logs

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
make certs-generate

# Verify certificate creation
ls -la certs/
make certs-verify
```

**Database connection failed:**

```bash
# Check PostgreSQL health
make db-status
docker compose exec postgres pg_isready -U xzepr

# Reset database completely
make db-reset

# Manual connection test
docker compose exec postgres psql -U xzepr -d xzepr -c "SELECT 1;"
```

**Redpanda connection issues:**

```bash
# Check Redpanda cluster health
docker exec xzepr-redpanda rpk cluster health
docker exec xzepr-redpanda rpk cluster info

# Test connectivity from app container
docker compose exec xzepr nc -zv redpanda-0 9092

# Restart Redpanda services
docker compose restart redpanda-0
docker compose restart console
```

**Build failures:**

```bash
# Clean and rebuild
make clean
make build

# Update dependencies
make update

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

## 📚 Next Steps

### Immediate Next Steps

- Explore the [API Documentation](api.md) for comprehensive endpoint details
- Try creating events and receivers using the API examples
- View real-time events in the Redpanda Console

### Development Workflow

- Review [makefile.md](makefile.md) for the complete development toolkit
- Set up your IDE with Rust tooling and formatting
- Run the test suite: `make test`
- Try the code quality checks: `make check`

### Production Preparation

- Read [docker.md](docker.md) for production deployment strategies
- Configure proper TLS certificates and secrets management
- Set up monitoring with Prometheus/Grafana:
  `docker compose --profile monitoring up -d`
- Configure OIDC authentication with Keycloak for enterprise auth

### Advanced Features

- Explore the comprehensive Makefile with 50+ automation commands
- Set up CI/CD using the included GitHub Actions workflow
- Configure event streaming patterns and consumer groups
- Implement custom event receivers and webhooks

## 🆘 Getting Help

### Self-Service Debugging

1. **Quick health check**: `make deploy-health`
2. **View all logs**: `make deploy-logs`
3. **Service status**: `docker compose ps`
4. **System resources**: `docker system df && free -h`
5. **Review troubleshooting**: See detailed section above

### Documentation Resources

- **Complete API Guide**: [api.md](api.md)
- **Docker Deployment**: [docker.md](docker.md)
- **Development Toolkit**: [makefile.md](makefile.md)
- **Architecture Overview**: Check the main README.md

### Common Solutions

- **Port conflicts**: Change ports in docker-compose.yaml
- **Resource issues**: Ensure 4GB+ RAM available
- **Certificate problems**: `make certs-generate`
- **Database issues**: `make db-reset`
- **Build problems**: `make clean && make build`

### Additional Help

- Check the comprehensive Makefile: `make help`
- Review the project structure and documentation in `docs/`
- All configuration is environment-variable based for easy customization
