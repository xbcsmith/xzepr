// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! JWT Service
//!
//! This module provides the main service for JWT token operations including
//! generation, validation, and refresh token flow.

use chrono::Utc;
use jsonwebtoken::{decode, encode, Header, Validation};
use tracing::{debug, instrument, warn};

use super::blacklist::TokenBlacklist;
use super::claims::{Claims, TokenType};
use super::config::JwtConfig;
use super::error::{JwtError, JwtResult};
use super::keys::KeyManager;

/// JWT service for token operations
#[derive(Clone)]
pub struct JwtService {
    /// Configuration
    config: JwtConfig,
    /// Key manager for signing and verification
    key_manager: KeyManager,
    /// Token blacklist for revocation
    blacklist: TokenBlacklist,
}

/// Token pair response (access + refresh tokens)
#[derive(Debug, Clone)]
pub struct TokenPair {
    /// Short-lived access token
    pub access_token: String,
    /// Long-lived refresh token
    pub refresh_token: String,
    /// Access token expiration time (seconds from now)
    pub expires_in: i64,
}

impl JwtService {
    /// Create a new JWT service
    ///
    /// # Arguments
    ///
    /// * `config` - JWT configuration
    /// * `key_manager` - Key manager for signing/verification
    /// * `blacklist` - Token blacklist for revocation
    pub fn new(config: JwtConfig, key_manager: KeyManager, blacklist: TokenBlacklist) -> Self {
        Self {
            config,
            key_manager,
            blacklist,
        }
    }

    /// Create a JWT service from configuration
    ///
    /// # Arguments
    ///
    /// * `config` - JWT configuration
    ///
    /// # Returns
    ///
    /// A configured JWT service ready for use
    pub fn from_config(config: JwtConfig) -> JwtResult<Self> {
        config.validate().map_err(JwtError::ConfigError)?;
        let key_manager = KeyManager::from_config(&config)?;
        let blacklist = TokenBlacklist::new();
        Ok(Self::new(config, key_manager, blacklist))
    }

