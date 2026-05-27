// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/middleware/opa.rs

//! OPA-based authorization middleware
//!
//! This module provides middleware for enforcing authorization policies using
//! Open Policy Agent (OPA). It integrates with the OPA client to evaluate
//! policies and make authorization decisions based on user context, resource
//! context, and requested actions.
//!
//! # Fail-Safe Modes
//!
//! When OPA is unavailable the middleware applies the configured
//! [`OpaFailSafeMode`]:
//!
//! - `FailClosed` - deny all requests (safe for production).
//! - `FailOpenDevelopment` - allow all requests; overridden to fail-closed
//!   when `is_production` is `true`.
//! - `LegacyRbacFallback` - evaluate the built-in RBAC engine and label the
//!   decision as `unavailable_legacy_rbac_*` in audit logs and metrics.

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
use crate::api::middleware::resource_context::{ResourceContextBuilder, ResourceContextError};
use crate::infrastructure::audit::AuditLogger;
use crate::infrastructure::metrics::PrometheusMetrics;
use crate::opa::client::OpaClient;
use crate::opa::types::{
    OpaDecisionOutcome, OpaFailSafeMode, OpaInput, ResourceContext, UserContext,
};

/// Holds the resource context builders for each supported resource type.
///
/// These builders query repositories to produce a complete [`ResourceContext`]
/// for OPA policy evaluation, including ownership and group membership data.
#[derive(Clone)]
pub struct ResourceContextBuilders {
    /// Builder for `Event` resources.
    pub event: Arc<dyn ResourceContextBuilder>,
    /// Builder for `EventReceiver` resources.
    pub receiver: Arc<dyn ResourceContextBuilder>,
    /// Builder for `EventReceiverGroup` and group-membership resources.
    pub group: Arc<dyn ResourceContextBuilder>,
}

/// State for the OPA authorization middleware.
///
/// Carries the OPA client, audit logger, Prometheus metrics, resource context
/// builders, the configured fail-safe mode, and a production-environment flag.
/// All fields are cheaply `Clone`able through `Arc`.
#[derive(Clone)]
pub struct OpaMiddlewareState {
    /// OPA client for policy evaluation with caching and circuit breaker.
    pub opa_client: Arc<OpaClient>,
    /// Audit logger for recording authorization decisions.
    pub audit_logger: Arc<AuditLogger>,
    /// Prometheus metrics registry.
    pub metrics: Arc<PrometheusMetrics>,
    /// Resource context builders, keyed by resource type.
    pub context_builders: ResourceContextBuilders,
    /// Fail-safe behavior when OPA is unavailable.
    pub fail_safe_mode: OpaFailSafeMode,
    /// `true` when `RUST_ENV=production`; prevents `FailOpenDevelopment` at runtime.
    pub is_production: bool,
}

impl OpaMiddlewareState {
    /// Creates a new OPA middleware state.
    ///
    /// # Arguments
    ///
    /// * `opa_client` - Initialized OPA client
    /// * `audit_logger` - Audit logger for authorization events
    /// * `metrics` - Prometheus metrics registry
    /// * `context_builders` - Resource context builders for event/receiver/group
    /// * `fail_safe_mode` - Behavior when OPA is unavailable
    /// * `is_production` - Whether the process is running in production mode
    pub fn new(
        opa_client: Arc<OpaClient>,
        audit_logger: Arc<AuditLogger>,
        metrics: Arc<PrometheusMetrics>,
        context_builders: ResourceContextBuilders,
        fail_safe_mode: OpaFailSafeMode,
        is_production: bool,
    ) -> Self {
        Self {
            opa_client,
            audit_logger,
            metrics,
            context_builders,
            fail_safe_mode,
            is_production,
        }
    }
}

