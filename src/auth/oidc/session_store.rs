// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OIDC Session Store
//!
//! This module provides session storage for OIDC authentication flows.
//! Sessions are keyed by the `state` parameter issued during the authorization
//! request and consumed exactly once during the callback.
//!
//! Two implementations are provided:
//!
//! - [`InMemoryOidcSessionStore`]: suitable for single-node deployments.
//! - [`NullOidcSessionStore`]: no-op implementation for OIDC-disabled builds
//!   and unit tests that do not require real session persistence.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::sync::RwLock;

use super::callback::OidcSession;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`OidcSessionStore`] operations.
#[derive(Error, Debug)]
pub enum SessionStoreError {
    /// A storage backend error occurred.
    #[error("Storage backend error: {0}")]
    Backend(String),

    /// The store has reached its maximum number of pending sessions.
    #[error("Session store at capacity ({0} pending sessions)")]
    CapacityExceeded(usize),
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Trait for OIDC session storage backends.
///
/// Implementors must be `Send + Sync` so they can be shared across async tasks
/// behind an `Arc`.
#[async_trait::async_trait]
pub trait OidcSessionStore: Send + Sync {
    /// Insert a session keyed by `state`, expiring after `ttl`.
    ///
    /// If a session already exists for `state` it is overwritten without
    /// counting against the capacity limit.
    ///
    /// # Arguments
    ///
    /// * `state` - Unique state string used as the session key
    /// * `session` - The OIDC session data to store
    /// * `ttl` - Time-to-live for the session entry
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError::CapacityExceeded`] if the store is at
    /// maximum capacity and `state` is not an existing key.
    /// Returns [`SessionStoreError::Backend`] on storage failure.
    async fn insert(
        &self,
        state: String,
        session: OidcSession,
        ttl: Duration,
    ) -> Result<(), SessionStoreError>;

    /// Remove and return the session for `state` if it exists and has not expired.
    ///
    /// Returns `None` if the session is missing or has expired. The entry is
    /// removed from the store in both cases (consume-once semantics).
    ///
    /// # Arguments
    ///
    /// * `state` - State key to look up
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError::Backend`] on storage failure.
    async fn take(&self, state: &str) -> Result<Option<OidcSession>, SessionStoreError>;

    /// Remove all sessions whose TTL has elapsed.
    ///
    /// Called periodically by background tasks to reclaim memory.
    ///
    /// # Returns
    ///
    /// Returns the count of sessions that were removed.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError::Backend`] on storage failure.
    async fn cleanup_expired(&self) -> Result<usize, SessionStoreError>;

    /// Return the count of currently pending (non-expired) sessions.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError::Backend`] on storage failure.
    async fn pending_count(&self) -> Result<usize, SessionStoreError>;
}

// ---------------------------------------------------------------------------
// InMemoryOidcSessionStore internals
// ---------------------------------------------------------------------------

/// Internal entry held in the in-memory session map.
struct StoredSession {
    /// The OIDC session payload.
    session: OidcSession,

    /// Absolute instant at which this entry expires.
    expires_at: Instant,
}

// ---------------------------------------------------------------------------
// InMemoryOidcSessionStore
// ---------------------------------------------------------------------------

/// In-memory OIDC session store backed by a [`tokio::sync::RwLock`].
///
/// This implementation is suitable for single-node deployments. For
/// multi-node or persistent deployments, a distributed backend (e.g., Redis)
/// should be used instead.
///
/// Sessions are stored in a `HashMap` protected by a `RwLock`. Read operations
/// (e.g., [`pending_count`](OidcSessionStore::pending_count)) acquire a shared
/// read lock. Mutations acquire an exclusive write lock.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use std::time::Duration;
/// use xzepr::auth::oidc::session_store::{InMemoryOidcSessionStore, OidcSessionStore};
/// use xzepr::auth::oidc::OidcSession;
///
/// # async fn example() {
/// let store = Arc::new(InMemoryOidcSessionStore::new(100, Duration::from_secs(300)));
/// let session = OidcSession {
///     state: "mystate".to_string(),
///     pkce_verifier: None,
///     nonce: "mynonce".to_string(),
///     redirect_to: None,
/// };
/// store.insert("mystate".to_string(), session, Duration::from_secs(60)).await.unwrap();
/// let result = store.take("mystate").await.unwrap();
/// assert!(result.is_some());
/// # }
/// ```
pub struct InMemoryOidcSessionStore {
    /// Session storage map protected by a read-write lock.
    sessions: Arc<RwLock<HashMap<String, StoredSession>>>,

