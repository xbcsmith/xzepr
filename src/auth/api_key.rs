// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! API key authentication service and related abstractions.
//!
//! This module provides the `ApiKeyService` for generating and verifying
//! long-lived API keys, along with the repository traits they depend on.

pub use crate::domain::entities::api_key::ApiKey;

use crate::domain::entities::user::User;
use crate::domain::repositories::user_repo::UserRepository;
use crate::domain::value_objects::{ApiKeyId, UserId};
use crate::error::{AuthError, DomainError};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;

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

/// The version of the key digest algorithm in use.
///
/// Each variant encodes both the algorithm and storage format so that keys
/// created under different versions can coexist in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyDigestVersion {
    /// Legacy SHA-256 with no server-side pepper.
    ///
    /// Used for keys issued before the peppered-hash migration.
    /// Stored as a 64-character lowercase hex string.
    V1,
    /// HMAC-SHA256 with a server-side pepper.
    ///
    /// Stored with the prefix `v2:` followed by a 64-character lowercase hex
    /// string.  The prefix allows the version to be identified from the
    /// stored value alone.
    V2,
}

/// Configuration for the API key digest strategy.
///
/// Controls which hashing algorithm is used when **new** keys are generated.
/// Existing keys stored with the legacy V1 algorithm continue to be verified
/// via the V1 fallback path in [`ApiKeyService::verify_api_key`].
///
/// # Examples
///
/// ```rust
/// use xzepr::auth::api_key::{KeyDigestConfig, KeyDigestVersion};
///
/// // Default: V1 (no pepper) for backward compatibility.
/// let config = KeyDigestConfig::default();
/// assert_eq!(config.version, KeyDigestVersion::V1);
///
/// // Production: V2 with a server-side pepper.
/// let secure = KeyDigestConfig::with_pepper("my-secret-pepper".to_string());
/// assert_eq!(secure.version, KeyDigestVersion::V2);
/// ```
#[derive(Debug, Clone)]
pub struct KeyDigestConfig {
    /// The digest version to use when generating new key hashes.
    pub version: KeyDigestVersion,
    /// Optional server-side pepper for V2 hashes.
    ///
    /// `None` when `version` is `V1`.  Required when `version` is `V2`.
    pepper: Option<String>,
}

impl KeyDigestConfig {
    /// Create a `KeyDigestConfig` using the V1 algorithm (plain SHA-256).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xzepr::auth::api_key::{KeyDigestConfig, KeyDigestVersion};
    ///
    /// let cfg = KeyDigestConfig::v1();
    /// assert_eq!(cfg.version, KeyDigestVersion::V1);
    /// ```
    pub fn v1() -> Self {
        Self {
            version: KeyDigestVersion::V1,
            pepper: None,
        }
    }

    /// Create a `KeyDigestConfig` using the V2 algorithm (HMAC-SHA256 with pepper).
    ///
    /// # Arguments
    ///
    /// * `pepper` - The server-side secret used as the HMAC key.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xzepr::auth::api_key::{KeyDigestConfig, KeyDigestVersion};
    ///
    /// let cfg = KeyDigestConfig::with_pepper("super-secret".to_string());
    /// assert_eq!(cfg.version, KeyDigestVersion::V2);
    /// ```
    pub fn with_pepper(pepper: String) -> Self {
        Self {
            version: KeyDigestVersion::V2,
            pepper: Some(pepper),
        }
    }

    /// Returns the pepper value, if set.
    pub fn pepper(&self) -> Option<&str> {
        self.pepper.as_deref()
    }
}

impl Default for KeyDigestConfig {
    /// Defaults to V1 for backward compatibility with existing deployments.
    fn default() -> Self {
        Self::v1()
    }
}

/// Application service for API key management.
///
/// Provides generation, verification, listing, and revocation of API keys.
/// The digest strategy used for new keys is controlled by [`KeyDigestConfig`].
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use xzepr::auth::api_key::{ApiKeyService, KeyDigestConfig};
///
/// let service = ApiKeyService::new(user_repo, api_key_repo);
/// // or with a pepper:
/// let peppered = ApiKeyService::with_digest_config(
///     user_repo,
///     api_key_repo,
///     KeyDigestConfig::with_pepper("server-pepper".to_string()),
/// );
/// ```
pub struct ApiKeyService {
    user_repo: Arc<dyn UserRepository>,
    api_key_repo: Arc<dyn ApiKeyRepository>,
    digest_config: KeyDigestConfig,
}

