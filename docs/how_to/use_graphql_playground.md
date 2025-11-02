# How to Use GraphQL Playground

This guide walks you through using the GraphQL Playground IDE to interact with the XZepr event tracking API.

## Prerequisites

- XZepr server running (default port: 8042)
- Web browser (Chrome, Firefox, Safari, or Edge)
- Basic understanding of GraphQL query syntax

## Starting the Server

Start the XZepr server:

```bash
cargo run --bin server
```

The server will log its URL:

```text
Server listening on http://0.0.0.0:8042
GraphQL Playground: http://0.0.0.0:8042/graphql/playground
```

## Accessing the Playground

Open your web browser and navigate to:

```text
http://localhost:8042/graphql/playground
```

You should see the GraphQL Playground interface with:

- Query editor on the left
- Results panel on the right
- Schema documentation tab
- Query history

## Basic Queries

### Query All Event Receivers

1. In the query editor, type:

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

2. Click the "Play" button (▶) or press `Ctrl+Enter` (Windows/Linux) or `Cmd+Enter` (Mac)

3. View the results in the right panel

### Query Event Receiver by ID

1. Write a query with a variable:

```graphql
query GetReceiverById($id: ID!) {
  eventReceiversById(id: $id) {
    id
    name
    type
    version
    description
    schema
    fingerprint
    createdAt
  }
}
```

2. Click the "Query Variables" panel at the bottom

3. Add your variables:

```json
{
  "id": "01234567-89ab-cdef-0123-456789abcdef"
}
```

4. Execute the query

### Filter Event Receivers

Query receivers by name:

```graphql
query FindReceiver {
  eventReceivers(eventReceiver: { name: "webhook" }) {
    id
    name
    type
  }
}
```

Query receivers by type and version:

```graphql
query FindByTypeVersion {
  eventReceivers(
    eventReceiver: {
      type: "webhook"
      version: "1.0.0"
    }
  ) {
    id
    name
    description
  }
}
```

## Creating Data with Mutations

### Create an Event Receiver

1. Write a mutation:

```graphql
mutation CreateWebhook($input: CreateEventReceiverInput!) {
  createEventReceiver(eventReceiver: $input)
}
```

2. Add variables:

```json
{
  "input": {
    "name": "my-webhook-receiver",
    "type": "webhook",
    "version": "1.0.0",
    "description": "Webhook for CI/CD events",
    "schema": {
      "type": "object",
      "properties": {
        "event": { "type": "string" },
        "payload": { "type": "object" }
      },
      "required": ["event"]
    }
  }
}
```

3. Execute the mutation

4. The response will contain the new receiver's ID:

```json
{
  "data": {
    "createEventReceiver": "01234567-89ab-cdef-0123-456789abcdef"
  }
}
```

### Create an Event Receiver Group

1. First, create or get event receiver IDs

2. Write the mutation:

```graphql
mutation CreateGroup($input: CreateEventReceiverGroupInput!) {
  createEventReceiverGroup(eventReceiverGroup: $input)
}
```

3. Add variables:

```json
{
  "input": {
    "name": "production-webhooks",
    "type": "webhook-group",
    "version": "1.0.0",
    "description": "Production webhook receivers",
    "enabled": true,
    "eventReceiverIds": [
      "01234567-89ab-cdef-0123-456789abcdef",
      "fedcba98-7654-3210-fedc-ba9876543210"
    ]
  }
}
```

4. Execute the mutation

### Enable or Disable a Group

Enable a group:

```graphql
mutation EnableGroup($id: ID!) {
  setEventReceiverGroupEnabled(id: $id)
}
```

Disable a group:

```graphql
mutation DisableGroup($id: ID!) {
  setEventReceiverGroupDisabled(id: $id)
}
```

Variables:

```json
{
  "id": "01234567-89ab-cdef-0123-456789abcdef"
}
```

## Working with Event Receiver Groups

### Query All Groups

```graphql
query {
  eventReceiverGroups(eventReceiverGroup: {}) {
    id
    name
    type
    version
    description
    enabled
    eventReceiverIds
    createdAt
    updatedAt
  }
}
```

### Query Groups by Type

```graphql
query GetWebhookGroups {
  eventReceiverGroups(
    eventReceiverGroup: { type: "webhook-group" }
  ) {
    id
    name
    enabled
    eventReceiverIds
  }
}
```

## Using the Schema Explorer

### View Available Types

1. Click the "DOCS" tab on the right side
2. Click "Query" to see all available queries
3. Click "Mutation" to see all available mutations
4. Click any type name to see its fields and description

### Autocomplete

The playground provides autocomplete:

1. Start typing a query
2. Press `Ctrl+Space` to trigger autocomplete
3. Select from available fields, types, and arguments

### Field Documentation

Hover over any field name to see its documentation and type information.

## Advanced Features

