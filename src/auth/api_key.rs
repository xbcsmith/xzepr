// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/auth/api_key.rs
use crate::domain::entities::user::User;
use crate::domain::value_objects::{ApiKeyId, UserId};
use crate::error::AuthError;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub user_id: UserId,
    pub key_hash: String,
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl ApiKey {
    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AuthError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AuthError>;
    async fn save(&self, user: &User) -> Result<(), AuthError>;
    async fn find_all(&self) -> Result<Vec<User>, AuthError>;
    async fn add_role(
        &self,
        user_id: &UserId,
        role: crate::auth::rbac::roles::Role,
    ) -> Result<(), AuthError>;
    async fn remove_role(
        &self,
        user_id: &UserId,
        role: crate::auth::rbac::roles::Role,
    ) -> Result<(), AuthError>;
}

#[async_trait::async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn save(&self, api_key: &ApiKey) -> Result<(), AuthError>;
    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, AuthError>;
    async fn update_last_used(&self, id: ApiKeyId) -> Result<(), AuthError>;
    async fn find_by_user_id(&self, user_id: UserId) -> Result<Vec<ApiKey>, AuthError>;
    async fn revoke(&self, id: ApiKeyId) -> Result<(), AuthError>;
}

pub struct ApiKeyService {
    user_repo: Arc<dyn UserRepository>,
    api_key_repo: Arc<dyn ApiKeyRepository>,
}

impl ApiKeyService {
    pub async fn list_user_keys(&self, user_id: &UserId) -> Result<Vec<ApiKey>, AuthError> {
        self.api_key_repo.find_by_user_id(*user_id).await
    }

    pub async fn revoke_key(&self, key_id: ApiKeyId) -> Result<(), AuthError> {
        self.api_key_repo.revoke(key_id).await
    }
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        api_key_repo: Arc<dyn ApiKeyRepository>,
    ) -> Self {
        Self {
            user_repo,
            api_key_repo,
        }
    }
    pub async fn generate_api_key(
        &self,
        user_id: UserId,
        name: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(String, ApiKey), AuthError> {
        // Generate random key
        let key = generate_random_key();
        let key_hash = hash_api_key(&key);

        // Create API key record
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

        // Return plaintext key only once
        Ok((key, api_key))
    }

    pub async fn verify_api_key(&self, key: &str) -> Result<User, AuthError> {
        let key_hash = hash_api_key(key);

        // Find API key
        let api_key = self
            .api_key_repo
            .find_by_hash(&key_hash)
            .await?
            .ok_or(AuthError::InvalidApiKey)?;

        // Check if enabled
        if !api_key.enabled {
            return Err(AuthError::ApiKeyDisabled);
        }

        // Check expiration
        if let Some(expires_at) = api_key.expires_at {
            if Utc::now() > expires_at {
                return Err(AuthError::ApiKeyExpired);
            }
        }

        // Update last used timestamp
        self.api_key_repo.update_last_used(api_key.id).await?;

        // Get user
        let user = self
            .user_repo
            .find_by_id(api_key.user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        Ok(user)
    }
}

fn generate_random_key() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    // Using hex encoding instead of base64 for simplicity
    format!("xzepr_{}", hex::encode(bytes))
}

fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

// Stub implementation for demo purposes
pub struct StubApiKeyRepository;

#[async_trait::async_trait]
impl ApiKeyRepository for StubApiKeyRepository {
    async fn save(&self, _api_key: &ApiKey) -> Result<(), AuthError> {
        // Stub: pretend to save
        Ok(())
    }

    async fn find_by_hash(&self, _hash: &str) -> Result<Option<ApiKey>, AuthError> {
        // Stub: no keys found
        Ok(None)
    }

    async fn update_last_used(&self, _id: ApiKeyId) -> Result<(), AuthError> {
        // Stub: pretend to update
        Ok(())
    }

    async fn find_by_user_id(&self, _user_id: UserId) -> Result<Vec<ApiKey>, AuthError> {
        // Stub: return empty list
        Ok(vec![])
    }

    async fn revoke(&self, _id: ApiKeyId) -> Result<(), AuthError> {
        // Stub: pretend to revoke
        Ok(())
    }
}
