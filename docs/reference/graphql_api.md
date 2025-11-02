# GraphQL API Reference

This document provides a comprehensive reference for the XZepr GraphQL API, including all available types, queries, mutations, and their usage.

## Endpoints

- **GraphQL Endpoint:** `POST /graphql`
- **GraphQL Playground:** `GET /graphql/playground`
- **Health Check:** `GET /graphql/health`

## Schema Overview

The XZepr GraphQL API provides access to event receivers, event receiver groups, and events through a strongly-typed schema.

## Custom Scalars

### Time

Represents a date-time value in RFC 3339 format.

**Format:** `YYYY-MM-DDTHH:MM:SS.sssZ`

**Example:**
```text
2024-01-15T14:30:00.000Z
```

### JSON

Represents arbitrary JSON data as a string.

**Example:**
```json
{
  "type": "object",
  "properties": {
    "message": { "type": "string" }
  }
}
```

## Query Types

### Query

The root query type for reading data.

#### eventReceiversById

Get event receivers by ID.

**Arguments:**
- `id: ID!` - The unique identifier of the event receiver

**Returns:** `[EventReceiver!]!`

**Example:**
```graphql
query {
  eventReceiversById(id: "01234567-89ab-cdef-0123-456789abcdef") {
    id
    name
    type
    version
  }
}
```

#### eventReceivers

Find event receivers matching specified criteria.

**Arguments:**
- `eventReceiver: FindEventReceiverInput!` - Search criteria

**Returns:** `[EventReceiver!]!`

**Example:**
```graphql
query {
  eventReceivers(eventReceiver: { type: "webhook" }) {
    id
    name
    version
  }
}
```

#### eventReceiverGroupsById

Get event receiver groups by ID.

**Arguments:**
- `id: ID!` - The unique identifier of the event receiver group

**Returns:** `[EventReceiverGroup!]!`

**Example:**
```graphql
query {
  eventReceiverGroupsById(id: "01234567-89ab-cdef-0123-456789abcdef") {
    id
    name
    enabled
    eventReceiverIds
  }
}
```

#### eventReceiverGroups

Find event receiver groups matching specified criteria.

**Arguments:**
- `eventReceiverGroup: FindEventReceiverGroupInput!` - Search criteria

**Returns:** `[EventReceiverGroup!]!`

**Example:**
```graphql
query {
  eventReceiverGroups(eventReceiverGroup: { enabled: true }) {
    id
    name
    type
  }
}
```

#### eventsById

Get events by ID.

**Arguments:**
- `id: ID!` - The unique identifier of the event

**Returns:** `[Event!]!`

**Example:**
```graphql
query {
  eventsById(id: "01234567-89ab-cdef-0123-456789abcdef") {
    id
    name
    success
    createdAt
  }
}
```

#### events

Find events matching specified criteria.

**Arguments:**
- `event: FindEventInput!` - Search criteria

**Returns:** `[Event!]!`

**Example:**
```graphql
query {
  events(event: { success: true }) {
    id
    name
    version
    platformId
  }
}
```

## Mutation Types

### Mutation

The root mutation type for modifying data.

#### createEventReceiver

Create a new event receiver.

**Arguments:**
- `eventReceiver: CreateEventReceiverInput!` - Event receiver data

**Returns:** `ID!` - The ID of the created event receiver

**Example:**
```graphql
mutation {
  createEventReceiver(
    eventReceiver: {
      name: "webhook-receiver"
      type: "webhook"
      version: "1.0.0"
      description: "Webhook event receiver"
      schema: {
        type: "object"
        properties: {
          message: { type: "string" }
        }
      }
    }
  )
}
```

#### createEventReceiverGroup

Create a new event receiver group.

**Arguments:**
- `eventReceiverGroup: CreateEventReceiverGroupInput!` - Event receiver group data

**Returns:** `ID!` - The ID of the created event receiver group

**Example:**
```graphql
mutation {
  createEventReceiverGroup(
    eventReceiverGroup: {
      name: "production-group"
      type: "webhook-group"
      version: "1.0.0"
      description: "Production webhook receivers"
      enabled: true
      eventReceiverIds: [
        "01234567-89ab-cdef-0123-456789abcdef"
      ]
    }
  )
}
```

#### setEventReceiverGroupEnabled

Enable an event receiver group.

**Arguments:**
- `id: ID!` - The ID of the event receiver group to enable

**Returns:** `ID!` - The ID of the enabled event receiver group

**Example:**
```graphql
mutation {
  setEventReceiverGroupEnabled(id: "01234567-89ab-cdef-0123-456789abcdef")
}
```

