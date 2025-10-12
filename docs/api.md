# XZEPR API Documentation

This document provides comprehensive API examples for the XZEPR event tracking server, including authentication, event management, and administrative operations.

## Overview

XZEPR provides a RESTful API with the following key features:

- Multiple authentication methods (local, OIDC, API keys)
- Role-based access control (RBAC)
- Event streaming via Redpanda
- Real-time event processing
- Comprehensive audit logging

## Base URL

All API endpoints are served over HTTPS:

```
https://localhost:8443/api/v1
```

## Authentication Endpoints

#### 1. Local Login

```bash
# Login with username/password
curl -X POST https://localhost:8443/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "password123"
  }'

# Response:
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": 1234567890,
  "user": {
    "id": "01234567-89ab-cdef-0123-456789abcdef",
    "username": "alice",
    "email": "alice@example.com",
    "roles": ["event_manager"]
  }
}
```

#### 2. OIDC Login (Keycloak)

```bash
# Step 1: Get authorization URL
curl https://localhost:8443/api/v1/auth/oidc/login

# Response:
{
  "authorization_url": "https://keycloak.example.com/realms/xzepr/protocol/openid-connect/auth?...",
  "state": "random-csrf-token"
}

# Step 2: Redirect user to authorization_url
# Step 3: User authenticates with Keycloak
# Step 4: Keycloak redirects to callback with code
# Step 5: Exchange code for token (handled automatically)

# Final response:
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": 1234567890,
  "user": {
    "id": "01234567-89ab-cdef-0123-456789abcdef",
    "username": "bob",
    "email": "bob@company.com",
    "roles": ["user"]
  }
}
```

#### 3. API Key Authentication

```bash
# Use API key in header
curl -X GET https://localhost:8443/api/v1/events \
  -H "X-API-Key: xzepr_base64encodedkey..."

# Or use Bearer token
curl -X GET https://localhost:8443/api/v1/events \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### Protected Endpoints with RBAC

```bash
# Create event (requires EventCreate permission)
curl -X POST https://localhost:8443/api/v1/events \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "build-completed",
    "version": "1.0.0",
    "release": "2024.10",
    "platform_id": "linux-x86_64",
    "package": "rpm",
    "description": "Build completed successfully",
    "payload": {"commit": "abc123", "duration": 120},
    "success": true,
    "event_receiver_id": "01234567-89ab-cdef-0123-456789abcdef"
  }'

# Success response (201 Created):
{
  "id": "01234567-89ab-cdef-0123-456789abcdef"
}

# Forbidden response (403) if user lacks permission:
{
  "error": "Forbidden",
  "message": "Missing required permission: EventCreate"
}
```

## Event Management API

### Create Event

```bash
curl -X POST https://localhost:8443/api/v1/events \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "deployment-success",
    "version": "2.1.0",
    "release": "2024.12",
    "platform_id": "kubernetes-amd64",
    "package": "helm",
    "description": "Successful deployment to production",
    "payload": {
      "namespace": "production",
      "replicas": 3,
      "image": "app:2.1.0",
      "duration_seconds": 45
    },
    "success": true,
    "event_receiver_id": "01234567-89ab-cdef-0123-456789abcdef"
  }'

# Response:
{
  "id": "98765432-10ab-cdef-9876-543210abcdef",
  "created_at": "2024-12-19T10:30:00Z",
  "status": "processed"
}
```

### List Events

```bash
# Get recent events with pagination
curl -X GET "https://localhost:8443/api/v1/events?limit=10&offset=0&sort=created_at:desc" \
  -H "Authorization: Bearer $TOKEN"

# Filter by success status
curl -X GET "https://localhost:8443/api/v1/events?success=true&limit=20" \
  -H "Authorization: Bearer $TOKEN"

# Filter by date range
curl -X GET "https://localhost:8443/api/v1/events?from=2024-12-01T00:00:00Z&to=2024-12-19T23:59:59Z" \
  -H "Authorization: Bearer $TOKEN"

# Response:
{
  "events": [
    {
      "id": "98765432-10ab-cdef-9876-543210abcdef",
      "name": "deployment-success",
      "version": "2.1.0",
      "release": "2024.12",
      "platform_id": "kubernetes-amd64",
      "package": "helm",
      "description": "Successful deployment to production",
      "success": true,
      "created_at": "2024-12-19T10:30:00Z",
      "event_receiver_id": "01234567-89ab-cdef-0123-456789abcdef"
    }
  ],
  "total_count": 1,
  "has_more": false
}
```

### Get Event by ID

```bash
curl -X GET https://localhost:8443/api/v1/events/98765432-10ab-cdef-9876-543210abcdef \
  -H "Authorization: Bearer $TOKEN"

# Response:
{
  "id": "98765432-10ab-cdef-9876-543210abcdef",
  "name": "deployment-success",
  "version": "2.1.0",
  "release": "2024.12",
  "platform_id": "kubernetes-amd64",
  "package": "helm",
  "description": "Successful deployment to production",
  "payload": {
    "namespace": "production",
    "replicas": 3,
    "image": "app:2.1.0",
    "duration_seconds": 45
  },
  "success": true,
  "created_at": "2024-12-19T10:30:00Z",
  "updated_at": "2024-12-19T10:30:00Z",
  "event_receiver_id": "01234567-89ab-cdef-0123-456789abcdef"
}
```

## Event Streaming API

XZEPR uses Redpanda for real-time event streaming. Events are automatically published to Redpanda topics when created.

### WebSocket Event Stream

```javascript
// Connect to real-time event stream
const ws = new WebSocket('wss://localhost:8443/api/v1/events/stream');

