// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OIDC Configuration
//!
//! This module defines the configuration structures for OpenID Connect (OIDC)
//! authentication, with specific support for Keycloak providers.

use serde::{Deserialize, Serialize};

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

    /// Validate the configuration
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the configuration is valid, otherwise returns an error message.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Issuer URL is empty or invalid
    /// - Client ID is empty
    /// - Client secret is empty
    /// - Redirect URL is empty or invalid
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if self.issuer_url.is_empty() {
            return Err("Issuer URL cannot be empty".to_string());
        }

        if !self.issuer_url.starts_with("https://") && !self.issuer_url.starts_with("http://") {
            return Err("Issuer URL must be a valid HTTP(S) URL".to_string());
        }

        if self.client_id.is_empty() {
            return Err("Client ID cannot be empty".to_string());
        }

        if self.client_secret.is_empty() {
            return Err("Client secret cannot be empty".to_string());
        }

        if self.client_secret.len() < 16 {
            return Err("Client secret must be at least 16 characters".to_string());
        }

        if self.redirect_url.is_empty() {
            return Err("Redirect URL cannot be empty".to_string());
        }

        if !self.redirect_url.starts_with("https://") && !self.redirect_url.starts_with("http://") {
            return Err("Redirect URL must be a valid HTTP(S) URL".to_string());
        }

        if self.scopes.is_empty() {
            return Err("At least one scope must be configured".to_string());
        }

        if !self.scopes.contains(&"openid".to_string()) {
            return Err("'openid' scope is required for OIDC".to_string());
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
        assert!(result.unwrap_err().contains("Issuer URL cannot be empty"));
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
        assert!(result.unwrap_err().contains("valid HTTP(S) URL"));
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
        assert!(result.unwrap_err().contains("Client ID cannot be empty"));
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
        assert!(result
            .unwrap_err()
            .contains("Client secret cannot be empty"));
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
        assert!(result.unwrap_err().contains("at least 16 characters"));
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
        assert!(result.unwrap_err().contains("Redirect URL cannot be empty"));
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
        assert!(result.unwrap_err().contains("'openid' scope is required"));
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
        assert!(result
            .unwrap_err()
            .contains("At least one scope must be configured"));
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
