# XZepr Docker Demo Tutorial

## Overview

This tutorial provides a complete step-by-step demonstration of XZepr using Docker and Docker Compose. You will learn how to:

- Start backend services (PostgreSQL, Redpanda, Keycloak)
- Build and run XZepr in a Docker container
- Create users with the admin CLI
- Use Redpanda Console to monitor event streams
- Interact with the GraphQL Playground
- Test the API with curl

## Prerequisites

- Docker Engine 20.10 or later
- Docker Compose v2.0 or later
- Basic understanding of Docker and command line
- At least 4GB of available RAM
- Ports available: 5432, 8042, 8080, 8081, 18081, 18082, 19092, 19644

## Architecture Overview

```text
┌─────────────────────────────────────────────────────────┐
│  XZepr Server (Docker Container)                        │
│  - REST API (port 8042)                                 │
│  - GraphQL API (port 8042)                              │
└─────────────────────────────────────────────────────────┘
                        │
                        ├─── PostgreSQL (port 5432)
                        ├─── Redpanda (ports 19092, 18081)
                        ├─── Redpanda Console (port 8081)
                        └─── Keycloak (port 8080)
```

## Step 1: Clone the Repository

```bash
git clone <repository-url>
cd xzepr
```

## Step 2: Create TLS Certificates

XZepr requires TLS certificates for secure communication. Generate self-signed certificates for development:

```bash
# Create certificates directory
mkdir -p certs

# Generate self-signed certificate (valid for 365 days)
openssl req -x509 -newkey rsa:4096 \
  -keyout certs/key.pem \
  -out certs/cert.pem \
  -days 365 -nodes \
  -subj "/C=US/ST=State/L=City/O=XZepr/CN=localhost"

# Verify certificates were created
ls -lh certs/
```

Expected output:

```text
-rw-r--r-- 1 user user 1.9K cert.pem
-rw------- 1 user user 3.2K key.pem
```

## Step 3: Start Backend Services

Start PostgreSQL, Redpanda, and Keycloak using Docker Compose:

```bash
# Start backend services in detached mode
docker compose -f docker-compose.services.yaml up -d

# View logs to monitor startup
docker compose -f docker-compose.services.yaml logs -f
```

Wait for the following log messages indicating services are ready:

- PostgreSQL: `database system is ready to accept connections`
- Redpanda: `Successfully started Redpanda`
- Keycloak: `Keycloak started`
- Redpanda Console: `Listening on port 8080`

Press `Ctrl+C` to exit log viewing.

Verify all services are running:

```bash
docker compose -f docker-compose.services.yaml ps
```

Expected output:

```text
NAME                 IMAGE                                    STATUS
postgres             postgres:16                              Up
keycloak             quay.io/keycloak/keycloak:24.0          Up
redpanda-0           docker.redpanda.com/redpandadata/...     Up
redpanda-console     docker.redpanda.com/redpandadata/...     Up
```

## Step 4: Build XZepr Docker Image

Build the XZepr Docker image:

```bash
# Build the image with tag 'xzepr:demo'
docker build -t xzepr:demo .

# Verify the image was created
docker images | grep xzepr
```

Expected output:

```text
xzepr        demo        <image-id>    2 minutes ago    XXX MB
```

The build process may take 5-10 minutes on first run as it downloads dependencies and compiles the Rust code.

## Step 5: Initialize the Database

The XZepr Docker image includes sqlx-cli for managing database migrations. Run migrations to set up the schema:

```bash
# Run migrations using sqlx-cli built into the Docker image
docker run --rm \
  --network xzepr_redpanda_network \
  -e DATABASE_URL=postgres://xzepr:password@postgres:5432/xzepr \
  xzepr:demo \
  ./sqlx migrate run
```

This command:

- Uses sqlx-cli which is built into the xzepr:demo image
- Tracks migrations in the `_sqlx_migrations` table
- Is idempotent and safe to run multiple times
- Provides proper error handling and rollback support

Check migration status:

```bash
# View applied migrations
docker run --rm \
  --network xzepr_redpanda_network \
  -e DATABASE_URL=postgres://xzepr:password@postgres:5432/xzepr \
  xzepr:demo \
  ./sqlx migrate info
```

Expected output:

```text
Applied migrations:
20240101000001/create_users_table (applied)
20240101000002/create_events_table (applied)
20240101000003/create_event_receivers_table (applied)
```

Alternatively, if you have sqlx-cli installed locally:

```bash
export DATABASE_URL=postgres://xzepr:password@localhost:5432/xzepr
sqlx migrate run
```

## Step 6: Create Admin User

Use the XZepr admin CLI to create an administrative user:

```bash
# Create admin user
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  create-user \
  --username admin \
  --email admin@xzepr.local \
  --password SecurePassword123! \
  --role admin
```

Expected output:

```text
✓ User created successfully!
  ID: <uuid>
  Username: admin
  Roles: user, admin
```

## Step 7: Create Additional Test Users

Create users with different roles for testing:

```bash
# Create an EventManager user
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  create-user \
  --username eventmanager \
  --email manager@xzepr.local \
  --password Manager123! \
  --role event_manager

# Create an EventViewer user
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  create-user \
  --username viewer \
  --email viewer@xzepr.local \
  --password Viewer123! \
  --role event_viewer

# Create a regular User
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  create-user \
  --username user \
  --email user@xzepr.local \
  --password User123! \
  --role user
```

## Step 8: List All Users

Verify users were created:

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  list-users
```

Expected output:

```text
ID                                   Username        Email                    Roles
----------------------------------------------------------------------------------------------------
<uuid>                               admin           admin@xzepr.local        user, admin
<uuid>                               eventmanager    manager@xzepr.local      user, event_manager
<uuid>                               viewer          viewer@xzepr.local       user, event_viewer
<uuid>                               user            user@xzepr.local         user
```

## Step 9: Generate API Key

Generate an API key for programmatic access:

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  generate-api-key \
  --username admin \
  --name "Demo API Key" \
  --expires-days 30
```

Expected output:

```text
✓ API Key generated successfully!
  Key ID: <uuid>
  API Key: xzepr_<random-string>
  Name: Demo API Key
  Expires: <date>

IMPORTANT: Save this API key now. It will not be shown again.
```

Save the API key for later use. Store it in an environment variable:

```bash
export XZEPR_API_KEY="xzepr_<random-string>"
```

## Step 10: Start XZepr Server

Run the XZepr server in a Docker container:

```bash
docker run \
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

Note: We're running without HTTPS for simplicity in this demo. In production, set `ENABLE_HTTPS=true`.

View server logs:

```bash
docker logs -f xzepr-server
```

Wait for the log message:

```text
Server listening on http://0.0.0.0:8443
Health check: http://0.0.0.0:8443/health
API documentation: http://0.0.0.0:8443/api/v1
GraphQL endpoint: http://0.0.0.0:8443/graphql
GraphQL Playground: http://0.0.0.0:8443/graphql/playground
XZepr server ready to accept connections
```

Press `Ctrl+C` to exit log viewing.

## Step 11: Verify Server Health

Check that the server is responding:

```bash
curl http://localhost:8042/health
```

Expected output:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2024-01-15T14:30:00Z"
}
```

Check GraphQL health:

```bash
curl http://localhost:8042/graphql/health
```

Expected output:

```json
{
  "status": "healthy",
  "service": "graphql"
}
```

## Step 12: Access Redpanda Console

Open your browser and navigate to:

```text
http://localhost:8081
```

Explore the Redpanda Console interface:

1. View topics and messages
2. Monitor consumer groups
3. Check schema registry
4. View cluster health

### Create a Test Topic

In the Redpanda Console:

1. Click "Topics" in the sidebar
2. Click "Create Topic"
3. Enter name: `xzepr.demo.events`
4. Set partitions: `3`
5. Click "Create"

## Step 13: Access GraphQL Playground

Open your browser and navigate to:

```text
http://localhost:8042/graphql/playground
```

The GraphQL Playground provides an interactive IDE for exploring the API.

