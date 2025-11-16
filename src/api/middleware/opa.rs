// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/middleware/opa.rs

//! OPA-based authorization middleware
//!
//! This module provides middleware for enforcing authorization policies using
//! Open Policy Agent (OPA). It integrates with the OPA client to evaluate
//! policies and make authorization decisions based on user context, resource
//! context, and requested actions.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::api::middleware::jwt::AuthenticatedUser;
use crate::infrastructure::audit::AuditLogger;
use crate::infrastructure::metrics::PrometheusMetrics;
use crate::opa::client::OpaClient;
use crate::opa::types::{
    AuthorizationDecision as OpaDecision, OpaInput, ResourceContext, UserContext,
};

/// State for OPA authorization middleware
#[derive(Clone)]
pub struct OpaMiddlewareState {
    /// OPA client for policy evaluation
    pub opa_client: Arc<OpaClient>,
    /// Audit logger for authorization decisions
    pub audit_logger: Arc<AuditLogger>,
    /// Metrics collector
    pub metrics: Arc<PrometheusMetrics>,
}

impl OpaMiddlewareState {
    /// Create new OPA middleware state
    pub fn new(
        opa_client: Arc<OpaClient>,
        audit_logger: Arc<AuditLogger>,
        metrics: Arc<PrometheusMetrics>,
    ) -> Self {
        Self {
            opa_client,
            audit_logger,
            metrics,
        }
    }
}

/// Authorization decision result
#[derive(Debug, Clone, Serialize)]
pub struct AuthorizationDecision {
    /// Whether access is allowed
    pub allowed: bool,
    /// Reason for the decision
    pub reason: Option<String>,
    /// User who made the request
    pub user_id: String,
    /// Action requested
    pub action: String,
    /// Resource being accessed
    pub resource_type: String,
    /// Resource ID
    pub resource_id: Option<String>,
}

/// OPA authorization middleware
///
/// This middleware extracts the authenticated user and resource context from
/// the request, evaluates the authorization policy using OPA, and allows or
/// denies the request based on the policy decision.
///
/// # Flow
///
/// 1. Extract authenticated user from request extensions
/// 2. Extract resource context from request path and method
/// 3. Build OPA input with user context, action, and resource context
/// 4. Evaluate policy using OPA client (with caching and circuit breaker)
/// 5. Log authorization decision to audit log
/// 6. Record metrics
/// 7. Allow or deny request
///
/// # Fallback
///
/// If OPA is unavailable or returns an error, the middleware falls back to
/// legacy RBAC checks using the user's roles and permissions.
pub async fn opa_authorize_middleware(
    State(state): State<OpaMiddlewareState>,
    user: AuthenticatedUser,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthorizationError> {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let user_id = user.user_id().to_string();

    debug!(
        user_id = %user_id,
        method = %method,
        path = %path,
        "Evaluating OPA authorization"
    );

    // Extract resource context from request
    let resource_context = match extract_resource_context(&request).await {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!(
                user_id = %user_id,
                error = %e,
                "Failed to extract resource context, using path-based context"
            );
            // Fallback: use path to infer resource type
            extract_resource_from_path(&path)
        }
    };

    // Determine action from HTTP method and path
    let action = determine_action(&method, &path);

    // Build OPA input
    let user_context = UserContext {
        user_id: user_id.clone(),
        username: user.claims.sub.clone(),
        roles: user.claims.roles.clone(),
        groups: vec![],
    };

    let opa_input = OpaInput {
        user: user_context,
        action: action.clone(),
        resource: resource_context.clone(),
    };

    // Evaluate policy
    let start = std::time::Instant::now();
    let decision = match state.opa_client.evaluate(opa_input).await {
        Ok(result) => {
            let duration = start.elapsed();
            state
                .metrics
                .record_auth_duration(&format!("opa_{}", action), duration.as_secs_f64());

            result
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "OPA policy evaluation failed, falling back to legacy RBAC"
            );

            // Fallback to legacy RBAC
            let allowed = legacy_rbac_check(&user, &action, &resource_context);

            OpaDecision {
                allow: allowed,
                reason: Some("OPA unavailable, used legacy RBAC".to_string()),
                metadata: None,
            }
        }
    };

    // Log authorization decision
    log_authorization_decision(&state, &user_id, &action, &resource_context, &decision).await;

    // Check if access is allowed
    if !decision.allow {
        warn!(
            user_id = %user_id,
            action = %action,
            resource_type = %resource_context.resource_type,
            resource_id = ?resource_context.resource_id,
            reason = ?decision.reason,
            "Authorization denied"
        );

        return Err(AuthorizationError::Forbidden {
            reason: decision
                .reason
                .unwrap_or_else(|| "Access denied".to_string()),
        });
    }

    info!(
        user_id = %user_id,
        action = %action,
        resource_type = %resource_context.resource_type,
        resource_id = ?resource_context.resource_id,
        "Authorization granted"
    );

    // Store authorization decision in request extensions for handlers to use
    request.extensions_mut().insert(decision);

    Ok(next.run(request).await)
}

