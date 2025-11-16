// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Authorization cache with TTL and resource-version-based invalidation
//!
//! This module provides a cache for OPA authorization decisions with automatic
//! expiration and invalidation when resources are updated.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache key for authorization decisions
///
/// Uniquely identifies a cached authorization decision based on user,
/// action, resource type, resource ID, and resource version.
///
/// # Examples
///
/// ```
/// use xzepr::opa::cache::CacheKey;
///
/// let key = CacheKey {
///     user_id: "user123".to_string(),
///     action: "read".to_string(),
///     resource_type: "event_receiver".to_string(),
///     resource_id: "receiver123".to_string(),
///     resource_version: 1,
/// };
/// ```
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    /// User ID performing the action
    pub user_id: String,

    /// Action being performed
    pub action: String,

    /// Type of resource
    pub resource_type: String,

    /// ID of the resource
    pub resource_id: String,

    /// Version of the resource (incremented on updates)
    pub resource_version: i32,
}

/// Cached authorization decision entry
///
/// Contains the authorization decision and expiration metadata.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Whether the action is allowed
    pub decision: bool,

    /// When this entry expires
    pub expires_at: DateTime<Utc>,

    /// When this entry was cached
    pub cached_at: DateTime<Utc>,
}

impl CacheEntry {
    /// Checks if the cache entry is expired
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::CacheEntry;
    /// use chrono::{Utc, Duration};
    ///
    /// let entry = CacheEntry {
    ///     decision: true,
    ///     expires_at: Utc::now() + Duration::minutes(5),
    ///     cached_at: Utc::now(),
    /// };
    ///
    /// assert!(!entry.is_expired());
    /// ```
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Authorization cache with TTL and resource-version invalidation
///
/// Caches authorization decisions with automatic expiration and provides
/// methods to invalidate cache entries when resources are updated.
///
/// # Examples
///
/// ```
/// use xzepr::opa::cache::{AuthorizationCache, CacheKey};
/// use chrono::Duration;
///
/// # tokio_test::block_on(async {
/// let cache = AuthorizationCache::new(Duration::minutes(5));
///
/// let key = CacheKey {
///     user_id: "user123".to_string(),
///     action: "read".to_string(),
///     resource_type: "event_receiver".to_string(),
///     resource_id: "receiver123".to_string(),
///     resource_version: 1,
/// };
///
/// cache.set(key.clone(), true).await;
/// let decision = cache.get(&key).await;
/// assert_eq!(decision, Some(true));
/// # });
/// ```
pub struct AuthorizationCache {
    /// Internal cache storage
    cache: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,

    /// Default TTL for cache entries
    default_ttl: Duration,
}

impl AuthorizationCache {
    /// Creates a new authorization cache with the specified default TTL
    ///
    /// # Arguments
    ///
    /// * `default_ttl` - Default time-to-live for cache entries
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::AuthorizationCache;
    /// use chrono::Duration;
    ///
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    /// ```
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    /// Retrieves a cached authorization decision
    ///
    /// Returns `None` if the entry is not found or has expired.
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key identifying the authorization decision
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::{AuthorizationCache, CacheKey};
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    /// let key = CacheKey {
    ///     user_id: "user123".to_string(),
    ///     action: "read".to_string(),
    ///     resource_type: "event_receiver".to_string(),
    ///     resource_id: "receiver123".to_string(),
    ///     resource_version: 1,
    /// };
    ///
    /// let decision = cache.get(&key).await;
    /// assert_eq!(decision, None);
    /// # });
    /// ```
    pub async fn get(&self, key: &CacheKey) -> Option<bool> {
        let cache = self.cache.read().await;
        cache.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.decision)
            }
        })
    }

    /// Stores an authorization decision in the cache
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key identifying the authorization decision
    /// * `decision` - Whether the action is allowed
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::{AuthorizationCache, CacheKey};
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    /// let key = CacheKey {
    ///     user_id: "user123".to_string(),
    ///     action: "read".to_string(),
    ///     resource_type: "event_receiver".to_string(),
    ///     resource_id: "receiver123".to_string(),
    ///     resource_version: 1,
    /// };
    ///
    /// cache.set(key, true).await;
    /// # });
    /// ```
    pub async fn set(&self, key: CacheKey, decision: bool) {
        let now = Utc::now();
        let entry = CacheEntry {
            decision,
            expires_at: now + self.default_ttl,
            cached_at: now,
        };

        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }

    /// Invalidates all cache entries for a specific resource
    ///
    /// This should be called when a resource is updated to ensure
    /// cached authorization decisions are re-evaluated.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - Type of resource (e.g., "event_receiver")
    /// * `resource_id` - ID of the resource
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::AuthorizationCache;
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    ///
    /// cache.invalidate_resource("event_receiver", "receiver123").await;
    /// # });
    /// ```
    pub async fn invalidate_resource(&self, resource_type: &str, resource_id: &str) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, _| {
            !(key.resource_type == resource_type && key.resource_id == resource_id)
        });
    }

    /// Invalidates all cache entries for a specific user
    ///
    /// This should be called when a user's permissions change.
    ///
    /// # Arguments
    ///
    /// * `user_id` - ID of the user
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::AuthorizationCache;
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    ///
    /// cache.invalidate_user("user123").await;
    /// # });
    /// ```
    pub async fn invalidate_user(&self, user_id: &str) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, _| key.user_id != user_id);
    }

    /// Clears all entries from the cache
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::AuthorizationCache;
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    ///
    /// cache.clear().await;
    /// # });
    /// ```
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Evicts expired entries from the cache
    ///
    /// This should be called periodically to free memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::AuthorizationCache;
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    ///
    /// cache.evict_expired().await;
    /// # });
    /// ```
    pub async fn evict_expired(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Returns the number of entries in the cache
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::{AuthorizationCache, CacheKey};
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    /// assert_eq!(cache.len().await, 0);
    /// # });
    /// ```
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Checks if the cache is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::AuthorizationCache;
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    /// assert!(cache.is_empty().await);
    /// # });
    /// ```
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }
}

