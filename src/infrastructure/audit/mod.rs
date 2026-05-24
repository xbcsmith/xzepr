// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Audit Logging Infrastructure
//!
//! This module provides structured audit logging for security-relevant events.
//! Audit logs are emitted as structured JSON to stdout/stderr for ingestion
//! into centralized logging systems like ELK, Datadog, or similar.
//!
//! # Example
//!
//! ```rust
//! use xzepr::infrastructure::audit::{AuditLogger, AuditEvent, AuditAction, AuditOutcome};
//!
//! let logger = AuditLogger::new();
//!
//! let event = AuditEvent::builder()
//!     .user_id("user123")
//!     .action(AuditAction::Login)
//!     .resource("/api/v1/auth/login")
//!     .outcome(AuditOutcome::Success)
//!     .ip_address("192.168.1.100")
//!     .user_agent("Mozilla/5.0")
//!     .build()
//!     // SAFETY: all required fields (action, resource, outcome) are set above
//!     .expect("all required builder fields are set");
//!
//! logger.log_event(event);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};

/// Errors that can occur when building an audit event.
///
/// An `AuditEvent` requires `action`, `resource`, and `outcome` fields.
/// These errors indicate programmer error (missing required fields in builder).
#[derive(Error, Debug, PartialEq)]
pub enum AuditBuildError {
    /// The required `action` field was not set on the builder.
    #[error("Audit event builder requires an action to be set")]
    MissingAction,
    /// The required `resource` field was not set on the builder.
    #[error("Audit event builder requires a resource to be set")]
    MissingResource,
    /// The required `outcome` field was not set on the builder.
    #[error("Audit event builder requires an outcome to be set")]
    MissingOutcome,
}

/// Audit event action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// User login attempt
    Login,
    /// User logout
    Logout,
    /// Token refresh
    TokenRefresh,
    /// Token validation
    TokenValidation,
    /// Permission check
    PermissionCheck,
    /// Authorization decision (OPA/RBAC)
    AuthorizationDecision,
    /// Authorization denial
    AuthorizationDenial,
    /// User creation
    UserCreate,
    /// User update
    UserUpdate,
    /// User deletion
    UserDelete,
    /// Role assignment
    RoleAssign,
    /// Role removal
    RoleRemove,
    /// OIDC authentication
    OidcAuth,
    /// OIDC callback
    OidcCallback,
    /// API access
    ApiAccess,
    /// Resource creation
    ResourceCreate,
    /// Resource read
    ResourceRead,
    /// Resource update
    ResourceUpdate,
    /// Resource deletion
    ResourceDelete,
    /// Configuration change
    ConfigChange,
    /// Security policy change
    SecurityPolicyChange,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditAction::Login => write!(f, "login"),
            AuditAction::Logout => write!(f, "logout"),
            AuditAction::TokenRefresh => write!(f, "token_refresh"),
            AuditAction::TokenValidation => write!(f, "token_validation"),
            AuditAction::PermissionCheck => write!(f, "permission_check"),
            AuditAction::AuthorizationDecision => write!(f, "authorization_decision"),
            AuditAction::AuthorizationDenial => write!(f, "authorization_denial"),
            AuditAction::UserCreate => write!(f, "user_create"),
            AuditAction::UserUpdate => write!(f, "user_update"),
            AuditAction::UserDelete => write!(f, "user_delete"),
            AuditAction::RoleAssign => write!(f, "role_assign"),
            AuditAction::RoleRemove => write!(f, "role_remove"),
            AuditAction::OidcAuth => write!(f, "oidc_auth"),
            AuditAction::OidcCallback => write!(f, "oidc_callback"),
            AuditAction::ApiAccess => write!(f, "api_access"),
            AuditAction::ResourceCreate => write!(f, "resource_create"),
            AuditAction::ResourceRead => write!(f, "resource_read"),
            AuditAction::ResourceUpdate => write!(f, "resource_update"),
            AuditAction::ResourceDelete => write!(f, "resource_delete"),
            AuditAction::ConfigChange => write!(f, "config_change"),
            AuditAction::SecurityPolicyChange => write!(f, "security_policy_change"),
        }
    }
}

/// Audit event outcome
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// Action succeeded
    Success,
    /// Action failed
    Failure,
    /// Action denied by authorization
    Denied,
    /// Action denied by rate limiting
    RateLimited,
    /// Action resulted in error
    Error,
}

