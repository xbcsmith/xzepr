# Docker Commands Quick Reference

## Overview

This document provides a quick reference for all Docker commands used in the XZepr demo tutorial. Commands are organized by task for easy lookup.

## Backend Services

### Start All Backend Services

```bash
docker compose -f docker-compose.services.yaml up -d
```

### Stop All Backend Services

```bash
docker compose -f docker-compose.services.yaml down
```

### Stop and Remove Volumes

```bash
docker compose -f docker-compose.services.yaml down -v
```

### View Service Status

```bash
docker compose -f docker-compose.services.yaml ps
```

### View Service Logs

```bash
# All services
docker compose -f docker-compose.services.yaml logs -f

# Specific service
docker compose -f docker-compose.services.yaml logs postgres
docker compose -f docker-compose.services.yaml logs redpanda-0
docker compose -f docker-compose.services.yaml logs keycloak
docker compose -f docker-compose.services.yaml logs console
```

### Restart Services

```bash
docker compose -f docker-compose.services.yaml restart
```

## XZepr Server

### Build Image

```bash
docker build -t xzepr:demo .
```

### Run Server Container

```bash
docker run -d \
  --name xzepr-server \
  --network xzepr_redpanda_network \
  -p 8042:8443 \
  -e XZEPR__SERVER__HOST=0.0.0.0 \
  -e XZEPR__SERVER__PORT=8443 \
  -e XZEPR__SERVER__ENABLE_HTTPS=false \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  -e XZEPR__KAFKA__BROKERS=redpanda-0:9092 \
  -e XZEPR__AUTH__KEYCLOAK__ISSUER_URL=http://keycloak:8080/realms/xzepr \
  -e RUST_LOG=info,xzepr=debug \
  -v "$(pwd)/certs:/app/certs:ro" \
  xzepr:demo
```

### Stop Server

```bash
docker stop xzepr-server
```

### Remove Server Container

```bash
docker rm xzepr-server
```

### View Server Logs

```bash
docker logs xzepr-server

# Follow logs in real-time
docker logs -f xzepr-server
```

### Execute Commands in Running Container

```bash
docker exec -it xzepr-server /bin/bash
```

## Admin CLI

### Create User

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  create-user \
  --username USERNAME \
  --email EMAIL \
  --password PASSWORD \
  --role ROLE
```

### List Users

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  list-users
```

### Add Role to User

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  add-role \
  --username USERNAME \
  --role ROLE
```

### Remove Role from User

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  remove-role \
  --username USERNAME \
  --role ROLE
```

### Generate API Key

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  generate-api-key \
  --username USERNAME \
  --name "KEY_NAME" \
  --expires-days DAYS
```

### List API Keys

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  list-api-keys \
  --username USERNAME
```

### Revoke API Key

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  revoke-api-key \
  --key-id KEY_ID
```

## Database Operations

### Run Database Migrations

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e DATABASE_URL=postgres://xzepr:password@postgres:5432/xzepr \
  -v "$(pwd)/migrations:/migrations:ro" \
  --entrypoint sh \
  xzepr:demo -c "apt-get update && apt-get install -y postgresql-client && \
    for file in /migrations/*.sql; do \
      echo \"Running \$file...\"; \
      PGPASSWORD=password psql -h postgres -U xzepr -d xzepr -f \"\$file\"; \
    done"
```

### Access PostgreSQL CLI

```bash
docker exec -it $(docker ps -qf "name=postgres") \
  psql -U xzepr -d xzepr
```

### Run SQL Query

```bash
docker exec -it $(docker ps -qf "name=postgres") \
  psql -U xzepr -d xzepr -c "SELECT * FROM users;"
```

### Backup Database

```bash
docker exec $(docker ps -qf "name=postgres") \
  pg_dump -U xzepr xzepr > backup.sql
```

### Restore Database

```bash
cat backup.sql | docker exec -i $(docker ps -qf "name=postgres") \
  psql -U xzepr -d xzepr
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
docker rmi xzepr:demo
```