    /// Generate a new token pair (access + refresh)
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID
    /// * `roles` - User roles
    /// * `permissions` - User permissions
    ///
    /// # Returns
    ///
    /// A TokenPair containing both tokens
    #[instrument(skip(self, roles, permissions))]
    pub fn generate_token_pair(
        &self,
        user_id: String,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> JwtResult<TokenPair> {
        let access_token =
            self.generate_access_token(user_id.clone(), roles.clone(), permissions.clone())?;
        let refresh_token = self.generate_refresh_token(user_id)?;

        debug!("Generated token pair");

        Ok(TokenPair {
            access_token,
            refresh_token,
            expires_in: self.config.access_token_expiration_seconds,
        })
    }

    /// Generate an access token
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID
    /// * `roles` - User roles
    /// * `permissions` - User permissions
    ///
    /// # Returns
    ///
    /// An encoded JWT access token
    #[instrument(skip(self, roles, permissions))]
    pub fn generate_access_token(
        &self,
        user_id: String,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> JwtResult<String> {
        let claims = Claims::new_access_token(
            user_id,
            roles,
            permissions,
            self.config.issuer.clone(),
            self.config.audience.clone(),
            self.config.access_token_expiration(),
        );

        let header = Header::new(self.key_manager.current().algorithm());
        let token = encode(&header, &claims, self.key_manager.current().encoding_key())
            .map_err(|e| JwtError::EncodingError(e.to_string()))?;

        debug!(jti = %claims.jti, "Generated access token");
        Ok(token)
    }

    /// Generate a refresh token
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID
    ///
    /// # Returns
    ///
    /// An encoded JWT refresh token
    #[instrument(skip(self))]
    pub fn generate_refresh_token(&self, user_id: String) -> JwtResult<String> {
        let claims = Claims::new_refresh_token(
            user_id,
            self.config.issuer.clone(),
            self.config.audience.clone(),
            self.config.refresh_token_expiration(),
        );

        let header = Header::new(self.key_manager.current().algorithm());
        let token = encode(&header, &claims, self.key_manager.current().encoding_key())
            .map_err(|e| JwtError::EncodingError(e.to_string()))?;

        debug!(jti = %claims.jti, "Generated refresh token");
        Ok(token)
    }

    /// Validate and decode a token
    ///
    /// # Arguments
    ///
    /// * `token` - The JWT token to validate
    ///
    /// # Returns
    ///
    /// The decoded claims if valid
    #[instrument(skip(self, token))]
    pub async fn validate_token(&self, token: &str) -> JwtResult<Claims> {
        let mut validation = Validation::new(self.key_manager.current().algorithm());
        validation.leeway = self.config.leeway_seconds;
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        // Try current key first
        let claims = match decode::<Claims>(
            token,
            self.key_manager.current().decoding_key(),
            &validation,
        ) {
            Ok(token_data) => token_data.claims,
            Err(e) => {
                // If current key fails, try previous key (for rotation grace period)
                if let Some(previous) = self.key_manager.verification_keys().get(1) {
                    debug!("Trying previous key for token validation");
                    let validation_prev = Validation::new(previous.algorithm());
                    decode::<Claims>(token, previous.decoding_key(), &validation_prev)
                        .map(|token_data| token_data.claims)
                        .map_err(|_| JwtError::from(e))?
                } else {
                    return Err(JwtError::from(e));
                }
            }
        };

        // Additional validation
        claims.validate(&self.config.issuer, &self.config.audience)?;

        // Check if token is blacklisted
        self.blacklist.is_revoked(&claims.jti).await?;

        debug!(jti = %claims.jti, "Token validated successfully");
        Ok(claims)
    }

    /// Refresh an access token using a refresh token
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - The refresh token
    /// * `roles` - User roles (fetched from database)
    /// * `permissions` - User permissions (fetched from database)
    ///
    /// # Returns
    ///
    /// A new TokenPair with rotated tokens
    #[instrument(skip(self, refresh_token, roles, permissions))]
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> JwtResult<TokenPair> {
        // Validate the refresh token
        let claims = self.validate_token(refresh_token).await?;

        // Ensure it's actually a refresh token
        if claims.token_type != TokenType::Refresh {
            warn!("Attempted to use access token for refresh");
            return Err(JwtError::InvalidClaim("Not a refresh token".to_string()));
        }

        // Generate new token pair
        let new_pair = self.generate_token_pair(claims.sub.clone(), roles, permissions)?;

        // If rotation is enabled, revoke the old refresh token
        if self.config.enable_token_rotation {
            let expiration = Utc::now()
                + chrono::Duration::seconds(self.config.refresh_token_expiration_seconds);
            self.blacklist
                .revoke(claims.jti.clone(), expiration)
                .await?;
            debug!(jti = %claims.jti, "Old refresh token revoked");
        }

        Ok(new_pair)
    }

    /// Revoke a token
    ///
    /// # Arguments
    ///
    /// * `token` - The token to revoke
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    #[instrument(skip(self, token))]
    pub async fn revoke_token(&self, token: &str) -> JwtResult<()> {
        // Decode without full validation to get the claims
        let mut validation = Validation::new(self.key_manager.current().algorithm());
        validation.validate_exp = false; // Allow expired tokens to be revoked
        validation.insecure_disable_signature_validation();
        validation.set_required_spec_claims::<&str>(&[]); // Don't require any standard claims
        validation.validate_aud = false; // Don't validate audience
        validation.validate_nbf = false; // Don't validate not before

        let token_data = decode::<Claims>(
            token,
            self.key_manager.current().decoding_key(),
            &validation,
        )
        .map_err(|e| JwtError::DecodingError(e.to_string()))?;

        let expiration = chrono::DateTime::from_timestamp(token_data.claims.exp, 0)
            .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(24));

        self.blacklist
            .revoke(token_data.claims.jti.clone(), expiration)
            .await?;

        debug!(jti = %token_data.claims.jti, "Token revoked");
        Ok(())
    }

    /// Get the blacklist reference (for cleanup tasks)
    pub fn blacklist(&self) -> &TokenBlacklist {
        &self.blacklist
    }

    /// Get the configuration
    pub fn config(&self) -> &JwtConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> JwtService {
        let config = JwtConfig::development();
        JwtService::from_config(config).unwrap()
    }

    #[test]
    fn test_create_service() {
        let service = create_test_service();
        assert_eq!(service.config().issuer, "xzepr-dev");
    }

    #[test]
    fn test_generate_access_token() {
        let service = create_test_service();
        let token = service
            .generate_access_token(
                "user123".to_string(),
                vec!["admin".to_string()],
                vec!["read".to_string()],
            )
            .unwrap();

        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_generate_refresh_token() {
        let service = create_test_service();
        let token = service
            .generate_refresh_token("user123".to_string())
            .unwrap();

        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_generate_token_pair() {
        let service = create_test_service();
        let pair = service
            .generate_token_pair(
                "user123".to_string(),
                vec!["admin".to_string()],
                vec!["read".to_string()],
            )
            .unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert_eq!(pair.expires_in, 900);
    }

    #[tokio::test]
    async fn test_validate_access_token() {
        let service = create_test_service();
        let token = service
            .generate_access_token(
                "user123".to_string(),
                vec!["admin".to_string()],
                vec!["read".to_string()],
            )
            .unwrap();

        let claims = service.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.token_type, TokenType::Access);
        assert_eq!(claims.roles, vec!["admin"]);
    }

    #[tokio::test]
    async fn test_validate_refresh_token() {
        let service = create_test_service();
        let token = service
            .generate_refresh_token("user123".to_string())
            .unwrap();

        let claims = service.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.token_type, TokenType::Refresh);
    }

    #[tokio::test]
    async fn test_validate_invalid_token() {
        let service = create_test_service();
        let result = service.validate_token("invalid.token.here").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refresh_access_token() {
        let service = create_test_service();
        let refresh_token = service
            .generate_refresh_token("user123".to_string())
            .unwrap();

        let new_pair = service
            .refresh_access_token(
                &refresh_token,
                vec!["user".to_string()],
                vec!["read".to_string()],
            )
            .await
            .unwrap();

        assert!(!new_pair.access_token.is_empty());
        assert!(!new_pair.refresh_token.is_empty());

        // Validate the new access token
        let claims = service
            .validate_token(&new_pair.access_token)
            .await
            .unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.roles, vec!["user"]);
    }

    #[tokio::test]
    async fn test_refresh_with_access_token_fails() {
        let service = create_test_service();
        let access_token = service
            .generate_access_token("user123".to_string(), vec!["admin".to_string()], vec![])
            .unwrap();

        let result = service
            .refresh_access_token(&access_token, vec![], vec![])
            .await;

        assert!(matches!(result, Err(JwtError::InvalidClaim(_))));
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let service = create_test_service();
        let token = service
            .generate_access_token("user123".to_string(), vec![], vec![])
            .unwrap();

        // Token should be valid initially
        assert!(service.validate_token(&token).await.is_ok());

        // Revoke the token
        service.revoke_token(&token).await.unwrap();

        // Token should now be invalid
        let result = service.validate_token(&token).await;
        assert!(matches!(result, Err(JwtError::Revoked)));
    }

    #[tokio::test]
    async fn test_token_rotation_on_refresh() {
        let service = create_test_service();
        let refresh_token = service
            .generate_refresh_token("user123".to_string())
            .unwrap();

        // Use the refresh token
        service
            .refresh_access_token(&refresh_token, vec![], vec![])
            .await
            .unwrap();

        // Old refresh token should be revoked
        let result = service.validate_token(&refresh_token).await;
        assert!(matches!(result, Err(JwtError::Revoked)));
    }

    #[tokio::test]
    async fn test_blacklist_cleanup() {
        let service = create_test_service();

        // Generate and revoke a token
        let token = service
            .generate_access_token("user123".to_string(), vec![], vec![])
            .unwrap();
        service.revoke_token(&token).await.unwrap();

        // Blacklist should have one entry
        assert_eq!(service.blacklist().size().await, 1);

        // Cleanup won't remove it because it's not expired yet
        let removed = service.blacklist().cleanup_expired().await;
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_from_config_invalid() {
        let mut config = JwtConfig::development();
        config.access_token_expiration_seconds = -1;

        let result = JwtService::from_config(config);
        assert!(result.is_err());
    }
}
