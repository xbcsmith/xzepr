//! JWT Claims
//!
//! This module defines the claims structure for JWT tokens and provides
//! validation logic for token claims.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::error::{JwtError, JwtResult};

/// Standard JWT claims with custom fields for XZepr
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Not before (Unix timestamp)
    pub nbf: i64,
    /// JWT ID (for revocation tracking)
    pub jti: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// User roles
    pub roles: Vec<String>,
    /// Specific permissions
    pub permissions: Vec<String>,
    /// Token type (access or refresh)
    pub token_type: TokenType,
}

/// Token type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    /// Short-lived access token
    Access,
    /// Long-lived refresh token
    Refresh,
}

impl Claims {
    /// Create new claims for an access token
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID (subject)
    /// * `roles` - User roles
    /// * `permissions` - User permissions
    /// * `issuer` - Token issuer
    /// * `audience` - Token audience
    /// * `expiration` - Token expiration duration
    ///
    /// # Returns
    ///
    /// A new Claims instance with populated fields
    pub fn new_access_token(
        user_id: String,
        roles: Vec<String>,
        permissions: Vec<String>,
        issuer: String,
        audience: String,
        expiration: Duration,
    ) -> Self {
        let now = Utc::now();
        let exp = (now + expiration).timestamp();
        let iat = now.timestamp();
        let nbf = now.timestamp();
        let jti = Ulid::new().to_string();

        Self {
            sub: user_id,
            exp,
            iat,
            nbf,
            jti,
            iss: issuer,
            aud: audience,
            roles,
            permissions,
            token_type: TokenType::Access,
        }
    }

    /// Create new claims for a refresh token
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user ID (subject)
    /// * `issuer` - Token issuer
    /// * `audience` - Token audience
    /// * `expiration` - Token expiration duration
    ///
    /// # Returns
    ///
    /// A new Claims instance for a refresh token
    pub fn new_refresh_token(
        user_id: String,
        issuer: String,
        audience: String,
        expiration: Duration,
    ) -> Self {
        let now = Utc::now();
        let exp = (now + expiration).timestamp();
        let iat = now.timestamp();
        let nbf = now.timestamp();
        let jti = Ulid::new().to_string();

        Self {
            sub: user_id,
            exp,
            iat,
            nbf,
            jti,
            iss: issuer,
            aud: audience,
            roles: vec![],
            permissions: vec![],
            token_type: TokenType::Refresh,
        }
    }

    /// Validate the claims
    ///
    /// Checks expiration, not-before time, issuer, and audience.
    ///
    /// # Arguments
    ///
    /// * `expected_issuer` - Expected issuer value
    /// * `expected_audience` - Expected audience value
    ///
    /// # Returns
    ///
    /// Ok(()) if valid, or a JwtError describing the validation failure
    pub fn validate(&self, expected_issuer: &str, expected_audience: &str) -> JwtResult<()> {
        let now = Utc::now().timestamp();

        // Check expiration
        if self.exp <= now {
            return Err(JwtError::Expired);
        }

        // Check not-before
        if self.nbf > now {
            return Err(JwtError::NotYetValid);
        }

        // Check issuer
        if self.iss != expected_issuer {
            return Err(JwtError::InvalidIssuer {
                expected: expected_issuer.to_string(),
                actual: self.iss.clone(),
            });
        }

        // Check audience
        if self.aud != expected_audience {
            return Err(JwtError::InvalidAudience {
                expected: expected_audience.to_string(),
                actual: self.aud.clone(),
            });
        }

        Ok(())
    }

