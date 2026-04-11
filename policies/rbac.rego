# SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
# SPDX-License-Identifier: Apache-2.0

# XZepr RBAC Policy
#
# This policy implements role-based access control with resource ownership
# and group membership for the XZepr event tracking system.

package xzepr.rbac

default allow = false

# Allow if user is an admin
allow {
    input.user.roles[_] == "admin"
}

# Allow if user is the owner of the resource
allow {
    is_owner
}

# Allow if user is a group member and action is allowed for members
allow {
    is_group_member
    member_actions[input.action]
}

# Check if user is the resource owner
is_owner {
    input.resource.owner_id == input.user.user_id
}

# Check if user is a group member
is_group_member {
    input.resource.group_id != null
    input.resource.members[_] == input.user.user_id
}

# Actions allowed for group members
member_actions = {
    "read": true,
    "create": true,
}

# Admin users can perform all actions
allow {
    input.user.roles[_] == "admin"
    admin_actions[input.action]
}

admin_actions = {
    "create": true,
    "read": true,
    "update": true,
    "delete": true,
    "manage_members": true,
}

# Owner can perform owner actions
allow {
    is_owner
    owner_actions[input.action]
}

owner_actions = {
    "create": true,
    "read": true,
    "update": true,
    "delete": true,
    "manage_members": true,
}

# Provide reason for the decision
reason = "User is admin" {
    input.user.roles[_] == "admin"
}

reason = "User is owner" {
    is_owner
}

reason = "User is group member" {
    is_group_member
}

reason = "Access denied" {
    not allow
}

# Metadata about the decision
metadata = {
    "evaluated_at": time.now_ns(),
    "policy_version": "1.0.0",
    "resource_type": input.resource.resource_type,
    "action": input.action,
}
