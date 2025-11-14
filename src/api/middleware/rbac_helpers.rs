// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! RBAC Helper Functions for REST API
//!
//! This module provides helper functions for mapping HTTP methods and routes
//! to required permissions for RBAC enforcement in REST endpoints.

use crate::auth::rbac::permissions::Permission;
use axum::http::Method;

/// Maps HTTP method and route path to required permission
///
/// This function determines which permission is required based on the
/// HTTP method and the resource being accessed.
///
/// # Arguments
///
/// * `method` - The HTTP method (GET, POST, PUT, DELETE)
/// * `path` - The API route path (e.g., "/api/v1/events")
///
/// # Returns
///
/// Returns `Some(Permission)` if the route requires a specific permission,
/// or `None` if the route is public or doesn't map to a known permission.
///
/// # Examples
///
/// ```
/// use axum::http::Method;
/// use xzepr::api::middleware::rbac_helpers::route_to_permission;
/// use xzepr::auth::rbac::permissions::Permission;
///
/// let perm = route_to_permission(&Method::POST, "/api/v1/events");
/// assert_eq!(perm, Some(Permission::EventCreate));
///
/// let perm = route_to_permission(&Method::GET, "/api/v1/receivers/123");
/// assert_eq!(perm, Some(Permission::ReceiverRead));
/// ```
pub fn route_to_permission(method: &Method, path: &str) -> Option<Permission> {
    // Health check is always public
    if path == "/health" || path.starts_with("/graphql") {
        return None;
    }

    // Determine resource type from path
    let resource = if path.contains("/events") {
        "event"
    } else if path.contains("/receivers") {
        "receiver"
    } else if path.contains("/groups") {
        "group"
    } else {
        return None;
    };

    // Map HTTP method to action
    let action = match method {
        &Method::GET => "read",
        &Method::POST => "create",
        &Method::PUT | &Method::PATCH => "update",
        &Method::DELETE => "delete",
        _ => return None,
    };

    Permission::from_action(resource, action)
}

/// Helper to get all permissions for a specific resource
///
/// Returns a vector of all CRUD permissions for the given resource type.
///
/// # Arguments
///
/// * `resource` - The resource type ("event", "receiver", or "group")
///
/// # Returns
///
/// A vector of permissions for the resource, or empty vector if resource is unknown.
///
/// # Examples
///
/// ```
/// use xzepr::api::middleware::rbac_helpers::get_resource_permissions;
/// use xzepr::auth::rbac::permissions::Permission;
///
/// let perms = get_resource_permissions("event");
/// assert!(perms.contains(&Permission::EventCreate));
/// assert!(perms.contains(&Permission::EventRead));
/// assert_eq!(perms.len(), 4);
/// ```
pub fn get_resource_permissions(resource: &str) -> Vec<Permission> {
    match resource {
        "event" => vec![
            Permission::EventCreate,
            Permission::EventRead,
            Permission::EventUpdate,
            Permission::EventDelete,
        ],
        "receiver" => vec![
            Permission::ReceiverCreate,
            Permission::ReceiverRead,
            Permission::ReceiverUpdate,
            Permission::ReceiverDelete,
        ],
        "group" => vec![
            Permission::GroupCreate,
            Permission::GroupRead,
            Permission::GroupUpdate,
            Permission::GroupDelete,
        ],
        _ => vec![],
    }
}

/// Check if a route is public (doesn't require authentication)
///
/// # Arguments
///
/// * `path` - The API route path
///
/// # Returns
///
/// `true` if the route is public, `false` otherwise
///
/// # Examples
///
/// ```
/// use xzepr::api::middleware::rbac_helpers::is_public_route;
///
/// assert!(is_public_route("/health"));
/// assert!(is_public_route("/graphql/health"));
/// assert!(!is_public_route("/api/v1/events"));
/// ```
pub fn is_public_route(path: &str) -> bool {
    matches!(path, "/health" | "/graphql/health" | "/graphql/playground")
}

