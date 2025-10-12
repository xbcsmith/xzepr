// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/auth/rbac/permissions.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Event permissions
    EventCreate,
    EventRead,
    EventUpdate,
    EventDelete,

    // Receiver permissions
    ReceiverCreate,
    ReceiverRead,
    ReceiverUpdate,
    ReceiverDelete,

    // Group permissions
    GroupCreate,
    GroupRead,
    GroupUpdate,
    GroupDelete,

    // Admin permissions
    UserManage,
    RoleManage,
}

impl Permission {
    pub fn from_action(resource: &str, action: &str) -> Option<Self> {
        match (resource, action) {
            ("event", "create") => Some(Permission::EventCreate),
            ("event", "read") => Some(Permission::EventRead),
            ("event", "update") => Some(Permission::EventUpdate),
            ("event", "delete") => Some(Permission::EventDelete),
            ("receiver", "create") => Some(Permission::ReceiverCreate),
            ("receiver", "read") => Some(Permission::ReceiverRead),
            ("receiver", "update") => Some(Permission::ReceiverUpdate),
            ("receiver", "delete") => Some(Permission::ReceiverDelete),
            ("group", "create") => Some(Permission::GroupCreate),
            ("group", "read") => Some(Permission::GroupRead),
            ("group", "update") => Some(Permission::GroupUpdate),
            ("group", "delete") => Some(Permission::GroupDelete),
            _ => None,
        }
    }
}
