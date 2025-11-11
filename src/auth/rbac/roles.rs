// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_role_permissions() {
        let role = Role::Admin;
        let perms = role.permissions();

        assert!(perms.contains(&Permission::EventCreate));
        assert!(perms.contains(&Permission::EventRead));
        assert!(perms.contains(&Permission::EventUpdate));
        assert!(perms.contains(&Permission::EventDelete));
        assert!(perms.contains(&Permission::ReceiverCreate));
        assert!(perms.contains(&Permission::ReceiverRead));
        assert!(perms.contains(&Permission::ReceiverUpdate));
        assert!(perms.contains(&Permission::ReceiverDelete));
        assert!(perms.contains(&Permission::GroupCreate));
        assert!(perms.contains(&Permission::GroupRead));
        assert!(perms.contains(&Permission::GroupUpdate));
        assert!(perms.contains(&Permission::GroupDelete));
        assert!(perms.contains(&Permission::UserManage));
        assert!(perms.contains(&Permission::RoleManage));
    }

    #[test]
    fn test_event_manager_role_permissions() {
        let role = Role::EventManager;
        let perms = role.permissions();

        assert!(perms.contains(&Permission::EventCreate));
        assert!(perms.contains(&Permission::EventRead));
        assert!(perms.contains(&Permission::EventUpdate));
        assert!(!perms.contains(&Permission::EventDelete));
        assert!(perms.contains(&Permission::ReceiverCreate));
        assert!(perms.contains(&Permission::ReceiverRead));
        assert!(perms.contains(&Permission::ReceiverUpdate));
        assert!(!perms.contains(&Permission::ReceiverDelete));
        assert!(perms.contains(&Permission::GroupCreate));
        assert!(perms.contains(&Permission::GroupRead));
        assert!(perms.contains(&Permission::GroupUpdate));
        assert!(!perms.contains(&Permission::GroupDelete));
        assert!(!perms.contains(&Permission::UserManage));
        assert!(!perms.contains(&Permission::RoleManage));
    }

    #[test]
    fn test_event_viewer_role_permissions() {
        let role = Role::EventViewer;
        let perms = role.permissions();

        assert!(!perms.contains(&Permission::EventCreate));
        assert!(perms.contains(&Permission::EventRead));
        assert!(!perms.contains(&Permission::EventUpdate));
        assert!(!perms.contains(&Permission::EventDelete));
        assert!(!perms.contains(&Permission::ReceiverCreate));
        assert!(perms.contains(&Permission::ReceiverRead));
        assert!(!perms.contains(&Permission::ReceiverUpdate));
        assert!(!perms.contains(&Permission::ReceiverDelete));
        assert!(!perms.contains(&Permission::GroupCreate));
        assert!(perms.contains(&Permission::GroupRead));
        assert!(!perms.contains(&Permission::GroupUpdate));
        assert!(!perms.contains(&Permission::GroupDelete));
        assert!(!perms.contains(&Permission::UserManage));
        assert!(!perms.contains(&Permission::RoleManage));
    }

    #[test]
    fn test_user_role_permissions() {
        let role = Role::User;
        let perms = role.permissions();

        assert!(!perms.contains(&Permission::EventCreate));
        assert!(perms.contains(&Permission::EventRead));
        assert!(!perms.contains(&Permission::EventUpdate));
        assert!(!perms.contains(&Permission::EventDelete));
        assert!(!perms.contains(&Permission::ReceiverCreate));
        assert!(!perms.contains(&Permission::ReceiverRead));
        assert!(!perms.contains(&Permission::UserManage));
    }

    #[test]
    fn test_has_permission_admin() {
        let role = Role::Admin;
        assert!(role.has_permission(&Permission::EventCreate));
        assert!(role.has_permission(&Permission::UserManage));
        assert!(role.has_permission(&Permission::RoleManage));
    }

    #[test]
    fn test_has_permission_event_manager() {
        let role = Role::EventManager;
        assert!(role.has_permission(&Permission::EventCreate));
        assert!(role.has_permission(&Permission::EventRead));
        assert!(!role.has_permission(&Permission::UserManage));
    }

    #[test]
    fn test_has_permission_event_viewer() {
        let role = Role::EventViewer;
        assert!(!role.has_permission(&Permission::EventCreate));
        assert!(role.has_permission(&Permission::EventRead));
        assert!(!role.has_permission(&Permission::UserManage));
    }

    #[test]
    fn test_has_permission_user() {
        let role = Role::User;
        assert!(!role.has_permission(&Permission::EventCreate));
        assert!(role.has_permission(&Permission::EventRead));
        assert!(!role.has_permission(&Permission::UserManage));
    }

    #[test]
    fn test_role_display_admin() {
        let role = Role::Admin;
        assert_eq!(role.to_string(), "admin");
    }

    #[test]
    fn test_role_display_event_manager() {
        let role = Role::EventManager;
        assert_eq!(role.to_string(), "event_manager");
    }

    #[test]
    fn test_role_display_event_viewer() {
        let role = Role::EventViewer;
        assert_eq!(role.to_string(), "event_viewer");
    }

    #[test]
    fn test_role_display_user() {
        let role = Role::User;
        assert_eq!(role.to_string(), "user");
    }

    #[test]
    fn test_role_from_str_admin() {
        let role = Role::from_str("admin");
        assert!(role.is_ok());
        assert_eq!(role.unwrap(), Role::Admin);
    }

    #[test]
    fn test_role_from_str_event_manager() {
        let role = Role::from_str("event_manager");
        assert!(role.is_ok());
        assert_eq!(role.unwrap(), Role::EventManager);
    }

    #[test]
    fn test_role_from_str_event_viewer() {
        let role = Role::from_str("event_viewer");
        assert!(role.is_ok());
        assert_eq!(role.unwrap(), Role::EventViewer);
    }

    #[test]
    fn test_role_from_str_user() {
        let role = Role::from_str("user");
        assert!(role.is_ok());
        assert_eq!(role.unwrap(), Role::User);
    }

    #[test]
    fn test_role_from_str_invalid() {
        let role = Role::from_str("invalid_role");
        assert!(role.is_err());
    }

    #[test]
    fn test_role_from_str_case_insensitive() {
        assert_eq!(Role::from_str("ADMIN").unwrap(), Role::Admin);
        assert_eq!(Role::from_str("Event_Manager").unwrap(), Role::EventManager);
        assert_eq!(Role::from_str("EVENT_VIEWER").unwrap(), Role::EventViewer);
        assert_eq!(Role::from_str("USER").unwrap(), Role::User);
    }

    #[test]
    fn test_role_equality() {
        assert_eq!(Role::Admin, Role::Admin);
        assert_ne!(Role::Admin, Role::User);
    }

    #[test]
    fn test_role_clone() {
        let role1 = Role::Admin;
        let role2 = role1;
        assert_eq!(role1, role2);
    }

    #[test]
    fn test_role_serialization() {
        let role = Role::Admin;
        let serialized = serde_json::to_string(&role);
        assert!(serialized.is_ok());
    }

    #[test]
    fn test_role_deserialization() {
        let json = "\"Admin\"";
        let role: Result<Role, _> = serde_json::from_str(json);
        assert!(role.is_ok());
        assert_eq!(role.unwrap(), Role::Admin);
    }
}
