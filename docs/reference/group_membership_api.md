# Group Membership API Reference

## Overview

The Group Membership API provides endpoints for managing user membership in event receiver groups. This enables fine-grained access control through group-based permissions managed by Open Policy Agent (OPA).

## Authentication

All endpoints require authentication via JWT token in the Authorization header:

```text
Authorization: Bearer <jwt_token>
```

## Authorization

Group membership operations require specific permissions:

- **Add Member**: User must be the group owner or have `group:manage_members` permission
- **Remove Member**: User must be the group owner or have `group:manage_members` permission
- **List Members**: User must be a group member or owner

## Endpoints

### Add Group Member

Adds a user to an event receiver group.

**Endpoint**: `POST /api/v1/groups/{group_id}/members`

**Path Parameters**:

- `group_id` (UUID, required) - The ID of the event receiver group

**Request Headers**:

```text
Authorization: Bearer <jwt_token>
Content-Type: application/json
```

**Request Body**:

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Request Body Schema**:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| user_id | UUID | Yes | The ID of the user to add to the group |

**Response**: `200 OK`

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "john.doe",
  "email": "john.doe@example.com",
  "added_at": "2024-01-15T10:30:00Z",
  "added_by": "660e8400-e29b-41d4-a716-446655440000"
}
```

**Response Schema**:

| Field | Type | Description |
|-------|------|-------------|
| user_id | UUID | The ID of the added user |
| username | String | The username of the added user |
| email | String | The email address of the added user |
| added_at | ISO8601 DateTime | When the user was added to the group |
| added_by | UUID | The ID of the user who added this member |

**Error Responses**:

- `400 Bad Request` - Invalid request body or user_id format
- `401 Unauthorized` - Missing or invalid authentication token
- `403 Forbidden` - User lacks permission to add members to this group
- `404 Not Found` - Group not found or user not found
- `409 Conflict` - User is already a member of the group
- `500 Internal Server Error` - Server error

**Example Request**:

```bash
curl -X POST https://api.example.com/api/v1/groups/770e8400-e29b-41d4-a716-446655440000/members \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..." \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "550e8400-e29b-41d4-a716-446655440000"
  }'
```

**Example Error Response** (`403 Forbidden`):

```json
{
  "error": {
    "code": "AUTHORIZATION_DENIED",
    "message": "User does not have permission to add members to this group",
    "details": {
      "user_id": "880e8400-e29b-41d4-a716-446655440000",
      "group_id": "770e8400-e29b-41d4-a716-446655440000",
      "required_permission": "group:manage_members"
    }
  }
}
```

---

### Remove Group Member

Removes a user from an event receiver group.

**Endpoint**: `DELETE /api/v1/groups/{group_id}/members/{user_id}`

**Path Parameters**:

- `group_id` (UUID, required) - The ID of the event receiver group
- `user_id` (UUID, required) - The ID of the user to remove

**Request Headers**:

```text
Authorization: Bearer <jwt_token>
```

**Response**: `204 No Content`

No response body is returned on success.

**Error Responses**:

- `401 Unauthorized` - Missing or invalid authentication token
- `403 Forbidden` - User lacks permission to remove members from this group
- `404 Not Found` - Group not found, user not found, or user is not a member
- `409 Conflict` - Cannot remove the group owner
- `500 Internal Server Error` - Server error

**Example Request**:

```bash
curl -X DELETE https://api.example.com/api/v1/groups/770e8400-e29b-41d4-a716-446655440000/members/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

**Example Error Response** (`409 Conflict`):

```json
{
  "error": {
    "code": "OPERATION_NOT_ALLOWED",
    "message": "Cannot remove the group owner from the group",
    "details": {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "group_id": "770e8400-e29b-41d4-a716-446655440000",
      "reason": "user_is_owner"
    }
  }
}
```

---

### List Group Members

Retrieves all members of an event receiver group.

**Endpoint**: `GET /api/v1/groups/{group_id}/members`

**Path Parameters**:

- `group_id` (UUID, required) - The ID of the event receiver group

**Query Parameters**:

- `page` (integer, optional) - Page number for pagination (default: 1)
- `page_size` (integer, optional) - Number of members per page (default: 50, max: 100)

**Request Headers**:

```text
Authorization: Bearer <jwt_token>
```

**Response**: `200 OK`

```json
{
  "group_id": "770e8400-e29b-41d4-a716-446655440000",
  "members": [
    {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "john.doe",
      "email": "john.doe@example.com",
      "added_at": "2024-01-15T10:30:00Z",
      "added_by": "660e8400-e29b-41d4-a716-446655440000"
    },
    {
      "user_id": "660e8400-e29b-41d4-a716-446655440000",
      "username": "jane.smith",
      "email": "jane.smith@example.com",
      "added_at": "2024-01-10T08:15:00Z",
      "added_by": "660e8400-e29b-41d4-a716-446655440000"
    }
  ],
  "pagination": {
    "current_page": 1,
    "page_size": 50,
    "total_members": 2,
    "total_pages": 1
  }
}
```

**Response Schema**:

| Field | Type | Description |
|-------|------|-------------|
| group_id | UUID | The ID of the group |
| members | Array | List of group members |
| members[].user_id | UUID | The ID of the member user |
| members[].username | String | The username of the member |
| members[].email | String | The email address of the member |
| members[].added_at | ISO8601 DateTime | When the user was added to the group |
| members[].added_by | UUID | The ID of the user who added this member |
| pagination | Object | Pagination information |
| pagination.current_page | Integer | Current page number |
| pagination.page_size | Integer | Number of members per page |
| pagination.total_members | Integer | Total number of members in the group |
| pagination.total_pages | Integer | Total number of pages |

**Error Responses**:

- `400 Bad Request` - Invalid query parameters
- `401 Unauthorized` - Missing or invalid authentication token
- `403 Forbidden` - User lacks permission to view group members
- `404 Not Found` - Group not found
- `500 Internal Server Error` - Server error

**Example Request**:

```bash
curl -X GET "https://api.example.com/api/v1/groups/770e8400-e29b-41d4-a716-446655440000/members?page=1&page_size=50" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

**Example Request with Pagination**:

```bash
curl -X GET "https://api.example.com/api/v1/groups/770e8400-e29b-41d4-a716-446655440000/members?page=2&page_size=25" \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

---

## GraphQL API

The Group Membership API is also available via GraphQL.

### Add Group Member Mutation

```graphql
mutation AddGroupMember($groupId: UUID!, $userId: UUID!) {
  addGroupMember(groupId: $groupId, userId: $userId) {
    userId
    username
    email
    addedAt
    addedBy
  }
}
```

**Variables**:

```json
{
  "groupId": "770e8400-e29b-41d4-a716-446655440000",
  "userId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response**:

```json
{
  "data": {
    "addGroupMember": {
      "userId": "550e8400-e29b-41d4-a716-446655440000",
      "username": "john.doe",
      "email": "john.doe@example.com",
      "addedAt": "2024-01-15T10:30:00Z",
      "addedBy": "660e8400-e29b-41d4-a716-446655440000"
    }
  }
}
```

### Remove Group Member Mutation

```graphql
mutation RemoveGroupMember($groupId: UUID!, $userId: UUID!) {
  removeGroupMember(groupId: $groupId, userId: $userId)
}
```

**Variables**:

```json
{
  "groupId": "770e8400-e29b-41d4-a716-446655440000",
  "userId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response**:

```json
{
  "data": {
    "removeGroupMember": true
  }
}
```

### List Group Members Query

```graphql
query GetGroupMembers($groupId: UUID!, $page: Int, $pageSize: Int) {
  groupMembers(groupId: $groupId, page: $page, pageSize: $pageSize) {
    groupId
    members {
      userId
      username
      email
      addedAt
      addedBy
    }
    pagination {
      currentPage
      pageSize
      totalMembers
      totalPages
    }
  }
}
```

**Variables**:

```json
{
  "groupId": "770e8400-e29b-41d4-a716-446655440000",
  "page": 1,
  "pageSize": 50
}
```

**Response**:

```json
{
  "data": {
    "groupMembers": {
      "groupId": "770e8400-e29b-41d4-a716-446655440000",
      "members": [
        {
          "userId": "550e8400-e29b-41d4-a716-446655440000",
          "username": "john.doe",
          "email": "john.doe@example.com",
          "addedAt": "2024-01-15T10:30:00Z",
          "addedBy": "660e8400-e29b-41d4-a716-446655440000"
        }
      ],
      "pagination": {
        "currentPage": 1,
        "pageSize": 50,
        "totalMembers": 1,
        "totalPages": 1
      }
    }
  }
}
```

---

## Error Codes

The API uses standard HTTP status codes and returns detailed error information:

| Code | HTTP Status | Description |
|------|-------------|-------------|
| INVALID_REQUEST | 400 | Request body or parameters are invalid |
| AUTHENTICATION_REQUIRED | 401 | Valid authentication token is required |
| AUTHORIZATION_DENIED | 403 | User lacks required permissions |
| RESOURCE_NOT_FOUND | 404 | Requested resource does not exist |
| OPERATION_NOT_ALLOWED | 409 | Operation conflicts with current state |
| INTERNAL_ERROR | 500 | Internal server error occurred |

**Error Response Format**:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {
      "field": "Additional context"
    }
  }
}
```

---

## Rate Limiting

API requests are rate-limited per user:

- **Standard Users**: 100 requests per minute
- **Premium Users**: 1000 requests per minute

Rate limit information is included in response headers:

```text
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1705320600
```

When rate limit is exceeded, the API returns `429 Too Many Requests`:

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Please retry after the reset time.",
    "details": {
      "limit": 100,
      "reset_at": "2024-01-15T10:30:00Z"
    }
  }
}
```

---

## Best Practices

### Error Handling

Always check the HTTP status code and parse error responses:

```rust
match response.status() {
    StatusCode::OK => {
        let member: GroupMemberResponse = response.json().await?;
        Ok(member)
    }
    StatusCode::FORBIDDEN => {
        let error: ErrorResponse = response.json().await?;
        Err(AuthorizationError::Denied(error.error.message))
    }
    StatusCode::NOT_FOUND => {
        Err(AuthorizationError::NotFound)
    }
    _ => {
        let error: ErrorResponse = response.json().await?;
        Err(AuthorizationError::Other(error.error.message))
    }
}
```

### Pagination

When listing members, use pagination to avoid large responses:

```rust
let mut all_members = Vec::new();
let mut page = 1;
let page_size = 50;

loop {
    let response = client
        .get_group_members(group_id, page, page_size)
        .await?;

    all_members.extend(response.members);

    if page >= response.pagination.total_pages {
        break;
    }

    page += 1;
}
```

### Idempotency

Add and remove operations are idempotent:

- Adding a user who is already a member returns `409 Conflict`
- Removing a user who is not a member returns `404 Not Found`

Handle these cases gracefully in your client code.

---

## Related Documentation

- [OPA Authorization Architecture](../explanation/opa_authorization_architecture.md)
- [OPA Policy Development Guide](../how-to/opa_policy_development.md)
- [Event Receiver API Reference](event_receiver_api.md)
- [Authentication Guide](../how-to/authentication.md)

---

## Support

For questions or issues with the Group Membership API:

- GitHub Issues: https://github.com/xbcsmith/xzepr/issues
- Documentation: https://xzepr.io/docs
- API Status: https://status.xzepr.io