### Using Multiple Queries

Define multiple queries in one document:

```graphql
query GetReceivers {
  eventReceivers(eventReceiver: {}) {
    id
    name
  }
}

query GetGroups {
  eventReceiverGroups(eventReceiverGroup: {}) {
    id
    name
  }
}
```

Select which query to execute from the dropdown menu next to the play button.

### Query Fragments

Reuse field selections with fragments:

```graphql
fragment ReceiverFields on EventReceiver {
  id
  name
  type
  version
  description
}

query {
  eventReceivers(eventReceiver: {}) {
    ...ReceiverFields
    createdAt
  }
}
```

### Inline Variables

Use GraphQL variables for dynamic queries:

```graphql
query FindReceivers(
  $name: String
  $type: String
  $version: String
) {
  eventReceivers(
    eventReceiver: {
      name: $name
      type: $type
      version: $version
    }
  ) {
    id
    name
    type
  }
}
```

Variables panel:

```json
{
  "name": "webhook",
  "type": "webhook",
  "version": "1.0.0"
}
```

## Keyboard Shortcuts

- `Ctrl/Cmd + Enter` - Execute query
- `Ctrl/Cmd + Space` - Autocomplete
- `Ctrl/Cmd + K` - Format query
- `Ctrl/Cmd + /` - Comment/uncomment line
- `Ctrl/Cmd + D` - Duplicate line
- `Ctrl/Cmd + F` - Find in editor
- `Shift + Ctrl/Cmd + F` - Find and replace

## Troubleshooting

### Query Returns Empty Results

Check that:

- Data has been created using mutations
- Filter criteria matches existing data
- Field names are spelled correctly

### Syntax Errors

The playground will highlight syntax errors:

- Red underlines indicate errors
- Hover over them to see error messages
- Use the format button to fix indentation

### Network Errors

If queries fail to execute:

- Verify the server is running
- Check the server URL in the playground settings
- Look at browser console for CORS errors
- Ensure `/graphql` endpoint is accessible

### Variables Not Working

Ensure:

- Variable names match between query and variables panel
- Variable types match the schema
- JSON in variables panel is valid
- Required variables have values

## Best Practices

### Query Only What You Need

Request only the fields you need:

```graphql
query {
  eventReceivers(eventReceiver: {}) {
    id
    name
  }
}
```

Instead of:

```graphql
query {
  eventReceivers(eventReceiver: {}) {
    id
    name
    type
    version
    description
    schema
    fingerprint
    createdAt
  }
}
```

### Use Descriptive Operation Names

Good:

```graphql
query GetProductionWebhooks {
  eventReceivers(eventReceiver: { type: "webhook" }) {
    id
    name
  }
}
```

Bad:

```graphql
query {
  eventReceivers(eventReceiver: { type: "webhook" }) {
    id
    name
  }
}
```

### Save Frequently Used Queries

Use the query history feature:

1. Click the "HISTORY" tab
2. Find previous queries
3. Click to reload them into the editor

### Test with Variables

Use variables instead of hardcoded values:

```graphql
query GetReceiver($id: ID!) {
  eventReceiversById(id: $id) {
    id
    name
  }
}
```

This makes queries reusable and easier to test.

## Example Workflow

### Complete Event Receiver Setup

1. Create an event receiver:

```graphql
mutation CreateReceiver($input: CreateEventReceiverInput!) {
  createEventReceiver(eventReceiver: $input)
}
```

Variables:

```json
{
  "input": {
    "name": "ci-webhook",
    "type": "webhook",
    "version": "1.0.0",
    "description": "CI/CD webhook receiver",
    "schema": {
      "type": "object",
      "properties": {
        "build": { "type": "string" },
        "status": { "type": "string" }
      }
    }
  }
}
```

2. Verify creation:

```graphql
query {
  eventReceivers(eventReceiver: { name: "ci-webhook" }) {
    id
    name
    fingerprint
  }
}
```

3. Create a group:

```graphql
mutation CreateGroup($input: CreateEventReceiverGroupInput!) {
  createEventReceiverGroup(eventReceiverGroup: $input)
}
```

Variables:

```json
{
  "input": {
    "name": "ci-receivers",
    "type": "ci-group",
    "version": "1.0.0",
    "description": "CI webhook receivers",
    "enabled": true,
    "eventReceiverIds": ["<receiver-id-from-step-1>"]
  }
}
```

4. Verify the group:

```graphql
query {
  eventReceiverGroups(eventReceiverGroup: { name: "ci-receivers" }) {
    id
    name
    enabled
    eventReceiverIds
  }
}
```

## Next Steps

- Explore the schema documentation to discover all available queries
- Read the GraphQL Playground explanation documentation
- Try creating complex queries with multiple levels
- Experiment with different filter combinations
- Learn about GraphQL best practices and optimization