    /// Maximum number of concurrently pending (non-expired) sessions.
    max_pending: usize,

    /// Default TTL used when callers do not supply their own duration.
    default_ttl: Duration,
}

impl InMemoryOidcSessionStore {
    /// Create a new in-memory session store.
    ///
    /// # Arguments
    ///
    /// * `max_pending` - Maximum number of concurrently pending sessions
    /// * `default_ttl` - Default TTL for sessions inserted via this store
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use xzepr::auth::oidc::session_store::InMemoryOidcSessionStore;
    ///
    /// let store = InMemoryOidcSessionStore::new(500, Duration::from_secs(300));
    /// assert_eq!(store.max_pending(), 500);
    /// ```
    pub fn new(max_pending: usize, default_ttl: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_pending,
            default_ttl,
        }
    }

    /// Return the default TTL configured for this store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use xzepr::auth::oidc::session_store::InMemoryOidcSessionStore;
    ///
    /// let store = InMemoryOidcSessionStore::new(100, Duration::from_secs(120));
    /// assert_eq!(store.default_ttl(), Duration::from_secs(120));
    /// ```
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }

    /// Return the maximum number of pending sessions allowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use xzepr::auth::oidc::session_store::InMemoryOidcSessionStore;
    ///
    /// let store = InMemoryOidcSessionStore::new(42, Duration::from_secs(60));
    /// assert_eq!(store.max_pending(), 42);
    /// ```
    pub fn max_pending(&self) -> usize {
        self.max_pending
    }

    /// Spawn a background Tokio task that periodically calls
    /// [`cleanup_expired`](OidcSessionStore::cleanup_expired).
    ///
    /// The task runs until the `Arc<Self>` is the last owner, at which point
    /// it will be dropped with the store. Removed session counts are logged at
    /// `DEBUG` level; errors are logged at `WARN` level.
    ///
    /// # Arguments
    ///
    /// * `interval` - How often to run the cleanup sweep
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use xzepr::auth::oidc::session_store::InMemoryOidcSessionStore;
    ///
    /// let store = Arc::new(InMemoryOidcSessionStore::new(100, Duration::from_secs(300)));
    /// store.clone().spawn_cleanup_task(Duration::from_secs(60));
    /// ```
    pub fn spawn_cleanup_task(self: Arc<Self>, interval: Duration) {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                match self.cleanup_expired().await {
                    Ok(removed) => {
                        if removed > 0 {
                            tracing::debug!(
                                removed,
                                "OIDC session cleanup removed expired sessions"
                            );
                        }
                    }
                    Err(err) => {
                        tracing::warn!("OIDC session cleanup failed: {}", err);
                    }
                }
            }
        });
    }
}

#[async_trait::async_trait]
impl OidcSessionStore for InMemoryOidcSessionStore {
    async fn insert(
        &self,
        state: String,
        session: OidcSession,
        ttl: Duration,
    ) -> Result<(), SessionStoreError> {
        let expires_at = Instant::now() + ttl;
        // Acquire write lock for the entire operation to avoid TOCTOU issues
        // between the capacity check and the actual insert.
        let mut sessions = self.sessions.write().await;

        let already_exists = sessions.contains_key(&state);
        if !already_exists {
            // Count only live sessions so expired-but-not-yet-cleaned-up
            // entries do not eat into the capacity limit.
            let now = Instant::now();
            let pending = sessions.values().filter(|s| s.expires_at >= now).count();
            if pending >= self.max_pending {
                return Err(SessionStoreError::CapacityExceeded(pending));
            }
        }

        sessions.insert(
            state,
            StoredSession {
                session,
                expires_at,
            },
        );
        Ok(())
    }

    async fn take(&self, state: &str) -> Result<Option<OidcSession>, SessionStoreError> {
        let mut sessions = self.sessions.write().await;
        match sessions.remove(state) {
            None => Ok(None),
            Some(stored) => {
                if stored.expires_at < Instant::now() {
                    // Entry has expired; discard without returning it.
                    Ok(None)
                } else {
                    Ok(Some(stored.session))
                }
            }
        }
    }

    async fn cleanup_expired(&self) -> Result<usize, SessionStoreError> {
        let now = Instant::now();
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        sessions.retain(|_, stored| stored.expires_at >= now);
        Ok(before - sessions.len())
    }

