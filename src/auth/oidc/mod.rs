// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OpenID Connect (OIDC) Authentication Module
//!
//! This module provides OpenID Connect authentication support with specific
//! optimizations for Keycloak, including:
//!
//! - Discovery and client initialization
//! - Authorization code flow with PKCE
//! - Token exchange and refresh
//! - ID token verification
//! - User provisioning from OIDC claims
//! - Role mapping from provider-specific claims (e.g., Keycloak realm_access.roles)
//!
//! # Examples
//!
//! ## Basic OIDC Client Setup
//!
//! ```no_run
//! use xzepr::auth::oidc::{OidcClient, OidcConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = OidcConfig::keycloak(
//!     "https://keycloak.example.com/realms/xzepr".to_string(),
//!     "xzepr-client".to_string(),
//!     "secret-at-least-16-chars".to_string(),
//!     "https://app.example.com/api/v1/auth/oidc/callback".to_string(),
//! );
//!
//! let client = OidcClient::new(config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Authorization Flow
//!
//! ```no_run
//! # use xzepr::auth::oidc::{OidcClient, OidcConfig};
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = OidcConfig::keycloak(
//! #     "https://keycloak.example.com/realms/xzepr".to_string(),
//! #     "xzepr-client".to_string(),
//! #     "secret-at-least-16-chars".to_string(),
//! #     "https://app.example.com/callback".to_string(),
//! # );
//! # let client = OidcClient::new(config).await?;
//! // Generate authorization URL
//! let auth_request = client.authorization_url();
//! println!("Redirect user to: {}", auth_request.url);
//!
//! // After user authenticates and returns with code...
//! let result = client.exchange_code(
//!     "authorization_code".to_string(),
//!     "state_from_callback".to_string(),
//!     auth_request.state,
//!     auth_request.pkce_verifier,
//!     auth_request.nonce,
//! ).await?;
//!
//! println!("User authenticated: {}", result.claims.sub);
//! # Ok(())
//! # }
//! ```
//!
//! ## Role Mapping
//!
//! ```
//! use xzepr::auth::oidc::callback::RoleMappings;
//! use xzepr::auth::rbac::Role;
//!
//! let mut mappings = RoleMappings::new();
//! mappings.add_mapping("keycloak-admin".to_string(), Role::Admin);
//! mappings.add_mapping("keycloak-manager".to_string(), Role::EventManager);
//!
//! let roles = mappings.map_roles(&vec!["keycloak-admin".to_string()]);
//! assert_eq!(roles, vec![Role::Admin]);
//! ```

pub mod callback;
pub mod client;
pub mod config;

pub use callback::{
    CallbackError, OidcCallbackHandler, OidcCallbackQuery, OidcSession, OidcUserData, RoleMappings,
};
pub use client::{AuthorizationRequest, OidcAuthResult, OidcClaims, OidcClient, OidcError};
pub use config::OidcConfig;
