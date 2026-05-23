# Docker Commands Quick Reference

## Overview

This document provides a quick reference for Docker Compose commands used with
the current XZepr stack. Commands are organized by task for easy lookup and use
the default `docker-compose.yaml` workflow.

## Core Stack Commands

### Start All Services

```bash
docker compose up -d --build
```

### Stop All Services

```bash
docker compose down
```

### Stop and Remove Volumes

```bash
docker compose down -v
```

### View Service Status

```bash
docker compose ps
```

### View Service Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs xzepr
docker compose logs postgres
docker compose logs redpanda-0
docker compose logs keycloak
docker compose logs console
```

### Restart Services

```bash
docker compose restart
```

## XZepr Application

### Build the Application Image

```bash
docker compose build xzepr
```

### Start Only the Application Service

```bash
docker compose up -d xzepr
```

### Restart the Application Service

```bash
docker compose restart xzepr
```

### View Application Logs

```bash
docker compose logs xzepr

# Follow logs in real time
docker compose logs -f xzepr
```

### Execute Commands in the Running Application Container

```bash
docker compose exec xzepr /bin/bash
```

## Admin CLI

### Create User

```bash
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  create-user \
  --username USERNAME \
  --email EMAIL \
  --password PASSWORD \
  --role ROLE
```

### List Users

```bash
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  list-users
```

### Add Role to User

```bash
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  add-role \
  --username USERNAME \
  --role ROLE
```

### Remove Role from User

```bash
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  remove-role \
  --username USERNAME \
  --role ROLE
```

### Generate API Key

```bash
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  generate-api-key \
  --username USERNAME \
  --name "KEY_NAME" \
  --expires-days DAYS
```

### List API Keys

```bash
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  list-api-keys \
  --username USERNAME
```

### Revoke API Key

```bash
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  revoke-api-key \
  --key-id KEY_ID
```

## Database Operations

### Check PostgreSQL Readiness

```bash
docker compose exec postgres pg_isready -U xzepr
```

### Access PostgreSQL CLI

```bash
docker compose exec postgres psql -U xzepr -d xzepr
```

### Run a Simple Database Query

```bash
docker compose exec postgres psql -U xzepr -d xzepr -c "SELECT 1;"
```

## Redpanda Operations

### Check Cluster Info

```bash
docker exec redpanda-0 rpk cluster info
```

### Check Cluster Health

```bash
docker exec redpanda-0 rpk cluster health
```

### List Topics

```bash
docker exec redpanda-0 rpk topic list
```

### Consume Messages from a Topic

```bash
docker exec redpanda-0 rpk topic consume TOPIC_NAME
```

## Health and Verification

### Check Application Health

```bash
curl -k https://localhost:8443/health
```

### Check API Login

```bash
curl -k -X POST https://localhost:8443/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'
```

### Create the Default Admin User

```bash
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  create-user \
  --username admin \
  --email admin@xzepr.local \
  --password admin123 \
  --role admin
```

### Run SQL Query

```bash
docker compose exec postgres \
  psql -U xzepr -d xzepr -c "SELECT * FROM users;"
```

### Backup Database

```bash
docker compose exec postgres \
  pg_dump -U xzepr xzepr > backup.sql
```

### Restore Database

```bash
docker compose exec -T postgres \
  psql -U xzepr -d xzepr < backup.sql
