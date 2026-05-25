// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OPA types for authorization requests and responses
//!
//! This module defines the core types used for communication with the Open Policy Agent
//! service, including request/response structures, configuration, and error types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// OPA fail-safe behavior when policy evaluation is unavailable.
///
/// Three modes are available:
///
/// - `FailClosed` (default): deny all requests when OPA is unreachable; safe for production.
/// - `FailOpenDevelopment`: allow all requests when OPA is unreachable; restricted to non-production
///   environments only.
/// - `LegacyRbacFallback`: fall back to the built-in RBAC engine when OPA is unreachable;
///   permitted in production with a startup warning.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpaFailSafeMode {
    /// Deny requests when OPA cannot produce a decision.
    #[default]
    FailClosed,
    /// Allow development fallback behavior when OPA is unavailable.
    ///
    /// This mode is NOT permitted in production deployments.
    FailOpenDevelopment,
    /// Fall back to legacy RBAC checks when OPA is unavailable.
    ///
    /// This allows the system to continue operating using the built-in role-based
    /// access control when the OPA server cannot be reached. Audit records will
    /// label these decisions as `unavailable_legacy_rbac_*` to distinguish them
    /// from normal OPA decisions. Allowed in production with a startup warning.
    LegacyRbacFallback,
}

/// Outcome label for an OPA authorization decision.
///
/// Used to distinguish between normal OPA decisions and fallback/error cases
/// in audit logs and Prometheus metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpaDecisionOutcome {
    /// OPA evaluated the policy and allowed the request.
    OpaAllow,
    /// OPA evaluated the policy and denied the request.
    OpaDeny,
    /// OPA was unavailable; fail-closed mode denied the request.
    UnavailableFailClosed,
    /// OPA was unavailable; fail-open-development mode allowed the request
    /// (only permitted when `RUST_ENV` is not `production`).
    UnavailableFailOpenDevelopment,
    /// OPA was unavailable; legacy RBAC fallback allowed the request.
    UnavailableLegacyRbacAllow,
    /// OPA was unavailable; legacy RBAC fallback denied the request.
    UnavailableLegacyRbacDeny,
    /// Resource context could not be built; the request was denied.
    ResourceContextMissing,
}

impl OpaDecisionOutcome {
    /// Returns a short snake_case label suitable for use as a Prometheus metric label.
    pub fn as_metric_label(&self) -> &'static str {
        match self {
            OpaDecisionOutcome::OpaAllow => "opa_allow",
            OpaDecisionOutcome::OpaDeny => "opa_deny",
            OpaDecisionOutcome::UnavailableFailClosed => "unavailable_fail_closed",
            OpaDecisionOutcome::UnavailableFailOpenDevelopment => "unavailable_fail_open_dev",
            OpaDecisionOutcome::UnavailableLegacyRbacAllow => "unavailable_legacy_rbac_allow",
            OpaDecisionOutcome::UnavailableLegacyRbacDeny => "unavailable_legacy_rbac_deny",
            OpaDecisionOutcome::ResourceContextMissing => "resource_context_missing",
        }
    }

    /// Returns `true` if this outcome represents an allowed request.
    pub fn is_allowed(&self) -> bool {
        matches!(
            self,
            OpaDecisionOutcome::OpaAllow
                | OpaDecisionOutcome::UnavailableFailOpenDevelopment
                | OpaDecisionOutcome::UnavailableLegacyRbacAllow
        )
    }
}

/// OPA client configuration
///
/// Configuration for connecting to and interacting with an OPA server.
///
/// # Examples
///
/// ```
/// use xzepr::opa::types::{OpaConfig, OpaFailSafeMode};
///
/// let config = OpaConfig {
///     enabled: true,
///     url: "http://localhost:8181".to_string(),
///     timeout_seconds: 5,
///     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
///     bundle_url: None,
///     cache_ttl_seconds: 300,
///     allowed_hosts: vec!["localhost:8181".to_string()],
///     fail_safe_mode: OpaFailSafeMode::FailClosed,
/// };
///
/// assert_eq!(config.url, "http://localhost:8181");
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpaConfig {
    /// Whether OPA authorization is enabled
    #[serde(default)]
    pub enabled: bool,

    /// OPA server URL (e.g., http://localhost:8181)
    pub url: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Policy evaluation path (e.g., /v1/data/xzepr/rbac/allow)
    pub policy_path: String,

    /// Optional bundle server URL for policy distribution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_url: Option<String>,

    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,

    /// Allowed OPA and bundle server hosts for production deployments.
    #[serde(default)]
    pub allowed_hosts: Vec<String>,

    /// Fail-safe behavior when OPA is unavailable.
    #[serde(default)]
    pub fail_safe_mode: OpaFailSafeMode,
}

