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
}