/// Authorization decision produced by the OPA middleware.
///
/// Inserted into the request extensions on every allowed request so that
/// downstream handlers can inspect the decision without re-evaluating OPA.
///
/// # Usage
///
/// ```no_run
/// use axum::extract::Request;
/// use xzepr::api::middleware::opa::AuthorizationDecision;
///
/// fn handler(request: Request) {
///     if let Some(decision) = request.extensions().get::<AuthorizationDecision>() {
///         println!("allowed={} outcome={}", decision.allowed, decision.outcome.as_metric_label());
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct AuthorizationDecision {
    /// Whether access is allowed.
    pub allowed: bool,
    /// Human-readable reason for the decision (may be absent on success).
    pub reason: Option<String>,
    /// ID of the user who made the request.
    pub user_id: String,
    /// Action that was evaluated (e.g., `"read"`, `"delete"`).
    pub action: String,
    /// Type of resource that was evaluated (e.g., `"events"`, `"groups"`).
    pub resource_type: String,
    /// Specific resource ID, if the request targeted one.
    pub resource_id: Option<String>,
    /// The labeled outcome for this decision.
    pub outcome: OpaDecisionOutcome,
}

/// OPA authorization middleware.
///
/// Extracts the authenticated user and resource context from the request,
/// evaluates the authorization policy using OPA, and allows or denies the
/// request based on the policy decision.
///
/// # Flow
///
/// 1. Extract user, method, and path from the request.
/// 2. Determine whether the path is resource-specific (has an ID segment).
/// 3. If resource-specific, call [`build_resource_context_for_path`] to
///    construct a full context via the configured repository builders.
/// 4. If not resource-specific, build a minimal context from the path.
/// 5. Build [`OpaInput`] from user, action, and resource context.
/// 6. Call [`OpaClient::evaluate_with_circuit_breaker`]; on error apply
///    the configured fail-safe mode.
/// 7. Record Prometheus metrics and emit an audit log entry.
/// 8. If allowed, insert an [`AuthorizationDecision`] into request extensions
///    and pass control to the next handler.
/// 9. If denied, return an [`AuthorizationError`].
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

    // Determine action from HTTP method and path.
    let action = determine_action(&method, &path);

    let start = std::time::Instant::now();

    // Build the resource context: full for resource-specific paths, minimal otherwise.
    let resource_context = if is_resource_specific_path(&path) {
        match build_resource_context_for_path(&state, &path).await {
            Ok(ctx) => ctx,
            Err(e) => {
                let duration = start.elapsed().as_secs_f64();
                let resource_type = resource_type_from_path(&path);
                record_opa_outcome(
                    &state,
                    OpaDecisionOutcome::ResourceContextMissing,
                    &action,
                    resource_type,
                    duration,
                );
                warn!(
                    user_id = %user_id,
                    path = %path,
                    error = %e,
                    "Failed to build resource context for authorization"
                );
                return Err(e);
            }
        }
    } else {
        ResourceContext {
            resource_type: resource_type_from_path(&path).to_string(),
            resource_id: None,
            owner_id: None,
            group_id: None,
            members: Vec::new(),
            resource_version: 1,
        }
    };

    // Build OPA input.
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

    // Evaluate policy with caching and circuit breaker.
    // resource_version is i64 in the domain model but the OPA client accepts i32;
    // values that overflow i32 (impossible in practice) clamp to i32::MAX.
    let resource_version = i32::try_from(resource_context.resource_version).unwrap_or(i32::MAX);
    let opa_result = state
        .opa_client
        .evaluate_with_circuit_breaker(opa_input, resource_version)
        .await;

    let outcome = match opa_result {
        Ok(decision) => {
            if decision.allow {
                OpaDecisionOutcome::OpaAllow
            } else {
                OpaDecisionOutcome::OpaDeny
            }
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "OPA policy evaluation failed, applying fail-safe mode"
            );
            match state.fail_safe_mode {
                OpaFailSafeMode::FailClosed => OpaDecisionOutcome::UnavailableFailClosed,
                OpaFailSafeMode::FailOpenDevelopment => {
                    if state.is_production {
                        warn!(
                            user_id = %user_id,
                            "FailOpenDevelopment mode requested in production; overriding to FailClosed"
                        );
                        OpaDecisionOutcome::UnavailableFailClosed
                    } else {
                        OpaDecisionOutcome::UnavailableFailOpenDevelopment
                    }
                }
                OpaFailSafeMode::LegacyRbacFallback => {
                    if legacy_rbac_check(&user, &action, &resource_context) {
                        OpaDecisionOutcome::UnavailableLegacyRbacAllow
                    } else {
                        OpaDecisionOutcome::UnavailableLegacyRbacDeny
                    }
                }
            }
        }
    };

    let duration = start.elapsed().as_secs_f64();
    let resource_type = resource_context.resource_type.clone();

    // Record Prometheus metrics.
    record_opa_outcome(&state, outcome, &action, &resource_type, duration);

    // Emit audit log entry.
    log_authorization_decision(
        &state,
        &user_id,
        &action,
        &resource_context,
        outcome.is_allowed(),
        outcome,
    )
    .await;

    if outcome.is_allowed() {
        info!(
            user_id = %user_id,
            action = %action,
            resource_type = %resource_type,
            outcome = %outcome.as_metric_label(),
            "Authorization granted"
        );

        let decision = AuthorizationDecision {
            allowed: true,
            reason: None,
            user_id: user_id.clone(),
            action: action.clone(),
            resource_type: resource_type.clone(),
            resource_id: resource_context.resource_id.clone(),
            outcome,
        };
        request.extensions_mut().insert(decision);
        Ok(next.run(request).await)
    } else {
        warn!(
            user_id = %user_id,
            action = %action,
            resource_type = %resource_type,
            outcome = %outcome.as_metric_label(),
            "Authorization denied"
        );
        Err(AuthorizationError::Forbidden)
    }
}