impl std::fmt::Display for AuditOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditOutcome::Success => write!(f, "success"),
            AuditOutcome::Failure => write!(f, "failure"),
            AuditOutcome::Denied => write!(f, "denied"),
            AuditOutcome::RateLimited => write!(f, "rate_limited"),
            AuditOutcome::Error => write!(f, "error"),
        }
    }
}

/// Structured audit event for security and compliance logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Timestamp of the event (ISO 8601 format)
    pub timestamp: DateTime<Utc>,
    /// User ID performing the action (optional for anonymous)
    pub user_id: Option<String>,
    /// Action being performed
    pub action: AuditAction,
    /// Resource being accessed or modified
    pub resource: String,
    /// Outcome of the action
    pub outcome: AuditOutcome,
    /// Additional metadata about the event
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    /// IP address of the client
    pub ip_address: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// Session ID for correlation
    pub session_id: Option<String>,
    /// Request ID for distributed tracing correlation
    pub request_id: Option<String>,
    /// Error message if outcome is Failure or Error
    pub error_message: Option<String>,
    /// Duration of the operation in milliseconds
    pub duration_ms: Option<u64>,
}

impl AuditEvent {
    /// Create a new audit event builder
    pub fn builder() -> AuditEventBuilder {
        AuditEventBuilder::default()
    }

    /// Create a login event.
    ///
    /// # Errors
    ///
    /// Returns `AuditBuildError` if required builder fields are missing.
    pub fn login_success(user_id: &str, ip_address: Option<&str>) -> Result<Self, AuditBuildError> {
        Self::builder()
            .user_id(user_id)
            .action(AuditAction::Login)
            .resource("/auth/login")
            .outcome(AuditOutcome::Success)
            .ip_address_opt(ip_address)
            .build()
    }

    /// Create a login failure event.
    ///
    /// # Errors
    ///
    /// Returns `AuditBuildError` if required builder fields are missing.
    pub fn login_failure(reason: &str, ip_address: Option<&str>) -> Result<Self, AuditBuildError> {
        Self::builder()
            .action(AuditAction::Login)
            .resource("/auth/login")
            .outcome(AuditOutcome::Failure)
            .error_message(reason)
            .ip_address_opt(ip_address)
            .build()
    }

    /// Create a permission denied event.
    ///
    /// # Errors
    ///
    /// Returns `AuditBuildError` if required builder fields are missing.
    pub fn permission_denied(
        user_id: &str,
        resource: &str,
        permission: &str,
    ) -> Result<Self, AuditBuildError> {
        let mut metadata = HashMap::new();
        metadata.insert("permission".to_string(), permission.to_string());

        Self::builder()
            .user_id(user_id)
            .action(AuditAction::PermissionCheck)
            .resource(resource)
            .outcome(AuditOutcome::Denied)
            .metadata(metadata)
            .build()
    }

    /// Create a permission granted event.
    ///
    /// # Errors
    ///
    /// Returns `AuditBuildError` if required builder fields are missing.
    pub fn permission_granted(
        user_id: &str,
        resource: &str,
        permission: &str,
    ) -> Result<Self, AuditBuildError> {
        let mut metadata = HashMap::new();
        metadata.insert("permission".to_string(), permission.to_string());

        Self::builder()
            .user_id(user_id)
            .action(AuditAction::PermissionCheck)
            .resource(resource)
            .outcome(AuditOutcome::Success)
            .metadata(metadata)
            .build()
    }
}

/// Builder for AuditEvent
#[derive(Default)]
pub struct AuditEventBuilder {
    user_id: Option<String>,
    action: Option<AuditAction>,
    resource: Option<String>,
    outcome: Option<AuditOutcome>,
    metadata: HashMap<String, String>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    session_id: Option<String>,
    request_id: Option<String>,
    error_message: Option<String>,
    duration_ms: Option<u64>,
}

impl AuditEventBuilder {
    /// Set the user ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the user ID as Option
    pub fn user_id_opt(mut self, user_id: Option<impl Into<String>>) -> Self {
        self.user_id = user_id.map(|u| u.into());
        self
    }

    /// Set the action
    pub fn action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Set the resource
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Set the outcome
    pub fn outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a metadata entry
    pub fn add_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set IP address
    pub fn ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// Set IP address as Option
    pub fn ip_address_opt(mut self, ip: Option<impl Into<String>>) -> Self {
        self.ip_address = ip.map(|i| i.into());
        self
    }

    /// Set user agent
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Set user agent as Option
    pub fn user_agent_opt(mut self, ua: Option<impl Into<String>>) -> Self {
        self.user_agent = ua.map(|u| u.into());
        self
    }

