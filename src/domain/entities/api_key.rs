// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! API key domain entity.
//!
//! An `ApiKey` represents a long-lived credential associated with a user account.
//! The plaintext key is only ever returned at creation time; the repository stores
//! only the SHA-256 hash.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{ApiKeyId, UserId};

/// A hashed API key associated with a user account.
///
/// # Examples
///
/// ```rust,ignore
/// use xzepr::domain::entities::api_key::ApiKey;
/// use xzepr::domain::value_objects::{ApiKeyId, UserId};
/// use chrono::Utc;
///
/// let key = ApiKey {
///     id: ApiKeyId::new(),
///     user_id: UserId::new(),
///     key_hash: "abc123".to_string(),
///     name: "CI token".to_string(),
///     expires_at: None,
///     enabled: true,
///     created_at: Utc::now(),
///     last_used_at: None,
/// };
/// assert!(key.enabled());
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique identifier for this API key.
    pub id: ApiKeyId,
    /// Identifier of the user that owns this key.
    pub user_id: UserId,
    /// SHA-256 hash of the plaintext key.
    pub key_hash: String,
    /// Human-readable label for this key.
    pub name: String,
    /// Optional expiry timestamp.
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether the key can currently be used.
    pub enabled: bool,
    /// When the key was created.
    pub created_at: DateTime<Utc>,
    /// When the key was last successfully authenticated.
    pub last_used_at: Option<DateTime<Utc>>,
}

impl std::fmt::Debug for ApiKey {
    /// Formats the key for debug output, always redacting `key_hash`.
    ///
    /// The `key_hash` field is never included in debug output to prevent
    /// accidental exposure in logs, error traces, or test output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKey")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("key_hash", &"[REDACTED]")
            .field("name", &self.name)
            .field("expires_at", &self.expires_at)
            .field("enabled", &self.enabled)
            .field("created_at", &self.created_at)
            .field("last_used_at", &self.last_used_at)
            .finish()
    }
}

impl ApiKey {
    /// Returns a reference to the key's unique identifier.
    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }

    /// Returns the human-readable name of this key.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns `true` if the key is currently enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the optional expiry timestamp.
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_api_key() -> ApiKey {
        ApiKey {
            id: ApiKeyId::new(),
            user_id: UserId::new(),
            key_hash: "abc123".to_string(),
            name: "test".to_string(),
            expires_at: None,
            enabled: true,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    #[test]
    fn test_api_key_enabled() {
        let key = make_api_key();
        assert!(key.enabled());
    }

    #[test]
    fn test_api_key_disabled() {
        let mut key = make_api_key();
        key.enabled = false;
        assert!(!key.enabled());
    }

    #[test]
    fn test_api_key_name() {
        let key = make_api_key();
        assert_eq!(key.name(), "test");
    }

    #[test]
    fn test_api_key_no_expiry() {
        let key = make_api_key();
        assert!(key.expires_at().is_none());
    }

    #[test]
    fn test_api_key_with_expiry() {
        let mut key = make_api_key();
        let exp = Utc::now() + chrono::Duration::days(30);
        key.expires_at = Some(exp);
        assert!(key.expires_at().is_some());
    }

    #[test]
    fn test_api_key_debug_redacts_key_hash() {
        let key = make_api_key();
        let debug_output = format!("{:?}", key);
        assert!(
            !debug_output.contains("abc123"),
            "Debug output must not contain the key_hash value"
        );
        assert!(
            debug_output.contains("[REDACTED]"),
            "Debug output must contain [REDACTED] in place of key_hash"
        );
    }
}
