// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OIDC Callback Handler
//!
//! This module provides handlers for OIDC callback operations, including
//! authorization code exchange and user provisioning from OIDC claims.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use super::client::{OidcAuthResult, OidcClient, OidcError};
use crate::auth::rbac::Role;

/// Errors that can occur during callback handling
#[derive(Error, Debug)]
pub enum CallbackError {
    /// OIDC client error
    #[error("OIDC error: {0}")]
    Oidc(#[from] OidcError),

    /// User provisioning error
    #[error("Failed to provision user: {0}")]
    Provisioning(String),

    /// Missing required claim
    #[error("Missing required claim: {0}")]
    MissingClaim(String),

    /// Role mapping error
    #[error("Failed to map role: {0}")]
    RoleMapping(String),
}

/// OIDC callback query parameters
#[derive(Debug, Deserialize)]
pub struct OidcCallbackQuery {
    /// Authorization code
    pub code: String,

    /// State parameter (CSRF token)
    pub state: String,

    /// Optional error from provider
    pub error: Option<String>,

    /// Optional error description
    pub error_description: Option<String>,
}

/// Session data stored during OIDC flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcSession {
    /// Expected state value
    pub state: String,

    /// PKCE verifier
    pub pkce_verifier: Option<String>,

    /// Nonce for ID token validation
    pub nonce: String,

    /// Original redirect URL (where to send user after login)
    pub redirect_to: Option<String>,
}

/// User data extracted from OIDC claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserData {
    /// Subject (unique user ID from provider)
    pub sub: String,

    /// Email address
    pub email: Option<String>,

    /// Email verification status
    pub email_verified: bool,

    /// Preferred username
    pub username: String,

    /// Full name
    pub name: Option<String>,

    /// Given name
    pub given_name: Option<String>,

    /// Family name
    pub family_name: Option<String>,

    /// Mapped roles
    pub roles: Vec<Role>,
}

/// Callback handler for OIDC authentication
pub struct OidcCallbackHandler {
    /// OIDC client
    client: Arc<OidcClient>,

    /// Role mapping configuration
    role_mappings: RoleMappings,
}

/// Role mapping configuration
#[derive(Debug, Clone)]
pub struct RoleMappings {
    /// Map of OIDC role names to internal roles
    mappings: std::collections::HashMap<String, Role>,

    /// Default role if no roles are provided
    default_role: Role,
}

impl Default for RoleMappings {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleMappings {
    /// Create new role mappings with defaults
    ///
    /// Default mappings:
    /// - "admin" -> Role::Admin
    /// - "manager" -> Role::Manager
    /// - "user" -> Role::User
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::auth::oidc::callback::RoleMappings;
    ///
    /// let mappings = RoleMappings::new();
    /// ```
    pub fn new() -> Self {
        let mut mappings = std::collections::HashMap::new();
        mappings.insert("admin".to_string(), Role::Admin);
        mappings.insert("administrator".to_string(), Role::Admin);
        mappings.insert("manager".to_string(), Role::EventManager);
        mappings.insert("event_manager".to_string(), Role::EventManager);
        mappings.insert("user".to_string(), Role::User);

        Self {
            mappings,
            default_role: Role::User,
        }
    }

    /// Add a role mapping
    ///
    /// # Arguments
    ///
    /// * `oidc_role` - Role name from OIDC provider
    /// * `internal_role` - Internal role to map to
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::auth::oidc::callback::RoleMappings;
    /// use xzepr::auth::rbac::Role;
    ///
    /// let mut mappings = RoleMappings::new();
    /// mappings.add_mapping("superuser".to_string(), Role::Admin);
    /// ```
    pub fn add_mapping(&mut self, oidc_role: String, internal_role: Role) {
        self.mappings.insert(oidc_role, internal_role);
    }

    /// Set default role
    ///
    /// # Arguments
    ///
    /// * `role` - Default role to assign when no roles are provided
    pub fn set_default_role(&mut self, role: Role) {
        self.default_role = role;
    }

    /// Map OIDC roles to internal roles
    ///
    /// # Arguments
    ///
    /// * `oidc_roles` - List of role names from OIDC provider
    ///
    /// # Returns
    ///
    /// Returns list of internal roles. If no roles match, returns default role.
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::auth::oidc::callback::RoleMappings;
    /// use xzepr::auth::rbac::Role;
    ///
    /// let mappings = RoleMappings::new();
    /// let roles = mappings.map_roles(&vec!["admin".to_string()]);
    /// assert_eq!(roles, vec![Role::Admin]);
    /// ```
    pub fn map_roles(&self, oidc_roles: &[String]) -> Vec<Role> {
        if oidc_roles.is_empty() {
            return vec![self.default_role];
        }

        let mut mapped_roles = Vec::new();
        let mut role_set = std::collections::HashSet::new();

        for oidc_role in oidc_roles {
            let role_lower = oidc_role.to_lowercase();
            if let Some(role) = self.mappings.get(&role_lower) {
                if role_set.insert(*role) {
                    mapped_roles.push(*role);
                }
            }
        }

        if mapped_roles.is_empty() {
            mapped_roles.push(self.default_role);
        }

        mapped_roles
    }
}

impl OidcCallbackHandler {
    /// Create a new callback handler
    ///
    /// # Arguments
    ///
    /// * `client` - OIDC client
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use xzepr::auth::oidc::{OidcClient, OidcConfig, callback::OidcCallbackHandler};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = OidcConfig::keycloak(
    ///     "https://keycloak.example.com/realms/xzepr".to_string(),
    ///     "xzepr-client".to_string(),
    ///     "secret-at-least-16-chars".to_string(),
    ///     "https://app.example.com/callback".to_string(),
    /// );
    /// let client = Arc::new(OidcClient::new(config).await?);
    /// let handler = OidcCallbackHandler::new(client);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(client: Arc<OidcClient>) -> Self {
        Self {
            client,
            role_mappings: RoleMappings::new(),
        }
    }