/// Extract resource ID from path if present
///
/// Many routes follow the pattern `/api/v1/{resource}/{id}` for specific
/// resource operations. This helper extracts the ID portion.
///
/// # Arguments
///
/// * `path` - The API route path
///
/// # Returns
///
/// `Some(id)` if a resource ID is present in the path, `None` otherwise
///
/// # Examples
///
/// ```
/// use xzepr::api::middleware::rbac_helpers::extract_resource_id;
///
/// let id = extract_resource_id("/api/v1/events/123");
/// assert_eq!(id, Some("123"));
///
/// let id = extract_resource_id("/api/v1/events");
/// assert_eq!(id, None);
/// ```
pub fn extract_resource_id(path: &str) -> Option<&str> {
    // Remove trailing slash if present
    let path = path.trim_end_matches('/');
    let parts: Vec<&str> = path.split('/').collect();

    // Pattern: /api/v1/{resource}/{id}
    if parts.len() >= 5 && !parts[4].is_empty() {
        Some(parts[4])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_to_permission_event_create() {
        let perm = route_to_permission(&Method::POST, "/api/v1/events");
        assert_eq!(perm, Some(Permission::EventCreate));
    }

    #[test]
    fn test_route_to_permission_event_read() {
        let perm = route_to_permission(&Method::GET, "/api/v1/events/123");
        assert_eq!(perm, Some(Permission::EventRead));
    }

    #[test]
    fn test_route_to_permission_event_update() {
        let perm = route_to_permission(&Method::PUT, "/api/v1/events/123");
        assert_eq!(perm, Some(Permission::EventUpdate));
    }

    #[test]
    fn test_route_to_permission_event_delete() {
        let perm = route_to_permission(&Method::DELETE, "/api/v1/events/123");
        assert_eq!(perm, Some(Permission::EventDelete));
    }

    #[test]
    fn test_route_to_permission_receiver_create() {
        let perm = route_to_permission(&Method::POST, "/api/v1/receivers");
        assert_eq!(perm, Some(Permission::ReceiverCreate));
    }

    #[test]
    fn test_route_to_permission_receiver_read() {
        let perm = route_to_permission(&Method::GET, "/api/v1/receivers");
        assert_eq!(perm, Some(Permission::ReceiverRead));
    }

    #[test]
    fn test_route_to_permission_receiver_update() {
        let perm = route_to_permission(&Method::PUT, "/api/v1/receivers/456");
        assert_eq!(perm, Some(Permission::ReceiverUpdate));
    }

    #[test]
    fn test_route_to_permission_receiver_delete() {
        let perm = route_to_permission(&Method::DELETE, "/api/v1/receivers/456");
        assert_eq!(perm, Some(Permission::ReceiverDelete));
    }

    #[test]
    fn test_route_to_permission_group_create() {
        let perm = route_to_permission(&Method::POST, "/api/v1/groups");
        assert_eq!(perm, Some(Permission::GroupCreate));
    }

    #[test]
    fn test_route_to_permission_group_read() {
        let perm = route_to_permission(&Method::GET, "/api/v1/groups/789");
        assert_eq!(perm, Some(Permission::GroupRead));
    }

    #[test]
    fn test_route_to_permission_group_update() {
        let perm = route_to_permission(&Method::PATCH, "/api/v1/groups/789");
        assert_eq!(perm, Some(Permission::GroupUpdate));
    }

    #[test]
    fn test_route_to_permission_group_delete() {
        let perm = route_to_permission(&Method::DELETE, "/api/v1/groups/789");
        assert_eq!(perm, Some(Permission::GroupDelete));
    }

    #[test]
    fn test_route_to_permission_health_is_public() {
        let perm = route_to_permission(&Method::GET, "/health");
        assert_eq!(perm, None);
    }

    #[test]
    fn test_route_to_permission_graphql_is_public() {
        let perm = route_to_permission(&Method::POST, "/graphql");
        assert_eq!(perm, None);
    }

    #[test]
    fn test_route_to_permission_graphql_playground_is_public() {
        let perm = route_to_permission(&Method::GET, "/graphql/playground");
        assert_eq!(perm, None);
    }

    #[test]
    fn test_route_to_permission_unknown_route() {
        let perm = route_to_permission(&Method::GET, "/api/v1/unknown");
        assert_eq!(perm, None);
    }

    #[test]
    fn test_route_to_permission_unsupported_method() {
        let perm = route_to_permission(&Method::OPTIONS, "/api/v1/events");
        assert_eq!(perm, None);
    }

    #[test]
    fn test_get_resource_permissions_event() {
        let perms = get_resource_permissions("event");
        assert_eq!(perms.len(), 4);
        assert!(perms.contains(&Permission::EventCreate));
        assert!(perms.contains(&Permission::EventRead));
        assert!(perms.contains(&Permission::EventUpdate));
        assert!(perms.contains(&Permission::EventDelete));
    }

    #[test]
    fn test_get_resource_permissions_receiver() {
        let perms = get_resource_permissions("receiver");
        assert_eq!(perms.len(), 4);
        assert!(perms.contains(&Permission::ReceiverCreate));
        assert!(perms.contains(&Permission::ReceiverRead));
        assert!(perms.contains(&Permission::ReceiverUpdate));
        assert!(perms.contains(&Permission::ReceiverDelete));
    }

    #[test]
    fn test_get_resource_permissions_group() {
        let perms = get_resource_permissions("group");
        assert_eq!(perms.len(), 4);
        assert!(perms.contains(&Permission::GroupCreate));
        assert!(perms.contains(&Permission::GroupRead));
        assert!(perms.contains(&Permission::GroupUpdate));
        assert!(perms.contains(&Permission::GroupDelete));
    }

    #[test]
    fn test_get_resource_permissions_unknown() {
        let perms = get_resource_permissions("unknown");
        assert_eq!(perms.len(), 0);
    }

    #[test]
    fn test_is_public_route_health() {
        assert!(is_public_route("/health"));
    }

    #[test]
    fn test_is_public_route_graphql_health() {
        assert!(is_public_route("/graphql/health"));
    }

    #[test]
    fn test_is_public_route_graphql_playground() {
        assert!(is_public_route("/graphql/playground"));
    }

    #[test]
    fn test_is_public_route_api_is_protected() {
        assert!(!is_public_route("/api/v1/events"));
        assert!(!is_public_route("/api/v1/receivers"));
        assert!(!is_public_route("/api/v1/groups"));
    }

    #[test]
    fn test_extract_resource_id_with_id() {
        let id = extract_resource_id("/api/v1/events/123");
        assert_eq!(id, Some("123"));
    }

    #[test]
    fn test_extract_resource_id_receiver() {
        let id = extract_resource_id("/api/v1/receivers/abc-def");
        assert_eq!(id, Some("abc-def"));
    }

    #[test]
    fn test_extract_resource_id_group() {
        let id = extract_resource_id("/api/v1/groups/789");
        assert_eq!(id, Some("789"));
    }

    #[test]
    fn test_extract_resource_id_no_id() {
        let id = extract_resource_id("/api/v1/events");
        assert_eq!(id, None);
    }

    #[test]
    fn test_extract_resource_id_empty() {
        let id = extract_resource_id("/api/v1/events/");
        // Empty trailing slash doesn't produce an ID
        assert_eq!(id, None);
    }

    #[test]
    fn test_extract_resource_id_invalid_path() {
        let id = extract_resource_id("/health");
        assert_eq!(id, None);
    }
}