    async fn pending_count(&self) -> Result<usize, SessionStoreError> {
        let now = Instant::now();
        let sessions = self.sessions.read().await;
        Ok(sessions.values().filter(|s| s.expires_at >= now).count())
    }
}

// ---------------------------------------------------------------------------
// NullOidcSessionStore
// ---------------------------------------------------------------------------

/// A no-op session store for builds where OIDC is disabled or in tests that
/// do not require real session persistence.
///
/// All operations succeed immediately. `insert` discards the session, `take`
/// always returns `None`, and the count operations return zero.
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
/// use xzepr::auth::oidc::session_store::{NullOidcSessionStore, OidcSessionStore};
/// use xzepr::auth::oidc::OidcSession;
///
/// # async fn example() {
/// let store = NullOidcSessionStore;
/// let session = OidcSession {
///     state: "s".to_string(),
///     pkce_verifier: None,
///     nonce: "n".to_string(),
///     redirect_to: None,
/// };
/// store.insert("s".to_string(), session, Duration::from_secs(60)).await.unwrap();
/// let result = store.take("s").await.unwrap();
/// assert!(result.is_none());
/// # }
/// ```
pub struct NullOidcSessionStore;

#[async_trait::async_trait]
impl OidcSessionStore for NullOidcSessionStore {
    async fn insert(
        &self,
        _state: String,
        _session: OidcSession,
        _ttl: Duration,
    ) -> Result<(), SessionStoreError> {
        Ok(())
    }

    async fn take(&self, _state: &str) -> Result<Option<OidcSession>, SessionStoreError> {
        Ok(None)
    }

    async fn cleanup_expired(&self) -> Result<usize, SessionStoreError> {
        Ok(0)
    }