fn default_timeout() -> u64 {
    5
}

fn default_cache_ttl() -> u64 {
    300
}

impl Default for OpaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "http://localhost:8181".to_string(),
            timeout_seconds: default_timeout(),
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: default_cache_ttl(),
            allowed_hosts: Vec::new(),
            fail_safe_mode: OpaFailSafeMode::FailClosed,
        }
    }
}

impl OpaConfig {
    /// Validates the OPA configuration
    ///
    /// # Errors
    ///
    /// Returns `OpaError::ConfigurationError` if configuration is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::types::{OpaConfig, OpaFailSafeMode};
    ///
    /// let config = OpaConfig {
    ///     enabled: true,
    ///     url: "http://localhost:8181".to_string(),
    ///     timeout_seconds: 5,
    ///     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
    ///     bundle_url: None,
    ///     cache_ttl_seconds: 300,
    ///     allowed_hosts: vec!["localhost:8181".to_string()],
    ///     fail_safe_mode: OpaFailSafeMode::FailClosed,
    /// };
    ///
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), OpaError> {
        if self.enabled {
            if self.url.is_empty() {
                return Err(OpaError::ConfigurationError(
                    "OPA URL cannot be empty when enabled".to_string(),
                ));
            }

            if self.policy_path.is_empty() {
                return Err(OpaError::ConfigurationError(
                    "OPA policy path cannot be empty when enabled".to_string(),
                ));
            }

            if self.timeout_seconds == 0 {
                return Err(OpaError::ConfigurationError(
                    "OPA timeout must be greater than 0".to_string(),
                ));
            }

            if self.cache_ttl_seconds == 0 {
                return Err(OpaError::ConfigurationError(
                    "OPA cache TTL must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validates the OPA configuration for production use.
    ///
    /// # Errors
    ///
    /// Returns `OpaError::ConfigurationError` if OPA is enabled with insecure
    /// URLs, missing host allowlists, or fail-open behavior in production.
    pub fn validate_production(&self) -> Result<(), OpaError> {
        self.validate()?;

        if !self.enabled {
            return Ok(());
        }

        if !self.url.starts_with("https://") {
            return Err(OpaError::ConfigurationError(
                "OPA URL must use HTTPS in production".to_string(),
            ));
        }

        if self.allowed_hosts.is_empty() {
            return Err(OpaError::ConfigurationError(
                "OPA allowed_hosts cannot be empty in production".to_string(),
            ));
        }

        validate_allowed_host(&self.url, &self.allowed_hosts, "OPA URL")?;

        if let Some(bundle_url) = &self.bundle_url {
            if !bundle_url.starts_with("https://") {
                return Err(OpaError::ConfigurationError(
                    "OPA bundle URL must use HTTPS in production".to_string(),
                ));
            }
            validate_allowed_host(bundle_url, &self.allowed_hosts, "OPA bundle URL")?;
        }

        if self.fail_safe_mode == OpaFailSafeMode::FailOpenDevelopment {
            return Err(OpaError::ConfigurationError(
                "OPA fail_safe_mode cannot be fail_open_development in production; \
                 use fail_closed or legacy_rbac_fallback"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

fn validate_allowed_host(url: &str, allowed_hosts: &[String], label: &str) -> Result<(), OpaError> {
    let host = extract_host(url).ok_or_else(|| {
        OpaError::ConfigurationError(format!("{} must include a valid host", label))
    })?;

    if allowed_hosts.iter().any(|allowed| allowed == &host) {
        Ok(())
    } else {
        Err(OpaError::ConfigurationError(format!(
            "{} host '{}' is not in allowed_hosts",
            label, host
        )))
    }
}

fn extract_host(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let host = rest.split('/').next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

/// OPA client errors
///
/// Errors that can occur during OPA policy evaluation.
#[derive(Error, Debug)]
pub enum OpaError {
    /// HTTP request to OPA server failed
    #[error("OPA request failed: {0}")]
    RequestFailed(String),

    /// Invalid response from OPA server
    #[error("Invalid OPA response: {0}")]
    InvalidResponse(String),

    /// Policy evaluation returned an error
    #[error("Policy evaluation error: {0}")]
    EvaluationError(String),

    /// Request timed out
    #[error("OPA request timed out after {0} seconds")]
    Timeout(u64),

    /// Configuration error
    #[error("OPA configuration error: {0}")]
    ConfigurationError(String),

    /// Circuit breaker is open
    #[error("Circuit breaker is open, OPA unavailable")]
    CircuitOpen,
}

/// OPA policy evaluation request
///
/// Wrapper for the input sent to OPA for policy evaluation.
///
/// # Examples
///
/// ```
/// use xzepr::opa::types::{OpaRequest, OpaInput, UserContext, ResourceContext};
///
/// let input = OpaInput {
///     user: UserContext {
///         user_id: "user-123-abc".to_string(),
///         username: "alice".to_string(),
///         roles: vec!["user".to_string()],
///         groups: vec![],
///     },
///     action: "create".to_string(),
///     resource: ResourceContext {
///         resource_type: "event".to_string(),
///         resource_id: Some("event-456-def".to_string()),
///         owner_id: Some("user-123-abc".to_string()),
///         group_id: None,
///         members: vec![],
///         resource_version: 1,
///     },
/// };
///
/// let request = OpaRequest { input };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaRequest {
    /// Input data for policy evaluation
    pub input: OpaInput,
}

/// Input data for OPA policy evaluation
///
/// Contains user context, action being performed, and resource context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaInput {
    /// User performing the action
    pub user: UserContext,

    /// Action being performed (e.g., "create", "read", "update", "delete")
    pub action: String,

    /// Resource being accessed
    pub resource: ResourceContext,
}

/// User context for authorization
///
/// Information about the user requesting access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User ID (UUID)
    pub user_id: String,

    /// Username
    pub username: String,

    /// User roles (e.g., ["admin", "user"])
    pub roles: Vec<String>,

    /// Groups the user belongs to
    pub groups: Vec<String>,
}

/// Resource context for authorization
///
/// Information about the resource being accessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContext {
    /// Resource type (e.g., "event", "event_receiver", "event_receiver_group")
    pub resource_type: String,

    /// Optional resource ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,

    /// Optional resource owner ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,

    /// Optional group ID for group-owned resources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,

    /// Group members (user IDs) if resource is group-owned
    #[serde(default)]
    pub members: Vec<String>,

    /// Resource version for cache invalidation
    #[serde(default = "default_resource_version")]
    pub resource_version: i64,
}

fn default_resource_version() -> i64 {
    1
}

/// OPA policy evaluation response
///
/// Response from OPA containing the authorization decision.
///
/// # Examples
///
/// ```
/// use xzepr::opa::types::{OpaResponse, AuthorizationDecision};
///
/// let response = OpaResponse {
///     result: Some(AuthorizationDecision {
///         allow: true,
///         reason: Some("User is owner".to_string()),
///         metadata: None,
///     }),
/// };
///
/// assert_eq!(response.result.unwrap().allow, true);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaResponse {
    /// Authorization decision result
    pub result: Option<AuthorizationDecision>,
}

/// Authorization decision from OPA policy
///
/// The actual decision made by the policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationDecision {
    /// Whether the action is allowed
    pub allow: bool,

    /// Optional reason for the decision (for audit logging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Optional metadata about the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opa_config_validate_success() {
        let config = OpaConfig {
            enabled: true,
            url: "http://localhost:8181".to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
            ..OpaConfig::default()
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_opa_config_validate_empty_url() {
        let config = OpaConfig {
            enabled: true,
            url: "".to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
            ..OpaConfig::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_opa_config_validate_empty_policy_path() {
        let config = OpaConfig {
            enabled: true,
            url: "http://localhost:8181".to_string(),
            timeout_seconds: 5,
            policy_path: "".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
            ..OpaConfig::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_opa_config_validate_zero_timeout() {
        let config = OpaConfig {
            enabled: true,
            url: "http://localhost:8181".to_string(),
            timeout_seconds: 0,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
            ..OpaConfig::default()
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_opa_config_validate_disabled() {
        let config = OpaConfig {
            enabled: false,
            url: "".to_string(),
            timeout_seconds: 0,
            policy_path: "".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
            ..OpaConfig::default()
        };

        // Should pass validation when disabled
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_opa_request_serialization() {
        let input = OpaInput {
            user: UserContext {
                user_id: "user123".to_string(),
                username: "alice".to_string(),
                roles: vec!["user".to_string()],
                groups: vec![],
            },
            action: "create".to_string(),
            resource: ResourceContext {
                resource_type: "event".to_string(),
                resource_id: Some("event123".to_string()),
                owner_id: Some("user123".to_string()),
                group_id: None,
                members: vec![],
                resource_version: 1,
            },
        };

        let request = OpaRequest { input };
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("user123"));
        assert!(json.contains("create"));
        assert!(json.contains("event"));
    }

    #[test]
    fn test_opa_response_deserialization() {
        let json = r#"{
            "result": {
                "allow": true,
                "reason": "User is owner"
            }
        }"#;

        let response: OpaResponse = serde_json::from_str(json).unwrap();
        assert!(response.result.unwrap().allow);
    }

    #[test]
    fn test_opa_fail_safe_mode_legacy_rbac_fallback_default_serialization() {
        let mode = OpaFailSafeMode::LegacyRbacFallback;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"legacy_rbac_fallback\"");
    }

    #[test]
    fn test_opa_decision_outcome_is_allowed_opa_allow() {
        assert!(OpaDecisionOutcome::OpaAllow.is_allowed());
    }

    #[test]
    fn test_opa_decision_outcome_is_allowed_opa_deny() {
        assert!(!OpaDecisionOutcome::OpaDeny.is_allowed());
    }

    #[test]
    fn test_opa_decision_outcome_is_allowed_fail_closed() {
        assert!(!OpaDecisionOutcome::UnavailableFailClosed.is_allowed());
    }

    #[test]
    fn test_opa_decision_outcome_is_allowed_fail_open_dev() {
        assert!(OpaDecisionOutcome::UnavailableFailOpenDevelopment.is_allowed());
    }

    #[test]
    fn test_opa_decision_outcome_is_allowed_legacy_rbac_allow() {
        assert!(OpaDecisionOutcome::UnavailableLegacyRbacAllow.is_allowed());
    }

    #[test]
    fn test_opa_decision_outcome_is_allowed_legacy_rbac_deny() {
        assert!(!OpaDecisionOutcome::UnavailableLegacyRbacDeny.is_allowed());
    }

    #[test]
    fn test_opa_decision_outcome_is_allowed_resource_context_missing() {
        assert!(!OpaDecisionOutcome::ResourceContextMissing.is_allowed());
    }

    #[test]
    fn test_opa_decision_outcome_as_metric_label() {
        assert_eq!(OpaDecisionOutcome::OpaAllow.as_metric_label(), "opa_allow");
        assert_eq!(OpaDecisionOutcome::OpaDeny.as_metric_label(), "opa_deny");
        assert_eq!(
            OpaDecisionOutcome::UnavailableFailClosed.as_metric_label(),
            "unavailable_fail_closed"
        );
        assert_eq!(
            OpaDecisionOutcome::UnavailableFailOpenDevelopment.as_metric_label(),
            "unavailable_fail_open_dev"
        );
        assert_eq!(
            OpaDecisionOutcome::UnavailableLegacyRbacAllow.as_metric_label(),
            "unavailable_legacy_rbac_allow"
        );
        assert_eq!(
            OpaDecisionOutcome::UnavailableLegacyRbacDeny.as_metric_label(),
            "unavailable_legacy_rbac_deny"
        );
        assert_eq!(
            OpaDecisionOutcome::ResourceContextMissing.as_metric_label(),
            "resource_context_missing"
        );
    }

    #[test]
    fn test_validate_production_rejects_fail_open_development() {
        let config = OpaConfig {
            enabled: true,
            url: "https://opa.example.com".to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
            allowed_hosts: vec!["opa.example.com".to_string()],
            fail_safe_mode: OpaFailSafeMode::FailOpenDevelopment,
        };
        let result = config.validate_production();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("fail_open_development"));
    }

    #[test]
    fn test_validate_production_allows_legacy_rbac_fallback() {
        let config = OpaConfig {
            enabled: true,
            url: "https://opa.example.com".to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
            allowed_hosts: vec!["opa.example.com".to_string()],
            fail_safe_mode: OpaFailSafeMode::LegacyRbacFallback,
        };
        assert!(config.validate_production().is_ok());
    }

    #[test]
    fn test_opa_decision_outcome_serialization() {
        let outcome = OpaDecisionOutcome::OpaAllow;
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(json, "\"opa_allow\"");
    }

    #[test]
    fn test_authorization_decision_with_metadata() {
        let decision = AuthorizationDecision {
            allow: true,
            reason: Some("Admin access".to_string()),
            metadata: Some(HashMap::from([
                ("role".to_string(), serde_json::json!("admin")),
                ("scope".to_string(), serde_json::json!("global")),
            ])),
        };

        assert!(decision.allow);
        assert_eq!(decision.reason.unwrap(), "Admin access");
        assert!(decision.metadata.is_some());
    }
}