/// Extract resource context from request
///
/// This looks for resource context that may have been set by earlier
/// middleware or extracts it from request path parameters.
async fn extract_resource_context(request: &Request) -> Result<ResourceContext, String> {
    // Try to get pre-built resource context from extensions
    if let Some(ctx) = request.extensions().get::<ResourceContext>() {
        return Ok(ctx.clone());
    }

    Err("No resource context in request extensions".to_string())
}

/// Extract resource context from path as fallback
fn extract_resource_from_path(path: &str) -> ResourceContext {
    // Parse path to infer resource type and ID
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if parts.len() >= 2 {
        let resource_type = parts[0].to_string();
        let resource_id = if parts[1] != "" {
            Some(parts[1].to_string())
        } else {
            None
        };

        ResourceContext {
            resource_type,
            resource_id,
            owner_id: None,
            group_id: None,
            members: Vec::new(),
            resource_version: 1,
        }
    } else {
        ResourceContext {
            resource_type: parts.get(0).unwrap_or(&"unknown").to_string(),
            resource_id: None,
            owner_id: None,
            group_id: None,
            members: Vec::new(),
            resource_version: 1,
        }
    }
}

/// Determine action from HTTP method and path
fn determine_action(method: &str, path: &str) -> String {
    match method {
        "GET" => {
            if path.ends_with("/members") {
                "list_members".to_string()
            } else {
                "read".to_string()
            }
        }
        "POST" => {
            if path.contains("/members") {
                "add_member".to_string()
            } else {
                "create".to_string()
            }
        }
        "PUT" | "PATCH" => "update".to_string(),
        "DELETE" => {
            if path.contains("/members") {
                "remove_member".to_string()
            } else {
                "delete".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}

/// Legacy RBAC fallback check
///
/// When OPA is unavailable, fall back to simple role-based checks.
fn legacy_rbac_check(user: &AuthenticatedUser, action: &str, resource: &ResourceContext) -> bool {
    // Admin can do anything
    if user.has_role("admin") {
        return true;
    }

    // Owner can do anything with their own resources
    if let Some(owner_id) = &resource.owner_id {
        if owner_id == user.user_id() {
            return true;
        }
    }

    // Group members can read resources in their group
    if action == "read" {
        if let Some(group_id) = &resource.group_id {
            // Check if user is a member of the group
            if resource.members.contains(&user.user_id().to_string()) {
                return true;
            }
        }
    }

    // Check specific permissions
    let permission = format!("{}:{}", resource.resource_type, action);
    user.has_permission(&permission)
}

/// Log authorization decision to audit log
async fn log_authorization_decision(
    state: &OpaMiddlewareState,
    user_id: &str,
    action: &str,
    resource: &ResourceContext,
    decision: &OpaDecision,
) {
    use crate::infrastructure::audit::{AuditAction, AuditEvent, AuditOutcome};

    let outcome = if decision.allow {
        AuditOutcome::Success
    } else {
        AuditOutcome::Denied
    };

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("action".to_string(), action.to_string());
    metadata.insert("resource_type".to_string(), resource.resource_type.clone());
    if let Some(rid) = &resource.resource_id {
        metadata.insert("resource_id".to_string(), rid.clone());
    }
    if let Some(oid) = &resource.owner_id {
        metadata.insert("owner_id".to_string(), oid.clone());
    }
    if let Some(gid) = &resource.group_id {
        metadata.insert("group_id".to_string(), gid.clone());
    }
    metadata.insert("decision_allowed".to_string(), decision.allow.to_string());
    if let Some(reason) = &decision.reason {
        metadata.insert("decision_reason".to_string(), reason.clone());
    }

    let audit_event = AuditEvent::builder()
        .user_id(user_id)
        .action(AuditAction::PermissionCheck)
        .resource(format!(
            "{}:{}",
            resource.resource_type,
            resource.resource_id.as_deref().unwrap_or("*")
        ))
        .outcome(outcome)
        .metadata(metadata)
        .build();

    state.audit_logger.log_event(audit_event);
}

/// Authorization error
#[derive(Debug)]
pub enum AuthorizationError {
    /// Access forbidden
    Forbidden { reason: String },
    /// Internal error during authorization
    InternalError { message: String },
}

impl IntoResponse for AuthorizationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthorizationError::Forbidden { reason } => (StatusCode::FORBIDDEN, reason),
            AuthorizationError::InternalError { message } => {
                (StatusCode::INTERNAL_SERVER_ERROR, message)
            }
        };

        let body = serde_json::json!({
            "error": "authorization_error",
            "message": message,
        });

        (status, axum::Json(body)).into_response()
    }
}