```

## Network Operations

### List Networks

```bash
docker network ls
```

### Inspect Network

```bash
docker network inspect xzepr_redpanda_network
```

### Connect Container to Network

```bash
docker network connect xzepr_redpanda_network CONTAINER_NAME
```

## Volume Operations

### List Volumes

```bash
docker volume ls
```

### Inspect Volume

```bash
docker volume inspect xzepr_postgres_data
docker volume inspect xzepr_redpanda_data
```

### Remove Unused Volumes

```bash
docker volume prune
```

## Image Operations

### List Images

```bash
docker images
```

### Remove Image

```bash
docker rmi xzepr-xzepr
```

### Tag Image

```bash
docker tag xzepr-xzepr xzepr:v1.0.0
```

### Inspect Image

```bash
docker inspect xzepr-xzepr
```

## Container Management

### List Running Containers

```bash
docker ps
```

### List All Containers

```bash
docker ps -a
```

### Stop the XZepr Stack

```bash
docker compose stop
```

### Remove All Stopped Containers

```bash
docker container prune
```

### View Container Resource Usage

```bash
docker stats
```

### View Container Processes

```bash
docker compose top xzepr
```

## Troubleshooting

### Check Container Health

```bash
docker compose ps
```

### View Container Environment

```bash
docker compose exec xzepr env
```

### Copy Files from Container

```bash
docker compose cp xzepr:/app/logs ./local-logs
```

### Copy Files to Container

```bash
docker compose cp local-config.yaml xzepr:/app/config/local-config.yaml
```

### Test Network Connectivity

```bash
docker compose exec xzepr ping postgres
docker compose exec xzepr nc -zv redpanda-0 9092
```

## System Cleanup

### Clean Everything

```bash
# Stop the XZepr stack
docker compose down -v

# Remove unused containers
docker container prune -f

# Remove unused images
docker image prune -a -f

# Remove unused volumes
docker volume prune -f

# Remove unused networks
docker network prune -f
```

### Clean XZepr Specific Resources

```bash
# Stop the stack and remove volumes
docker compose down -v

# Remove the XZepr image
docker rmi xzepr-xzepr
```

## Common Patterns

### Rebuild and Restart

```bash
# Rebuild the application image
docker compose build --no-cache xzepr

# Restart the stack
docker compose up -d
```

### View All Logs

```bash
# All services
docker compose logs -f

# Application only
docker compose logs -f xzepr
```

### Quick Status Check

```bash
# Check all services
docker compose ps

# Check XZepr health
curl -k https://localhost:8443/health

# Check PostgreSQL
docker compose exec postgres pg_isready -U xzepr

# Check Redpanda
curl http://localhost:19644/v1/status/ready
```

## Environment Variables

### Common XZepr Variables

```bash
XZEPR__SERVER__HOST=0.0.0.0
XZEPR__SERVER__PORT=8443
XZEPR__SERVER__ENABLE_HTTPS=false
XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr
XZEPR__KAFKA__BROKERS=redpanda-0:9092
XZEPR__AUTH__KEYCLOAK__ISSUER_URL=http://keycloak:8080/realms/xzepr
RUST_LOG=info,xzepr=debug
```

### Setting Variables in Compose Commands

```bash
docker compose run --rm -T \
  -e VAR1=value1 \
  -e VAR2=value2 \
  xzepr env
```

### Using Environment File

```bash
# Create .env file
cat > xzepr.env <<EOF
XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr
XZEPR__KAFKA__BROKERS=redpanda-0:9092
RUST_LOG=debug
EOF

# Use in a one-off compose command
docker compose run --rm -T --env-file xzepr.env xzepr env
```

## Port Mappings

### Standard Ports

- `5432` - PostgreSQL
- `8443` - XZepr Server (HTTPS)
- `8080` - Keycloak
- `8081` - Redpanda Console
- `18081` - Redpanda Schema Registry
- `18082` - Redpanda Proxy
- `19092` - Redpanda Kafka
- `19644` - Redpanda Admin API

### Custom Port Mapping

```bash
# Map the XZepr HTTPS port to a different host port
docker compose run --rm --service-ports -p 9000:8443 xzepr
```

## Best Practices

### Prefer Compose Service Names

```bash
docker compose ps
docker compose logs xzepr
docker compose exec xzepr /bin/bash
```

### Use Resource Limits

```bash
docker compose up -d

# Define memory and CPU limits in your compose configuration for repeatable deployments.
```

### Use Health Checks

```bash
docker compose ps
curl -k https://localhost:8443/health
```

### Use Restart Policies

```bash
# Configure restart policies in docker-compose.yaml for persistent environments.
docker compose up -d
```

## Additional Resources

- [Docker Documentation](https://docs.docker.com/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [XZepr Docker Tutorial](../tutorials/docker_demo.md)
- [XZepr Architecture](../explanations/architecture.md)