ws.onopen = function() {
    // Subscribe to specific event types
    ws.send(JSON.stringify({
        "action": "subscribe",
        "filters": {
            "event_names": ["deployment-success", "build-completed"],
            "success": true
        }
    }));
};

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('New event:', data);
};

// Sample streamed event:
{
  "type": "event",
  "data": {
    "id": "98765432-10ab-cdef-9876-543210abcdef",
    "name": "deployment-success",
    "version": "2.1.0",
    "success": true,
    "created_at": "2024-12-19T10:30:00Z"
  }
}
```

### Redpanda Topic Configuration

Events are published to Redpanda topics based on the event name:

```bash
# List available topics
curl -X GET http://localhost:18082/topics \
  -H "Content-Type: application/vnd.kafka.v2+json"

# Response:
[
  "xzepr.events.deployment-success",
  "xzepr.events.build-completed",
  "xzepr.events.test-results"
]
```

### Consumer Example

```bash
# Create a consumer group
curl -X POST http://localhost:18082/consumers/xzepr-consumer-group \
  -H "Content-Type: application/vnd.kafka.v2+json" \
  -d '{
    "name": "event-processor",
    "format": "json",
    "auto.offset.reset": "earliest"
  }'

# Subscribe to topics
curl -X POST http://localhost:18082/consumers/xzepr-consumer-group/instances/event-processor/subscription \
  -H "Content-Type: application/vnd.kafka.v2+json" \
  -d '{
    "topics": ["xzepr.events.deployment-success"]
  }'

# Consume messages
curl -X GET http://localhost:18082/consumers/xzepr-consumer-group/instances/event-processor/records \
  -H "Accept: application/vnd.kafka.json.v2+json"
```

## Event Receivers API

### Create Event Receiver

```bash
curl -X POST https://localhost:8443/api/v1/event-receivers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production CI/CD Pipeline",
    "description": "Receives events from production deployments",
    "webhook_url": "https://webhook.example.com/events",
    "secret": "webhook-secret-key",
    "enabled": true,
    "event_filters": {
      "event_names": ["deployment-success", "deployment-failure"],
      "platforms": ["kubernetes-amd64"]
    }
  }'

# Response:
{
  "id": "01234567-89ab-cdef-0123-456789abcdef",
  "name": "Production CI/CD Pipeline",
  "created_at": "2024-12-19T10:00:00Z",
  "api_key": "xzepr_base64encodedkey..."
}
```

### List Event Receivers

```bash
curl -X GET https://localhost:8443/api/v1/event-receivers \
  -H "Authorization: Bearer $TOKEN"

# Response:
{
  "receivers": [
    {
      "id": "01234567-89ab-cdef-0123-456789abcdef",
      "name": "Production CI/CD Pipeline",
      "description": "Receives events from production deployments",
      "enabled": true,
      "created_at": "2024-12-19T10:00:00Z",
      "event_count": 42
    }
  ],
  "total_count": 1
}
```

## User Management API (Admin)

### Create User

```bash
curl -X POST https://localhost:8443/api/v1/admin/users \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "developer",
    "email": "dev@example.com",
    "password": "secure-password",
    "roles": ["event_manager"]
  }'

# Response:
{
  "id": "11111111-2222-3333-4444-555555555555",
  "username": "developer",
  "email": "dev@example.com",
  "roles": ["event_manager"],
  "created_at": "2024-12-19T11:00:00Z"
}
```

### List Users

```bash
curl -X GET https://localhost:8443/api/v1/admin/users \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# Response:
{
  "users": [
    {
      "id": "11111111-2222-3333-4444-555555555555",
      "username": "developer",
      "email": "dev@example.com",
      "roles": ["event_manager"],
      "created_at": "2024-12-19T11:00:00Z",
      "last_login": "2024-12-19T11:30:00Z"
    }
  ],
  "total_count": 1
}
```

## Health and Status API

### Health Check

```bash
curl -k https://localhost:8443/health

# Response:
{
  "status": "healthy",
  "version": "1.0.0",
  "timestamp": "2024-12-19T12:00:00Z",
  "services": {
    "database": "healthy",
    "redpanda": "healthy",
    "authentication": "healthy"
  }
}
```

### Metrics

```bash
curl -X GET https://localhost:8443/api/v1/metrics \
  -H "Authorization: Bearer $TOKEN"

# Response:
{
  "events": {
    "total_count": 1234,
    "success_rate": 0.96,
    "events_per_hour": 45.2
  },
  "users": {
    "active_users": 12,
    "total_users": 25
  },
  "system": {
    "uptime_seconds": 86400,
    "memory_usage": "512MB",
    "cpu_usage": "15%"
  }
}
```

## Error Responses

All API endpoints return consistent error responses:

### 400 Bad Request

```json
{
  "error": "Bad Request",
  "message": "Invalid request body",
  "details": {
    "field": "event_name",
    "issue": "cannot be empty"
  }
}
```

### 401 Unauthorized

```json
{
  "error": "Unauthorized",
  "message": "Invalid or missing authentication token"
}
```

### 403 Forbidden

```json
{
  "error": "Forbidden",
  "message": "Missing required permission: EventCreate"
}
```

### 404 Not Found

```json
{
  "error": "Not Found",
  "message": "Event with ID 98765432-10ab-cdef-9876-543210abcdef not found"
}
```

### 429 Too Many Requests

```json
{
  "error": "Too Many Requests",
  "message": "Rate limit exceeded. Try again in 60 seconds."
}
```

### 500 Internal Server Error

```json
{
  "error": "Internal Server Error",
  "message": "An unexpected error occurred"
}
```
