// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OIDC Configuration
//!
//! This module defines the configuration structures for OpenID Connect (OIDC)
//! authentication, with specific support for Keycloak providers.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during OIDC configuration validation.
#[derive(Error, Debug, PartialEq)]
pub enum OidcConfigError {
    /// The OIDC issuer URL is empty.
    #[error("Issuer URL cannot be empty")]
    EmptyIssuerUrl,
    /// The OIDC issuer URL does not start with http:// or https://.
    #[error("Issuer URL must be a valid HTTP(S) URL")]
    InvalidIssuerUrl,
    /// The OAuth2 client ID is empty.
    #[error("Client ID cannot be empty")]
    EmptyClientId,
    /// The OAuth2 client secret is empty.
    #[error("Client secret cannot be empty")]
    EmptyClientSecret,
    /// The OAuth2 client secret is too short (minimum 16 characters).
    #[error("Client secret must be at least 16 characters")]
    ClientSecretTooShort,
    /// The redirect URL is empty.
    #[error("Redirect URL cannot be empty")]
    EmptyRedirectUrl,
    /// The redirect URL does not start with http:// or https://.
    #[error("Redirect URL must be a valid HTTP(S) URL")]
    InvalidRedirectUrl,
    /// No scopes are configured.
    #[error("At least one scope must be configured")]
    EmptyScopes,
    /// The required 'openid' scope is missing.
    #[error("'openid' scope is required for OIDC")]
    MissingOpenIdScope,
    /// The OIDC issuer URL must use HTTPS in production.
    #[error("OIDC issuer URL must use HTTPS in production")]
    HttpIssuerInProduction,
    /// The OIDC redirect URL must use HTTPS in production.
    #[error("OIDC redirect URL must use HTTPS in production")]
    HttpRedirectInProduction,
    /// The redirect host is not in the allowed list.
    #[error("Redirect host '{host}' is not in the allowed redirect hosts list")]
    DisallowedRedirectHost { host: String },
}

/// OIDC client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// Whether OIDC authentication is enabled
    #[serde(default)]
    pub enabled: bool,

    /// OIDC issuer URL (e.g., https://keycloak.example.com/realms/xzepr)
    pub issuer_url: String,

    /// OAuth2 client ID
    pub client_id: String,

    /// OAuth2 client secret
    pub client_secret: String,

    /// Redirect URL for OIDC callback (e.g., https://app.example.com/api/v1/auth/oidc/callback)
    pub redirect_url: String,

    /// Scopes to request (default: openid, profile, email)
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,

    /// Claim name containing user roles (default: roles)
    /// For Keycloak, this is typically found in realm_access.roles
    #[serde(default = "default_role_claim")]
    pub role_claim_path: String,

    /// Whether to use Keycloak-specific features
    #[serde(default = "default_true")]
    pub keycloak_mode: bool,

    /// Additional parameters to include in authorization request
    #[serde(default)]
    pub additional_params: std::collections::HashMap<String, String>,

    /// Allowed redirect URL hostname allowlist.
    ///
    /// When non-empty, the redirect URL's host must appear in this list.
    /// An empty list disables host allowlist checking (not recommended in production).
    #[serde(default)]
    pub allowed_redirect_hosts: Vec<String>,

    /// Session TTL in seconds (default: 3600 = 1 hour).
    ///
    /// Expired sessions should be cleaned up by a background task.
    /// For multi-instance deployments, use a distributed store (e.g., Redis).
    #[serde(default = "default_oidc_session_ttl")]
    pub session_ttl_seconds: u64,
}

impl OidcConfig {
    /// Create a new OIDC configuration
    ///
    /// # Arguments
    ///
    /// * `issuer_url` - The OIDC issuer URL (Keycloak realm URL)
    /// * `client_id` - OAuth2 client ID
    /// * `client_secret` - OAuth2 client secret
    /// * `redirect_url` - Callback URL for OIDC flow
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::auth::oidc::config::OidcConfig;
    ///
    /// let config = OidcConfig::new(
    ///     "https://keycloak.example.com/realms/xzepr".to_string(),
    ///     "xzepr-client".to_string(),
    ///     "secret".to_string(),
    ///     "https://app.example.com/api/v1/auth/oidc/callback".to_string(),
    /// );
    /// ```
    pub fn new(
        issuer_url: String,
        client_id: String,
        client_secret: String,
        redirect_url: String,
    ) -> Self {
        Self {
            enabled: true,
            issuer_url,
            client_id,
            client_secret,
            redirect_url,
            scopes: default_scopes(),
            role_claim_path: default_role_claim(),
            keycloak_mode: true,
            additional_params: std::collections::HashMap::new(),
            allowed_redirect_hosts: vec![],
            session_ttl_seconds: default_oidc_session_ttl(),
        }
    }

