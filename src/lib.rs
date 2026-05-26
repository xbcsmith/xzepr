// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! XZepr event tracking server library.
//!
//! This crate provides the core XZepr functionality including domain entities,
//! application services, authentication, and infrastructure adapters.
//!
//! # Stable Public API
//!
//! The following re-exports form the intentionally stable crate root:
//!
//! - Domain entities: [`domain::entities`] (Event, EventReceiver, EventReceiverGroup, User)
//! - Value objects: [`domain::value_objects`] (typed IDs)
//! - Auth service: [`auth::api_key::ApiKeyService`]
//! - Auth types: [`auth::rbac`] (Role, Permission)
//! - Configuration: [`infrastructure::config::Settings`]
//! - Error types: [`error::Error`], [`error::Result`]
//!
//! All other types are accessible via their full module paths.

pub mod api;
pub mod application;
pub mod auth;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod opa;

// Stable crate-root re-exports
pub use auth::api_key::ApiKeyService;
pub use auth::rbac::permissions::{Permission, PermissionParseError};
pub use auth::rbac::roles::{Role, RoleParseError};
pub use domain::entities::{
    event::Event, event_receiver::EventReceiver, event_receiver_group::EventReceiverGroup,
    user::User,
};
pub use domain::value_objects::{ApiKeyId, EventId, EventReceiverGroupId, EventReceiverId, UserId};
pub use error::{Error, Result};
pub use infrastructure::config::Settings;

/// Compile-time API surface tests verifying that the stable crate root
/// exports are accessible.
///
/// These tests do not execute meaningful logic; they exist to produce
/// compile errors if a deliberately stable export is accidentally removed.
#[cfg(test)]
mod api_surface_tests {
    use super::*;

    /// Verifies error types are re-exported at the crate root.
    #[test]
    fn test_error_types_are_exported() {
        let _: Result<()> = Ok(());
        let _err: Error = Error::Internal {
            message: "surface test".to_string(),
        };
    }

    /// Verifies value-object ID types are re-exported at the crate root.
    #[test]
    fn test_value_objects_are_exported() {
        let _id = EventId::new();
        let _id = UserId::new();
        let _id = EventReceiverId::new();
        let _id = EventReceiverGroupId::new();
        let _id = ApiKeyId::new();
    }

    /// Verifies RBAC types are re-exported at the crate root.
    #[test]
    fn test_auth_types_are_exported() {
        let _role: Role = Role::User;
        let _perm: Permission = Permission::EventRead;
    }

    /// Verifies Settings is re-exported at the crate root.
    #[test]
    fn test_settings_is_exported() {
        let _ = std::any::TypeId::of::<Settings>();
    }

    /// Verifies ApiKeyService is re-exported at the crate root.
    #[test]
    fn test_api_key_service_is_exported() {
        let _ = std::any::TypeId::of::<ApiKeyService>();
    }
}
