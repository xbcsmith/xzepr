# XZEPR Docker Deployment Guide

This guide covers building, deploying, and managing the XZEPR event tracking server using Docker containers with Red Hat UBI 9 base images. The system uses Redpanda for event streaming and PostgreSQL for data persistence.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Building the Image](#building-the-image)
- [Configuration](#configuration)
- [Production Deployment](#production-deployment)
- [Development Setup](#development-setup)
- [Redpanda Configuration](#redpanda-configuration)
- [Monitoring and Health Checks](#monitoring-and-health-checks)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)

## Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+
- At least 4GB RAM available for containers
- 10GB free disk space for images and volumes

## Quick Start

1. **Clone and prepare the project:**

   ```bash
   git clone <repository-url>
   cd xzepr
   cp .env.example .env
   ```

2. **Generate TLS certificates:**

   ```bash
   mkdir -p certs
   openssl req -x509 -newkey rsa:4096 -keyout certs/key.pem -out certs/cert.pem -days 365 -nodes
   ```

3. **Build and start services:**

   ```bash
   docker-compose up -d
   ```

4. **Verify deployment:**

   ```bash
   curl -k https://localhost:8443/health

   # Check Redpanda Console (optional)
   open http://localhost:8081
   ```

## Building the Image

### Using the Build Script

The recommended way to build the Docker image:

```bash
# Basic build
./scripts/build-docker.sh

# Build with custom tag
./scripts/build-docker.sh -t v1.0.0

# Build and push to registry
./scripts/build-docker.sh -r registry.example.com -p -t v1.0.0

# Multi-platform build
./scripts/build-docker.sh --platform linux/amd64,linux/arm64 -t latest
```

### Manual Build

```bash
# Build the image
docker build -t xzepr:latest .

# Build with custom build args
docker build --build-arg RUST_VERSION=1.75 -t xzepr:dev .
```

### Build Options

The Dockerfile supports several build arguments:

- `RUST_VERSION`: Rust toolchain version (default: stable)
- `BUILD_MODE`: Build mode (release/debug, default: release)
- `FEATURES`: Cargo features to enable

Example with build args:

```bash
docker build \
  --build-arg RUST_VERSION=1.75 \
  --build-arg BUILD_MODE=release \
  -t xzepr:latest .
```

## Configuration

### Environment Variables

Copy `.env.example` to `.env` and customize:

```bash
cp .env.example .env
```

Key configuration sections:

#### Server Configuration

```env
XZEPR__SERVER__HOST=0.0.0.0
XZEPR__SERVER__PORT=8443
XZEPR__SERVER__ENABLE_HTTPS=true
```

#### Database Configuration

```env
XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr
POSTGRES_PASSWORD=your-secure-password
```

#### Redpanda Configuration

```env
XZEPR__KAFKA__BROKERS=redpanda-0:9092
REDPANDA_VERSION=latest
REDPANDA_CONSOLE_VERSION=latest
```

#### Security Configuration

```env
XZEPR__AUTH__JWT_SECRET=your-super-secure-jwt-secret-256-bits-minimum
XZEPR__CORS__ALLOWED_ORIGINS=https://xzepr.example.com
```

### TLS Certificates

#### Self-Signed Certificates (Development)

```bash
mkdir -p certs
openssl req -x509 -newkey rsa:4096 \
  -keyout certs/key.pem \
  -out certs/cert.pem \
  -days 365 -nodes \
  -subj "/CN=localhost"
```

#### Production Certificates

Mount your production certificates:

```yaml
volumes:
  - /path/to/production/certs:/app/certs:ro
```

## Production Deployment

### Using Docker Compose

1. **Prepare production environment:**

   ```bash
   cp .env.example .env.prod
   # Edit .env.prod with production values
   ```

2. **Deploy with production compose:**

   ```bash
   docker-compose -f docker-compose.prod.yaml --env-file .env.prod up -d
   ```

3. **Scale services if needed:**
   ```bash
   docker-compose -f docker-compose.prod.yaml up -d --scale xzepr=3
   ```

### Kubernetes Deployment

Example Kubernetes manifests:

#### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: xzepr-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: xzepr-server
  template:
    metadata:
      labels:
        app: xzepr-server
    spec:
      containers:
        - name: xzepr
          image: xzepr:latest
          ports:
            - containerPort: 8443
          env:
            - name: XZEPR__DATABASE__URL
              valueFrom:
                secretKeyRef:
                  name: xzepr-secrets
                  key: database-url
          volumeMounts:
            - name: tls-certs
              mountPath: /app/certs
              readOnly: true
          livenessProbe:
            httpGet:
              path: /health
              port: 8443
              scheme: HTTPS
            initialDelaySeconds: 30
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /ready
              port: 8443
              scheme: HTTPS
            initialDelaySeconds: 5
            periodSeconds: 10
      volumes:
        - name: tls-certs
          secret:
            secretName: xzepr-tls
```

#### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: xzepr-service
spec:
  selector:
    app: xzepr-server
  ports:
    - port: 443
      targetPort: 8443
      protocol: TCP
  type: LoadBalancer
```

### Docker Swarm

```bash
# Initialize swarm
docker swarm init

# Deploy stack
docker stack deploy -c docker-compose.prod.yaml xzepr
```

## Development Setup

### Development Compose

```bash
# Start development environment
docker-compose -f docker-compose.yaml -f docker-compose.dev.yaml up -d

# View logs
docker-compose logs -f xzepr

# Access container shell
docker-compose exec xzepr bash
```

### Volume Mounts for Development

```yaml
volumes:
  - ./src:/app/src:ro
  - ./config:/app/config:ro
  - ./target:/app/target # Cache build artifacts
```

### Hot Reload Setup

For development with hot reload:

```dockerfile
# Development Dockerfile
FROM registry.redhat.io/ubi9/ubi:9.6

# Install cargo-watch for hot reload
RUN cargo install cargo-watch

# Use cargo-watch for development
CMD ["cargo", "watch", "-x", "run"]
```

## Monitoring and Health Checks

### Health Check Endpoints

The container exposes several health check endpoints:

- `/health` - Overall application health
- `/health/db` - Database connectivity
- `/health/messaging` - Kafka/Redpanda connectivity
- `/ready` - Readiness probe for Kubernetes

### Custom Health Checks

Use the provided health check script:

```bash
# Basic health check
./scripts/health-check.sh

# With custom endpoint
./scripts/health-check.sh -e https://xzepr.example.com/health

# Simple check
./scripts/health-check.sh --simple
```

### Monitoring Stack

Enable monitoring with Docker Compose profiles:

```bash
# Start with monitoring
docker-compose --profile monitoring up -d

# Access Grafana
open http://localhost:3000
```

### Log Management

#### Centralized Logging

```yaml
logging:
  driver: "json-file"
  options:
    max-size: "10m"
    max-file: "3"
```

### Log Aggregation with ELK

```yaml
logging:
  driver: "gelf"
  options:
    gelf-address: "udp://logstash:12201"
    tag: "xzepr-server"
```

## Redpanda Configuration

XZEPR uses Redpanda as its event streaming platform. Redpanda is a Kafka-compatible streaming platform that's simpler to deploy and manage.

### Redpanda Topics

Events are automatically published to Redpanda topics based on the event name:

```bash
# List topics using rpk
docker exec xzepr-redpanda rpk topic list

# Create a custom topic
docker exec xzepr-redpanda rpk topic create xzepr.events.custom

# View topic configuration
docker exec xzepr-redpanda rpk topic describe xzepr.events.deployment
```

### Redpanda Console

Access the Redpanda Console for monitoring and management:

```bash
# Console is available at http://localhost:8081
# View topics, consumers, and message throughput
open http://localhost:8081
```

### Event Stream Monitoring

```bash
# Monitor event production
docker exec xzepr-redpanda rpk topic consume xzepr.events.deployment --print-headers

# Check consumer group status
docker exec xzepr-redpanda rpk group list

# View consumer lag
docker exec xzepr-redpanda rpk group describe xzepr-consumer-group
```

### Redpanda Performance Tuning

For production deployments, consider these Redpanda optimizations:

```yaml
# docker-compose.prod.yaml
redpanda-0:
  command:
    - redpanda
    - start
    - --smp 4 # Use 4 CPU cores
    - --memory 2G # Allocate 2GB memory
    - --reserve-memory 500M # Reserve 500MB for OS
    - --default-log-level=warn # Reduce log noise
```

## Troubleshooting

### Common Issues

#### Container Won't Start

```bash
# Check container logs
docker logs xzepr-server

# Check container status
docker ps -a

# Inspect container configuration
docker inspect xzepr-server
```

#### Database Connection Issues

```bash
# Test database connectivity
docker-compose exec postgres psql -U xzepr -d xzepr -c "SELECT 1;"

# Check database logs
docker-compose logs postgres
```

#### Redpanda Connection Issues

```bash
# Check Redpanda status
docker exec xzepr-redpanda rpk cluster info

# Test topic creation
docker exec xzepr-redpanda rpk topic create test-topic

# Check Redpanda logs
docker-compose logs redpanda-0

# Test connectivity from app container
docker exec xzepr-server nc -zv redpanda-0 9092
```

#### Certificate Issues

```bash
# Verify certificate validity
openssl x509 -in certs/cert.pem -text -noout

# Test TLS connection
openssl s_client -connect localhost:8443 -servername localhost
```

### Performance Issues

#### Memory Usage

```bash
# Monitor container resources
docker stats xzepr-server

# Check memory limits
docker exec xzepr-server free -h
```

#### CPU Usage

```bash
# Profile application
docker exec xzepr-server top
```

### Debugging

#### Enable Debug Logging

```env
RUST_LOG=debug,xzepr=trace
```

#### Access Container Shell

```bash
# Interactive shell
docker exec -it xzepr-server bash

# Run admin commands
docker exec xzepr-server ./admin list-users
```

## Security Considerations

### Container Security

1. **Non-root User**: The container runs as user `xzepr` (UID 1001)
2. **Read-only Filesystem**: Mount application as read-only where possible
3. **Security Scanning**: Use `docker scan` to check for vulnerabilities

### Network Security

```yaml
networks:
  xzepr_network:
    driver: bridge
    internal: true # Isolate from external networks
```

### Secrets Management

```yaml
# Use Docker secrets
secrets:
  jwt_secret:
    external: true
  db_password:
    external: true

services:
  xzepr:
    secrets:
      - jwt_secret
      - db_password
```

### Resource Limits

```yaml
deploy:
  resources:
    limits:
      cpus: "2.0"
      memory: 1G
    reservations:
      cpus: "0.5"
      memory: 512M
```

## Image Registry

### Building for Registry

```bash
# Build and tag for registry
docker build -t registry.example.com/xzepr:v1.0.0 .

# Push to registry
docker push registry.example.com/xzepr:v1.0.0
```

### Multi-architecture Builds

```bash
# Create and use buildx builder
docker buildx create --name multiarch --use

# Build for multiple platforms
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t registry.example.com/xzepr:v1.0.0 \
  --push .
```

## Backup and Recovery

### Database Backups

```bash
# Backup database
docker exec postgres pg_dump -U xzepr xzepr > backup.sql

# Restore database
docker exec -i postgres psql -U xzepr xzepr < backup.sql
```

### Volume Backups

```bash
# Backup volumes
docker run --rm -v xzepr_postgres_data:/data -v $(pwd):/backup \
  alpine tar czf /backup/postgres_backup.tar.gz -C /data .
```

## Performance Tuning

### JVM Options (if using GraalVM)

```dockerfile
ENV JAVA_OPTS="-Xmx1g -Xms512m -XX:+UseG1GC"
```

### Database Connection Tuning

```env
XZEPR__DATABASE__MAX_CONNECTIONS=20
XZEPR__DATABASE__IDLE_TIMEOUT=600
```

### Redpanda Producer Tuning

```env
XZEPR__KAFKA__PRODUCER__BATCH_SIZE=32768
XZEPR__KAFKA__PRODUCER__LINGER_MS=10
XZEPR__KAFKA__PRODUCER__COMPRESSION_TYPE=snappy
```

### Redpanda Cluster Scaling

For high-throughput scenarios, scale Redpanda horizontally:

```yaml
# docker-compose.scale.yaml
version: "3.8"
services:
  redpanda-1:
    image: docker.redpanda.com/redpandadata/redpanda:latest
    command:
      - redpanda
      - start
      - --seeds redpanda-0:33145
      - --kafka-addr internal://0.0.0.0:9092,external://0.0.0.0:19093
      - --advertise-kafka-addr internal://redpanda-1:9092,external://localhost:19093
      - --rpc-addr redpanda-1:33145
      - --advertise-rpc-addr redpanda-1:33145
    volumes:
      - redpanda_data_1:/var/lib/redpanda/data
    ports:
      - "19093:19092"
```

---

For more information, see:

- [Main README](../README.md)
- [Configuration Guide](CONFIGURATION.md)
- [API Documentation](API.md)
