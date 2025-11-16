// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OPA types for authorization requests and responses
//!
//! This module defines the core types used for communication with the Open Policy Agent
//! service, including request/response structures, configuration, and error types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// OPA client configuration
///
/// Configuration for connecting to and interacting with an OPA server.
///
/// # Examples
///
/// ```
/// use xzepr::opa::types::OpaConfig;
///
/// let config = OpaConfig {
///     enabled: true,
///     url: "http://localhost:8181".to_string(),
///     timeout_seconds: 5,
///     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
///     bundle_url: None,
///     cache_ttl_seconds: 300,
/// };
///
/// assert_eq!(config.url, "http://localhost:8181");
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

fn default_timeout() -> u64 {
    5
}

fn default_cache_ttl() -> u64 {
    300
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
    /// use xzepr::opa::types::OpaConfig;
    ///
    /// let config = OpaConfig {
    ///     enabled: true,
    ///     url: "http://localhost:8181".to_string(),
    ///     timeout_seconds: 5,
    ///     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
    ///     bundle_url: None,
    ///     cache_ttl_seconds: 300,
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
        }

        Ok(())
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
/// use xzepr::opa::types::{OpaRequest, OpaInput};
/// use uuid::Uuid;
///
/// let input = OpaInput {
///     user: UserContext {
///         user_id: Uuid::new_v4().to_string(),
///         username: "alice".to_string(),
///         roles: vec!["user".to_string()],
///         groups: vec![],
///     },
///     action: "create".to_string(),
///     resource: ResourceContext {
///         resource_type: "event".to_string(),
///         resource_id: Some(Uuid::new_v4().to_string()),
///         owner_id: Some(Uuid::new_v4().to_string()),
///         group_id: None,
///         members: vec![],
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
}

/// OPA policy evaluation response
///
/// Response from OPA containing the authorization decision.
///
/// # Examples
///
/// ```
/// use xzepr::opa::types::OpaResponse;
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
        assert_eq!(response.result.unwrap().allow, true);
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
