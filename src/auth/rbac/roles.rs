// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/auth/rbac/roles.rs
use super::permissions::Permission;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    EventManager,
    EventViewer,
    User,
}

impl Role {
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Role::Admin => vec![
                Permission::EventCreate,
                Permission::EventRead,
                Permission::EventUpdate,
                Permission::EventDelete,
                Permission::ReceiverCreate,
                Permission::ReceiverRead,
                Permission::ReceiverUpdate,
                Permission::ReceiverDelete,
                Permission::GroupCreate,
                Permission::GroupRead,
                Permission::GroupUpdate,
                Permission::GroupDelete,
                Permission::UserManage,
                Permission::RoleManage,
            ],
            Role::EventManager => vec![
                Permission::EventCreate,
                Permission::EventRead,
                Permission::EventUpdate,
                Permission::ReceiverCreate,
                Permission::ReceiverRead,
                Permission::ReceiverUpdate,
                Permission::GroupCreate,
                Permission::GroupRead,
                Permission::GroupUpdate,
            ],
            Role::EventViewer => vec![
                Permission::EventRead,
                Permission::ReceiverRead,
                Permission::GroupRead,
            ],
            Role::User => vec![Permission::EventRead],
        }
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions().contains(permission)
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::EventManager => write!(f, "event_manager"),
            Role::EventViewer => write!(f, "event_viewer"),
            Role::User => write!(f, "user"),
        }
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Role::Admin),
            "event_manager" => Ok(Role::EventManager),
            "event_viewer" => Ok(Role::EventViewer),
            "user" => Ok(Role::User),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}