/// Determines whether a path refers to a specific resource (has an ID segment).
///
/// Returns `true` for paths like `/api/v1/events/ID` or
/// `/api/v1/groups/ID/members`.
/// Returns `false` for list/create paths like `/api/v1/events`.
///
/// The canonical API path structure is `/api/v1/{resource_type}/{id}`.
/// After trimming the leading `/` and splitting on `/`, the resulting slice is
/// `["api", "v1", resource_type, id?, ...]`.  The path is resource-specific
/// when the segment at index 3 is present, non-empty, and is not the literal
/// string `"members"`.
fn is_resource_specific_path(path: &str) -> bool {
    let trimmed = path.trim_start_matches('/');
    let segments: Vec<&str> = trimmed.split('/').collect();
    segments.len() >= 4 && !segments[3].is_empty() && segments[3] != "members"
}

/// Extracts the resource type string from a canonical API path.
///
/// E.g., `/api/v1/events/ID` returns `"events"`, `/api/v1/receivers` returns
/// `"receivers"`.  Returns `"unknown"` if the path does not match the expected
/// API structure (fewer than three `/`-separated segments after stripping the
/// leading `/`).
fn resource_type_from_path(path: &str) -> &str {
    let trimmed = path.trim_start_matches('/');
    let segments: Vec<&str> = trimmed.split('/').collect();
    // ["api", "v1", resource_type, ...]
    segments.get(2).copied().unwrap_or("unknown")
}

/// Extracts the resource ID string from a canonical API path.
///
/// Returns `Some(id)` for paths like `/api/v1/events/ID` or
/// `/api/v1/groups/ID/members`.
/// Returns `None` for list/create paths that have no ID segment (e.g.,
/// `/api/v1/events`), and also `None` when the segment at index 3 is the
/// literal string `"members"`.
fn resource_id_from_path(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    let segments: Vec<&str> = trimmed.split('/').collect();
    // ["api", "v1", resource_type, id?, ...]
    segments
        .get(3)
        .copied()
        .filter(|s| !s.is_empty() && *s != "members")
}