### Example Query: List Event Receivers

In the playground, enter this query:

```graphql
query {
  eventReceivers(eventReceiver: {}) {
    id
    name
    type
    version
    description
    createdAt
  }
}
```

Click the "Play" button or press `Ctrl+Enter` to execute.

### Example Mutation: Create Event Receiver

```graphql
mutation {
  createEventReceiver(
    eventReceiver: {
      name: "webhook-receiver"
      type: "webhook"
      version: "1.0.0"
      description: "Webhook event receiver for CI/CD"
      schema: {
        type: "object"
        properties: { event: { type: "string" }, status: { type: "string" } }
        required: ["event"]
      }
    }
  )
}
```

Expected response:

```json
{
  "data": {
    "createEventReceiver": "<uuid>"
  }
}
```

Save the returned UUID for the next steps.

### Example Query with Variables

Query an event receiver by ID using variables:

Query:

```graphql
query GetReceiver($id: ID!) {
  eventReceiversById(id: $id) {
    id
    name
    type
    version
    schema
    fingerprint
  }
}
```

Variables (click "Query Variables" at bottom):

```json
{
  "id": "<uuid-from-previous-step>"
}
```

### Example: Create Event Receiver Group

```graphql
mutation CreateGroup($input: CreateEventReceiverGroupInput!) {
  createEventReceiverGroup(eventReceiverGroup: $input)
}
```

Variables:

```json
{
  "input": {
    "name": "production-webhooks",
    "type": "webhook-group",
    "version": "1.0.0",
    "description": "Production webhook receivers",
    "enabled": true,
    "eventReceiverIds": ["<uuid-from-receiver>"]
  }
}
```

## Step 14: Test REST API with curl

### Create Event Receiver via REST API

```bash
curl -X POST http://localhost:8042/api/v1/receivers \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $XZEPR_API_KEY" \
  -d '{
    "name": "rest-webhook",
    "type": "webhook",
    "version": "1.0.0",
    "description": "REST API webhook receiver",
    "schema": {
      "type": "object",
      "properties": {
        "message": {"type": "string"}
      }
    }
  }'
```

Expected response:

```json
{
  "id": "<uuid>",
  "name": "rest-webhook",
  "type": "webhook",
  "version": "1.0.0",
  "description": "REST API webhook receiver",
  "fingerprint": "<hash>",
  "created_at": "2024-01-15T14:30:00Z"
}
```

### List Event Receivers

```bash
curl http://localhost:8042/api/v1/receivers \
  -H "Authorization: Bearer $XZEPR_API_KEY"
```

### Get Event Receiver by ID

```bash
curl http://localhost:8042/api/v1/receivers/<receiver-id> \
  -H "Authorization: Bearer $XZEPR_API_KEY"
```

### Create Event

```bash
curl -X POST http://localhost:8042/api/v1/events \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $XZEPR_API_KEY" \
  -d '{
    "name": "build-completed",
    "version": "1.0.0",
    "release": "v2024.01.15",
    "platform_id": "linux-x64",
    "package": "myapp",
    "description": "Build completed successfully",
    "payload": {
      "status": "success",
      "duration": 120,
      "tests_passed": 45
    },
    "event_receiver_id": "<receiver-id>",
    "success": true
  }'
```

### Create Event Receiver Group

```bash
curl -X POST http://localhost:8042/api/v1/groups \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $XZEPR_API_KEY" \
  -d '{
    "name": "ci-cd-group",
    "type": "ci-group",
    "version": "1.0.0",
    "description": "CI/CD event receivers",
    "enabled": true,
    "event_receiver_ids": ["<receiver-id>"]
  }'
```

## Step 15: Test GraphQL API with curl

### Execute GraphQL Query

```bash
curl -X POST http://localhost:8042/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ eventReceivers(eventReceiver: {}) { id name type version } }"
  }'
```

### Execute GraphQL Query with Variables

```bash
curl -X POST http://localhost:8042/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query GetReceiver($id: ID!) { eventReceiversById(id: $id) { id name type } }",
    "variables": {
      "id": "<receiver-id>"
    }
  }'
```

