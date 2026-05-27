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

// ---------------------------------------------------------------------------
// RedirectValidationError
// ---------------------------------------------------------------------------

/// Errors returned when validating a `redirect_to` parameter.
///
/// Validation enforces the following rules:
/// - `None` values are always accepted.
/// - Relative paths (starting with `/`) are always accepted.
/// - Absolute HTTPS URLs are accepted only when the host appears in the
///   caller-supplied allowlist.
/// - HTTP URLs and unrecognised formats are rejected unconditionally.
#[derive(Error, Debug, PartialEq)]
pub enum RedirectValidationError {
    /// The `redirect_to` value is `Some` but contains an empty string.
    #[error("redirect_to must not be empty")]
    EmptyValue,

    /// The URL uses the `http://` scheme, which is not allowed.
    #[error("redirect_to must use https, not http")]
    HttpNotAllowed,

    /// The URL host is not in the configured allowlist.
    #[error("redirect host '{host}' is not in the allowed list")]
    HostNotAllowed {
        /// The host that was extracted from the URL.
        host: String,
    },

    /// The value is not `None`, not a relative path, and not a recognised
    /// absolute URL scheme.
    #[error("redirect_to format is not recognized")]
    InvalidFormat,
}

// ---------------------------------------------------------------------------
// validate_redirect_to
// ---------------------------------------------------------------------------

/// Validate a `redirect_to` parameter before storing it in an OIDC session.
///
/// Accepted values:
/// - `None`: always valid (no post-login redirect).
/// - `Some(path)` starting with `/`: always valid (server-relative path).
/// - `Some(url)` starting with `https://`: valid when the hostname is
///   contained in `allowed_hosts`.
///
/// # Arguments
///
/// * `redirect_to` - The candidate redirect destination.
/// * `allowed_hosts` - Allowlist of hostnames (e.g. `["app.example.com"]`).
///   Only consulted for absolute HTTPS URLs.
///
/// # Returns
///
/// `Ok(())` when the value is acceptable.
///
/// # Errors
///
/// Returns [`RedirectValidationError::EmptyValue`] if the string is non-`None`
/// but empty.
/// Returns [`RedirectValidationError::HttpNotAllowed`] if the URL uses
/// `http://`.
/// Returns [`RedirectValidationError::HostNotAllowed`] if the host is not in
/// `allowed_hosts`.
/// Returns [`RedirectValidationError::InvalidFormat`] if the URL format is not
/// recognised.
///
/// # Examples
///
/// ```
/// use xzepr::auth::oidc::callback::validate_redirect_to;
///
/// // None is always valid.
/// assert!(validate_redirect_to(None, &[]).is_ok());
///
/// // Relative paths are always valid.
/// assert!(validate_redirect_to(Some("/dashboard"), &[]).is_ok());
///
/// // Absolute HTTPS URL with host in the allowlist.
/// let hosts = vec!["app.example.com".to_string()];
/// assert!(validate_redirect_to(Some("https://app.example.com/home"), &hosts).is_ok());
///
/// // HTTP is never allowed.
/// assert!(validate_redirect_to(Some("http://app.example.com/"), &[]).is_err());
/// ```
pub fn validate_redirect_to(
    redirect_to: Option<&str>,
    allowed_hosts: &[String],
) -> Result<(), RedirectValidationError> {
    let value = match redirect_to {
        None => return Ok(()),
        Some(v) => v,
    };

    if value.is_empty() {
        return Err(RedirectValidationError::EmptyValue);
    }

    // Relative paths are safe: they stay on the same origin.
    if value.starts_with('/') {
        return Ok(());
    }

    // Reject plain HTTP.
    if value.starts_with("http://") {
        return Err(RedirectValidationError::HttpNotAllowed);
    }

    // Accept HTTPS only when the host is in the allowlist.
    if value.starts_with("https://") {
        let host = extract_https_host(value).ok_or(RedirectValidationError::InvalidFormat)?;
        if allowed_hosts.iter().any(|h| h == host) {
            return Ok(());
        }
        return Err(RedirectValidationError::HostNotAllowed {
            host: host.to_string(),
        });
    }

    Err(RedirectValidationError::InvalidFormat)
}