    /// Create a new callback handler with custom role mappings
    ///
    /// # Arguments
    ///
    /// * `client` - OIDC client
    /// * `role_mappings` - Custom role mappings
    pub fn with_role_mappings(client: Arc<OidcClient>, role_mappings: RoleMappings) -> Self {
        Self {
            client,
            role_mappings,
        }
    }

    /// Handle OIDC callback
    ///
    /// # Arguments
    ///
    /// * `query` - Callback query parameters
    /// * `session` - OIDC session data
    ///
    /// # Returns
    ///
    /// Returns authentication result and user data
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Provider returns error
    /// - State validation fails
    /// - Code exchange fails
    /// - Required claims are missing
    pub async fn handle_callback(
        &self,
        query: OidcCallbackQuery,
        session: OidcSession,
    ) -> Result<(OidcAuthResult, OidcUserData), CallbackError> {
        if let Some(error) = query.error {
            let description = query
                .error_description
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(CallbackError::Oidc(OidcError::TokenExchange(format!(
                "{}: {}",
                error, description
            ))));
        }

        let auth_result = self
            .client
            .exchange_code(
                query.code,
                query.state,
                session.state,
                session.pkce_verifier,
                session.nonce,
            )
            .await?;

        let user_data = self.extract_user_data(&auth_result)?;

        Ok((auth_result, user_data))
    }

    /// Extract user data from authentication result
    fn extract_user_data(
        &self,
        auth_result: &OidcAuthResult,
    ) -> Result<OidcUserData, CallbackError> {
        let claims = &auth_result.claims;

        let username = claims
            .preferred_username
            .clone()
            .or_else(|| claims.email.clone())
            .ok_or_else(|| {
                CallbackError::MissingClaim("preferred_username or email".to_string())
            })?;

        let roles = self.role_mappings.map_roles(&claims.roles);

        Ok(OidcUserData {
            sub: claims.sub.clone(),
            email: claims.email.clone(),
            email_verified: claims.email_verified.unwrap_or(false),
            username,
            name: claims.name.clone(),
            given_name: claims.given_name.clone(),
            family_name: claims.family_name.clone(),
            roles,
        })
    }

    /// Get role mappings
    pub fn role_mappings(&self) -> &RoleMappings {
        &self.role_mappings
    }

    /// Get role mappings mutably
    pub fn role_mappings_mut(&mut self) -> &mut RoleMappings {
        &mut self.role_mappings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_mappings_new() {
        let mappings = RoleMappings::new();
        assert_eq!(mappings.default_role, Role::User);
    }

    #[test]
    fn test_role_mappings_add_mapping() {
        let mut mappings = RoleMappings::new();
        mappings.add_mapping("superuser".to_string(), Role::Admin);

        let roles = mappings.map_roles(&["superuser".to_string()]);
        assert_eq!(roles, vec![Role::Admin]);
    }

    #[test]
    fn test_role_mappings_default() {
        let mappings = RoleMappings::new();
        let roles = mappings.map_roles(&[]);
        assert_eq!(roles, vec![Role::User]);
    }

    #[test]
    fn test_role_mappings_unknown_role() {
        let mappings = RoleMappings::new();
        let roles = mappings.map_roles(&["unknown".to_string()]);
        assert_eq!(roles, vec![Role::User]);
    }

    #[test]
    fn test_role_mappings_multiple_roles() {
        let mappings = RoleMappings::new();
        let roles = mappings.map_roles(&["admin".to_string(), "manager".to_string()]);
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&Role::Admin));
        assert!(roles.contains(&Role::EventManager));
    }

    #[test]
    fn test_role_mappings_case_insensitive() {
        let mappings = RoleMappings::new();
        let roles = mappings.map_roles(&["ADMIN".to_string()]);
        assert_eq!(roles, vec![Role::Admin]);
    }

    #[test]
    fn test_role_mappings_duplicates() {
        let mappings = RoleMappings::new();
        let roles = mappings.map_roles(&["admin".to_string(), "administrator".to_string()]);
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], Role::Admin);
    }

    #[test]
    fn test_oidc_session_serialization() {
        let session = OidcSession {
            state: "state123".to_string(),
            pkce_verifier: Some("verifier".to_string()),
            nonce: "nonce123".to_string(),
            redirect_to: Some("/dashboard".to_string()),
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("state123"));
        assert!(json.contains("nonce123"));
    }

    #[test]
    fn test_oidc_callback_query_with_error() {
        let json = r#"{
            "code": "",
            "state": "state",
            "error": "access_denied",
            "error_description": "User denied access"
        }"#;

        let query: OidcCallbackQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.error, Some("access_denied".to_string()));
        assert!(query.error_description.is_some());
    }

    #[test]
    fn test_oidc_user_data_serialization() {
        let user_data = OidcUserData {
            sub: "user123".to_string(),
            email: Some("user@example.com".to_string()),
            email_verified: true,
            username: "user123".to_string(),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            roles: vec![Role::Admin],
        };

        let json = serde_json::to_string(&user_data).unwrap();
        assert!(json.contains("user123"));
        assert!(json.contains("user@example.com"));
    }

    #[test]
    fn test_callback_error_display() {
        let error = CallbackError::MissingClaim("email".to_string());
        assert!(error.to_string().contains("Missing required claim"));

        let error = CallbackError::Provisioning("DB error".to_string());
        assert!(error.to_string().contains("Failed to provision user"));
    }
}