/// Builds a complete resource context using the appropriate builder.
///
/// Selects the builder based on the resource type extracted from `path` and
/// calls [`ResourceContextBuilder::build_context`] with the extracted resource
/// ID.  Unknown resource types (those that are not `"events"`, `"receivers"`,
/// or `"groups"`) receive a minimal context with no owner, group, or members.
///
/// # Errors
///
/// Returns `AuthorizationError::Forbidden` when:
/// - The resource is not found in the repository.
/// - The resource ID string is malformed.
///
/// Returns `AuthorizationError::InternalError` on repository failure. The
/// underlying error message is logged with `error!()` and is **never**
/// forwarded to the caller.
async fn build_resource_context_for_path(
    state: &OpaMiddlewareState,
    path: &str,
) -> Result<ResourceContext, AuthorizationError> {
    let resource_type = resource_type_from_path(path);

    let id = match resource_id_from_path(path) {
        Some(id) => id,
        None => {
            // No ID present; return a minimal context so callers can still
            // evaluate access on the resource type alone.
            return Ok(ResourceContext {
                resource_type: resource_type.to_string(),
                resource_id: None,
                owner_id: None,
                group_id: None,
                members: Vec::new(),
                resource_version: 1,
            });
        }
    };

    let builder: &Arc<dyn ResourceContextBuilder> = match resource_type {
        "events" => &state.context_builders.event,
        "receivers" => &state.context_builders.receiver,
        "groups" => &state.context_builders.group,
        _ => {
            // Unknown resource type; return a minimal context without
            // owner/group so OPA can still evaluate on type alone.
            return Ok(ResourceContext {
                resource_type: resource_type.to_string(),
                resource_id: None,
                owner_id: None,
                group_id: None,
                members: Vec::new(),
                resource_version: 1,
            });
        }
    };

    builder.build_context(id).await.map_err(|e| match e {
        ResourceContextError::NotFound { .. } => AuthorizationError::ResourceNotFound,
        ResourceContextError::InvalidId { .. } => AuthorizationError::InvalidResourceIdentifier,
        ResourceContextError::RepositoryFailure { source } => {
            error!(
                error = %source,
                "Repository failure building OPA resource context"
            );
            AuthorizationError::InternalError
        }
    })
}

/// Records OPA authorization outcome metrics via the Prometheus registry.
///
/// Delegates to [`PrometheusMetrics::record_authorization_decision`] with the
/// `fallback_used` flag set to `true` only when the outcome is a legacy RBAC
/// result.
fn record_opa_outcome(
    state: &OpaMiddlewareState,
    outcome: OpaDecisionOutcome,
    action: &str,
    resource_type: &str,
    duration_secs: f64,
) {
    state.metrics.record_authorization_decision(
        outcome.is_allowed(),
        resource_type,
        action,
        duration_secs,
        matches!(
            outcome,
            OpaDecisionOutcome::UnavailableLegacyRbacAllow
                | OpaDecisionOutcome::UnavailableLegacyRbacDeny
        ),
    );
}

/// Determines the authorization action from the HTTP method and request path.
///
/// # Arguments
///
/// * `method` - HTTP method string (e.g., `"GET"`, `"POST"`).
/// * `path` - Request path (e.g., `"/api/v1/groups/123/members"`).
///
/// # Returns
///
/// Returns a string action label used as the OPA policy action:
/// `"read"`, `"create"`, `"update"`, `"delete"`, `"list_members"`,
/// `"add_member"`, `"remove_member"`, or `"unknown"`.
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

