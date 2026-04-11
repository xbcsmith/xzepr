# OPA Policy Development Guide

## Overview

This guide explains how to develop, test, and deploy Open Policy Agent (OPA) policies for XZepr's authorization system. It covers policy structure, Rego language basics, testing strategies, and best practices.

## Prerequisites

- OPA CLI installed (https://www.openpolicyagent.org/docs/latest/#running-opa)
- Basic understanding of authorization concepts
- Familiarity with JSON and YAML
- Text editor with Rego syntax support (VS Code, IntelliJ, etc.)

## Policy Structure

### Directory Layout

XZepr policies follow this structure:

```text
config/opa/policies/
├── authz.rego                    # Main authorization entry point
├── event_receiver.rego           # Event receiver permissions
├── event_receiver_group.rego     # Group permissions
├── event.rego                    # Event permissions
├── helpers.rego                  # Shared utility functions
└── test/
    ├── authz_test.rego
    ├── event_receiver_test.rego
    ├── event_receiver_group_test.rego
    ├── event_test.rego
    └── helpers_test.rego
```

### Package Naming

All XZepr policies use the base package:

```rego
package xzepr.authz
```

This allows accessing policies at the OPA endpoint:

```text
POST /v1/data/xzepr/authz/allow
```

## Rego Language Basics

### Policy Structure

A basic policy consists of rules that evaluate to true or false:

```rego
package xzepr.authz

# Default deny - this is crucial for security
default allow = false

# Allow if user is admin
allow {
    input.user.roles[_] == "admin"
}

# Allow if user owns the resource
allow {
    input.user.id == input.resource.owner_id
}
```

### Input Structure

Policies receive structured input from XZepr:

```json
{
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "roles": ["owner"],
    "permissions": ["event_receiver:read"]
  },
  "action": "event_receiver:read",
  "resource": {
    "type": "event_receiver",
    "id": "770e8400-e29b-41d4-a716-446655440000",
    "owner_id": "550e8400-e29b-41d4-a716-446655440000",
    "group_id": "880e8400-e29b-41d4-a716-446655440000",
    "members": ["550e8400-e29b-41d4-a716-446655440000"]
  }
}
```

### Variables and Assignments

```rego
# Simple assignment
user_id := input.user.id

# Multiple assignments
user_id := input.user.id
owner_id := input.resource.owner_id

# Conditional assignment
is_owner {
    input.user.id == input.resource.owner_id
}
```

### Arrays and Iteration

```rego
# Check if value exists in array
allow {
    input.user.roles[_] == "admin"
}

# Iterate over array with index
member_usernames[username] {
    member := input.resource.members[_]
    username := get_username(member)
}

# Check if user is in members list
is_member {
    input.resource.members[_] == input.user.id
}
```

### Objects and Nested Data

```rego
# Access nested fields
allow {
    input.user.metadata.department == "engineering"
}

# Build objects
user_info := {
    "id": input.user.id,
    "roles": input.user.roles,
    "is_admin": is_admin
}
```

### Functions

```rego
# Simple function
is_admin {
    input.user.roles[_] == "admin"
}

# Function with parameters
has_role(role) {
    input.user.roles[_] == role
}

# Function returning value
get_resource_type = type {
    type := input.resource.type
}
```

### Sets

```rego
# Define a set
admin_actions := {"read", "write", "delete", "manage"}

# Check membership
allow {
    admin_actions[input.action]
}
```

### Comprehensions

```rego
# Array comprehension
user_ids := [user.id | user := input.users[_]]

# Set comprehension
admin_ids := {user.id | user := input.users[_]; user.role == "admin"}

# Object comprehension
user_map := {user.id: user.name | user := input.users[_]}
```

## Writing Authorization Policies

### Policy Template

Use this template for new resource types:

```rego
package xzepr.authz

# Resource-specific rules
allow {
    input.action == "resource_type:read"
    can_read_resource
}

allow {
    input.action == "resource_type:update"
    can_update_resource
}

allow {
    input.action == "resource_type:delete"
    can_delete_resource
}

# Permission checks
can_read_resource {
    is_admin
}

can_read_resource {
    is_owner
}

can_read_resource {
    is_member
}

can_update_resource {
    is_admin
}

can_update_resource {
    is_owner
}

can_delete_resource {
    is_admin
}

can_delete_resource {
    is_owner
}

# Helper rules
is_admin {
    input.user.roles[_] == "admin"
}

is_owner {
    input.user.id == input.resource.owner_id
}

is_member {
    input.resource.members[_] == input.user.id
}
```

### Event Receiver Policies

Example event receiver authorization policy:

```rego
package xzepr.authz

# Event Receiver: Read
allow {
    input.action == "event_receiver:read"
    input.resource.type == "event_receiver"
    event_receiver_read_allowed
}

event_receiver_read_allowed {
    is_admin
}

event_receiver_read_allowed {
    is_owner
}

event_receiver_read_allowed {
    is_group_member
}

# Event Receiver: Update
allow {
    input.action == "event_receiver:update"
    input.resource.type == "event_receiver"
    event_receiver_update_allowed
}

event_receiver_update_allowed {
    is_admin
}

event_receiver_update_allowed {
    is_owner
}

# Event Receiver: Delete
allow {
    input.action == "event_receiver:delete"
    input.resource.type == "event_receiver"
    event_receiver_delete_allowed
}

event_receiver_delete_allowed {
    is_admin
}

event_receiver_delete_allowed {
    is_owner
}

# Helper: Check if user is group member
is_group_member {
    input.resource.group_id
    input.resource.members[_] == input.user.id
}
```

### Group Management Policies

Example group membership authorization policy:

```rego
package xzepr.authz

# Group: Add Member
allow {
    input.action == "group:add_member"
    input.resource.type == "event_receiver_group"
    group_add_member_allowed
}

group_add_member_allowed {
    is_admin
}

group_add_member_allowed {
    is_group_owner
}

group_add_member_allowed {
    has_permission("group:manage_members")
}

# Group: Remove Member
allow {
    input.action == "group:remove_member"
    input.resource.type == "event_receiver_group"
    group_remove_member_allowed
}

group_remove_member_allowed {
    is_admin
}

group_remove_member_allowed {
    is_group_owner
}

# Cannot remove group owner
group_remove_member_allowed {
    input.target_user_id != input.resource.owner_id
    has_permission("group:manage_members")
}

# Helper: Check permission
has_permission(permission) {
    input.user.permissions[_] == permission
}

is_group_owner {
    input.user.id == input.resource.owner_id
}
```

## Testing Policies

### Test File Structure

Create test files alongside policies:

```rego
package xzepr.authz

# Test admin can read any resource
test_admin_can_read_event_receiver {
    allow with input as {
        "user": {
            "id": "admin1",
            "roles": ["admin"]
        },
        "action": "event_receiver:read",
        "resource": {
            "type": "event_receiver",
            "owner_id": "user1"
        }
    }
}

# Test owner can read their resource
test_owner_can_read_event_receiver {
    allow with input as {
        "user": {
            "id": "user1",
            "roles": ["owner"]
        },
        "action": "event_receiver:read",
        "resource": {
            "type": "event_receiver",
            "owner_id": "user1"
        }
    }
}

# Test non-owner cannot read
test_non_owner_cannot_read_event_receiver {
    not allow with input as {
        "user": {
            "id": "user2",
            "roles": ["viewer"]
        },
        "action": "event_receiver:read",
        "resource": {
            "type": "event_receiver",
            "owner_id": "user1"
        }
    }
}

# Test member can read group resource
test_member_can_read_group_event_receiver {
    allow with input as {
        "user": {
            "id": "user2",
            "roles": ["member"]
        },
        "action": "event_receiver:read",
        "resource": {
            "type": "event_receiver",
            "owner_id": "user1",
            "group_id": "group1",
            "members": ["user1", "user2"]
        }
    }
}

# Test non-member cannot update
test_non_member_cannot_update_event_receiver {
    not allow with input as {
        "user": {
            "id": "user3",
            "roles": ["viewer"]
        },
        "action": "event_receiver:update",
        "resource": {
            "type": "event_receiver",
            "owner_id": "user1",
            "group_id": "group1",
            "members": ["user1", "user2"]
        }
    }
}
```

### Running Tests

Run tests using the OPA CLI:

```bash
# Run all tests
opa test config/opa/policies/

# Run specific test file
opa test config/opa/policies/test/event_receiver_test.rego

# Run with verbose output
opa test -v config/opa/policies/

# Run with coverage report
opa test --coverage config/opa/policies/

# Run and fail on insufficient coverage
opa test --coverage --threshold 80 config/opa/policies/
```

Expected output:

```text
PASS: 23/23
```

### Test Coverage

Aim for high test coverage:

```bash
# Generate coverage report
opa test --coverage --format=json config/opa/policies/ > coverage.json

# View coverage summary
opa test --coverage config/opa/policies/
```

Target coverage thresholds:

- Overall coverage: >80%
- Critical paths: 100%
- Edge cases: >90%

### Test Best Practices

1. Test both positive and negative cases
2. Test edge cases and boundary conditions
3. Test all permission combinations
4. Use descriptive test names
5. Group related tests together
6. Test helper functions separately

## Local Policy Development

### Step 1: Set Up Development Environment

Create a development directory:

```bash
mkdir -p ~/opa-dev/xzepr-policies
cd ~/opa-dev/xzepr-policies

# Copy existing policies
cp -r /path/to/xzepr/config/opa/policies/* .

# Create test directory if not exists
mkdir -p test
```

### Step 2: Write New Policy

Create a new policy file:

```bash
cat > custom_resource.rego <<'EOF'
package xzepr.authz

# Custom resource authorization
allow {
    input.action == "custom_resource:read"
    input.resource.type == "custom_resource"
    custom_resource_read_allowed
}

custom_resource_read_allowed {
    is_admin
}

custom_resource_read_allowed {
    is_owner
}
EOF
```

### Step 3: Write Tests

Create test file:

```bash
cat > test/custom_resource_test.rego <<'EOF'
package xzepr.authz

test_admin_can_read_custom_resource {
    allow with input as {
        "user": {"id": "admin1", "roles": ["admin"]},
        "action": "custom_resource:read",
        "resource": {"type": "custom_resource", "owner_id": "user1"}
    }
}

test_owner_can_read_custom_resource {
    allow with input as {
        "user": {"id": "user1", "roles": ["owner"]},
        "action": "custom_resource:read",
        "resource": {"type": "custom_resource", "owner_id": "user1"}
    }
}

test_non_owner_cannot_read_custom_resource {
    not allow with input as {
        "user": {"id": "user2", "roles": ["viewer"]},
        "action": "custom_resource:read",
        "resource": {"type": "custom_resource", "owner_id": "user1"}
    }
}
EOF
```

### Step 4: Test Locally

```bash
# Check syntax
opa check *.rego

# Run tests
opa test -v .

# Check coverage
opa test --coverage .
```

### Step 5: Test with Sample Data

Create test input:

```bash
cat > input.json <<'EOF'
{
  "user": {
    "id": "user1",
    "roles": ["owner"]
  },
  "action": "custom_resource:read",
  "resource": {
    "type": "custom_resource",
    "owner_id": "user1"
  }
}
EOF
```

Evaluate policy:

```bash
# Evaluate with OPA CLI
opa eval -d . -i input.json 'data.xzepr.authz.allow'

# Expected output:
# {
#   "result": [
#     {
#       "expressions": [
#         {
#           "value": true,
#           "text": "data.xzepr.authz.allow",
#           "location": {
#             "row": 1,
#             "col": 1
#           }
#         }
#       ]
#     }
#   ]
# }
```

### Step 6: Test with OPA Server

Start OPA server with policies:

```bash
# Start OPA server
opa run --server --addr=localhost:8181 .

# In another terminal, test policy
curl -X POST http://localhost:8181/v1/data/xzepr/authz/allow \
  -H "Content-Type: application/json" \
  -d @input.json
```

## Debugging Policies

### Print Debugging

Add trace statements to debug:

```rego
allow {
    trace(sprintf("User ID: %v", [input.user.id]))
    trace(sprintf("Action: %v", [input.action]))
    trace(sprintf("Resource Owner: %v", [input.resource.owner_id]))

    input.user.id == input.resource.owner_id
}
```

Run with trace output:

```bash
opa eval --explain=full -d . -i input.json 'data.xzepr.authz.allow'
```

### Interactive Debugging

Use OPA REPL for interactive testing:

```bash
# Start REPL with policies loaded
opa run .

# In REPL, set input and evaluate
> input := {"user": {"id": "user1"}, "action": "read"}
> data.xzepr.authz.allow
false

> input.user.roles := ["admin"]
> data.xzepr.authz.allow
true
```

### Explain Mode

Use explain mode to understand policy evaluation:

```bash
# Full explanation
opa eval --explain=full -d . -i input.json 'data.xzepr.authz.allow'

# Notes explanation (concise)
opa eval --explain=notes -d . -i input.json 'data.xzepr.authz.allow'
```

## Policy Deployment

### Step 1: Validate Policies

Before deployment, validate thoroughly:

```bash
# Check syntax
opa check config/opa/policies/*.rego

# Run all tests
opa test config/opa/policies/

# Verify coverage
opa test --coverage --threshold 80 config/opa/policies/

# Test with real-world inputs
opa eval -d config/opa/policies/ -i test-inputs/*.json 'data.xzepr.authz.allow'
```

### Step 2: Build Bundle

Create policy bundle for deployment:

```bash
cd config/opa/policies/

# Create manifest
cat > .manifest <<EOF
{
  "revision": "$(date +%Y%m%d-%H%M%S)",
  "roots": ["xzepr"],
  "metadata": {
    "version": "1.0.0",
    "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  }
}
EOF

# Build bundle
opa build -b . -o xzepr-policies.tar.gz

# Verify bundle
tar -tzf xzepr-policies.tar.gz
```

### Step 3: Deploy to OPA Server

Upload bundle to bundle server:

```bash
# Upload to bundle server
scp xzepr-policies.tar.gz user@bundle-server:/opt/opa-bundles/

# Or upload to S3
aws s3 cp xzepr-policies.tar.gz s3://xzepr-opa-bundles/
```

### Step 4: Verify Deployment

Check OPA loaded the new policies:

```bash
# Check OPA health
curl http://opa-server:8181/health

# List loaded policies
curl http://opa-server:8181/v1/policies

# Test a policy
curl -X POST http://opa-server:8181/v1/data/xzepr/authz/allow \
  -H "Content-Type: application/json" \
  -d '{
    "input": {
      "user": {"id": "user1", "roles": ["owner"]},
      "action": "event_receiver:read",
      "resource": {"type": "event_receiver", "owner_id": "user1"}
    }
  }'
```

## Best Practices

### Security Best Practices

1. **Default Deny**: Always start with `default allow = false`
2. **Explicit Permissions**: Require explicit permission for each action
3. **Input Validation**: Validate all input fields exist
4. **Least Privilege**: Grant minimum necessary permissions
5. **Test Negative Cases**: Test denial scenarios thoroughly

### Performance Best Practices

1. **Early Exit**: Place most specific rules first
2. **Avoid Redundancy**: Extract common logic to helper functions
3. **Minimize Iteration**: Use sets instead of arrays when possible
4. **Cache Results**: Use comprehensions for expensive operations
5. **Profile Policies**: Use `opa eval --profile` to identify bottlenecks

### Maintainability Best Practices

1. **Clear Naming**: Use descriptive rule and variable names
2. **Comments**: Document complex logic
3. **Modularity**: Split policies by resource type
4. **Consistent Style**: Follow team coding standards
5. **Version Control**: Track policy changes in git

### Testing Best Practices

1. **Comprehensive Coverage**: Test all permission paths
2. **Edge Cases**: Test boundary conditions
3. **Negative Tests**: Test denial scenarios
4. **Integration Tests**: Test with real OPA server
5. **Regression Tests**: Add tests for fixed bugs

## Common Patterns

### Permission Checking

```rego
# Check user has specific permission
has_permission(permission) {
    input.user.permissions[_] == permission
}

# Check user has any of multiple permissions
has_any_permission(permissions) {
    permission := permissions[_]
    input.user.permissions[_] == permission
}

# Check user has all required permissions
has_all_permissions(permissions) {
    required := {p | p := permissions[_]}
    granted := {p | p := input.user.permissions[_]}
    required == granted
}
```

### Role Checking

```rego
# Check user has role
has_role(role) {
    input.user.roles[_] == role
}

# Check user has any role
has_any_role(roles) {
    role := roles[_]
    input.user.roles[_] == role
}

# Check role hierarchy
is_privileged_user {
    has_any_role(["admin", "superuser", "operator"])
}
```

### Resource Ownership

```rego
# Direct ownership
is_owner {
    input.user.id == input.resource.owner_id
}

# Nested ownership (e.g., comment owner is post owner)
is_resource_owner {
    resource := get_parent_resource(input.resource.id)
    resource.owner_id == input.user.id
}

# Delegated ownership
is_delegate {
    owner := get_resource_owner(input.resource.id)
    owner.delegates[_] == input.user.id
}
```

## Troubleshooting

### Policy Not Evaluating

Check these common issues:

1. Package name mismatch
2. Rule conditions not met
3. Input structure incorrect
4. Missing default deny rule

### Tests Failing

Debug test failures:

```bash
# Run tests with verbose output
opa test -v config/opa/policies/

# Run specific test
opa test config/opa/policies/test/event_receiver_test.rego

# Use explain mode
opa eval --explain=notes -d config/opa/policies/ \
  -i test-input.json 'data.xzepr.authz.allow'
```

### Performance Issues

Profile policy performance:

```bash
# Profile policy evaluation
opa eval --profile -d config/opa/policies/ \
  -i input.json 'data.xzepr.authz.allow'

# Identify slow rules
opa eval --profile --profile-sort=total_time_ns \
  -d config/opa/policies/ -i input.json 'data.xzepr.authz.allow'
```

## Related Documentation

- [OPA Authorization Architecture](../explanation/opa_authorization_architecture.md)
- [OPA Bundle Server Setup](opa_bundle_server_setup.md)
- [Group Membership API Reference](../reference/group_membership_api.md)

## References

- OPA Documentation: https://www.openpolicyagent.org/docs/latest/
- Rego Language: https://www.openpolicyagent.org/docs/latest/policy-language/
- Policy Testing: https://www.openpolicyagent.org/docs/latest/policy-testing/
- Policy Performance: https://www.openpolicyagent.org/docs/latest/policy-performance/