    /// Set session ID
    pub fn session_id(mut self, sid: impl Into<String>) -> Self {
        self.session_id = Some(sid.into());
        self
    }

    /// Set session ID as Option
    pub fn session_id_opt(mut self, sid: Option<impl Into<String>>) -> Self {
        self.session_id = sid.map(|s| s.into());
        self
    }

    /// Set request ID
    pub fn request_id(mut self, rid: impl Into<String>) -> Self {
        self.request_id = Some(rid.into());
        self
    }

    /// Set request ID as Option
    pub fn request_id_opt(mut self, rid: Option<impl Into<String>>) -> Self {
        self.request_id = rid.map(|r| r.into());
        self
    }

    /// Set error message
    pub fn error_message(mut self, msg: impl Into<String>) -> Self {
        self.error_message = Some(msg.into());
        self
    }

    /// Set error message as Option
    pub fn error_message_opt(mut self, msg: Option<impl Into<String>>) -> Self {
        self.error_message = msg.map(|m| m.into());
        self
    }

    /// Set duration in milliseconds
    pub fn duration_ms(mut self, duration: u64) -> Self {
        self.duration_ms = Some(duration);
        self
    }

    /// Build the audit event.
    ///
    /// # Errors
    ///
    /// Returns `AuditBuildError::MissingAction` if `action()` was not called.
    /// Returns `AuditBuildError::MissingResource` if `resource()` was not called.
    /// Returns `AuditBuildError::MissingOutcome` if `outcome()` was not called.
    pub fn build(self) -> Result<AuditEvent, AuditBuildError> {
        let action = self.action.ok_or(AuditBuildError::MissingAction)?;
        let resource = self.resource.ok_or(AuditBuildError::MissingResource)?;
        let outcome = self.outcome.ok_or(AuditBuildError::MissingOutcome)?;
        Ok(AuditEvent {
            timestamp: Utc::now(),
            user_id: self.user_id,
            action,
            resource,
            outcome,
            metadata: self.metadata,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
            session_id: self.session_id,
            request_id: self.request_id,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
        })
    }
}

/// Parameters for recording an authorization decision in the audit log.
///
/// Bundles all inputs to [`AuditLogger::log_authorization_decision`] into a
/// single value so the call site is readable and the implementation is free
/// of clippy argument-count suppressions.
///
/// # Examples
///
/// ```rust
/// use xzepr::infrastructure::audit::{AuditLogger, AuthorizationDecisionParams};
///
/// let logger = AuditLogger::new();
/// logger.log_authorization_decision(&AuthorizationDecisionParams {
///     user_id: "user123".to_string(),
///     action: "read".to_string(),
///     resource_type: "event_receiver".to_string(),
///     resource_id: "recv456".to_string(),
///     decision: true,
///     duration_ms: 25,
///     fallback_used: false,
///     policy_version: Some("1.0.0".to_string()),
///     reason: None,
///     request_id: Some("req789".to_string()),
/// });
/// ```
#[derive(Debug, Clone)]
pub struct AuthorizationDecisionParams {
    /// Identifier of the user whose access is being decided.
    pub user_id: String,
    /// Action being authorized (e.g., "read", "write", "delete").
    pub action: String,
    /// Type of the resource (e.g., "event_receiver", "event").
    pub resource_type: String,
    /// Identifier of the specific resource instance.
    pub resource_id: String,
    /// `true` if access was granted, `false` if denied.
    pub decision: bool,
    /// Time taken for the authorization decision in milliseconds.
    pub duration_ms: u64,
    /// Whether the decision fell back to legacy RBAC instead of OPA.
    pub fallback_used: bool,
    /// OPA policy version, if available.
    pub policy_version: Option<String>,
    /// Reason for denial, if applicable.
    pub reason: Option<String>,
    /// Request identifier for correlation, if available.
    pub request_id: Option<String>,
}

/// Audit logger that emits structured JSON logs
#[derive(Debug, Clone)]
pub struct AuditLogger {
    /// Application name for log context
    app_name: String,
    /// Environment (production, staging, development)
    environment: String,
}

impl AuditLogger {
    /// Create a new audit logger with default settings
    pub fn new() -> Self {
        Self {
            app_name: "xzepr".to_string(),
            environment: std::env::var("XZEPR_ENVIRONMENT")
                .unwrap_or_else(|_| "development".to_string()),
        }
    }

