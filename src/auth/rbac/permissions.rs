// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! RBAC permission variants.
//!
//! Each [`Permission`] represents a single fine-grained capability that can be
//! granted to a [`super::roles::Role`].  Permissions use a lowercase
//! snake_case canonical string form (e.g. `"event_create"`) for [`Display`]
//! and [`std::str::FromStr`] while preserving the PascalCase Serde
//! representation (e.g. `"EventCreate"`) that existing serialised data depends
//! upon.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error returned when parsing a permission string fails.
///
/// Produced by [`Permission`]'s [`std::str::FromStr`] implementation when the
/// input does not match any known variant.
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use xzepr::auth::rbac::permissions::{Permission, PermissionParseError};
///
/// let err = Permission::from_str("no_such_perm").unwrap_err();
/// assert!(matches!(err, PermissionParseError::UnknownPermission(_)));
/// assert!(err.to_string().contains("no_such_perm"));
/// ```
#[derive(Error, Debug, PartialEq, Clone)]
pub enum PermissionParseError {
    /// The supplied string did not match any known permission variant.
    #[error("Unknown permission: '{0}'")]
    UnknownPermission(String),
}

/// Fine-grained capability that can be assigned to a role.
///
/// Permissions are grouped by resource type (events, receivers, groups) plus
/// cross-cutting administrative capabilities (`UserManage`, `RoleManage`).
///
/// # String representation
///
/// - [`Display`] / [`std::str::FromStr`]: lowercase snake_case (`"event_create"`)
/// - Serde: PascalCase derived representation (`"EventCreate"`) for wire/storage
///   compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // -- Event permissions --
    /// Allows creating new events.
    EventCreate,
    /// Allows reading existing events.
    EventRead,
    /// Allows updating existing events.
    EventUpdate,
    /// Allows deleting existing events.
    EventDelete,

    // -- Receiver permissions --
    /// Allows creating new event receivers.
    ReceiverCreate,
    /// Allows reading existing event receivers.
    ReceiverRead,
    /// Allows updating existing event receivers.
    ReceiverUpdate,
    /// Allows deleting existing event receivers.
    ReceiverDelete,

    // -- Group permissions --
    /// Allows creating new event receiver groups.
    GroupCreate,
    /// Allows reading existing event receiver groups.
    GroupRead,
    /// Allows updating existing event receiver groups.
    GroupUpdate,
    /// Allows deleting existing event receiver groups.
    GroupDelete,

    // -- Administrative permissions --
    /// Allows managing user accounts.
    UserManage,
    /// Allows managing role assignments.
    RoleManage,
}

impl Permission {
    /// Map a `(resource, action)` pair to the corresponding [`Permission`], if any.
    ///
    /// Returns `None` if the combination is not recognised.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource type string (e.g., `"event"`, `"receiver"`, `"group"`).
    /// * `action`   - The action string (e.g., `"create"`, `"read"`, `"update"`, `"delete"`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xzepr::auth::rbac::permissions::Permission;
    ///
    /// assert_eq!(Permission::from_action("event", "create"), Some(Permission::EventCreate));
    /// assert_eq!(Permission::from_action("unknown", "create"), None);
    /// ```
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

/// Canonical lowercase snake_case display for a permission.
///
/// Note: this is intentionally different from the Serde representation
/// (`"EventCreate"`) which preserves backward compatibility with stored data.
impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Permission::EventCreate => write!(f, "event_create"),
            Permission::EventRead => write!(f, "event_read"),
            Permission::EventUpdate => write!(f, "event_update"),
            Permission::EventDelete => write!(f, "event_delete"),
            Permission::ReceiverCreate => write!(f, "receiver_create"),
            Permission::ReceiverRead => write!(f, "receiver_read"),
            Permission::ReceiverUpdate => write!(f, "receiver_update"),
            Permission::ReceiverDelete => write!(f, "receiver_delete"),
            Permission::GroupCreate => write!(f, "group_create"),
            Permission::GroupRead => write!(f, "group_read"),
            Permission::GroupUpdate => write!(f, "group_update"),
            Permission::GroupDelete => write!(f, "group_delete"),
            Permission::UserManage => write!(f, "user_manage"),
            Permission::RoleManage => write!(f, "role_manage"),
        }
    }
}