/// Legacy RBAC fallback check.
///
/// Evaluated when OPA is unavailable and [`OpaFailSafeMode::LegacyRbacFallback`]
/// is configured.  Applies simple role and ownership checks in lieu of a full
/// OPA policy evaluation.
///
/// # Arguments
///
/// * `user` - The authenticated request user.
/// * `action` - The action being performed (e.g., `"read"`, `"delete"`).
/// * `resource` - The resource context for the request.
///
/// # Returns
///
/// `true` if access should be granted, `false` otherwise.
fn legacy_rbac_check(user: &AuthenticatedUser, action: &str, resource: &ResourceContext) -> bool {
    // Admin can do anything.
    if user.has_role("admin") {
        return true;
    }

    // Owner can do anything with their own resources.
    if let Some(owner_id) = &resource.owner_id {
        if owner_id == user.user_id() {
            return true;
        }
    }

    // Group members can read resources in their group.
    if action == "read" {
        if let Some(_group_id) = &resource.group_id {
            if resource.members.contains(&user.user_id().to_string()) {
                return true;
            }
        }
    }

    // Check specific permissions encoded in the JWT claims.
    let permission = format!("{}:{}", resource.resource_type, action);
    user.has_permission(&permission)
}

/// Logs an authorization decision to the audit log.
///
/// Emits a structured audit event with the decision outcome, resource type,
/// and the OPA decision label under the `"opa_outcome"` metadata key.
///
/// # Arguments
///
/// * `state` - Middleware state containing the audit logger.
/// * `user_id` - ID of the requesting user.
/// * `action` - Action that was evaluated (e.g., `"read"`).
/// * `resource` - Resource context that was evaluated.
/// * `decision_allowed` - `true` if the request was allowed.
/// * `outcome` - The specific OPA decision outcome for metric labeling.
async fn log_authorization_decision(
    state: &OpaMiddlewareState,
    user_id: &str,
    action: &str,
    resource: &ResourceContext,
    decision_allowed: bool,
    outcome: OpaDecisionOutcome,
) {
    use crate::infrastructure::audit::{AuditAction, AuditEvent, AuditOutcome};

    let audit_outcome = if decision_allowed {
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
    metadata.insert("decision_allowed".to_string(), decision_allowed.to_string());
    metadata.insert(
        "opa_outcome".to_string(),
        outcome.as_metric_label().to_string(),
    );

    let audit_event = AuditEvent::builder()
        .user_id(user_id)
        .action(AuditAction::PermissionCheck)
        .resource(format!(
            "{}:{}",
            resource.resource_type,
            resource.resource_id.as_deref().unwrap_or("*")
        ))
        .outcome(audit_outcome)
        .metadata(metadata)
        .build();

    match audit_event {
        Ok(event) => state.audit_logger.log_event(event),
        Err(e) => tracing::error!(error = %e, "Failed to build OPA authorization audit event"),
    }
}

/// Authorization error returned by the OPA middleware.
///
/// Converted to an HTTP response via [`IntoResponse`]:
/// - `Forbidden` maps to `403 Forbidden`.
/// - `InternalError` maps to `500 Internal Server Error`.
#[derive(Debug)]
pub enum AuthorizationError {
    /// Access forbidden by policy.
    Forbidden,
    /// The requested resource was not found.
    ResourceNotFound,
    /// The requested resource identifier was malformed.
    InvalidResourceIdentifier,
    /// Internal error during authorization (e.g., repository failure).
    InternalError,
}

impl IntoResponse for AuthorizationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthorizationError::Forbidden => (StatusCode::FORBIDDEN, "Access denied".to_string()),
            AuthorizationError::ResourceNotFound => {
                (StatusCode::FORBIDDEN, "Resource not found".to_string())
            }
            AuthorizationError::InvalidResourceIdentifier => (
                StatusCode::FORBIDDEN,
                "Invalid resource identifier".to_string(),
            ),
            AuthorizationError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Authorization context unavailable".to_string(),
            ),
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
            AuthorizationError::Forbidden => write!(f, "Forbidden"),
            AuthorizationError::ResourceNotFound => write!(f, "Resource not found"),
            AuthorizationError::InvalidResourceIdentifier => {
                write!(f, "Invalid resource identifier")
            }
            AuthorizationError::InternalError => write!(f, "Internal authorization error"),
        }
    }
}

