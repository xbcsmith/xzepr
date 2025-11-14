// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OIDC Client Implementation
//!
//! This module provides the OpenID Connect client implementation for authentication
//! with external identity providers, with specific support for Keycloak.

use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreIdTokenClaims, CoreProviderMetadata,
};
use openidconnect::reqwest::async_http_client;
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use super::config::OidcConfig;

/// Errors that can occur during OIDC operations
#[derive(Error, Debug)]
pub enum OidcError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Discovery error
    #[error("Failed to discover OIDC provider: {0}")]
    Discovery(String),

    /// Token exchange error
    #[error("Failed to exchange authorization code: {0}")]
    TokenExchange(String),

    /// Token verification error
    #[error("Failed to verify ID token: {0}")]
    TokenVerification(String),

    /// User info error
    #[error("Failed to fetch user info: {0}")]
    UserInfo(String),

    /// Invalid state parameter (CSRF protection)
    #[error("Invalid state parameter")]
    InvalidState,

    /// Token refresh error
    #[error("Failed to refresh token: {0}")]
    RefreshFailed(String),

    /// HTTP request error
    #[error("HTTP request failed: {0}")]
    Http(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// OIDC authentication result
#[derive(Debug, Clone)]
pub struct OidcAuthResult {
    /// Access token
    pub access_token: String,

    /// ID token
    pub id_token: String,

    /// Refresh token (optional)
    pub refresh_token: Option<String>,

    /// Token expiration in seconds
    pub expires_in: Option<u64>,

    /// ID token claims
    pub claims: OidcClaims,
}

/// OIDC claims extracted from ID token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    /// Subject (user ID from provider)
    pub sub: String,

    /// Email address
    pub email: Option<String>,

    /// Email verification status
    pub email_verified: Option<bool>,

    /// Preferred username
    pub preferred_username: Option<String>,

    /// Full name
    pub name: Option<String>,

    /// Given name
    pub given_name: Option<String>,

    /// Family name
    pub family_name: Option<String>,

    /// Roles (extracted from provider-specific claim)
    pub roles: Vec<String>,

    /// Additional claims
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

/// Authorization URL and state for OIDC flow
#[derive(Debug, Clone)]
pub struct AuthorizationRequest {
    /// Authorization URL to redirect user to
    pub url: String,

    /// CSRF state token
    pub state: String,

    /// PKCE verifier (stored for code exchange)
    pub pkce_verifier: Option<String>,

    /// Nonce (stored for ID token validation)
    pub nonce: String,
}

/// OIDC client for authentication operations
pub struct OidcClient {
    /// OpenID Connect configuration
    config: OidcConfig,

    /// Core OIDC client
    client: CoreClient,

    /// Provider metadata
    provider_metadata: CoreProviderMetadata,
}

impl OidcClient {
    /// Create a new OIDC client
    ///
    /// # Arguments
    ///
    /// * `config` - OIDC configuration
    ///
    /// # Returns
    ///
    /// Returns the OIDC client or an error if discovery fails
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Configuration is invalid
    /// - Provider discovery fails
    /// - Network errors occur
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xzepr::auth::oidc::{OidcClient, OidcConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = OidcConfig::keycloak(
    ///     "https://keycloak.example.com/realms/xzepr".to_string(),
    ///     "xzepr-client".to_string(),
    ///     "secret-at-least-16-chars".to_string(),
    ///     "https://app.example.com/api/v1/auth/oidc/callback".to_string(),
    /// );
    ///
    /// let client = OidcClient::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: OidcConfig) -> Result<Self, OidcError> {
        config
            .validate()
            .map_err(|e| OidcError::Config(e.to_string()))?;

        let issuer_url = IssuerUrl::new(config.issuer_url.clone())
            .map_err(|e| OidcError::Config(format!("Invalid issuer URL: {}", e)))?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, async_http_client)
            .await
            .map_err(|e| OidcError::Discovery(e.to_string()))?;

        let redirect_url = RedirectUrl::new(config.redirect_url.clone())
            .map_err(|e| OidcError::Config(format!("Invalid redirect URL: {}", e)))?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata.clone(),
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
        )
        .set_redirect_uri(redirect_url);

        Ok(Self {
            config,
            client,
            provider_metadata,
        })
    }

    /// Generate authorization URL for user to visit
    ///
    /// # Returns
    ///
    /// Returns authorization request with URL, state, and PKCE verifier
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xzepr::auth::oidc::{OidcClient, OidcConfig};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = OidcConfig::keycloak(
    /// #     "https://keycloak.example.com/realms/xzepr".to_string(),
    /// #     "xzepr-client".to_string(),
    /// #     "secret-at-least-16-chars".to_string(),
    /// #     "https://app.example.com/callback".to_string(),
    /// # );
    /// # let client = OidcClient::new(config).await?;
    /// let auth_request = client.authorization_url();
    /// println!("Redirect user to: {}", auth_request.url);
    /// # Ok(())
    /// # }
    /// ```
    pub fn authorization_url(&self) -> AuthorizationRequest {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_request = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .set_pkce_challenge(pkce_challenge);

        for scope in &self.config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        for (key, value) in &self.config.additional_params {
            auth_request = auth_request.add_extra_param(key, value);
        }

        let (url, csrf_token, nonce) = auth_request.url();

        AuthorizationRequest {
            url: url.to_string(),
            state: csrf_token.secret().clone(),
            pkce_verifier: Some(pkce_verifier.secret().clone()),
            nonce: nonce.secret().clone(),
        }
    }

    /// Exchange authorization code for tokens
    ///
    /// # Arguments
    ///
    /// * `code` - Authorization code from callback
    /// * `state` - State parameter from callback (for CSRF validation)
    /// * `expected_state` - Expected state value
    /// * `pkce_verifier` - PKCE verifier from authorization request
    /// * `nonce` - Nonce from authorization request
    ///
    /// # Returns
    ///
    /// Returns authentication result with tokens and claims
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - State validation fails (CSRF protection)
    /// - Code exchange fails
    /// - Token verification fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xzepr::auth::oidc::{OidcClient, OidcConfig};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = OidcConfig::keycloak(
    /// #     "https://keycloak.example.com/realms/xzepr".to_string(),
    /// #     "xzepr-client".to_string(),
    /// #     "secret-at-least-16-chars".to_string(),
    /// #     "https://app.example.com/callback".to_string(),
    /// # );
    /// # let client = OidcClient::new(config).await?;
    /// let auth_request = client.authorization_url();
    /// // User visits auth_request.url and returns with code and state
    /// let result = client.exchange_code(
    ///     "authorization_code".to_string(),
    ///     "state_from_callback".to_string(),
    ///     auth_request.state,
    ///     auth_request.pkce_verifier,
    ///     auth_request.nonce,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exchange_code(
        &self,
        code: String,
        state: String,
        expected_state: String,
        pkce_verifier: Option<String>,
        nonce: String,
    ) -> Result<OidcAuthResult, OidcError> {
        if state != expected_state {
            return Err(OidcError::InvalidState);
        }

        let mut token_request = self.client.exchange_code(AuthorizationCode::new(code));

        if let Some(verifier) = pkce_verifier {
            token_request = token_request.set_pkce_verifier(PkceCodeVerifier::new(verifier));
        }

        let token_response = token_request
            .request_async(async_http_client)
            .await
            .map_err(|e| OidcError::TokenExchange(e.to_string()))?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| OidcError::TokenVerification("No ID token in response".to_string()))?;

        let id_token_verifier = self.client.id_token_verifier();
        let claims = id_token
            .claims(&id_token_verifier, &Nonce::new(nonce))
            .map_err(|e| OidcError::TokenVerification(e.to_string()))?;

        let oidc_claims = self.extract_claims(claims)?;

        Ok(OidcAuthResult {
            access_token: token_response.access_token().secret().clone(),
            id_token: id_token.to_string(),
            refresh_token: token_response.refresh_token().map(|t| t.secret().clone()),
            expires_in: token_response.expires_in().map(|d| d.as_secs()),
            claims: oidc_claims,
        })
    }

    /// Refresh access token using refresh token
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - Refresh token from previous authentication
    ///
    /// # Returns
    ///
    /// Returns new access token and optional new refresh token
    ///
    /// # Errors
    ///
    /// Returns error if refresh fails or token is invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xzepr::auth::oidc::{OidcClient, OidcConfig};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = OidcConfig::keycloak(
    /// #     "https://keycloak.example.com/realms/xzepr".to_string(),
    /// #     "xzepr-client".to_string(),
    /// #     "secret-at-least-16-chars".to_string(),
    /// #     "https://app.example.com/callback".to_string(),
    /// # );
    /// # let client = OidcClient::new(config).await?;
    /// let refreshed = client.refresh_token("refresh_token_value".to_string()).await?;
    /// println!("New access token: {}", refreshed.access_token);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refresh_token(&self, refresh_token: String) -> Result<OidcAuthResult, OidcError> {
        let token_response = self
            .client
            .exchange_refresh_token(&RefreshToken::new(refresh_token))
            .request_async(async_http_client)
            .await
            .map_err(|e| OidcError::RefreshFailed(e.to_string()))?;

        let id_token = token_response.id_token();
        let claims = if let Some(id_token) = id_token {
            let id_token_verifier = self.client.id_token_verifier();
            let claims = id_token
                .claims(&id_token_verifier, &Nonce::new_random())
                .map_err(|e| OidcError::TokenVerification(e.to_string()))?;
            self.extract_claims(claims)?
        } else {
            OidcClaims {
                sub: String::new(),
                email: None,
                email_verified: None,
                preferred_username: None,
                name: None,
                given_name: None,
                family_name: None,
                roles: Vec::new(),
                additional: HashMap::new(),
            }
        };

        Ok(OidcAuthResult {
            access_token: token_response.access_token().secret().clone(),
            id_token: id_token.map(|t| t.to_string()).unwrap_or_default(),
            refresh_token: token_response.refresh_token().map(|t| t.secret().clone()),
            expires_in: token_response.expires_in().map(|d| d.as_secs()),
            claims,
        })
    }

    /// Verify an ID token
    ///
    /// # Arguments
    ///
    /// * `id_token` - ID token string to verify
    /// * `nonce` - Expected nonce value
    ///
    /// # Returns
    ///
    /// Returns verified claims
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub async fn verify_id_token(
        &self,
        _id_token: &str,
        _nonce: &str,
    ) -> Result<OidcClaims, OidcError> {
        // Parse ID token string - for now, we'll skip direct verification
        // In production, you'd want to properly parse and verify the token
        Err(OidcError::TokenVerification(
            "Direct ID token verification not supported - use exchange_code instead".to_string(),
        ))
    }

    /// Extract claims from CoreIdTokenClaims
    fn extract_claims(&self, claims: &CoreIdTokenClaims) -> Result<OidcClaims, OidcError> {
        let additional_claims = claims.additional_claims();
        let mut additional_map = HashMap::new();
        let mut roles = Vec::new();

        // Extract additional claims if present
        let json_value = serde_json::to_value(additional_claims)
            .map_err(|e| OidcError::Serialization(e.to_string()))?;

        if let serde_json::Value::Object(map) = json_value {
            if !map.is_empty() {
                for (key, value) in map {
                    additional_map.insert(key.clone(), value.clone());
                }

                if self.config.keycloak_mode {
                    if let Some(realm_access) = additional_map.get("realm_access") {
                        if let Some(realm_roles) = realm_access.get("roles") {
                            if let Some(roles_array) = realm_roles.as_array() {
                                for role in roles_array {
                                    if let Some(role_str) = role.as_str() {
                                        roles.push(role_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }

                if roles.is_empty() {
                    let role_parts: Vec<&str> = self.config.role_claim_path.split('.').collect();
                    let mut current = &additional_map;

                    for (i, part) in role_parts.iter().enumerate() {
                        if i == role_parts.len() - 1 {
                            if let Some(roles_value) = current.get(*part) {
                                if let Some(roles_array) = roles_value.as_array() {
                                    for role in roles_array {
                                        if let Some(role_str) = role.as_str() {
                                            roles.push(role_str.to_string());
                                        }
                                    }
                                }
                            }
                        } else if let Some(obj) = current.get(*part) {
                            if let Some(map) = obj.as_object() {
                                let map_hashmap: HashMap<String, serde_json::Value> =
                                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                                current = Box::leak(Box::new(map_hashmap));
                            }
                        }
                    }
                }
            }
        }

        Ok(OidcClaims {
            sub: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
            email_verified: claims.email_verified(),
            preferred_username: claims.preferred_username().map(|u| u.to_string()),
            name: claims
                .name()
                .and_then(|n| n.get(None).map(|localized| localized.to_string())),
            given_name: claims
                .given_name()
                .and_then(|n| n.get(None).map(|localized| localized.to_string())),
            family_name: claims
                .family_name()
                .and_then(|n| n.get(None).map(|localized| localized.to_string())),
            roles,
            additional: additional_map,
        })
    }

    /// Get provider metadata
    pub fn provider_metadata(&self) -> &CoreProviderMetadata {
        &self.provider_metadata
    }

    /// Get configuration
    pub fn config(&self) -> &OidcConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oidc_claims_serialization() {
        let claims = OidcClaims {
            sub: "user123".to_string(),
            email: Some("user@example.com".to_string()),
            email_verified: Some(true),
            preferred_username: Some("user123".to_string()),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            roles: vec!["admin".to_string(), "user".to_string()],
            additional: HashMap::new(),
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("user123"));
        assert!(json.contains("user@example.com"));
        assert!(json.contains("admin"));
    }

    #[test]
    fn test_oidc_claims_deserialization() {
        let json = r#"{
            "sub": "user123",
            "email": "user@example.com",
            "email_verified": true,
            "preferred_username": "user123",
            "name": "Test User",
            "given_name": "Test",
            "family_name": "User",
            "roles": ["admin", "user"]
        }"#;

        let claims: OidcClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.roles.len(), 2);
    }

    #[test]
    fn test_authorization_request_fields() {
        let auth_req = AuthorizationRequest {
            url: "https://example.com/auth".to_string(),
            state: "random_state".to_string(),
            pkce_verifier: Some("verifier".to_string()),
            nonce: "nonce".to_string(),
        };

        assert!(auth_req.url.contains("example.com"));
        assert_eq!(auth_req.state, "random_state");
        assert!(auth_req.pkce_verifier.is_some());
    }

    #[test]
    fn test_oidc_auth_result_fields() {
        let result = OidcAuthResult {
            access_token: "access".to_string(),
            id_token: "id".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_in: Some(3600),
            claims: OidcClaims {
                sub: "user123".to_string(),
                email: None,
                email_verified: None,
                preferred_username: None,
                name: None,
                given_name: None,
                family_name: None,
                roles: Vec::new(),
                additional: HashMap::new(),
            },
        };

        assert_eq!(result.access_token, "access");
        assert_eq!(result.expires_in, Some(3600));
        assert!(result.refresh_token.is_some());
    }

    #[test]
    fn test_oidc_error_display() {
        let error = OidcError::Config("test error".to_string());
        assert!(error.to_string().contains("Configuration error"));

        let error = OidcError::InvalidState;
        assert!(error.to_string().contains("Invalid state"));

        let error = OidcError::TokenExchange("failed".to_string());
        assert!(error.to_string().contains("exchange authorization code"));
    }
}