    /// Create a configuration for Keycloak
    ///
    /// # Arguments
    ///
    /// * `realm_url` - Keycloak realm URL (e.g., https://keycloak.example.com/realms/xzepr)
    /// * `client_id` - OAuth2 client ID
    /// * `client_secret` - OAuth2 client secret
    /// * `redirect_url` - Callback URL for OIDC flow
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::auth::oidc::config::OidcConfig;
    ///
    /// let config = OidcConfig::keycloak(
    ///     "https://keycloak.example.com/realms/xzepr".to_string(),
    ///     "xzepr-client".to_string(),
    ///     "secret".to_string(),
    ///     "https://app.example.com/api/v1/auth/oidc/callback".to_string(),
    /// );
    /// ```
    pub fn keycloak(
        realm_url: String,
        client_id: String,
        client_secret: String,
        redirect_url: String,
    ) -> Self {
        Self {
            enabled: true,
            issuer_url: realm_url,
            client_id,
            client_secret,
            redirect_url,
            scopes: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "roles".to_string(), // Keycloak-specific scope
            ],
            role_claim_path: "realm_access.roles".to_string(),
            keycloak_mode: true,
            additional_params: std::collections::HashMap::new(),
            allowed_redirect_hosts: vec![],
            session_ttl_seconds: default_oidc_session_ttl(),
        }
    }

    /// Validate the configuration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the configuration is valid.
    ///
    /// # Errors
    ///
    /// Returns `OidcConfigError::EmptyIssuerUrl` if issuer URL is empty.
    /// Returns `OidcConfigError::InvalidIssuerUrl` if issuer URL is not HTTP(S).
    /// Returns `OidcConfigError::EmptyClientId` if client ID is empty.
    /// Returns `OidcConfigError::EmptyClientSecret` if client secret is empty.
    /// Returns `OidcConfigError::ClientSecretTooShort` if secret is under 16 characters.
    /// Returns `OidcConfigError::EmptyRedirectUrl` if redirect URL is empty.
    /// Returns `OidcConfigError::InvalidRedirectUrl` if redirect URL is not HTTP(S).
    /// Returns `OidcConfigError::EmptyScopes` if no scopes are configured.
    /// Returns `OidcConfigError::MissingOpenIdScope` if 'openid' scope is absent.
    /// Returns `OidcConfigError::DisallowedRedirectHost` if redirect host is not allowlisted.
    pub fn validate(&self) -> Result<(), OidcConfigError> {
        if !self.enabled {
            return Ok(());
        }

        if self.issuer_url.is_empty() {
            return Err(OidcConfigError::EmptyIssuerUrl);
        }

        if !self.issuer_url.starts_with("https://") && !self.issuer_url.starts_with("http://") {
            return Err(OidcConfigError::InvalidIssuerUrl);
        }

        if self.client_id.is_empty() {
            return Err(OidcConfigError::EmptyClientId);
        }

        if self.client_secret.is_empty() {
            return Err(OidcConfigError::EmptyClientSecret);
        }

        if self.client_secret.len() < 16 {
            return Err(OidcConfigError::ClientSecretTooShort);
        }

        if self.redirect_url.is_empty() {
            return Err(OidcConfigError::EmptyRedirectUrl);
        }

        if !self.redirect_url.starts_with("https://") && !self.redirect_url.starts_with("http://") {
            return Err(OidcConfigError::InvalidRedirectUrl);
        }

        if self.scopes.is_empty() {
            return Err(OidcConfigError::EmptyScopes);
        }

        if !self.scopes.contains(&"openid".to_string()) {
            return Err(OidcConfigError::MissingOpenIdScope);
        }

        // Validate redirect URL against allowlist if configured.
        if !self.allowed_redirect_hosts.is_empty() {
            let host = extract_host(&self.redirect_url);
            if !self.allowed_redirect_hosts.iter().any(|h| h == &host) {
                return Err(OidcConfigError::DisallowedRedirectHost { host });
            }
        }

        Ok(())
    }

    /// Validate the configuration for production use.
    ///
    /// In addition to all checks in `validate()`, this method enforces:
    /// - OIDC issuer URL must use `https://`
    /// - Redirect URL must use `https://`
    /// - Redirect URL host must appear in `allowed_redirect_hosts` when non-empty
    ///
    /// # Errors
    ///
    /// Returns `OidcConfigError::HttpIssuerInProduction` if the issuer URL is not HTTPS.
    /// Returns `OidcConfigError::HttpRedirectInProduction` if the redirect URL is not HTTPS.
    /// Returns `OidcConfigError::DisallowedRedirectHost` if the redirect host is not allowlisted.
    pub fn validate_production(&self) -> Result<(), OidcConfigError> {
        // Run base validation first.
        self.validate()?;

        if !self.enabled {
            return Ok(());
        }

        if !self.issuer_url.starts_with("https://") {
            return Err(OidcConfigError::HttpIssuerInProduction);
        }

        if !self.redirect_url.starts_with("https://") {
            return Err(OidcConfigError::HttpRedirectInProduction);
        }

        if !self.allowed_redirect_hosts.is_empty() {
            let host = extract_host(&self.redirect_url);
            if !self.allowed_redirect_hosts.iter().any(|h| h == &host) {
                return Err(OidcConfigError::DisallowedRedirectHost { host });
            }
        }

        Ok(())
    }

    /// Get the discovery URL for the OIDC provider
    ///
    /// # Returns
    ///
    /// Returns the well-known OpenID configuration URL
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::auth::oidc::config::OidcConfig;
    ///
    /// let config = OidcConfig::keycloak(
    ///     "https://keycloak.example.com/realms/xzepr".to_string(),
    ///     "client".to_string(),
    ///     "secret-at-least-16-chars".to_string(),
    ///     "https://app.example.com/callback".to_string(),
    /// );
    ///
    /// let discovery_url = config.discovery_url();
    /// assert!(discovery_url.contains("/.well-known/openid-configuration"));
    /// ```
    pub fn discovery_url(&self) -> String {
        format!("{}/.well-known/openid-configuration", self.issuer_url)
    }
}