### Execute GraphQL Mutation

```bash
curl -X POST http://localhost:8042/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation CreateReceiver($input: CreateEventReceiverInput!) { createEventReceiver(eventReceiver: $input) }",
    "variables": {
      "input": {
        "name": "curl-webhook",
        "type": "webhook",
        "version": "1.0.0",
        "description": "Created via curl",
        "schema": {
          "type": "object",
          "properties": {
            "data": {"type": "string"}
          }
        }
      }
    }
  }'
```

## Step 16: Monitor Events in Redpanda Console

1. Go to Redpanda Console: `http://localhost:8081`
2. Click "Topics" in the sidebar
3. Click on `xzepr.dev.events` topic (if created)
4. View messages being produced by XZepr
5. Inspect message keys, values, and headers

## Step 17: View Container Logs

### XZepr Server Logs

```bash
docker logs xzepr-server
```

View real-time logs:

```bash
docker logs -f xzepr-server
```

### Backend Services Logs

```bash
# PostgreSQL logs
docker compose -f docker-compose.services.yaml logs postgres

# Redpanda logs
docker compose -f docker-compose.services.yaml logs redpanda-0

# Keycloak logs
docker compose -f docker-compose.services.yaml logs keycloak
```

## Step 18: Advanced Admin Operations

### List User API Keys

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  list-api-keys \
  --username admin
```

### Add Role to User

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  add-role \
  --username user \
  --role event_manager
```

### Remove Role from User

```bash
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  remove-role \
  --username user \
  --role event_manager
```

## Step 19: Testing Complete Workflow

### Complete End-to-End Test

```bash
#!/bin/bash
# Save as test-workflow.sh

echo "1. Creating event receiver..."
RECEIVER_RESPONSE=$(curl -s -X POST http://localhost:8042/api/v1/receivers \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $XZEPR_API_KEY" \
  -d '{
    "name": "test-receiver",
    "type": "webhook",
    "version": "1.0.0",
    "description": "Test receiver",
    "schema": {"type": "object"}
  }')

RECEIVER_ID=$(echo $RECEIVER_RESPONSE | jq .data)
echo "✓ Receiver created: $RECEIVER_ID"

echo ""
echo "2. Creating event..."
EVENT_RESPONSE=$(curl -s -X POST http://localhost:8042/api/v1/events \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $XZEPR_API_KEY" \
  -d "{
    \"name\": \"test-event\",
    \"version\": \"1.0.0\",
    \"release\": \"v1.0.0\",
    \"platform_id\": \"linux\",
    \"package\": \"test\",
    \"description\": \"Test event\",
    \"payload\": {\"test\": true},
    \"event_receiver_id\": \"$RECEIVER_ID\",
    \"success\": true
  }")

EVENT_ID=$(echo $EVENT_RESPONSE | jq .data)
echo "✓ Event created: $EVENT_ID"

echo ""
echo "3. Creating receiver group..."
GROUP_RESPONSE=$(curl -s -X POST http://localhost:8042/api/v1/groups \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $XZEPR_API_KEY" \
  -d "{
    \"name\": \"test-group\",
    \"type\": \"test\",
    \"version\": \"1.0.0\",
    \"description\": \"Test group\",
    \"enabled\": true,
    \"event_receiver_ids\": [\"$RECEIVER_ID\"]
  }")

GROUP_ID=$(echo $GROUP_RESPONSE | jq .data)
echo "✓ Group created: $GROUP_ID"

echo ""
echo "4. Verifying via GraphQL..."
curl -s -X POST http://localhost:8042/graphql \
  -H "Content-Type: application/json" \
  -d "{
    \"query\": \"{ eventReceiversById(id: \\\"$RECEIVER_ID\\\") { id name } }\"
  }" | grep -q "$RECEIVER_ID" && echo "✓ GraphQL query successful"

echo ""
echo "Workflow test complete!"
```

Run the test:

```bash
chmod +x test-workflow.sh
./test-workflow.sh
```

## Step 20: Cleanup