impl ApiKeyService {
    /// Create a new `ApiKeyService` using the default V1 digest strategy.
    ///
    /// This constructor is backward-compatible with existing callers.
    /// For production deployments with a server-side pepper, use
    /// [`ApiKeyService::with_digest_config`] instead.
    ///
    /// # Arguments
    ///
    /// * `user_repo` - Repository for user lookup
    /// * `api_key_repo` - Repository for API key persistence
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        api_key_repo: Arc<dyn ApiKeyRepository>,
    ) -> Self {
        Self {
            user_repo,
            api_key_repo,
            digest_config: KeyDigestConfig::default(),
        }
    }

    /// Create a new `ApiKeyService` with an explicit digest configuration.
    ///
    /// Use this constructor in production to enable the peppered V2 strategy.
    ///
    /// # Arguments
    ///
    /// * `user_repo` - Repository for user lookup
    /// * `api_key_repo` - Repository for API key persistence
    /// * `digest_config` - Controls the hash algorithm for new keys
    pub fn with_digest_config(
        user_repo: Arc<dyn UserRepository>,
        api_key_repo: Arc<dyn ApiKeyRepository>,
        digest_config: KeyDigestConfig,
    ) -> Self {
        Self {
            user_repo,
            api_key_repo,
            digest_config,
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
        let key_hash = hash_api_key_with_config(&key, &self.digest_config);

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
    /// Verification tries the V2 (peppered) hash first.  If the key is not
    /// found, it retries with the V1 (plain SHA-256) hash to support keys
    /// issued before the digest migration.
    ///
    /// # Errors
    ///
    /// - `AuthError::InvalidApiKey` if the key does not exist under either hash
    /// - `AuthError::ApiKeyDisabled` if the key is disabled
    /// - `AuthError::ApiKeyExpired` if the key has passed its expiry date
    /// - `AuthError::UserNotFound` if the owning user no longer exists
    pub async fn verify_api_key(&self, key: &str) -> Result<User, AuthError> {
        // Attempt V2 (peppered) lookup first.
        let primary_hash = hash_api_key_with_config(key, &self.digest_config);
        let api_key_opt = self.api_key_repo.find_by_hash(&primary_hash).await?;

        // Fall back to V1 (plain SHA-256) for keys issued before migration.
        let api_key = match api_key_opt {
            Some(k) => k,
            None => {
                let v1_hash = hash_api_key_v1(key);
                // Avoid a second DB round-trip when the primary hash is already V1.
                if v1_hash == primary_hash {
                    return Err(AuthError::InvalidApiKey);
                }
                self.api_key_repo
                    .find_by_hash(&v1_hash)
                    .await?
                    .ok_or(AuthError::InvalidApiKey)?
            }
        };

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
            .find_by_id(&api_key.user_id)
            .await
            .map_err(map_domain_error_to_auth_storage)?
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

fn map_domain_error_to_auth_storage(error: DomainError) -> AuthError {
    AuthError::StorageError {
        message: error.to_string(),
    }
}

/// Generate a cryptographically-random hex-encoded API key prefixed with `xzepr_`.
fn generate_random_key() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    format!("xzepr_{}", hex::encode(bytes))
}

/// Compute the V1 (plain SHA-256) hash of a plaintext API key.
///
/// Returns a 64-character lowercase hex string.  This is the legacy format
/// used for keys created before the V2 migration.
fn hash_api_key_v1(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Compute the V2 (HMAC-SHA256 with pepper) hash of a plaintext API key.
///
/// Returns a string prefixed with `v2:` followed by a 64-character lowercase
/// hex string.  The prefix allows stored hashes to be identified by version.
///
/// # Arguments
///
/// * `key` - The plaintext API key.
/// * `pepper` - The server-side secret used as the HMAC key.
fn hash_api_key_v2(key: &str, pepper: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    // SAFETY: HMAC accepts keys of any length; this cannot fail.
    let mut mac =
        HmacSha256::new_from_slice(pepper.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(key.as_bytes());
    format!("v2:{:x}", mac.finalize().into_bytes())
}

/// Compute a hash for a plaintext API key using the given [`KeyDigestConfig`].
///
/// Dispatches to [`hash_api_key_v1`] or [`hash_api_key_v2`] based on the
/// configured version.
///
/// # Arguments
///
/// * `key` - The plaintext API key.
/// * `config` - The digest configuration.
pub fn hash_api_key_with_config(key: &str, config: &KeyDigestConfig) -> String {
    match config.version {
        KeyDigestVersion::V1 => hash_api_key_v1(key),
        KeyDigestVersion::V2 => {
            let pepper = config.pepper().unwrap_or("");
            hash_api_key_v2(key, pepper)
        }
    }
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
    fn test_hash_api_key_v1_deterministic() {
        let h1 = hash_api_key_v1("test-key");
        let h2 = hash_api_key_v1("test-key");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_api_key_v1_different_inputs() {
        let h1 = hash_api_key_v1("key-a");
        let h2 = hash_api_key_v1("key-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_api_key_v2_has_prefix() {
        let h = hash_api_key_v2("test-key", "my-pepper");
        assert!(h.starts_with("v2:"), "V2 hash must start with 'v2:'");
    }

    #[test]
    fn test_hash_api_key_v2_deterministic() {
        let h1 = hash_api_key_v2("test-key", "my-pepper");
        let h2 = hash_api_key_v2("test-key", "my-pepper");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_api_key_v2_different_pepper() {
        let h1 = hash_api_key_v2("test-key", "pepper-a");
        let h2 = hash_api_key_v2("test-key", "pepper-b");
        assert_ne!(h1, h2, "Different peppers must produce different hashes");
    }

    #[test]
    fn test_hash_api_key_v1_v2_differ() {
        let v1 = hash_api_key_v1("test-key");
        let v2 = hash_api_key_v2("test-key", "pepper");
        assert_ne!(v1, v2, "V1 and V2 hashes must differ");
    }

    #[test]
    fn test_hash_api_key_with_config_v1() {
        let config = KeyDigestConfig::v1();
        let h = hash_api_key_with_config("test-key", &config);
        assert_eq!(h, hash_api_key_v1("test-key"));
    }

    #[test]
    fn test_hash_api_key_with_config_v2() {
        let config = KeyDigestConfig::with_pepper("my-pepper".to_string());
        let h = hash_api_key_with_config("test-key", &config);
        assert!(h.starts_with("v2:"));
    }

    #[test]
    fn test_key_digest_config_default_is_v1() {
        let config = KeyDigestConfig::default();
        assert_eq!(config.version, KeyDigestVersion::V1);
        assert!(config.pepper().is_none());
    }

    #[test]
    fn test_key_digest_config_with_pepper_is_v2() {
        let config = KeyDigestConfig::with_pepper("secret".to_string());
        assert_eq!(config.version, KeyDigestVersion::V2);
        assert_eq!(config.pepper(), Some("secret"));
    }
}