/// Resource update event for cache invalidation
///
/// Events that trigger cache invalidation when resources are modified.
#[derive(Debug, Clone)]
pub enum ResourceUpdatedEvent {
    /// Event receiver was updated
    EventReceiverUpdated {
        /// ID of the event receiver
        receiver_id: String,
        /// New version number
        version: i32,
    },

    /// Event receiver group was updated
    EventReceiverGroupUpdated {
        /// ID of the group
        group_id: String,
        /// New version number
        version: i32,
    },

    /// Event was updated
    EventUpdated {
        /// ID of the event
        event_id: String,
        /// New version number
        version: i32,
    },

    /// User permissions changed (e.g., role or group membership change)
    UserPermissionsChanged {
        /// ID of the user
        user_id: String,
    },
}

impl ResourceUpdatedEvent {
    /// Applies this event to the cache by invalidating affected entries
    ///
    /// # Arguments
    ///
    /// * `cache` - The authorization cache to update
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::cache::{AuthorizationCache, ResourceUpdatedEvent};
    /// use chrono::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let cache = AuthorizationCache::new(Duration::minutes(5));
    /// let event = ResourceUpdatedEvent::EventReceiverUpdated {
    ///     receiver_id: "receiver123".to_string(),
    ///     version: 2,
    /// };
    ///
    /// event.apply(&cache).await;
    /// # });
    /// ```
    pub async fn apply(&self, cache: &AuthorizationCache) {
        match self {
            ResourceUpdatedEvent::EventReceiverUpdated { receiver_id, .. } => {
                cache
                    .invalidate_resource("event_receiver", receiver_id)
                    .await;
            }
            ResourceUpdatedEvent::EventReceiverGroupUpdated { group_id, .. } => {
                cache
                    .invalidate_resource("event_receiver_group", group_id)
                    .await;
            }
            ResourceUpdatedEvent::EventUpdated { event_id, .. } => {
                cache.invalidate_resource("event", event_id).await;
            }
            ResourceUpdatedEvent::UserPermissionsChanged { user_id } => {
                cache.invalidate_user(user_id).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_get_set() {
        let cache = AuthorizationCache::new(Duration::minutes(5));
        let key = CacheKey {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };

        assert_eq!(cache.get(&key).await, None);

        cache.set(key.clone(), true).await;
        assert_eq!(cache.get(&key).await, Some(true));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = AuthorizationCache::new(Duration::milliseconds(100));
        let key = CacheKey {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };

        cache.set(key.clone(), true).await;
        assert_eq!(cache.get(&key).await, Some(true));

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        assert_eq!(cache.get(&key).await, None);
    }

    #[tokio::test]
    async fn test_invalidate_resource() {
        let cache = AuthorizationCache::new(Duration::minutes(5));
        let key1 = CacheKey {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };
        let key2 = CacheKey {
            user_id: "user456".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };
        let key3 = CacheKey {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver456".to_string(),
            resource_version: 1,
        };

        cache.set(key1.clone(), true).await;
        cache.set(key2.clone(), true).await;
        cache.set(key3.clone(), true).await;

        cache
            .invalidate_resource("event_receiver", "receiver123")
            .await;

        assert_eq!(cache.get(&key1).await, None);
        assert_eq!(cache.get(&key2).await, None);
        assert_eq!(cache.get(&key3).await, Some(true));
    }

    #[tokio::test]
    async fn test_invalidate_user() {
        let cache = AuthorizationCache::new(Duration::minutes(5));
        let key1 = CacheKey {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };
        let key2 = CacheKey {
            user_id: "user456".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };

        cache.set(key1.clone(), true).await;
        cache.set(key2.clone(), true).await;

        cache.invalidate_user("user123").await;

        assert_eq!(cache.get(&key1).await, None);
        assert_eq!(cache.get(&key2).await, Some(true));
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = AuthorizationCache::new(Duration::minutes(5));
        let key = CacheKey {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };

        cache.set(key.clone(), true).await;
        assert_eq!(cache.len().await, 1);

        cache.clear().await;
        assert_eq!(cache.len().await, 0);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_evict_expired() {
        let cache = AuthorizationCache::new(Duration::milliseconds(100));
        let key1 = CacheKey {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };

        cache.set(key1.clone(), true).await;
        assert_eq!(cache.len().await, 1);

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        cache.evict_expired().await;
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_resource_updated_event_apply() {
        let cache = AuthorizationCache::new(Duration::minutes(5));
        let key = CacheKey {
            user_id: "user123".to_string(),
            action: "read".to_string(),
            resource_type: "event_receiver".to_string(),
            resource_id: "receiver123".to_string(),
            resource_version: 1,
        };

        cache.set(key.clone(), true).await;
        assert_eq!(cache.get(&key).await, Some(true));

        let event = ResourceUpdatedEvent::EventReceiverUpdated {
            receiver_id: "receiver123".to_string(),
            version: 2,
        };
        event.apply(&cache).await;

        assert_eq!(cache.get(&key).await, None);
    }

    #[test]
    fn test_cache_entry_is_expired() {
        let entry = CacheEntry {
            decision: true,
            expires_at: Utc::now() - Duration::minutes(1),
            cached_at: Utc::now() - Duration::minutes(6),
        };
        assert!(entry.is_expired());

        let entry = CacheEntry {
            decision: true,
            expires_at: Utc::now() + Duration::minutes(5),
            cached_at: Utc::now(),
        };
        assert!(!entry.is_expired());
    }
}