#### setEventReceiverGroupDisabled

Disable an event receiver group.

**Arguments:**
- `id: ID!` - The ID of the event receiver group to disable

**Returns:** `ID!` - The ID of the disabled event receiver group

**Example:**
```graphql
mutation {
  setEventReceiverGroupDisabled(id: "01234567-89ab-cdef-0123-456789abcdef")
}
```

#### createEvent

Create a new event.

**Arguments:**
- `event: CreateEventInput!` - Event data

**Returns:** `ID!` - The ID of the created event

**Example:**
```graphql
mutation {
  createEvent(
    event: {
      name: "build-completed"
      version: "1.0.0"
      release: "v2024.01"
      platformId: "linux-x64"
      package: "myapp"
      description: "Build completed successfully"
      payload: {
        status: "success"
        duration: 120
      }
      eventReceiverId: "01234567-89ab-cdef-0123-456789abcdef"
      success: true
    }
  )
}
```

## Object Types

### EventReceiver

Represents an event receiver that can process events.

**Fields:**

- `id: ID!` - Unique identifier
- `name: String!` - Event receiver name
- `type: String!` - Event receiver type (e.g., "webhook")
- `version: String!` - Event receiver version
- `description: String!` - Human-readable description
- `schema: JSON!` - JSON schema for event validation
- `fingerprint: String!` - Unique fingerprint based on schema
- `createdAt: Time!` - Creation timestamp

**Example:**
```graphql
{
  id: "01234567-89ab-cdef-0123-456789abcdef"
  name: "webhook-receiver"
  type: "webhook"
  version: "1.0.0"
  description: "Webhook event receiver"
  schema: {...}
  fingerprint: "abc123..."
  createdAt: "2024-01-15T14:30:00.000Z"
}
```

### EventReceiverGroup

Represents a group of event receivers.

**Fields:**

- `id: ID!` - Unique identifier
- `name: String!` - Group name
- `type: String!` - Group type
- `version: String!` - Group version
- `description: String!` - Human-readable description
- `enabled: Boolean!` - Whether the group is enabled
- `eventReceiverIds: [ID!]!` - List of event receiver IDs in the group
- `createdAt: Time!` - Creation timestamp
- `updatedAt: Time!` - Last update timestamp

**Example:**
```graphql
{
  id: "fedcba98-7654-3210-fedc-ba9876543210"
  name: "production-group"
  type: "webhook-group"
  version: "1.0.0"
  description: "Production webhook receivers"
  enabled: true
  eventReceiverIds: ["01234567-89ab-cdef-0123-456789abcdef"]
  createdAt: "2024-01-15T14:30:00.000Z"
  updatedAt: "2024-01-15T14:30:00.000Z"
}
```

### Event

Represents an event that has been processed.

**Fields:**

- `id: ID!` - Unique identifier
- `name: String!` - Event name
- `version: String!` - Event version
- `release: String!` - Release identifier
- `platformId: String!` - Platform identifier
- `package: String!` - Package name
- `description: String!` - Event description
- `payload: JSON!` - Event payload data
- `eventReceiverId: ID!` - Associated event receiver ID
- `success: Boolean!` - Whether the event was successful
- `createdAt: Time!` - Creation timestamp

**Example:**
```graphql
{
  id: "11111111-2222-3333-4444-555555555555"
  name: "build-completed"
  version: "1.0.0"
  release: "v2024.01"
  platformId: "linux-x64"
  package: "myapp"
  description: "Build completed"
  payload: {...}
  eventReceiverId: "01234567-89ab-cdef-0123-456789abcdef"
  success: true
  createdAt: "2024-01-15T14:30:00.000Z"
}
```

## Input Types

### CreateEventReceiverInput

Input for creating a new event receiver.

**Fields:**

- `name: String!` - Event receiver name
- `type: String!` - Event receiver type
- `version: String!` - Event receiver version
- `description: String!` - Description
- `schema: JSON!` - JSON schema for validation

### FindEventReceiverInput

Criteria for finding event receivers.

**Fields:**

- `id: ID` - Filter by ID
- `name: String` - Filter by name
- `type: String` - Filter by type
- `version: String` - Filter by version

All fields are optional. An empty input will return all receivers (subject to pagination limits).

### CreateEventReceiverGroupInput

Input for creating a new event receiver group.

**Fields:**

- `name: String!` - Group name
- `type: String!` - Group type
- `version: String!` - Group version
- `description: String!` - Description
- `enabled: Boolean!` - Whether the group is enabled
- `eventReceiverIds: [ID!]!` - List of event receiver IDs