/// Parse a permission from its canonical string representation.
///
/// Accepts lowercase snake_case (`"event_create"`), camelCase without
/// separators (`"eventcreate"`), and is case-insensitive.  Hyphens are
/// treated the same as underscores to accommodate various serialisation
/// conventions.
///
/// # Errors
///
/// Returns [`PermissionParseError::UnknownPermission`] for any string that
/// does not map to a known variant.
impl std::str::FromStr for Permission {
    type Err = PermissionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "event_create" | "eventcreate" => Ok(Permission::EventCreate),
            "event_read" | "eventread" => Ok(Permission::EventRead),
            "event_update" | "eventupdate" => Ok(Permission::EventUpdate),
            "event_delete" | "eventdelete" => Ok(Permission::EventDelete),
            "receiver_create" | "receivercreate" => Ok(Permission::ReceiverCreate),
            "receiver_read" | "receiverread" => Ok(Permission::ReceiverRead),
            "receiver_update" | "receiverupdate" => Ok(Permission::ReceiverUpdate),
            "receiver_delete" | "receiverdelete" => Ok(Permission::ReceiverDelete),
            "group_create" | "groupcreate" => Ok(Permission::GroupCreate),
            "group_read" | "groupread" => Ok(Permission::GroupRead),
            "group_update" | "groupupdate" => Ok(Permission::GroupUpdate),
            "group_delete" | "groupdelete" => Ok(Permission::GroupDelete),
            "user_manage" | "usermanage" => Ok(Permission::UserManage),
            "role_manage" | "rolemanage" => Ok(Permission::RoleManage),
            _ => Err(PermissionParseError::UnknownPermission(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
        // Serde uses the derived PascalCase representation, NOT the Display form.
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

    // -- Display tests --

    #[test]
    fn test_permission_display_event_create() {
        assert_eq!(Permission::EventCreate.to_string(), "event_create");
    }

    #[test]
    fn test_permission_display_role_manage() {
        assert_eq!(Permission::RoleManage.to_string(), "role_manage");
    }

    #[test]
    fn test_permission_display_all_variants_snake_case() {
        // Verify every variant produces a non-empty lowercase snake_case string.
        let variants = [
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
        ];
        for v in &variants {
            let s = v.to_string();
            assert!(!s.is_empty(), "Display must not be empty for {:?}", v);
            assert_eq!(s, s.to_lowercase(), "Display must be lowercase for {:?}", v);
            assert!(
                s.contains('_') || !s.contains(char::is_uppercase),
                "Display must use snake_case for {:?}",
                v
            );
        }
    }

    // -- FromStr tests --

    #[test]
    fn test_permission_from_str_event_create() {
        let perm = Permission::from_str("event_create");
        assert!(perm.is_ok());
        assert_eq!(perm.unwrap(), Permission::EventCreate);
    }

    #[test]
    fn test_permission_from_str_case_insensitive() {
        assert_eq!(
            Permission::from_str("EVENT_CREATE").unwrap(),
            Permission::EventCreate
        );
        assert_eq!(
            Permission::from_str("Event_Read").unwrap(),
            Permission::EventRead
        );
    }

    #[test]
    fn test_permission_from_str_hyphen_treated_as_underscore() {
        assert_eq!(
            Permission::from_str("event-create").unwrap(),
            Permission::EventCreate
        );
    }

    #[test]
    fn test_permission_from_str_camel_case_alias() {
        assert_eq!(
            Permission::from_str("eventcreate").unwrap(),
            Permission::EventCreate
        );
        assert_eq!(
            Permission::from_str("rolemanage").unwrap(),
            Permission::RoleManage
        );
    }

    #[test]
    fn test_permission_from_str_invalid() {
        let result = Permission::from_str("no_such_permission");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PermissionParseError::UnknownPermission(_)
        ));
    }

    #[test]
    fn test_permission_parse_error_display() {
        let err = PermissionParseError::UnknownPermission("bad_perm".to_string());
        assert!(err.to_string().contains("bad_perm"));
    }

    #[test]
    fn test_permission_roundtrip_display_from_str() {
        // Display -> FromStr roundtrip must be lossless for every variant.
        let variants = [
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
        ];
        for v in &variants {
            let s = v.to_string();
            let parsed =
                Permission::from_str(&s).unwrap_or_else(|_| panic!("roundtrip failed for {:?}", v));
            assert_eq!(*v, parsed, "roundtrip mismatch for {:?}", v);
        }
    }
}