/// Extract the hostname from a URL that starts with `https://`.
///
/// Returns `None` if no host portion can be found (e.g. `https://` alone).
fn extract_https_host(url: &str) -> Option<&str> {
    // Strip the scheme prefix; we already know it starts with "https://".
    let rest = &url["https://".len()..];
    // The host ends at the first path separator, query, or fragment character.
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..end];
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

// ---------------------------------------------------------------------------
// CallbackError
// ---------------------------------------------------------------------------

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

    /// Optional stable principal hint used for pending-session concurrency limits.
    ///
    /// This value is captured before the provider callback, so it is only a
    /// pre-authentication hint. After callback, application token/session
    /// limits must use the provisioned application user ID.
    #[serde(default)]
    pub session_principal: Option<String>,
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

    // -----------------------------------------------------------------------
    // validate_redirect_to
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_redirect_to_none_is_valid() {
        let result = validate_redirect_to(None, &[]);
        assert!(result.is_ok(), "None should always be valid");
    }

    #[test]
    fn test_validate_redirect_to_relative_path_is_valid() {
        let result = validate_redirect_to(Some("/dashboard"), &[]);
        assert!(result.is_ok(), "relative path should be valid");
    }

    #[test]
    fn test_validate_redirect_to_relative_path_with_empty_allowlist_is_valid() {
        // Relative paths must be accepted regardless of the allowlist contents.
        let result = validate_redirect_to(Some("/some/deep/path?q=1#section"), &[]);
        assert!(
            result.is_ok(),
            "relative path with query and fragment should be valid"
        );
    }

    #[test]
    fn test_validate_redirect_to_https_with_allowed_host_is_valid() {
        let hosts = vec!["app.example.com".to_string()];
        let result = validate_redirect_to(Some("https://app.example.com/home"), &hosts);
        assert!(
            result.is_ok(),
            "HTTPS URL with allowed host should be valid"
        );
    }

    #[test]
    fn test_validate_redirect_to_https_host_not_in_allowlist_is_rejected() {
        let hosts = vec!["trusted.example.com".to_string()];
        let result = validate_redirect_to(Some("https://evil.example.com/steal"), &hosts);
        assert_eq!(
            result,
            Err(RedirectValidationError::HostNotAllowed {
                host: "evil.example.com".to_string()
            })
        );
    }

    #[test]
    fn test_validate_redirect_to_http_is_rejected() {
        let result = validate_redirect_to(Some("http://example.com/page"), &[]);
        assert_eq!(result, Err(RedirectValidationError::HttpNotAllowed));
    }

    #[test]
    fn test_validate_redirect_to_empty_string_is_rejected() {
        let result = validate_redirect_to(Some(""), &[]);
        assert_eq!(result, Err(RedirectValidationError::EmptyValue));
    }

    #[test]
    fn test_validate_redirect_to_invalid_format_is_rejected() {
        // A bare hostname without a scheme is not accepted.
        let result = validate_redirect_to(Some("example.com/path"), &[]);
        assert_eq!(result, Err(RedirectValidationError::InvalidFormat));
    }

    #[test]
    fn test_validate_redirect_to_ftp_scheme_is_rejected() {
        let result = validate_redirect_to(Some("ftp://files.example.com/"), &[]);
        assert_eq!(result, Err(RedirectValidationError::InvalidFormat));
    }

    #[test]
    fn test_validate_redirect_to_https_with_no_host_is_invalid_format() {
        // "https://" with no host should be an invalid format.
        let hosts = vec!["app.example.com".to_string()];
        let result = validate_redirect_to(Some("https://"), &hosts);
        assert_eq!(result, Err(RedirectValidationError::InvalidFormat));
    }

    #[test]
    fn test_validate_redirect_to_https_with_path_and_query_is_valid() {
        let hosts = vec!["app.example.com".to_string()];
        let result = validate_redirect_to(
            Some("https://app.example.com/path?key=value#anchor"),
            &hosts,
        );
        assert!(
            result.is_ok(),
            "full HTTPS URL with path/query/fragment should be valid"
        );
    }

    // -----------------------------------------------------------------------
    // RoleMappings
    // -----------------------------------------------------------------------

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
            session_principal: Some("alice@example.com".to_string()),
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