### Tag Image

```bash
docker tag xzepr:demo xzepr:v1.0.0
```

### Inspect Image

```bash
docker inspect xzepr:demo
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

### Stop All Containers

```bash
docker stop $(docker ps -q)
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
docker top xzepr-server
```

## Troubleshooting

### Check Container Health

```bash
docker inspect --format='{{json .State.Health}}' xzepr-server | jq
```

### View Container Environment

```bash
docker exec xzepr-server env
```

### Copy Files from Container

```bash
docker cp xzepr-server:/app/logs/app.log ./local-logs/
```

### Copy Files to Container

```bash
docker cp local-config.yaml xzepr-server:/app/config/
```

### Test Network Connectivity

```bash
docker exec xzepr-server ping postgres
docker exec xzepr-server nc -zv redpanda-0 9092
```

## System Cleanup

### Clean Everything

```bash
# Stop all containers
docker stop $(docker ps -q)

# Remove all containers
docker container prune -f

# Remove all images
docker image prune -a -f

# Remove all volumes
docker volume prune -f

# Remove all networks
docker network prune -f
```

### Clean XZepr Specific Resources

```bash
# Stop and remove XZepr server
docker stop xzepr-server && docker rm xzepr-server

# Stop backend services and remove volumes
docker compose -f docker-compose.services.yaml down -v

# Remove XZepr image
docker rmi xzepr:demo
```

## Common Patterns

### Rebuild and Restart

```bash
# Rebuild image
docker build -t xzepr:demo .

# Stop old container
docker stop xzepr-server && docker rm xzepr-server

# Start new container
docker run -d --name xzepr-server \
  --network xzepr_redpanda_network \
  -p 8042:8443 \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  xzepr:demo
```

### View All Logs

```bash
# Backend services
docker compose -f docker-compose.services.yaml logs -f &

# XZepr server
docker logs -f xzepr-server
```

### Quick Status Check

```bash
# Check all containers
docker ps

# Check XZepr health
curl http://localhost:8042/health

# Check PostgreSQL
docker exec $(docker ps -qf "name=postgres") pg_isready -U xzepr

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

### Setting Variables in Run Command

```bash
docker run -d \
  -e VAR1=value1 \
  -e VAR2=value2 \
  xzepr:demo
```

### Using Environment File

```bash
# Create .env file
cat > xzepr.env <<EOF
XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr
XZEPR__KAFKA__BROKERS=redpanda-0:9092
RUST_LOG=debug
EOF

# Use in run command
docker run -d --env-file xzepr.env xzepr:demo
```

## Port Mappings

### Standard Ports

- `5432` - PostgreSQL
- `8042` - XZepr Server (HTTP)
- `8080` - Keycloak
- `8081` - Redpanda Console
- `18081` - Redpanda Schema Registry
- `18082` - Redpanda Proxy
- `19092` - Redpanda Kafka
- `19644` - Redpanda Admin API

### Custom Port Mapping

```bash
# Map container port 8443 to host port 9000
docker run -p 9000:8443 xzepr:demo
```

## Best Practices

### Always Use Named Containers

```bash
docker run -d --name xzepr-server xzepr:demo
```

### Use Resource Limits

```bash
docker run -d \
  --memory="2g" \
  --cpus="1.5" \
  --name xzepr-server \
  xzepr:demo
```

### Use Health Checks

```bash
docker run -d \
  --health-cmd="curl -f http://localhost:8443/health || exit 1" \
  --health-interval=30s \
  --health-timeout=10s \
  --health-retries=3 \
  --name xzepr-server \
  xzepr:demo
```

### Use Restart Policies

```bash
docker run -d \
  --restart=unless-stopped \
  --name xzepr-server \
  xzepr:demo
```

## Additional Resources

- [Docker Documentation](https://docs.docker.com/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [XZepr Docker Tutorial](../tutorials/docker_demo.md)
- [XZepr Architecture](../explanations/architecture.md)
