# XZepr OPA Policies

This directory contains Rego policy files for Open Policy Agent (OPA) authorization.

## Policy Files

### rbac.rego

Core RBAC policy implementing resource ownership and group membership rules.

**Package**: `xzepr.rbac`

**Policy Rules**:

1. **Admin Access** - Users with "admin" role have unrestricted access
2. **Owner Access** - Resource owners can perform CRUD operations and manage members
3. **Group Member Access** - Group members can read and create events

**Actions**:

- `create` - Create new resources
- `read` - Read/view resources
- `update` - Modify existing resources
- `delete` - Remove resources
- `manage_members` - Add/remove group members

## Testing Policies

### Using OPA CLI

Install OPA:

```bash
brew install opa  # macOS
# or download from https://www.openpolicyagent.org/docs/latest/#running-opa
```

Test policy evaluation:

```bash
# Check policy syntax
opa check rbac.rego

# Test admin access
opa eval -d rbac.rego \
  -i test_data/admin_input.json \
  'data.xzepr.rbac.allow'

# Run policy tests (if test file exists)
opa test rbac.rego rbac_test.rego
```

### Using Docker

Start OPA server with policies:

```bash
docker run -p 8181:8181 \
  -v $(pwd):/policies:ro \
  openpolicyagent/opa:latest \
  run --server --addr=0.0.0.0:8181 /policies
```

Test via HTTP API:

```bash
curl -X POST http://localhost:8181/v1/data/xzepr/rbac/allow \
  -H 'Content-Type: application/json' \
  -d '{
    "input": {
      "user": {
        "user_id": "user123",
        "username": "alice",
        "roles": ["user"],
        "groups": []
      },
      "action": "read",
      "resource": {
        "resource_type": "event_receiver",
        "resource_id": "receiver123",
        "owner_id": "user123",
        "group_id": null,
        "members": []
      }
    }
  }'
```

Expected response for owner access:

```json
{
  "result": {
    "allow": true,
    "reason": "User is owner",
    "metadata": {
      "evaluated_at": 1705776000000000000,
      "policy_version": "1.0.0",
      "resource_type": "event_receiver",
      "action": "read"
    }
  }
}
```

## Test Cases

### Test Case 1: Admin Access

**Input**:
- User: admin role
- Action: delete
- Resource: owned by different user

**Expected**: allow = true, reason = "User is admin"

### Test Case 2: Owner Access

**Input**:
- User: regular user
- Action: update
- Resource: owned by same user

**Expected**: allow = true, reason = "User is owner"

### Test Case 3: Group Member Access

**Input**:
- User: group member
- Action: create
- Resource: group-owned, user in members list

**Expected**: allow = true, reason = "User is group member"

### Test Case 4: Unauthorized Access

**Input**:
- User: regular user
- Action: delete
- Resource: owned by different user, not a group member

**Expected**: allow = false, reason = "Access denied"

## Policy Bundle Structure

For production deployment with bundle server:

```
policies/
├── rbac.rego           # Core RBAC policy
├── data.json           # Optional static data
└── .manifest           # Bundle manifest
```

Build bundle:

```bash
opa build -b policies/ -o bundle.tar.gz
```

## Development Workflow

1. Edit policy file: `rbac.rego`
2. Check syntax: `opa check rbac.rego`
3. Test locally: `opa eval -d rbac.rego 'data.xzepr.rbac.allow'`
4. Start OPA server: `docker-compose up opa`
5. Test via XZepr application
6. Commit changes to version control

## Production Deployment

Policies are deployed via OPA bundle server:

1. Build bundle: `opa build -b policies/ -o bundle.tar.gz`
2. Upload to bundle server: `curl -T bundle.tar.gz http://bundle-server/bundles/xzepr-rbac.tar.gz`
3. OPA polls bundle server for updates
4. New policies take effect within polling interval (default: 1 minute)

## References

- OPA Documentation: https://www.openpolicyagent.org/docs/latest/
- Rego Language: https://www.openpolicyagent.org/docs/latest/policy-language/
- Policy Testing: https://www.openpolicyagent.org/docs/latest/policy-testing/
- Bundle API: https://www.openpolicyagent.org/docs/latest/management-bundles/
