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
//!     .build();
//!
//! logger.log_event(event);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

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

    /// Create a login event
    pub fn login_success(user_id: &str, ip_address: Option<&str>) -> Self {
        Self::builder()
            .user_id(user_id)
            .action(AuditAction::Login)
            .resource("/auth/login")
            .outcome(AuditOutcome::Success)
            .ip_address_opt(ip_address)
            .build()
    }

    /// Create a login failure event
    pub fn login_failure(reason: &str, ip_address: Option<&str>) -> Self {
        Self::builder()
            .action(AuditAction::Login)
            .resource("/auth/login")
            .outcome(AuditOutcome::Failure)
            .error_message(reason)
            .ip_address_opt(ip_address)
            .build()
    }

    /// Create a permission denied event
    pub fn permission_denied(user_id: &str, resource: &str, permission: &str) -> Self {
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

    /// Create a permission granted event
    pub fn permission_granted(user_id: &str, resource: &str, permission: &str) -> Self {
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

    /// Build the audit event
    pub fn build(self) -> AuditEvent {
        AuditEvent {
            timestamp: Utc::now(),
            user_id: self.user_id,
            action: self.action.expect("action is required"),
            resource: self.resource.expect("resource is required"),
            outcome: self.outcome.expect("outcome is required"),
            metadata: self.metadata,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
            session_id: self.session_id,
            request_id: self.request_id,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
        }
    }
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
        let event = if success {
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

        self.log_event(event);
    }

    /// Log permission check
    pub fn log_permission_check(
        &self,
        user_id: &str,
        resource: &str,
        permission: &str,
        granted: bool,
    ) {
        let event = if granted {
            AuditEvent::permission_granted(user_id, resource, permission)
        } else {
            AuditEvent::permission_denied(user_id, resource, permission)
        };

        self.log_event(event);
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

        let event = AuditEvent::builder()
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

        self.log_event(event);
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
            .build();

        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.action, AuditAction::Login);
        assert_eq!(event.resource, "/auth/login");
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_login_success_event() {
        let event = AuditEvent::login_success("user123", Some("192.168.1.1"));

        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.action, AuditAction::Login);
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_login_failure_event() {
        let event = AuditEvent::login_failure("Invalid credentials", Some("192.168.1.1"));

        assert_eq!(event.action, AuditAction::Login);
        assert_eq!(event.outcome, AuditOutcome::Failure);
        assert_eq!(event.error_message, Some("Invalid credentials".to_string()));
        assert_eq!(event.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_permission_denied_event() {
        let event = AuditEvent::permission_denied("user123", "/api/admin", "admin:write");

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
        let event = AuditEvent::permission_granted("user123", "/api/events", "event:read");

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
        let event = AuditEvent::login_success("user123", Some("192.168.1.1"));

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
            .build();

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
            .build();

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
        assert_eq!(AuditOutcome::RateLimited.to_string(), "rate_limited");
    }
}
