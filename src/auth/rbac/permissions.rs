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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_event_create() {
        let perm = Permission::from_action("event", "create");
        assert_eq!(perm, Some(Permission::EventCreate));
    }

    #[test]
    fn test_permission_event_read() {
        let perm = Permission::from_action("event", "read");
        assert_eq!(perm, Some(Permission::EventRead));
    }

    #[test]
    fn test_permission_event_update() {
        let perm = Permission::from_action("event", "update");
        assert_eq!(perm, Some(Permission::EventUpdate));
    }

    #[test]
    fn test_permission_event_delete() {
        let perm = Permission::from_action("event", "delete");
        assert_eq!(perm, Some(Permission::EventDelete));
    }

    #[test]
    fn test_permission_receiver_create() {
        let perm = Permission::from_action("receiver", "create");
        assert_eq!(perm, Some(Permission::ReceiverCreate));
    }

    #[test]
    fn test_permission_receiver_read() {
        let perm = Permission::from_action("receiver", "read");
        assert_eq!(perm, Some(Permission::ReceiverRead));
    }

    #[test]
    fn test_permission_receiver_update() {
        let perm = Permission::from_action("receiver", "update");
        assert_eq!(perm, Some(Permission::ReceiverUpdate));
    }

    #[test]
    fn test_permission_receiver_delete() {
        let perm = Permission::from_action("receiver", "delete");
        assert_eq!(perm, Some(Permission::ReceiverDelete));
    }

    #[test]
    fn test_permission_group_create() {
        let perm = Permission::from_action("group", "create");
        assert_eq!(perm, Some(Permission::GroupCreate));
    }

    #[test]
    fn test_permission_group_read() {
        let perm = Permission::from_action("group", "read");
        assert_eq!(perm, Some(Permission::GroupRead));
    }

    #[test]
    fn test_permission_group_update() {
        let perm = Permission::from_action("group", "update");
        assert_eq!(perm, Some(Permission::GroupUpdate));
    }

    #[test]
    fn test_permission_group_delete() {
        let perm = Permission::from_action("group", "delete");
        assert_eq!(perm, Some(Permission::GroupDelete));
    }

    #[test]
    fn test_permission_invalid_resource() {
        let perm = Permission::from_action("invalid", "read");
        assert_eq!(perm, None);
    }

    #[test]
    fn test_permission_invalid_action() {
        let perm = Permission::from_action("event", "invalid");
        assert_eq!(perm, None);
    }

    #[test]
    fn test_permission_invalid_both() {
        let perm = Permission::from_action("invalid", "invalid");
        assert_eq!(perm, None);
    }

    #[test]
    fn test_permission_equality() {
        assert_eq!(Permission::EventCreate, Permission::EventCreate);
        assert_ne!(Permission::EventCreate, Permission::EventRead);
    }

    #[test]
    fn test_permission_clone() {
        let perm1 = Permission::EventCreate;
        let perm2 = perm1;
        assert_eq!(perm1, perm2);
    }

    #[test]
    fn test_permission_serialization() {
        let perm = Permission::EventCreate;
        let serialized = serde_json::to_string(&perm);
        assert!(serialized.is_ok());
    }

    #[test]
    fn test_permission_deserialization() {
        let json = "\"EventCreate\"";
        let perm: Result<Permission, _> = serde_json::from_str(json);
        assert!(perm.is_ok());
        assert_eq!(perm.unwrap(), Permission::EventCreate);
    }

    #[test]
    fn test_all_event_permissions() {
        assert!(Permission::from_action("event", "create").is_some());
        assert!(Permission::from_action("event", "read").is_some());
        assert!(Permission::from_action("event", "update").is_some());
        assert!(Permission::from_action("event", "delete").is_some());
    }

    #[test]
    fn test_all_receiver_permissions() {
        assert!(Permission::from_action("receiver", "create").is_some());
        assert!(Permission::from_action("receiver", "read").is_some());
        assert!(Permission::from_action("receiver", "update").is_some());
        assert!(Permission::from_action("receiver", "delete").is_some());
    }

    #[test]
    fn test_all_group_permissions() {
        assert!(Permission::from_action("group", "create").is_some());
        assert!(Permission::from_action("group", "read").is_some());
        assert!(Permission::from_action("group", "update").is_some());
        assert!(Permission::from_action("group", "delete").is_some());
    }
}