impl std::error::Error for AuthorizationError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- determine_action -------------------------------------------------

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

    // ---- path helpers -----------------------------------------------------

    #[test]
    fn test_is_resource_specific_path_with_id() {
        assert!(is_resource_specific_path("/api/v1/events/01JXABC123"));
        assert!(is_resource_specific_path("/api/v1/receivers/01JXABC123"));
        assert!(is_resource_specific_path("/api/v1/groups/01JXABC123"));
        assert!(is_resource_specific_path(
            "/api/v1/groups/01JXABC123/members"
        ));
    }

    #[test]
    fn test_is_resource_specific_path_without_id() {
        assert!(!is_resource_specific_path("/api/v1/events"));
        assert!(!is_resource_specific_path("/api/v1/receivers"));
        assert!(!is_resource_specific_path("/api/v1/groups"));
        assert!(!is_resource_specific_path("/health"));
        assert!(!is_resource_specific_path("/"));
    }

    #[test]
    fn test_resource_type_from_path() {
        assert_eq!(resource_type_from_path("/api/v1/events/123"), "events");
        assert_eq!(resource_type_from_path("/api/v1/receivers"), "receivers");
        assert_eq!(
            resource_type_from_path("/api/v1/groups/123/members"),
            "groups"
        );
        assert_eq!(resource_type_from_path("/health"), "unknown");
    }

    #[test]
    fn test_resource_id_from_path() {
        assert_eq!(
            resource_id_from_path("/api/v1/events/01JXABC"),
            Some("01JXABC")
        );
        assert_eq!(resource_id_from_path("/api/v1/events"), None);
        assert_eq!(
            resource_id_from_path("/api/v1/groups/01JXABC/members"),
            Some("01JXABC")
        );
    }

    // ---- AuthorizationDecision outcome field ------------------------------

    #[test]
    fn test_authorization_decision_outcome_field() {
        let decision = AuthorizationDecision {
            allowed: true,
            reason: None,
            user_id: "user1".to_string(),
            action: "read".to_string(),
            resource_type: "event".to_string(),
            resource_id: None,
            outcome: OpaDecisionOutcome::OpaAllow,
        };
        assert!(decision.outcome.is_allowed());
    }

    // ---- legacy_rbac_check ------------------------------------------------

    #[test]
    fn test_legacy_rbac_admin() {
        use crate::auth::jwt::claims::TokenType;
        use crate::auth::jwt::Claims;

        let claims = Claims {
            sub: "user123".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec![],
            exp: 9999999999,
            iat: 0,
            nbf: 0,
            jti: "test-jti-1".to_string(),
            iss: "xzepr".to_string(),
            aud: "xzepr-api".to_string(),
            token_type: TokenType::Access,
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
        use crate::auth::jwt::claims::TokenType;
        use crate::auth::jwt::Claims;

        let claims = Claims {
            sub: "user123".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            exp: 9999999999,
            iat: 0,
            nbf: 0,
            jti: "test-jti-2".to_string(),
            iss: "xzepr".to_string(),
            aud: "xzepr-api".to_string(),
            token_type: TokenType::Access,
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
        use crate::auth::jwt::claims::TokenType;
        use crate::auth::jwt::Claims;

        let claims = Claims {
            sub: "user123".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            exp: 9999999999,
            iat: 0,
            nbf: 0,
            jti: "test-jti-3".to_string(),
            iss: "xzepr".to_string(),
            aud: "xzepr-api".to_string(),
            token_type: TokenType::Access,
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
        use crate::auth::jwt::claims::TokenType;
        use crate::auth::jwt::Claims;

        let claims = Claims {
            sub: "user123".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec![],
            exp: 9999999999,
            iat: 0,
            nbf: 0,
            jti: "test-jti-4".to_string(),
            iss: "xzepr".to_string(),
            aud: "xzepr-api".to_string(),
            token_type: TokenType::Access,
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