    /// Check if the token has expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp <= now
    }

    /// Check if the token is not yet valid
    pub fn is_not_yet_valid(&self) -> bool {
        let now = Utc::now().timestamp();
        self.nbf > now
    }

    /// Get time until expiration in seconds
    pub fn time_until_expiration(&self) -> i64 {
        let now = Utc::now().timestamp();
        self.exp - now
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[String]) -> bool {
        self.roles.iter().any(|r| roles.contains(r))
    }

    /// Check if user has all of the specified roles
    pub fn has_all_roles(&self, roles: &[String]) -> bool {
        roles.iter().all(|r| self.roles.contains(r))
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[String]) -> bool {
        self.permissions.iter().any(|p| permissions.contains(p))
    }

    /// Check if user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[String]) -> bool {
        permissions.iter().all(|p| self.permissions.contains(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_access_token() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec!["admin".to_string()],
            vec!["read".to_string(), "write".to_string()],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.roles, vec!["admin"]);
        assert_eq!(claims.permissions, vec!["read", "write"]);
        assert_eq!(claims.token_type, TokenType::Access);
        assert_eq!(claims.iss, "xzepr");
        assert_eq!(claims.aud, "xzepr-api");
        assert!(!claims.jti.is_empty());
    }

    #[test]
    fn test_new_refresh_token() {
        let claims = Claims::new_refresh_token(
            "user123".to_string(),
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::days(7),
        );

        assert_eq!(claims.sub, "user123");
        assert!(claims.roles.is_empty());
        assert!(claims.permissions.is_empty());
        assert_eq!(claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn test_validate_success() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert!(claims.validate("xzepr", "xzepr-api").is_ok());
    }

    #[test]
    fn test_validate_expired() {
        let mut claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        // Set expiration to the past
        claims.exp = Utc::now().timestamp() - 100;

        let result = claims.validate("xzepr", "xzepr-api");
        assert!(matches!(result, Err(JwtError::Expired)));
    }

    #[test]
    fn test_validate_not_yet_valid() {
        let mut claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        // Set nbf to the future
        claims.nbf = Utc::now().timestamp() + 1000;

        let result = claims.validate("xzepr", "xzepr-api");
        assert!(matches!(result, Err(JwtError::NotYetValid)));
    }

    #[test]
    fn test_validate_invalid_issuer() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        let result = claims.validate("other-issuer", "xzepr-api");
        assert!(matches!(result, Err(JwtError::InvalidIssuer { .. })));
    }

    #[test]
    fn test_validate_invalid_audience() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        let result = claims.validate("xzepr", "other-audience");
        assert!(matches!(result, Err(JwtError::InvalidAudience { .. })));
    }

    #[test]
    fn test_is_expired() {
        let mut claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert!(!claims.is_expired());

        claims.exp = Utc::now().timestamp() - 100;
        assert!(claims.is_expired());
    }

    #[test]
    fn test_has_role() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec!["admin".to_string(), "user".to_string()],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert!(claims.has_role("admin"));
        assert!(claims.has_role("user"));
        assert!(!claims.has_role("superadmin"));
    }

    #[test]
    fn test_has_any_role() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec!["admin".to_string()],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert!(claims.has_any_role(&["admin".to_string(), "superadmin".to_string()]));
        assert!(!claims.has_any_role(&["user".to_string(), "guest".to_string()]));
    }

    #[test]
    fn test_has_all_roles() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec!["admin".to_string(), "user".to_string()],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert!(claims.has_all_roles(&["admin".to_string(), "user".to_string()]));
        assert!(!claims.has_all_roles(&["admin".to_string(), "superadmin".to_string()]));
    }

    #[test]
    fn test_has_permission() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec!["read".to_string(), "write".to_string()],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert!(claims.has_permission("read"));
        assert!(claims.has_permission("write"));
        assert!(!claims.has_permission("delete"));
    }

    #[test]
    fn test_has_any_permission() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec!["read".to_string()],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert!(claims.has_any_permission(&["read".to_string(), "write".to_string()]));
        assert!(!claims.has_any_permission(&["write".to_string(), "delete".to_string()]));
    }

    #[test]
    fn test_has_all_permissions() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec!["read".to_string(), "write".to_string()],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        assert!(claims.has_all_permissions(&["read".to_string(), "write".to_string()]));
        assert!(!claims.has_all_permissions(&["read".to_string(), "delete".to_string()]));
    }

    #[test]
    fn test_time_until_expiration() {
        let claims = Claims::new_access_token(
            "user123".to_string(),
            vec![],
            vec![],
            "xzepr".to_string(),
            "xzepr-api".to_string(),
            Duration::minutes(15),
        );

        let time_left = claims.time_until_expiration();
        assert!(time_left > 0);
        assert!(time_left <= 900); // 15 minutes = 900 seconds
    }
}
