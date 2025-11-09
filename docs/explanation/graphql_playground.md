# GraphQL Playground IDE

## Overview

The XZepr server includes a GraphQL Playground IDE, an interactive web-based GraphQL development environment. This document explains the architecture, implementation details, and design decisions behind the GraphQL API and its accompanying playground interface.

## What is GraphQL Playground?

GraphQL Playground is an interactive, in-browser GraphQL IDE that provides a user-friendly interface for exploring and testing GraphQL APIs. It offers features like:

- Schema introspection and documentation
- Syntax highlighting and auto-completion
- Query history
- Variable and header management
- Multiple tabs for different queries

## Architecture

The GraphQL implementation in XZepr follows a clean architecture pattern with clear separation of concerns:

### Layer Structure

```text
┌─────────────────────────────────────────┐
│         GraphQL Playground UI           │
│      (Browser-based Interface)          │
└─────────────────────────────────────────┘
                    │
                    │ HTTP POST
                    ▼
┌─────────────────────────────────────────┐
│         GraphQL Handler Layer            │
│    (src/api/graphql/handlers.rs)        │
│  - Request parsing                       │
│  - Response serialization                │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│         GraphQL Schema Layer             │
│    (src/api/graphql/schema.rs)          │
│  - Query resolvers                       │
│  - Mutation resolvers                    │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│       Application Handler Layer          │
│  (src/application/handlers/)            │
│  - Business logic                        │
│  - Validation                            │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│         Domain & Repository Layer        │
│  (src/domain/)                          │
│  - Entities                              │
│  - Repositories                          │
└─────────────────────────────────────────┘
```

## Implementation Details

### GraphQL Handlers

The GraphQL handler implementation is located in `src/api/graphql/handlers.rs` and provides three main endpoints:

#### 1. GraphQL Query Endpoint

**Endpoint:** `POST /graphql`

Processes GraphQL queries and mutations. Accepts JSON payloads with the following structure:

```json
{
  "query": "query GetReceivers { eventReceivers(eventReceiver: {}) { id name type } }",
  "operationName": "GetReceivers",
  "variables": {}
}
```

The handler:
- Parses the incoming JSON request
- Builds an async-graphql Request object
- Executes the query against the schema
- Returns a JSON response with data or errors

#### 2. GraphQL Playground Endpoint

**Endpoint:** `GET /graphql/playground`

Serves the GraphQL Playground HTML interface. The playground is configured to send queries to the `/graphql` endpoint.

#### 3. GraphQL Health Check

**Endpoint:** `GET /graphql/health`

Provides a health status check for the GraphQL service.

### Schema Definition

The GraphQL schema is defined in `src/api/graphql/schema.rs` and includes:

#### Queries

- `eventReceiversById(id: ID!)` - Get event receivers by ID
- `eventReceivers(eventReceiver: FindEventReceiverInput!)` - Find event receivers with criteria
- `eventReceiverGroupsById(id: ID!)` - Get event receiver groups by ID
- `eventReceiverGroups(eventReceiverGroup: FindEventReceiverGroupInput!)` - Find groups with criteria
- `eventsById(id: ID!)` - Get events by ID
- `events(event: FindEventInput!)` - Find events with criteria

#### Mutations

- `createEventReceiver(eventReceiver: CreateEventReceiverInput!)` - Create a new event receiver
- `createEventReceiverGroup(eventReceiverGroup: CreateEventReceiverGroupInput!)` - Create a new group
- `setEventReceiverGroupEnabled(id: ID!)` - Enable a group
- `setEventReceiverGroupDisabled(id: ID!)` - Disable a group
- `createEvent(event: CreateEventInput!)` - Create a new event

### Type System

Custom GraphQL types are defined in `src/api/graphql/types.rs`:

#### Custom Scalars

- `Time` - DateTime<Utc> wrapper for RFC 3339 timestamps
- `JSON` - JsonValue wrapper for arbitrary JSON data

#### Object Types

- `EventReceiverType` - Represents an event receiver
- `EventReceiverGroupType` - Represents an event receiver group
- `EventType` - Represents an event

#### Input Types

- `CreateEventReceiverInput` - Input for creating event receivers
- `FindEventReceiverInput` - Criteria for finding event receivers
- `CreateEventReceiverGroupInput` - Input for creating groups
- `FindEventReceiverGroupInput` - Criteria for finding groups
- `CreateEventInput` - Input for creating events
- `FindEventInput` - Criteria for finding events

## Design Decisions

### Direct Integration vs. async-graphql-axum

We chose to implement GraphQL handlers directly using `async-graphql` without the `async-graphql-axum` integration crate. This decision was made because:

1. **Version Compatibility:** Avoids version conflicts between axum dependencies
2. **Control:** Provides more control over request/response handling
3. **Simplicity:** Reduces dependency chain complexity
4. **Flexibility:** Easier to customize error handling and response formatting

### Manual Request/Response Structures

The GraphQL request and response structures are defined manually in the handlers module rather than relying on external types. This provides:

- Clear documentation of the expected API contract
- Type safety without external dependencies
- Easy serialization/deserialization with serde
- Complete control over the wire format

### Schema Context Data

The GraphQL schema uses Axum's state management to inject application handlers:

```rust
Schema::build(Query, Mutation, EmptySubscription)
    .data(event_receiver_handler)
    .data(event_receiver_group_handler)
    .finish()
```

This approach:
- Maintains separation of concerns
- Allows easy testing with mock handlers
- Follows dependency injection patterns
- Keeps the schema independent of HTTP layer details

### Multiple Router States

The router implementation uses multiple `.with_state()` calls to support both GraphQL and REST endpoints:

```rust
Router::new()
    .route("/graphql", post(graphql_handler))
    .with_state(schema.clone())
    .route("/api/v1/events", post(create_event))
    .with_state(app_state)
```

This pattern allows different route groups to have different state types while sharing the same router.

## Usage Examples

### Accessing the Playground

Navigate to `http://localhost:8042/graphql/playground` in your browser to access the interactive GraphQL Playground.

### Example Queries

#### Query Event Receivers

```graphql
query GetAllReceivers {
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

#### Query with Variables

```graphql
query GetReceiverByName($name: String!) {
  eventReceivers(eventReceiver: { name: $name }) {
    id
    name
    type
    schema
  }
}
```

Variables:
```json
{
  "name": "webhook-receiver"
}
```

#### Create Event Receiver

```graphql
mutation CreateReceiver($input: CreateEventReceiverInput!) {
  createEventReceiver(eventReceiver: $input)
}
```

Variables:
```json
{
  "input": {
    "name": "my-webhook",
    "type": "webhook",
    "version": "1.0.0",
    "description": "A webhook receiver",
    "schema": {
      "type": "object",
      "properties": {
        "message": { "type": "string" }
      }
    }
  }
}
```

#### Query Event Receiver Groups

```graphql
query GetGroups {
  eventReceiverGroups(eventReceiverGroup: {}) {
    id
    name
    type
    enabled
    eventReceiverIds
  }
}
```

### Programmatic API Usage

For programmatic access, send HTTP POST requests to `/graphql`:

```bash
curl -X POST http://localhost:8042/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ eventReceivers(eventReceiver: {}) { id name } }"
  }'
```

## Error Handling

The GraphQL implementation provides structured error responses:

### Validation Errors

```json
{
  "errors": [
    {
      "message": "Invalid EventReceiverId: invalid UUID format",
      "path": ["eventReceiversById"]
    }
  ]
}
```

### Business Logic Errors

```json
{
  "errors": [
    {
      "message": "Failed to create event receiver: Event receiver with same name and type already exists"
    }
  ]
}
```

## Performance Considerations

### Query Complexity

The GraphQL schema does not currently implement query complexity analysis. For production use, consider adding:

- Maximum query depth limits
- Query complexity scoring
- Rate limiting per client
- Request timeout configuration

### Pagination

Queries that return collections should use pagination:

```graphql
query PaginatedReceivers {
  eventReceivers(eventReceiver: {}) {
    id
    name
  }
}
```

Currently, the repository layer supports limit/offset pagination through the `FindEventReceiverCriteria`.

### Caching

Consider implementing:
- DataLoader for N+1 query prevention
- Response caching for frequently accessed data
- Persisted queries for production deployments

## Security Considerations

### Authentication

The GraphQL endpoints are currently public. For production deployments:

1. Add authentication middleware to protected routes
2. Implement JWT token validation
3. Use the existing auth layer for user context
4. Add field-level permissions based on user roles

### Input Validation

All mutations perform validation through:
- Domain entity validation rules
- Repository-level uniqueness checks
- Schema validation for JSON fields
- Type safety from GraphQL schema

### Rate Limiting

Consider implementing rate limiting at:
- Query execution level
- Mutation level
- Per-client basis using IP or API keys

## Testing

The GraphQL handlers include comprehensive unit tests covering:

- Query execution
- Variable handling
- Operation name support
- Playground HTML rendering
- Health check endpoint
- Error scenarios

Run tests with:

```bash
cargo test api::graphql::handlers
```

## Future Enhancements

### Subscriptions

The schema currently uses `EmptySubscription`. Future enhancements could include:

- Real-time event streaming via WebSocket subscriptions
- Live updates for event receiver status
- Notifications for group changes

### Schema Evolution

Maintain backward compatibility when evolving the schema:

- Use field deprecation instead of removal
- Add new optional fields for new features
- Version mutations if breaking changes are necessary
- Document schema changes in changelog

### Observability

Add tracing and metrics:

- Query execution time tracking
- Error rate monitoring
- Popular query identification
- Slow query logging

## Troubleshooting

### Playground Not Loading

Check that:
- Server is running on the expected port
- `/graphql/playground` route is registered
- Browser can access the server URL
- No CORS issues in browser console

### Queries Failing

Verify:
- Query syntax is valid GraphQL
- Field names match schema definitions
- Variables are properly formatted JSON
- Required arguments are provided

### Schema Not Reflecting Changes

After modifying the schema:
- Restart the server
- Clear browser cache
- Refresh the playground page
- Check schema introspection is enabled

## References

- [GraphQL Specification](https://spec.graphql.org/)
- [async-graphql Documentation](https://async-graphql.github.io/async-graphql/)
- [GraphQL Best Practices](https://graphql.org/learn/best-practices/)
- [Axum Documentation](https://docs.rs/axum/)