    /// Create a new audit logger with custom settings
    pub fn with_config(app_name: impl Into<String>, environment: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            environment: environment.into(),
        }
    }

    /// Log an audit event
    ///
    /// Emits the event as a structured JSON log at INFO level for successful
    /// outcomes and WARN level for failures, denials, or errors.
    pub fn log_event(&self, event: AuditEvent) {
        match event.outcome {
            AuditOutcome::Success => {
                info!(
                    event_type = "audit",
                    app = %self.app_name,
                    env = %self.environment,
                    timestamp = %event.timestamp,
                    user_id = ?event.user_id,
                    action = %event.action,
                    resource = %event.resource,
                    outcome = %event.outcome,
                    ip_address = ?event.ip_address,
                    user_agent = ?event.user_agent,
                    session_id = ?event.session_id,
                    request_id = ?event.request_id,
                    duration_ms = ?event.duration_ms,
                    metadata = ?event.metadata,
                    "Audit event"
                );
            }
            AuditOutcome::Failure | AuditOutcome::Denied | AuditOutcome::RateLimited => {
                warn!(
                    event_type = "audit",
                    app = %self.app_name,
                    env = %self.environment,
                    timestamp = %event.timestamp,
                    user_id = ?event.user_id,
                    action = %event.action,
                    resource = %event.resource,
                    outcome = %event.outcome,
                    ip_address = ?event.ip_address,
                    user_agent = ?event.user_agent,
                    session_id = ?event.session_id,
                    request_id = ?event.request_id,
                    error_message = ?event.error_message,
                    duration_ms = ?event.duration_ms,
                    metadata = ?event.metadata,
                    "Audit event: {}",
                    event.error_message.as_deref().unwrap_or("Access denied")
                );
            }
            AuditOutcome::Error => {
                tracing::error!(
                    event_type = "audit",
                    app = %self.app_name,
                    env = %self.environment,
                    timestamp = %event.timestamp,
                    user_id = ?event.user_id,
                    action = %event.action,
                    resource = %event.resource,
                    outcome = %event.outcome,
                    ip_address = ?event.ip_address,
                    user_agent = ?event.user_agent,
                    session_id = ?event.session_id,
                    request_id = ?event.request_id,
                    error_message = ?event.error_message,
                    duration_ms = ?event.duration_ms,
                    metadata = ?event.metadata,
                    "Audit event error: {}",
                    event.error_message.as_deref().unwrap_or("Unknown error")
                );
            }
        }
    }

    /// Log authentication attempt
    pub fn log_auth_attempt(
        &self,
        user_id: Option<&str>,
        success: bool,
        reason: Option<&str>,
        ip_address: Option<&str>,
    ) {
        let result = if success {
            AuditEvent::builder()
                .user_id_opt(user_id)
                .action(AuditAction::Login)
                .resource("/auth/login")
                .outcome(AuditOutcome::Success)
                .ip_address_opt(ip_address)
                .build()
        } else {
            AuditEvent::builder()
                .user_id_opt(user_id)
                .action(AuditAction::Login)
                .resource("/auth/login")
                .outcome(AuditOutcome::Failure)
                .error_message_opt(reason)
                .ip_address_opt(ip_address)
                .build()
        };
        match result {
            Ok(event) => self.log_event(event),
            Err(e) => tracing::error!(error = %e, "Failed to build auth attempt audit event"),
        }
    }

    /// Log permission check
    pub fn log_permission_check(
        &self,
        user_id: &str,
        resource: &str,
        permission: &str,
        granted: bool,
    ) {
        let result = if granted {
            AuditEvent::permission_granted(user_id, resource, permission)
        } else {
            AuditEvent::permission_denied(user_id, resource, permission)
        };
        match result {
            Ok(event) => self.log_event(event),
            Err(e) => tracing::error!(error = %e, "Failed to build permission check audit event"),
        }
    }

    /// Log OIDC authentication
    pub fn log_oidc_auth(
        &self,
        user_id: &str,
        provider: &str,
        success: bool,
        ip_address: Option<&str>,
    ) {
        let mut metadata = HashMap::new();
        metadata.insert("provider".to_string(), provider.to_string());

        let result = AuditEvent::builder()
            .user_id(user_id)
            .action(AuditAction::OidcAuth)
            .resource("/auth/oidc/callback")
            .outcome(if success {
                AuditOutcome::Success
            } else {
                AuditOutcome::Failure
            })
            .metadata(metadata)
            .ip_address_opt(ip_address)
            .build();
        match result {
            Ok(event) => self.log_event(event),
            Err(e) => tracing::error!(error = %e, "Failed to build OIDC auth audit event"),
        }
    }

    /// Records an authorization decision in the audit log.
    ///
    /// Records authorization decisions including user, action, resource,
    /// decision outcome, duration, and whether fallback was used.
    ///
    /// # Arguments
    ///
    /// * `params` - Authorization decision parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xzepr::infrastructure::audit::{AuditLogger, AuthorizationDecisionParams};
    ///
    /// let logger = AuditLogger::new();
    /// logger.log_authorization_decision(&AuthorizationDecisionParams {
    ///     user_id: "user123".to_string(),
    ///     action: "read".to_string(),
    ///     resource_type: "event_receiver".to_string(),
    ///     resource_id: "recv456".to_string(),
    ///     decision: true,
    ///     duration_ms: 25,
    ///     fallback_used: false,
    ///     policy_version: Some("1.0.0".to_string()),
    ///     reason: None,
    ///     request_id: Some("req789".to_string()),
    /// });
    /// ```
    pub fn log_authorization_decision(&self, params: &AuthorizationDecisionParams) {
        let mut metadata = HashMap::new();
        metadata.insert("action".to_string(), params.action.clone());
        metadata.insert("resource_type".to_string(), params.resource_type.clone());
        metadata.insert("resource_id".to_string(), params.resource_id.clone());
        metadata.insert(
            "fallback_used".to_string(),
            params.fallback_used.to_string(),
        );

        if let Some(ref version) = params.policy_version {
            metadata.insert("policy_version".to_string(), version.clone());
        }

        if let Some(ref reason_text) = params.reason {
            metadata.insert("denial_reason".to_string(), reason_text.clone());
        }

        let resource_path = format!("/{}/{}", params.resource_type, params.resource_id);
        let audit_action = if params.decision {
            AuditAction::AuthorizationDecision
        } else {
            AuditAction::AuthorizationDenial
        };

        let outcome = if params.decision {
            AuditOutcome::Success
        } else {
            AuditOutcome::Denied
        };

        let result = AuditEvent::builder()
            .user_id(&params.user_id)
            .action(audit_action)
            .resource(&resource_path)
            .outcome(outcome)
            .metadata(metadata)
            .duration_ms(params.duration_ms)
            .request_id_opt(params.request_id.as_deref())
            .error_message_opt(params.reason.as_deref())
            .build();
        match result {
            Ok(event) => self.log_event(event),
            Err(e) => {
                tracing::error!(error = %e, "Failed to build authorization decision audit event")
            }
        }
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_builder() {
        let event = AuditEvent::builder()
            .user_id("user123")
            .action(AuditAction::Login)
            .resource("/auth/login")
            .outcome(AuditOutcome::Success)
            .ip_address("192.168.1.1")
            .build()
            .unwrap();

        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.action, AuditAction::Login);
        assert_eq!(event.resource, "/auth/login");
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_login_success_event() {
        let event = AuditEvent::login_success("user123", Some("192.168.1.1")).unwrap();

        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.action, AuditAction::Login);
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_login_failure_event() {
        let event = AuditEvent::login_failure("Invalid credentials", Some("192.168.1.1")).unwrap();

        assert_eq!(event.action, AuditAction::Login);
        assert_eq!(event.outcome, AuditOutcome::Failure);
        assert_eq!(event.error_message, Some("Invalid credentials".to_string()));
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_permission_denied_event() {
        let event = AuditEvent::permission_denied("user123", "/api/admin", "admin:write").unwrap();

        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.action, AuditAction::PermissionCheck);
        assert_eq!(event.resource, "/api/admin");
        assert_eq!(event.outcome, AuditOutcome::Denied);
        assert_eq!(
            event.metadata.get("permission"),
            Some(&"admin:write".to_string())
        );
    }

    #[test]
    fn test_permission_granted_event() {
        let event = AuditEvent::permission_granted("user123", "/api/events", "event:read").unwrap();

        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.action, AuditAction::PermissionCheck);
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(
            event.metadata.get("permission"),
            Some(&"event:read".to_string())
        );
    }

    #[test]
    fn test_audit_logger_creation() {
        let logger = AuditLogger::new();
        assert_eq!(logger.app_name, "xzepr");
    }

    #[test]
    fn test_audit_logger_with_config() {
        let logger = AuditLogger::with_config("test-app", "production");
        assert_eq!(logger.app_name, "test-app");
        assert_eq!(logger.environment, "production");
    }

    #[test]
    fn test_audit_logger_log_event() {
        let logger = AuditLogger::new();
        let event = AuditEvent::login_success("user123", Some("192.168.1.1")).unwrap();
        logger.log_event(event);
    }

    #[test]
    fn test_audit_logger_log_auth_attempt() {
        let logger = AuditLogger::new();

        logger.log_auth_attempt(Some("user123"), true, None, Some("192.168.1.1"));
        logger.log_auth_attempt(None, false, Some("Invalid token"), Some("192.168.1.2"));
    }

    #[test]
    fn test_audit_logger_log_permission_check() {
        let logger = AuditLogger::new();

        logger.log_permission_check("user123", "/api/events", "event:read", true);
        logger.log_permission_check("user456", "/api/admin", "admin:write", false);
    }

    #[test]
    fn test_audit_logger_log_oidc_auth() {
        let logger = AuditLogger::new();

        logger.log_oidc_auth("user123", "keycloak", true, Some("192.168.1.1"));
        logger.log_oidc_auth("user456", "google", false, Some("192.168.1.2"));
    }

    #[test]
    fn test_event_serialization() {
        let event = AuditEvent::builder()
            .user_id("user123")
            .action(AuditAction::Login)
            .resource("/auth/login")
            .outcome(AuditOutcome::Success)
            .ip_address("192.168.1.1")
            .user_agent("Mozilla/5.0")
            .build()
            .unwrap();

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("user123"));
        assert!(json.contains("login"));
        assert!(json.contains("success"));
    }

    #[test]
    fn test_metadata_builder() {
        let event = AuditEvent::builder()
            .user_id("user123")
            .action(AuditAction::UserUpdate)
            .resource("/api/users/user123")
            .outcome(AuditOutcome::Success)
            .add_metadata("field", "email")
            .add_metadata("old_value", "old@example.com")
            .add_metadata("new_value", "new@example.com")
            .build()
            .unwrap();

        assert_eq!(event.metadata.len(), 3);
        assert_eq!(event.metadata.get("field"), Some(&"email".to_string()));
    }

    #[test]
    fn test_action_display() {
        assert_eq!(AuditAction::Login.to_string(), "login");
        assert_eq!(AuditAction::PermissionCheck.to_string(), "permission_check");
        assert_eq!(AuditAction::OidcAuth.to_string(), "oidc_auth");
    }

    #[test]
    fn test_outcome_display() {
        assert_eq!(AuditOutcome::Success.to_string(), "success");
        assert_eq!(AuditOutcome::Failure.to_string(), "failure");
        assert_eq!(AuditOutcome::Denied.to_string(), "denied");
    }

    #[test]
    fn test_log_authorization_decision_allowed() {
        let logger = AuditLogger::new();

        logger.log_authorization_decision(&AuthorizationDecisionParams {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "recv456".to_string(),
            decision: true,
            duration_ms: 25,
            fallback_used: false,
            policy_version: Some("1.0.0".to_string()),
            reason: None,
            request_id: Some("req789".to_string()),
        });
    }

    #[test]
    fn test_log_authorization_decision_denied() {
        let logger = AuditLogger::new();

        logger.log_authorization_decision(&AuthorizationDecisionParams {
            user_id: "user123".to_string(),
            action: "write".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "recv456".to_string(),
            decision: false,
            duration_ms: 30,
            fallback_used: false,
            policy_version: Some("1.0.0".to_string()),
            reason: Some("insufficient_permissions".to_string()),
            request_id: Some("req789".to_string()),
        });
    }

    #[test]
    fn test_log_authorization_decision_with_fallback() {
        let logger = AuditLogger::new();

        logger.log_authorization_decision(&AuthorizationDecisionParams {
            user_id: "user123".to_string(),
            action: "delete".to_string(),
            resource_type: "event".to_string(),
            resource_id: "event789".to_string(),
            decision: true,
            duration_ms: 45,
            fallback_used: true,
            policy_version: None,
            reason: None,
            request_id: Some("req101".to_string()),
        });
    }

    #[test]
    fn test_log_authorization_decision_no_policy_version() {
        let logger = AuditLogger::new();

        logger.log_authorization_decision(&AuthorizationDecisionParams {
            user_id: "user456".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver_group".to_string(),
            resource_id: "group123".to_string(),
            decision: true,
            duration_ms: 20,
            fallback_used: false,
            policy_version: None,
            reason: None,
            request_id: None,
        });
    }
}