impl Default for OidcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            issuer_url: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            redirect_url: String::new(),
            scopes: default_scopes(),
            role_claim_path: default_role_claim(),
            keycloak_mode: true,
            additional_params: std::collections::HashMap::new(),
            allowed_redirect_hosts: vec![],
            session_ttl_seconds: default_oidc_session_ttl(),
        }
    }
}

/// Default scopes for OIDC authentication
fn default_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "profile".to_string(),
        "email".to_string(),
    ]
}

/// Default role claim path
fn default_role_claim() -> String {
    "roles".to_string()
}

/// Default true value for serde
fn default_true() -> bool {
    true
}

/// Default OIDC session TTL in seconds (1 hour).
fn default_oidc_session_ttl() -> u64 {
    3600
}

/// Extract the host (and optional port) portion from a URL string.
///
/// Strips the scheme prefix (`https://` or `http://`) and returns everything
/// up to the first path separator `/`.
///
/// # Examples
///
/// ```
/// // Internal helper - tested via validate() and validate_production()
/// ```
fn extract_host(url: &str) -> String {
    let after_scheme = url.find("://").map(|i| &url[i + 3..]).unwrap_or(url);
    after_scheme
        .split_once('/')
        .map(|(host, _)| host.to_string())
        .unwrap_or_else(|| after_scheme.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_config() {
        let config = OidcConfig::new(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );

        assert!(config.enabled);
        assert_eq!(
            config.issuer_url,
            "https://keycloak.example.com/realms/xzepr"
        );
        assert_eq!(config.client_id, "xzepr-client");
        assert!(config.keycloak_mode);
    }

    #[test]
    fn test_keycloak_config() {
        let config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );

        assert!(config.enabled);
        assert!(config.keycloak_mode);
        assert_eq!(config.role_claim_path, "realm_access.roles");
        assert!(config.scopes.contains(&"roles".to_string()));
    }

    #[test]
    fn test_default_config() {
        let config = OidcConfig::default();
        assert!(!config.enabled);
        assert!(config.issuer_url.is_empty());
        assert_eq!(config.scopes, default_scopes());
    }

    #[test]
    fn test_validate_success() {
        let config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_disabled_config() {
        let config = OidcConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_issuer() {
        let mut config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.issuer_url = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::EmptyIssuerUrl
        ));
    }

    #[test]
    fn test_validate_invalid_issuer_url() {
        let mut config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.issuer_url = "not-a-url".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::InvalidIssuerUrl
        ));
    }

    #[test]
    fn test_validate_empty_client_id() {
        let mut config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.client_id = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::EmptyClientId
        ));
    }

    #[test]
    fn test_validate_empty_client_secret() {
        let mut config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.client_secret = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::EmptyClientSecret
        ));
    }

    #[test]
    fn test_validate_short_client_secret() {
        let mut config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.client_secret = "short".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::ClientSecretTooShort
        ));
    }

    #[test]
    fn test_validate_empty_redirect_url() {
        let mut config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.redirect_url = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::EmptyRedirectUrl
        ));
    }

    #[test]
    fn test_validate_missing_openid_scope() {
        let mut config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.scopes = vec!["profile".to_string(), "email".to_string()];
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::MissingOpenIdScope
        ));
    }

    #[test]
    fn test_validate_empty_scopes() {
        let mut config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.scopes = vec![];
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OidcConfigError::EmptyScopes));
    }

    #[test]
    fn test_oidc_config_error_display() {
        assert!(OidcConfigError::EmptyIssuerUrl
            .to_string()
            .contains("Issuer URL"));
        assert!(OidcConfigError::ClientSecretTooShort
            .to_string()
            .contains("16"));
        assert!(OidcConfigError::MissingOpenIdScope
            .to_string()
            .contains("openid"));
    }

    #[test]
    fn test_discovery_url() {
        let config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );

        let discovery_url = config.discovery_url();
        assert_eq!(
            discovery_url,
            "https://keycloak.example.com/realms/xzepr/.well-known/openid-configuration"
        );
    }

    #[test]
    fn test_serde_serialization() {
        let config = OidcConfig::keycloak(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("keycloak.example.com"));
        assert!(json.contains("xzepr-client"));
    }

    #[test]
    fn test_serde_deserialization() {
        let json = r#"{
            "enabled": true,
            "issuer_url": "https://keycloak.example.com/realms/xzepr",
            "client_id": "xzepr-client",
            "client_secret": "secret-at-least-16-chars",
            "redirect_url": "https://app.example.com/callback",
            "scopes": ["openid", "profile", "email"],
            "role_claim_path": "realm_access.roles",
            "keycloak_mode": true,
            "additional_params": {}
        }"#;

        let config: OidcConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.client_id, "xzepr-client");
        assert!(config.keycloak_mode);
        assert_eq!(config.role_claim_path, "realm_access.roles");
    }

    #[test]
    fn test_validate_production_rejects_http_issuer() {
        let config = OidcConfig::new(
            "http://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        let result = config.validate_production();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::HttpIssuerInProduction
        ));
    }

    #[test]
    fn test_validate_production_rejects_http_redirect() {
        let config = OidcConfig::new(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "http://app.example.com/callback".to_string(),
        );
        let result = config.validate_production();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::HttpRedirectInProduction
        ));
    }

    #[test]
    fn test_validate_production_accepts_https_config() {
        let config = OidcConfig::new(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        assert!(config.validate_production().is_ok());
    }

    #[test]
    fn test_validate_redirect_host_allowlist() {
        let mut config = OidcConfig::new(
            "https://keycloak.example.com/realms/xzepr".to_string(),
            "xzepr-client".to_string(),
            "secret-at-least-16-chars".to_string(),
            "https://app.example.com/callback".to_string(),
        );
        config.allowed_redirect_hosts = vec!["other.example.com".to_string()];
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OidcConfigError::DisallowedRedirectHost { host } if host == "app.example.com"
        ));
    }

    #[test]
    fn test_oidc_session_ttl_default() {
        let config = OidcConfig::default();
        assert_eq!(config.session_ttl_seconds, 3600);
    }
}