impl std::fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthorizationError::Forbidden { reason } => write!(f, "Forbidden: {}", reason),
            AuthorizationError::InternalError { message } => {
                write!(f, "Internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for AuthorizationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_action_get() {
        assert_eq!(determine_action("GET", "/events/123"), "read");
        assert_eq!(
            determine_action("GET", "/groups/456/members"),
            "list_members"
        );
    }

    #[test]
    fn test_determine_action_post() {
        assert_eq!(determine_action("POST", "/events"), "create");
        assert_eq!(
            determine_action("POST", "/groups/123/members"),
            "add_member"
        );
    }

    #[test]
    fn test_determine_action_put() {
        assert_eq!(determine_action("PUT", "/events/123"), "update");
    }

    #[test]
    fn test_determine_action_delete() {
        assert_eq!(determine_action("DELETE", "/events/123"), "delete");
        assert_eq!(
            determine_action("DELETE", "/groups/123/members/456"),
            "remove_member"
        );
    }

    #[test]
    fn test_extract_resource_from_path() {
        let ctx = extract_resource_from_path("/events/123");
        assert_eq!(ctx.resource_type, "events");
        assert_eq!(ctx.resource_id, Some("123".to_string()));

        let ctx = extract_resource_from_path("/receivers");
        assert_eq!(ctx.resource_type, "receivers");
        assert_eq!(ctx.resource_id, None);

        let ctx = extract_resource_from_path("/");
        assert_eq!(ctx.resource_type, "");
    }

    #[test]
    fn test_legacy_rbac_admin() {
        use crate::auth::jwt::Claims;

        let claims = Claims {
            sub: "user123".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec![],
            exp: 0,
            iat: 0,
        };
        let user = AuthenticatedUser::new(claims);

        let resource = ResourceContext {
            resource_type: "event".to_string(),
            resource_id: Some("123".to_string()),
            owner_id: Some("other_user".to_string()),
            group_id: None,
            members: vec![],
            resource_version: 1,
        };

        assert!(legacy_rbac_check(&user, "delete", &resource));
    }

    #[test]
    fn test_legacy_rbac_owner() {
        use crate::auth::jwt::Claims;

        let claims = Claims {
            sub: "user123".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            exp: 0,
            iat: 0,
        };
        let user = AuthenticatedUser::new(claims);

        let resource = ResourceContext {
            resource_type: "event".to_string(),
            resource_id: Some("123".to_string()),
            owner_id: Some("user123".to_string()),
            group_id: None,
            members: vec![],
            resource_version: 1,
        };

        assert!(legacy_rbac_check(&user, "update", &resource));
    }

    #[test]
    fn test_legacy_rbac_group_member_read() {
        use crate::auth::jwt::Claims;

        let claims = Claims {
            sub: "user123".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            exp: 0,
            iat: 0,
        };
        let user = AuthenticatedUser::new(claims);

        let resource = ResourceContext {
            resource_type: "event".to_string(),
            resource_id: Some("123".to_string()),
            owner_id: Some("other_user".to_string()),
            group_id: Some("group456".to_string()),
            members: vec!["user123".to_string(), "user456".to_string()],
            resource_version: 1,
        };

        assert!(legacy_rbac_check(&user, "read", &resource));
        assert!(!legacy_rbac_check(&user, "update", &resource));
    }

    #[test]
    fn test_legacy_rbac_denied() {
        use crate::auth::jwt::Claims;

        let claims = Claims {
            sub: "user123".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            exp: 0,
            iat: 0,
        };
        let user = AuthenticatedUser::new(claims);

        let resource = ResourceContext {
            resource_type: "event".to_string(),
            resource_id: Some("123".to_string()),
            owner_id: Some("other_user".to_string()),
            group_id: None,
            members: vec![],
            resource_version: 1,
        };

        assert!(!legacy_rbac_check(&user, "delete", &resource));
    }
}