### FindEventReceiverGroupInput

Criteria for finding event receiver groups.

**Fields:**

- `id: ID` - Filter by ID
- `name: String` - Filter by name
- `type: String` - Filter by type
- `version: String` - Filter by version

All fields are optional. An empty input will return all groups (subject to pagination limits).

### CreateEventInput

Input for creating a new event.

**Fields:**

- `name: String!` - Event name
- `version: String!` - Event version
- `release: String!` - Release identifier
- `platformId: String!` - Platform identifier
- `package: String!` - Package name
- `description: String!` - Event description
- `payload: JSON!` - Event payload
- `eventReceiverId: ID!` - Associated event receiver ID
- `success: Boolean!` - Success status

### FindEventInput

Criteria for finding events.

**Fields:**

- `id: ID` - Filter by ID
- `name: String` - Filter by name
- `version: String` - Filter by version
- `release: String` - Filter by release
- `platformId: String` - Filter by platform ID
- `package: String` - Filter by package
- `success: Boolean` - Filter by success status
- `eventReceiverId: ID` - Filter by event receiver ID

All fields are optional.

## Error Handling

GraphQL errors are returned in the standard GraphQL error format:

```json
{
  "errors": [
    {
      "message": "Error description",
      "path": ["fieldName"],
      "locations": [{ "line": 2, "column": 3 }]
    }
  ]
}
```

### Common Error Types

#### Validation Errors

Returned when input validation fails:

```json
{
  "errors": [
    {
      "message": "Invalid EventReceiverId: invalid UUID format"
    }
  ]
}
```

#### Not Found Errors

Returned when a requested resource doesn't exist:

```json
{
  "errors": [
    {
      "message": "Failed to get event receiver: not found"
    }
  ]
}
```

#### Business Rule Violations

Returned when a business rule is violated:

```json
{
  "errors": [
    {
      "message": "Failed to create event receiver: Event receiver with same name and type already exists"
    }
  ]
}
```

## Pagination

Query results may be limited by default pagination settings. The current implementation uses:

- Default limit: 50 items
- Default offset: 0

Future versions may include explicit pagination arguments.

## Best Practices

### Query Only Required Fields

Request only the fields you need:

```graphql
query {
  eventReceivers(eventReceiver: {}) {
    id
    name
  }
}
```

### Use Variables

Use variables instead of hardcoded values:

```graphql
query GetReceiver($id: ID!) {
  eventReceiversById(id: $id) {
    id
    name
    type
  }
}
```

### Name Your Operations

Always name your queries and mutations:

```graphql
query GetWebhookReceivers {
  eventReceivers(eventReceiver: { type: "webhook" }) {
    id
    name
  }
}
```

### Use Fragments for Reusability

Define fragments for common field sets:

```graphql
fragment ReceiverFields on EventReceiver {
  id
  name
  type
  version
  description
  createdAt
}

query {
  eventReceivers(eventReceiver: {}) {
    ...ReceiverFields
  }
}
```

## Examples

### Complete Workflow Example

1. Create an event receiver:

```graphql
mutation CreateReceiver {
  createEventReceiver(
    eventReceiver: {
      name: "ci-webhook"
      type: "webhook"
      version: "1.0.0"
      description: "CI/CD webhook"
      schema: {
        type: "object"
        properties: {
          status: { type: "string" }
        }
      }
    }
  )
}
```

2. Query the created receiver:

```graphql
query {
  eventReceivers(eventReceiver: { name: "ci-webhook" }) {
    id
    name
    fingerprint
  }
}
```

3. Create a group with the receiver:

```graphql
mutation CreateGroup($receiverId: ID!) {
  createEventReceiverGroup(
    eventReceiverGroup: {
      name: "ci-group"
      type: "ci"
      version: "1.0.0"
      description: "CI receivers"
      enabled: true
      eventReceiverIds: [$receiverId]
    }
  )
}
```

4. Enable/disable the group:

```graphql
mutation DisableGroup($groupId: ID!) {
  setEventReceiverGroupDisabled(id: $groupId)
}
```

## Schema Introspection

The GraphQL API supports introspection queries:

```graphql
query {
  __schema {
    queryType {
      name
      fields {
        name
        description
      }
    }
  }
}
```

Get type information:

```graphql
query {
  __type(name: "EventReceiver") {
    name
    fields {
      name
      type {
        name
        kind
      }
    }
  }
}
```

## Version Information

- **API Version:** 1.0.0
- **Schema Version:** Based on async-graphql 7.0
- **Compatibility:** Follows GraphQL specification (October 2021)