### Stop XZepr Server

```bash
docker stop xzepr-server
docker rm xzepr-server
```

### Stop Backend Services

```bash
docker compose -f docker-compose.services.yaml down
```

### Remove All Data (Optional)

To completely reset and remove all data:

```bash
# Stop and remove containers, networks, and volumes
docker compose -f docker-compose.services.yaml down -v

# Remove XZepr image
docker rmi xzepr:demo

# Remove certificates
rm -rf certs/
```

### Keep Data for Next Run

To keep data but stop services:

```bash
# Stop services but keep volumes
docker compose -f docker-compose.services.yaml down
```

Restart later with:

```bash
docker compose -f docker-compose.services.yaml up -d
```

## Troubleshooting

### Port Already in Use

If you see errors about ports being in use:

```bash
# Check what's using a port
lsof -i :8042
lsof -i :5432

# Stop conflicting services or use different ports
docker run -p 8043:8443 ...  # Use port 8043 instead of 8042
```

### Container Won't Start

Check logs:

```bash
docker logs xzepr-server
docker compose -f docker-compose.services.yaml logs
```

### Database Connection Failed

Verify PostgreSQL is running:

```bash
docker compose -f docker-compose.services.yaml ps postgres

# Test connection
docker exec -it $(docker ps -qf "name=postgres") psql -U xzepr -d xzepr -c "SELECT 1;"
```

### Migration Failed

If migrations fail, check the status and fix issues:

```bash
# Check migration status
docker run --rm \
  --network xzepr_redpanda_network \
  -e DATABASE_URL=postgres://xzepr:password@postgres:5432/xzepr \
  xzepr:demo \
  ./sqlx migrate info

# View migration table directly
docker exec -it $(docker ps -qf "name=postgres") \
  psql -U xzepr -d xzepr -c "SELECT * FROM _sqlx_migrations ORDER BY installed_on;"

# If a migration is marked as failed, you may need to manually fix it
# Then revert the failed migration and try again
docker run --rm \
  --network xzepr_redpanda_network \
  -e DATABASE_URL=postgres://xzepr:password@postgres:5432/xzepr \
  xzepr:demo \
  ./sqlx migrate revert

# Run migrations again
docker run --rm \
  --network xzepr_redpanda_network \
  -e DATABASE_URL=postgres://xzepr:password@postgres:5432/xzepr \
  xzepr:demo \
  ./sqlx migrate run
```

### Cannot Connect to Redpanda

Verify Redpanda is running:

```bash
docker compose -f docker-compose.services.yaml ps redpanda-0

# Check Redpanda logs
docker compose -f docker-compose.services.yaml logs redpanda-0
```

### GraphQL Playground Not Loading

Verify XZepr is running and accessible:

```bash
curl http://localhost:8042/health
curl http://localhost:8042/graphql/health
```

Check browser console for errors and ensure JavaScript is enabled.

### API Returns 401 Unauthorized

Ensure you're using a valid API key:

```bash
# Verify API key is set
echo $XZEPR_API_KEY

# Generate new key if needed
docker run --rm \
  --network xzepr_redpanda_network \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr:demo \
  generate-api-key \
  --username admin \
  --name "New Key" \
  --expires-days 30
```

## Summary

You have successfully:

- Started backend services with Docker Compose
- Built and run XZepr in a Docker container
- Created users and generated API keys
- Explored the API using GraphQL Playground
- Monitored events with Redpanda Console
- Tested endpoints with curl
- Completed an end-to-end workflow

## Next Steps

- Read the [GraphQL API Reference](../reference/graphql_api.md)
- Learn about [Authentication and Authorization](../explanations/architecture.md)
- Explore [How to Use GraphQL Playground](../how_to/use_graphql_playground.md)
- Deploy to production with [Docker Production Guide](../how_to/docker_production.md)

## Additional Resources

- XZepr GitHub Repository: `<repository-url>`
- Redpanda Documentation: https://docs.redpanda.com/
- GraphQL Specification: https://spec.graphql.org/
- Docker Documentation: https://docs.docker.com/
