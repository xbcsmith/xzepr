// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! API key authentication service and related abstractions.
//!
//! This module provides the `ApiKeyService` for generating and verifying
//! long-lived API keys, along with the repository traits they depend on.

pub use crate::domain::entities::api_key::ApiKey;

use crate::domain::entities::user::User;
use crate::domain::value_objects::{ApiKeyId, UserId};
use crate::error::AuthError;
use chrono::{DateTime, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Repository for user lookup operations required by the auth layer.
///
/// This trait is distinct from the more comprehensive
/// `domain::repositories::user_repo::UserRepository` because the auth layer
/// needs only a small focused subset of user operations and maps all errors to
/// `AuthError`.
///
/// # Examples
///
/// ```rust,ignore
/// use xzepr::auth::api_key::{AuthUserRepository, ApiKey};
///
/// async fn example(repo: &impl AuthUserRepository) {
///     let user = repo.find_by_id(user_id).await.ok().flatten();
/// }
/// ```
#[async_trait::async_trait]
pub trait AuthUserRepository: Send + Sync {
    /// Find a user by their ULID identifier.
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AuthError>;
    /// Find a user by username.
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AuthError>;
    /// Upsert a user record (create or update by primary key).
    async fn save(&self, user: &User) -> Result<(), AuthError>;
    /// Return all users ordered by creation time descending.
    async fn find_all(&self) -> Result<Vec<User>, AuthError>;
    /// Add a role to a user.
    async fn add_role(
        &self,
        user_id: &UserId,
        role: crate::auth::rbac::roles::Role,
    ) -> Result<(), AuthError>;
    /// Remove a role from a user.
    async fn remove_role(
        &self,
        user_id: &UserId,
        role: crate::auth::rbac::roles::Role,
    ) -> Result<(), AuthError>;
}

/// Repository for API key persistence.
///
/// # Examples
///
/// ```rust,ignore
/// use xzepr::auth::api_key::{ApiKeyRepository, ApiKey};
///
/// async fn example(repo: &impl ApiKeyRepository, hash: &str) {
///     let key = repo.find_by_hash(hash).await.ok().flatten();
/// }
/// ```
#[async_trait::async_trait]
pub trait ApiKeyRepository: Send + Sync {
    /// Persist a new or updated API key.
    async fn save(&self, api_key: &ApiKey) -> Result<(), AuthError>;
    /// Find a key by its SHA-256 hash.
    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, AuthError>;
    /// Record the current timestamp as the last-used time for the given key.
    async fn update_last_used(&self, id: ApiKeyId) -> Result<(), AuthError>;
    /// Return all keys belonging to a user.
    async fn find_by_user_id(&self, user_id: UserId) -> Result<Vec<ApiKey>, AuthError>;
    /// Disable (revoke) a key by its identifier.
    async fn revoke(&self, id: ApiKeyId) -> Result<(), AuthError>;
}

/// Application service for API key management.
///
/// Provides generation, verification, listing, and revocation of API keys.
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use xzepr::auth::api_key::ApiKeyService;
///
/// let service = ApiKeyService::new(user_repo, api_key_repo);
/// let (plaintext, key) = service.generate_api_key(user_id, "CI".into(), None).await?;
/// ```
pub struct ApiKeyService {
    user_repo: Arc<dyn AuthUserRepository>,
    api_key_repo: Arc<dyn ApiKeyRepository>,
}

impl ApiKeyService {
    /// Create a new `ApiKeyService`.
    ///
    /// # Arguments
    ///
    /// * `user_repo` - Repository for user lookup
    /// * `api_key_repo` - Repository for API key persistence
    pub fn new(
        user_repo: Arc<dyn AuthUserRepository>,
        api_key_repo: Arc<dyn ApiKeyRepository>,
    ) -> Self {
        Self {
            user_repo,
            api_key_repo,
        }
    }

    /// Generate a new API key for a user.
    ///
    /// Returns the plaintext key (only available at creation time) and the
    /// persisted key record.
    ///
    /// # Errors
    ///
    /// Returns `AuthError` if the key cannot be saved.
    pub async fn generate_api_key(
        &self,
        user_id: UserId,
        name: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(String, ApiKey), AuthError> {
        let key = generate_random_key();
        let key_hash = hash_api_key(&key);

        let api_key = ApiKey {
            id: ApiKeyId::new(),
            user_id,
            key_hash,
            name,
            expires_at,
            enabled: true,
            created_at: Utc::now(),
            last_used_at: None,
        };

        self.api_key_repo.save(&api_key).await?;
        Ok((key, api_key))
    }

    /// Verify a plaintext API key and return the owning user on success.
    ///
    /// # Errors
    ///
    /// - `AuthError::InvalidApiKey` if the key does not exist
    /// - `AuthError::ApiKeyDisabled` if the key is disabled
    /// - `AuthError::ApiKeyExpired` if the key has passed its expiry date
    /// - `AuthError::UserNotFound` if the owning user no longer exists
    pub async fn verify_api_key(&self, key: &str) -> Result<User, AuthError> {
        let key_hash = hash_api_key(key);

        let api_key = self
            .api_key_repo
            .find_by_hash(&key_hash)
            .await?
            .ok_or(AuthError::InvalidApiKey)?;

        if !api_key.enabled {
            return Err(AuthError::ApiKeyDisabled);
        }

        if let Some(expires_at) = api_key.expires_at {
            if Utc::now() > expires_at {
                return Err(AuthError::ApiKeyExpired);
            }
        }

        self.api_key_repo.update_last_used(api_key.id).await?;

        let user = self
            .user_repo
            .find_by_id(api_key.user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        Ok(user)
    }

    /// Return all API keys belonging to a user.
    ///
    /// # Errors
    ///
    /// Returns `AuthError` if the repository query fails.
    pub async fn list_user_keys(&self, user_id: &UserId) -> Result<Vec<ApiKey>, AuthError> {
        self.api_key_repo.find_by_user_id(*user_id).await
    }

    /// Revoke (disable) an API key.
    ///
    /// # Errors
    ///
    /// Returns `AuthError` if the repository query fails.
    pub async fn revoke_key(&self, key_id: ApiKeyId) -> Result<(), AuthError> {
        self.api_key_repo.revoke(key_id).await
    }
}

/// Generate a cryptographically-random hex-encoded API key prefixed with `xzepr_`.
fn generate_random_key() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    format!("xzepr_{}", hex::encode(bytes))
}

/// Compute the SHA-256 hash of a plaintext key as a lowercase hex string.
fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
pub struct StubApiKeyRepository;

#[cfg(test)]
#[async_trait::async_trait]
impl ApiKeyRepository for StubApiKeyRepository {
    async fn save(&self, _api_key: &ApiKey) -> Result<(), AuthError> {
        Ok(())
    }

    async fn find_by_hash(&self, _hash: &str) -> Result<Option<ApiKey>, AuthError> {
        Ok(None)
    }

    async fn update_last_used(&self, _id: ApiKeyId) -> Result<(), AuthError> {
        Ok(())
    }

    async fn find_by_user_id(&self, _user_id: UserId) -> Result<Vec<ApiKey>, AuthError> {
        Ok(vec![])
    }

    async fn revoke(&self, _id: ApiKeyId) -> Result<(), AuthError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_key_prefix() {
        let key = generate_random_key();
        assert!(key.starts_with("xzepr_"));
    }

    #[test]
    fn test_hash_api_key_deterministic() {
        let h1 = hash_api_key("test-key");
        let h2 = hash_api_key("test-key");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_api_key_different_inputs() {
        let h1 = hash_api_key("key-a");
        let h2 = hash_api_key("key-b");
        assert_ne!(h1, h2);
    }
}