    async fn pending_count(&self) -> Result<usize, SessionStoreError> {
        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_session(state: &str) -> OidcSession {
        OidcSession {
            state: state.to_string(),
            pkce_verifier: Some("verifier".to_string()),
            nonce: "nonce".to_string(),
            redirect_to: None,
        }
    }

    fn make_store(max: usize) -> Arc<InMemoryOidcSessionStore> {
        Arc::new(InMemoryOidcSessionStore::new(max, Duration::from_secs(300)))
    }

    // -----------------------------------------------------------------------
    // InMemoryOidcSessionStore
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_insert_and_take_returns_session() {
        let store = make_store(10);
        let session = make_session("state1");

        store
            .insert("state1".to_string(), session, Duration::from_secs(60))
            .await
            .expect("insert should succeed");

        let result = store.take("state1").await.expect("take should succeed");
        assert!(result.is_some(), "session should be returned");

        let retrieved = result.unwrap();
        assert_eq!(retrieved.state, "state1");
        assert_eq!(retrieved.nonce, "nonce");
        assert_eq!(retrieved.pkce_verifier, Some("verifier".to_string()));
    }

    #[tokio::test]
    async fn test_take_expired_session_returns_none() {
        let store = make_store(10);
        let session = make_session("expired");

        // Insert with a TTL of 1 nanosecond so it expires almost immediately.
        store
            .insert("expired".to_string(), session, Duration::from_nanos(1))
            .await
            .expect("insert should succeed");

        // Sleep long enough for the entry to expire.
        tokio::time::sleep(Duration::from_millis(5)).await;

        let result = store.take("expired").await.expect("take should succeed");
        assert!(result.is_none(), "expired session should return None");
    }

    #[tokio::test]
    async fn test_take_missing_session_returns_none() {
        let store = make_store(10);

        let result = store
            .take("nonexistent")
            .await
            .expect("take should succeed for missing key");

        assert!(result.is_none(), "missing key should return None");
    }

    #[tokio::test]
    async fn test_capacity_exceeded_returns_error() {
        let store = make_store(2);

        store
            .insert(
                "s1".to_string(),
                make_session("s1"),
                Duration::from_secs(60),
            )
            .await
            .expect("first insert should succeed");

        store
            .insert(
                "s2".to_string(),
                make_session("s2"),
                Duration::from_secs(60),
            )
            .await
            .expect("second insert should succeed");

        let err = store
            .insert(
                "s3".to_string(),
                make_session("s3"),
                Duration::from_secs(60),
            )
            .await
            .expect_err("third insert must fail at capacity");

        assert!(
            matches!(err, SessionStoreError::CapacityExceeded(2)),
            "expected CapacityExceeded(2), got: {err}"
        );
    }

    #[tokio::test]
    async fn test_cleanup_expired_removes_stale_entries() {
        let store = make_store(10);

        store
            .insert(
                "live".to_string(),
                make_session("live"),
                Duration::from_secs(60),
            )
            .await
            .expect("insert live");

        store
            .insert(
                "stale".to_string(),
                make_session("stale"),
                Duration::from_nanos(1),
            )
            .await
            .expect("insert stale");

        tokio::time::sleep(Duration::from_millis(5)).await;

        let removed = store
            .cleanup_expired()
            .await
            .expect("cleanup should succeed");

        assert_eq!(removed, 1, "only the stale entry should be removed");

        // The live entry must still be retrievable after cleanup.
        let result = store.take("live").await.expect("take live after cleanup");
        assert!(result.is_some(), "live session must survive cleanup");
    }

    #[tokio::test]
    async fn test_pending_count_excludes_expired() {
        let store = make_store(10);

        store
            .insert("a".to_string(), make_session("a"), Duration::from_secs(60))
            .await
            .expect("insert a");

        store
            .insert("b".to_string(), make_session("b"), Duration::from_nanos(1))
            .await
            .expect("insert b");

        tokio::time::sleep(Duration::from_millis(5)).await;

        let count = store
            .pending_count()
            .await
            .expect("pending_count should succeed");

        assert_eq!(count, 1, "only the non-expired session should be counted");
    }

    #[tokio::test]
    async fn test_state_is_consumed_one_time() {
        let store = make_store(10);

        store
            .insert(
                "once".to_string(),
                make_session("once"),
                Duration::from_secs(60),
            )
            .await
            .expect("insert");

        let first = store.take("once").await.expect("first take");
        assert!(first.is_some(), "first take should return the session");

        let second = store.take("once").await.expect("second take");
        assert!(second.is_none(), "second take must return None (consumed)");
    }

    #[tokio::test]
    async fn test_overwrite_existing_state_does_not_require_capacity() {
        // A store at capacity should allow overwriting an existing key.
        let store = make_store(1);

        store
            .insert(
                "key".to_string(),
                make_session("key"),
                Duration::from_secs(60),
            )
            .await
            .expect("initial insert should succeed");

        // Overwrite the same key even though max_pending == 1.
        store
            .insert(
                "key".to_string(),
                make_session("key"),
                Duration::from_secs(120),
            )
            .await
            .expect("overwrite of existing key must succeed at capacity");
    }

    // -----------------------------------------------------------------------
    // NullOidcSessionStore
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_null_store_insert_and_take() {
        let store = NullOidcSessionStore;
        let session = make_session("s");

        store
            .insert("s".to_string(), session, Duration::from_secs(60))
            .await
            .expect("null insert should succeed");

        let result = store.take("s").await.expect("null take should succeed");
        assert!(result.is_none(), "null store always returns None from take");
    }

    #[tokio::test]
    async fn test_null_store_cleanup_returns_zero() {
        let store = NullOidcSessionStore;
        let removed = store
            .cleanup_expired()
            .await
            .expect("null cleanup should succeed");
        assert_eq!(removed, 0);
    }

    #[tokio::test]
    async fn test_null_store_pending_count_returns_zero() {
        let store = NullOidcSessionStore;
        let count = store
            .pending_count()
            .await
            .expect("null pending_count should succeed");
        assert_eq!(count, 0);
    }

    // -----------------------------------------------------------------------
    // SessionStoreError display
    // -----------------------------------------------------------------------

    #[test]
    fn test_session_store_error_backend_display() {
        let err = SessionStoreError::Backend("connection refused".to_string());
        assert!(
            err.to_string().contains("Storage backend error"),
            "Backend variant should include 'Storage backend error'"
        );
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_session_store_error_capacity_exceeded_display() {
        let err = SessionStoreError::CapacityExceeded(99);
        let msg = err.to_string();
        assert!(msg.contains("capacity"), "must mention capacity");
        assert!(msg.contains("99"), "must include the count");
    }

    // -----------------------------------------------------------------------
    // InMemoryOidcSessionStore accessors
    // -----------------------------------------------------------------------

    #[test]
    fn test_in_memory_store_default_ttl_accessor() {
        let store = InMemoryOidcSessionStore::new(10, Duration::from_secs(180));
        assert_eq!(store.default_ttl(), Duration::from_secs(180));
    }

    #[test]
    fn test_in_memory_store_max_pending_accessor() {
        let store = InMemoryOidcSessionStore::new(77, Duration::from_secs(60));
        assert_eq!(store.max_pending(), 77);
    }
}
