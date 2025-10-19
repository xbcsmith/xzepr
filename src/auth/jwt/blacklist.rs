//! JWT Token Blacklist
//!
//! This module implements token revocation through a blacklist mechanism.
//! Tokens can be added to the blacklist to prevent their use even if they
//! haven't expired yet.
//!
//! Currently uses an in-memory implementation. For production deployments
//! across multiple servers, consider using Redis or another distributed cache.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

use super::error::{JwtError, JwtResult};

/// Token blacklist for revoking tokens before expiration
#[derive(Clone)]
pub struct TokenBlacklist {
    /// Map of token JTI to expiration time
    tokens: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl TokenBlacklist {
    /// Create a new empty blacklist
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a token to the blacklist
    ///
    /// # Arguments
    ///
    /// * `jti` - The JWT ID to blacklist
    /// * `expiration` - When the token expires (for cleanup purposes)
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    #[instrument(skip(self))]
    pub async fn revoke(&self, jti: String, expiration: DateTime<Utc>) -> JwtResult<()> {
        let mut tokens = self.tokens.write().await;
        tokens.insert(jti.clone(), expiration);
        info!(jti = %jti, "Token revoked");
        Ok(())
    }

    /// Check if a token is blacklisted
    ///
    /// # Arguments
    ///
    /// * `jti` - The JWT ID to check
    ///
    /// # Returns
    ///
    /// Ok(()) if not blacklisted, Err(JwtError::Revoked) if blacklisted
    #[instrument(skip(self))]
    pub async fn is_revoked(&self, jti: &str) -> JwtResult<()> {
        let tokens = self.tokens.read().await;
        if tokens.contains_key(jti) {
            debug!(jti = %jti, "Token is blacklisted");
            return Err(JwtError::Revoked);
        }
        Ok(())
    }

    /// Remove expired tokens from the blacklist
    ///
    /// This should be called periodically to prevent memory growth.
    ///
    /// # Returns
    ///
    /// The number of tokens removed
    #[instrument(skip(self))]
    pub async fn cleanup_expired(&self) -> usize {
        let mut tokens = self.tokens.write().await;
        let now = Utc::now();
        let initial_count = tokens.len();

        tokens.retain(|_, expiration| *expiration > now);

        let removed = initial_count - tokens.len();
        if removed > 0 {
            info!(removed = removed, "Cleaned up expired blacklisted tokens");
        }
        removed
    }

    /// Get the number of blacklisted tokens
    pub async fn size(&self) -> usize {
        let tokens = self.tokens.read().await;
        tokens.len()
    }

    /// Clear all blacklisted tokens
    ///
    /// This is primarily useful for testing.
    #[cfg(test)]
    pub async fn clear(&self) {
        let mut tokens = self.tokens.write().await;
        tokens.clear();
    }
}

impl Default for TokenBlacklist {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for token blacklist implementations
///
/// This allows for alternative implementations (e.g., Redis-based)
#[async_trait::async_trait]
pub trait Blacklist: Send + Sync {
    /// Revoke a token
    async fn revoke(&self, jti: String, expiration: DateTime<Utc>) -> JwtResult<()>;

    /// Check if a token is revoked
    async fn is_revoked(&self, jti: &str) -> JwtResult<()>;

    /// Clean up expired tokens
    async fn cleanup_expired(&self) -> usize;
}

#[async_trait::async_trait]
impl Blacklist for TokenBlacklist {
    async fn revoke(&self, jti: String, expiration: DateTime<Utc>) -> JwtResult<()> {
        self.revoke(jti, expiration).await
    }

    async fn is_revoked(&self, jti: &str) -> JwtResult<()> {
        self.is_revoked(jti).await
    }

    async fn cleanup_expired(&self) -> usize {
        self.cleanup_expired().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_new_blacklist() {
        let blacklist = TokenBlacklist::new();
        assert_eq!(blacklist.size().await, 0);
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let blacklist = TokenBlacklist::new();
        let jti = "test-jti-123".to_string();
        let expiration = Utc::now() + Duration::hours(1);

        blacklist.revoke(jti.clone(), expiration).await.unwrap();
        assert_eq!(blacklist.size().await, 1);
    }

    #[tokio::test]
    async fn test_is_revoked_not_blacklisted() {
        let blacklist = TokenBlacklist::new();
        let result = blacklist.is_revoked("unknown-jti").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_is_revoked_blacklisted() {
        let blacklist = TokenBlacklist::new();
        let jti = "test-jti-123".to_string();
        let expiration = Utc::now() + Duration::hours(1);

        blacklist.revoke(jti.clone(), expiration).await.unwrap();

        let result = blacklist.is_revoked(&jti).await;
        assert!(matches!(result, Err(JwtError::Revoked)));
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let blacklist = TokenBlacklist::new();

        // Add an expired token
        let jti1 = "expired-token".to_string();
        let expired = Utc::now() - Duration::hours(1);
        blacklist.revoke(jti1, expired).await.unwrap();

        // Add a valid token
        let jti2 = "valid-token".to_string();
        let valid = Utc::now() + Duration::hours(1);
        blacklist.revoke(jti2.clone(), valid).await.unwrap();

        assert_eq!(blacklist.size().await, 2);

        // Cleanup should remove the expired token
        let removed = blacklist.cleanup_expired().await;
        assert_eq!(removed, 1);
        assert_eq!(blacklist.size().await, 1);

        // The valid token should still be blacklisted
        let result = blacklist.is_revoked(&jti2).await;
        assert!(matches!(result, Err(JwtError::Revoked)));
    }

    #[tokio::test]
    async fn test_cleanup_expired_removes_nothing() {
        let blacklist = TokenBlacklist::new();

        // Add only valid tokens
        let jti = "valid-token".to_string();
        let expiration = Utc::now() + Duration::hours(1);
        blacklist.revoke(jti, expiration).await.unwrap();

        let removed = blacklist.cleanup_expired().await;
        assert_eq!(removed, 0);
        assert_eq!(blacklist.size().await, 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let blacklist = TokenBlacklist::new();

        // Add multiple tokens
        for i in 0..5 {
            let jti = format!("token-{}", i);
            let expiration = Utc::now() + Duration::hours(1);
            blacklist.revoke(jti, expiration).await.unwrap();
        }

        assert_eq!(blacklist.size().await, 5);

        blacklist.clear().await;
        assert_eq!(blacklist.size().await, 0);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let blacklist = TokenBlacklist::new();
        let blacklist_clone = blacklist.clone();

        // Spawn multiple tasks that revoke tokens concurrently
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let bl = blacklist.clone();
                tokio::spawn(async move {
                    let jti = format!("concurrent-token-{}", i);
                    let expiration = Utc::now() + Duration::hours(1);
                    bl.revoke(jti, expiration).await.unwrap();
                })
            })
            .collect();

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Check that all tokens were added
        assert_eq!(blacklist_clone.size().await, 10);
    }

    #[tokio::test]
    async fn test_trait_implementation() {
        let blacklist: Box<dyn Blacklist> = Box::new(TokenBlacklist::new());
        let jti = "trait-test-token".to_string();
        let expiration = Utc::now() + Duration::hours(1);

        blacklist.revoke(jti.clone(), expiration).await.unwrap();
        let result = blacklist.is_revoked(&jti).await;
        assert!(matches!(result, Err(JwtError::Revoked)));
    }
}
