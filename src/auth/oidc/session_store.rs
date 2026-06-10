// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OIDC Session Store
//!
//! This module provides session storage for OIDC authentication flows.
//! Sessions are keyed by the `state` parameter issued during the authorization
//! request and consumed exactly once during the callback.
//!
//! Three implementations are provided:
//!
//! - [`InMemoryOidcSessionStore`]: suitable for development and single-node deployments.
//! - [`RedisOidcSessionStore`]: distributed storage suitable for production.
//! - [`NullOidcSessionStore`]: no-op implementation for OIDC-disabled builds
//!   and unit tests that do not require real session persistence.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

    /// A principal has reached the configured pending session limit.
    #[error("Principal '{principal}' has {pending} pending sessions; maximum is {max}")]
    PrincipalLimitExceeded {
        /// Principal key used for limit tracking.
        principal: String,
        /// Current pending session count for the principal.
        pending: usize,
        /// Maximum pending sessions allowed for the principal.
        max: usize,
    },
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Result of consuming an OIDC session by state.
#[derive(Debug, Clone)]
pub enum OidcSessionTakeResult {
    /// A live session was found and consumed.
    Found(OidcSession),
    /// No session state was found.
    Missing,
    /// A previously issued session state was found but had expired.
    Expired,
}

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

    /// Remove and classify the session state for `state`.
    ///
    /// Backends that can distinguish expired states from never-issued states
    /// should override this method. The default implementation preserves
    /// backwards-compatible behavior by treating `None` from [`take`](Self::take)
    /// as [`OidcSessionTakeResult::Missing`].
    ///
    /// # Arguments
    ///
    /// * `state` - State key to look up
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError::Backend`] on storage failure.
    async fn take_with_status(
        &self,
        state: &str,
    ) -> Result<OidcSessionTakeResult, SessionStoreError> {
        match self.take(state).await? {
            Some(session) => Ok(OidcSessionTakeResult::Found(session)),
            None => Ok(OidcSessionTakeResult::Missing),
        }
    }

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

fn session_principal(session: &OidcSession) -> String {
    session
        .session_principal
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("anonymous")
        .to_string()
}

fn unix_timestamp_millis() -> Result<u64, SessionStoreError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
    u64::try_from(elapsed.as_millis()).map_err(|_| {
        SessionStoreError::Backend("system timestamp exceeds u64 milliseconds".to_string())
    })
}

fn duration_millis(ttl: Duration) -> u64 {
    let millis = ttl.as_millis().max(1);
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn redis_session_key(key_prefix: &str, state: &str) -> String {
    format!("{key_prefix}:session:{state}")
}

fn redis_state_principal_key(key_prefix: &str, state: &str) -> String {
    format!("{key_prefix}:session_principal:{state}")
}

fn redis_principal_key(key_prefix: &str, principal: &str) -> String {
    format!("{key_prefix}:principal_index:{principal}")
}

fn redis_session_index_key(key_prefix: &str) -> String {
    format!("{key_prefix}:session_index")
}

fn redis_principals_key(key_prefix: &str) -> String {
    format!("{key_prefix}:principals")
}

fn classify_missing_redis_take(
    principal_hint_present: bool,
    index_score_present: bool,
) -> OidcSessionTakeResult {
    if principal_hint_present || index_score_present {
        OidcSessionTakeResult::Expired
    } else {
        OidcSessionTakeResult::Missing
    }
}

const REDIS_INSERT_SCRIPT: &str = r#"
redis.call('ZREMRANGEBYSCORE', KEYS[3], '-inf', ARGV[6])

local state = ARGV[1]
local principal = ARGV[2]
local payload = ARGV[3]
local ttl_millis = ARGV[4]
local expires_at_millis = ARGV[5]
local max_pending = tonumber(ARGV[7])
local key_prefix = ARGV[8]
local existing_principal = redis.call('GET', KEYS[2])
local existing_live = redis.call('EXISTS', KEYS[1])
local principal_has_state = redis.call('ZSCORE', KEYS[3], state)

if existing_live == 0 or not principal_has_state or (existing_principal and existing_principal ~= principal) then
    local pending = redis.call('ZCARD', KEYS[3])
    if pending >= max_pending then
        return {0, pending}
    end
end

if existing_principal and existing_principal ~= principal then
    redis.call('ZREM', key_prefix .. ':principal_index:' .. existing_principal, state)
end

redis.call('SET', KEYS[1], payload, 'PX', ttl_millis)
redis.call('SET', KEYS[2], principal, 'PX', ttl_millis)
redis.call('ZADD', KEYS[3], expires_at_millis, state)
redis.call('ZADD', KEYS[4], expires_at_millis, state)
redis.call('SADD', KEYS[5], principal)

return {1, redis.call('ZCARD', KEYS[3])}
"#;

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
///     session_principal: Some("alice@example.com".to_string()),
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

    /// Maximum pending sessions per principal hint.
    max_pending_per_principal: usize,

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
        Self::new_with_principal_limit(max_pending, max_pending, default_ttl)
    }

    /// Create a new in-memory session store with a per-principal limit.
    ///
    /// # Arguments
    ///
    /// * `max_pending` - Maximum number of concurrently pending sessions
    /// * `max_pending_per_principal` - Maximum pending sessions per principal hint
    /// * `default_ttl` - Default TTL for sessions inserted via this store
    pub fn new_with_principal_limit(
        max_pending: usize,
        max_pending_per_principal: usize,
        default_ttl: Duration,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_pending,
            max_pending_per_principal,
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

            let principal = session_principal(&session);
            let principal_pending = sessions
                .values()
                .filter(|stored| {
                    stored.expires_at >= now && session_principal(&stored.session) == principal
                })
                .count();
            if principal_pending >= self.max_pending_per_principal {
                return Err(SessionStoreError::PrincipalLimitExceeded {
                    principal,
                    pending: principal_pending,
                    max: self.max_pending_per_principal,
                });
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
        match self.take_with_status(state).await? {
            OidcSessionTakeResult::Found(session) => Ok(Some(session)),
            OidcSessionTakeResult::Missing | OidcSessionTakeResult::Expired => Ok(None),
        }
    }

    async fn take_with_status(
        &self,
        state: &str,
    ) -> Result<OidcSessionTakeResult, SessionStoreError> {
        let mut sessions = self.sessions.write().await;
        match sessions.remove(state) {
            None => Ok(OidcSessionTakeResult::Missing),
            Some(stored) => {
                if stored.expires_at < Instant::now() {
                    // Entry has expired; discard without returning it.
                    Ok(OidcSessionTakeResult::Expired)
                } else {
                    Ok(OidcSessionTakeResult::Found(stored.session))
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
// RedisOidcSessionStore
// ---------------------------------------------------------------------------

/// Redis-backed OIDC session store for multi-instance production deployments.
///
/// Session payloads are stored with Redis key TTLs. State consumption uses
/// `GETDEL`, so callbacks consume state exactly once across all application
/// instances sharing the same Redis database.
pub struct RedisOidcSessionStore {
    /// Redis connection manager.
    manager: redis::aio::ConnectionManager,
    /// Prefix for all keys created by this store.
    key_prefix: String,
    /// Maximum pending sessions per principal hint.
    max_pending_per_principal: usize,
}

impl RedisOidcSessionStore {
    /// Create a Redis-backed OIDC session store.
    ///
    /// # Arguments
    ///
    /// * `redis_url` - Redis connection URL
    /// * `key_prefix` - Prefix for session keys
    /// * `max_pending_per_principal` - Maximum pending sessions per principal hint
    ///
    /// # Errors
    ///
    /// Returns [`SessionStoreError::Backend`] if the Redis client or connection
    /// manager cannot be created.
    pub async fn new(
        redis_url: &str,
        key_prefix: impl Into<String>,
        max_pending_per_principal: usize,
    ) -> Result<Self, SessionStoreError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
        let manager = client
            .get_connection_manager()
            .await
            .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
        Ok(Self {
            manager,
            key_prefix: key_prefix.into(),
            max_pending_per_principal,
        })
    }

    fn session_key(&self, state: &str) -> String {
        redis_session_key(&self.key_prefix, state)
    }

    fn state_principal_key(&self, state: &str) -> String {
        redis_state_principal_key(&self.key_prefix, state)
    }

    fn principal_key(&self, principal: &str) -> String {
        redis_principal_key(&self.key_prefix, principal)
    }

    fn session_index_key(&self) -> String {
        redis_session_index_key(&self.key_prefix)
    }

    fn principals_key(&self) -> String {
        redis_principals_key(&self.key_prefix)
    }

    async fn remove_index_entries(
        &self,
        conn: &mut redis::aio::ConnectionManager,
        state: &str,
        principal: Option<&str>,
    ) -> Result<(), SessionStoreError> {
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("ZREM")
            .arg(self.session_index_key())
            .arg(state)
            .ignore();

        if let Some(principal) = principal {
            pipe.cmd("ZREM")
                .arg(self.principal_key(principal))
                .arg(state)
                .ignore();
        }

        let _: () = pipe
            .query_async(conn)
            .await
            .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl OidcSessionStore for RedisOidcSessionStore {
    async fn insert(
        &self,
        state: String,
        session: OidcSession,
        ttl: Duration,
    ) -> Result<(), SessionStoreError> {
        let principal = session_principal(&session);
        let mut conn = self.manager.clone();
        let payload = serde_json::to_string(&session)
            .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
        let ttl_millis = duration_millis(ttl);
        let now_millis = unix_timestamp_millis()?;
        let expires_at_millis = now_millis.saturating_add(ttl_millis);
        let session_key = self.session_key(&state);
        let state_principal_key = self.state_principal_key(&state);
        let principal_key = self.principal_key(&principal);
        let session_index_key = self.session_index_key();
        let principals_key = self.principals_key();

        let (accepted, pending): (i64, usize) = redis::Script::new(REDIS_INSERT_SCRIPT)
            .key(&session_key)
            .key(&state_principal_key)
            .key(&principal_key)
            .key(&session_index_key)
            .key(&principals_key)
            .arg(&state)
            .arg(&principal)
            .arg(payload)
            .arg(ttl_millis)
            .arg(expires_at_millis)
            .arg(now_millis)
            .arg(self.max_pending_per_principal)
            .arg(&self.key_prefix)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| SessionStoreError::Backend(e.to_string()))?;

        if accepted == 0 {
            return Err(SessionStoreError::PrincipalLimitExceeded {
                principal,
                pending,
                max: self.max_pending_per_principal,
            });
        }

        Ok(())
    }

    async fn take(&self, state: &str) -> Result<Option<OidcSession>, SessionStoreError> {
        match self.take_with_status(state).await? {
            OidcSessionTakeResult::Found(session) => Ok(Some(session)),
            OidcSessionTakeResult::Missing | OidcSessionTakeResult::Expired => Ok(None),
        }
    }

    async fn take_with_status(
        &self,
        state: &str,
    ) -> Result<OidcSessionTakeResult, SessionStoreError> {
        let mut conn = self.manager.clone();
        let session_key = self.session_key(state);
        let state_principal_key = self.state_principal_key(state);
        let session_index_key = self.session_index_key();
        let (payload, principal_hint, index_score): (Option<String>, Option<String>, Option<f64>) =
            redis::pipe()
                .atomic()
                .cmd("GETDEL")
                .arg(&session_key)
                .cmd("GETDEL")
                .arg(&state_principal_key)
                .cmd("ZSCORE")
                .arg(&session_index_key)
                .arg(state)
                .query_async(&mut conn)
                .await
                .map_err(|e| SessionStoreError::Backend(e.to_string()))?;

        let Some(payload) = payload else {
            self.remove_index_entries(&mut conn, state, principal_hint.as_deref())
                .await?;
            return Ok(classify_missing_redis_take(
                principal_hint.is_some(),
                index_score.is_some(),
            ));
        };

        let session: OidcSession = match serde_json::from_str(&payload) {
            Ok(session) => session,
            Err(err) => {
                self.remove_index_entries(&mut conn, state, principal_hint.as_deref())
                    .await?;
                return Err(SessionStoreError::Backend(err.to_string()));
            }
        };
        let principal = principal_hint.unwrap_or_else(|| session_principal(&session));
        self.remove_index_entries(&mut conn, state, Some(&principal))
            .await?;

        Ok(OidcSessionTakeResult::Found(session))
    }

    async fn cleanup_expired(&self) -> Result<usize, SessionStoreError> {
        let mut conn = self.manager.clone();
        let now_millis = unix_timestamp_millis()?;
        let expired_sessions: usize = redis::cmd("ZREMRANGEBYSCORE")
            .arg(self.session_index_key())
            .arg("-inf")
            .arg(now_millis)
            .query_async(&mut conn)
            .await
            .map_err(|e| SessionStoreError::Backend(e.to_string()))?;

        let principals_key = self.principals_key();
        let mut cursor = 0_u64;
        loop {
            let (next_cursor, principals): (u64, Vec<String>) = redis::cmd("SSCAN")
                .arg(&principals_key)
                .arg(cursor)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| SessionStoreError::Backend(e.to_string()))?;

            for principal in principals {
                let principal_key = self.principal_key(&principal);
                let _: usize = redis::cmd("ZREMRANGEBYSCORE")
                    .arg(&principal_key)
                    .arg("-inf")
                    .arg(now_millis)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
                let remaining: usize = redis::cmd("ZCARD")
                    .arg(&principal_key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| SessionStoreError::Backend(e.to_string()))?;

                if remaining == 0 {
                    let _: () = redis::pipe()
                        .atomic()
                        .cmd("DEL")
                        .arg(&principal_key)
                        .ignore()
                        .cmd("SREM")
                        .arg(&principals_key)
                        .arg(&principal)
                        .ignore()
                        .query_async(&mut conn)
                        .await
                        .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
                }
            }

            if next_cursor == 0 {
                break;
            }
            cursor = next_cursor;
        }

        Ok(expired_sessions)
    }

    async fn pending_count(&self) -> Result<usize, SessionStoreError> {
        let mut conn = self.manager.clone();
        let now_millis = unix_timestamp_millis()?;
        let session_index_key = self.session_index_key();
        let _: usize = redis::cmd("ZREMRANGEBYSCORE")
            .arg(&session_index_key)
            .arg("-inf")
            .arg(now_millis)
            .query_async(&mut conn)
            .await
            .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
        let count: usize = redis::cmd("ZCARD")
            .arg(session_index_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| SessionStoreError::Backend(e.to_string()))?;
        Ok(count)
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
///     session_principal: None,
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
        make_session_for_principal(state, "alice@example.com")
    }

    fn make_session_for_principal(state: &str, principal: &str) -> OidcSession {
        OidcSession {
            state: state.to_string(),
            pkce_verifier: Some("verifier".to_string()),
            nonce: "nonce".to_string(),
            redirect_to: None,
            session_principal: Some(principal.to_string()),
        }
    }

    fn make_store(max: usize) -> Arc<InMemoryOidcSessionStore> {
        Arc::new(InMemoryOidcSessionStore::new(max, Duration::from_secs(300)))
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct RedisSessionRefFixture {
        state: String,
        expires_at_millis: u64,
    }

    fn prune_expired_redis_session_refs(
        refs: &mut Vec<RedisSessionRefFixture>,
        now_millis: u64,
    ) -> usize {
        let before = refs.len();
        refs.retain(|session_ref| session_ref.expires_at_millis > now_millis);
        before - refs.len()
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
    async fn test_take_with_status_expired_session_returns_expired() {
        let store = make_store(10);
        let session = make_session("expired-status");

        store
            .insert(
                "expired-status".to_string(),
                session,
                Duration::from_nanos(1),
            )
            .await
            .expect("insert should succeed");

        tokio::time::sleep(Duration::from_millis(5)).await;

        let result = store
            .take_with_status("expired-status")
            .await
            .expect("take_with_status should succeed");
        assert!(matches!(result, OidcSessionTakeResult::Expired));
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
    async fn test_take_with_status_missing_session_returns_missing() {
        let store = make_store(10);

        let result = store
            .take_with_status("nonexistent-status")
            .await
            .expect("take_with_status should succeed for missing key");

        assert!(matches!(result, OidcSessionTakeResult::Missing));
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
    async fn test_per_principal_limit_returns_error() {
        let store = Arc::new(InMemoryOidcSessionStore::new_with_principal_limit(
            10,
            1,
            Duration::from_secs(300),
        ));

        store
            .insert(
                "s1".to_string(),
                make_session_for_principal("s1", "alice@example.com"),
                Duration::from_secs(60),
            )
            .await
            .expect("first principal session should succeed");

        let err = store
            .insert(
                "s2".to_string(),
                make_session_for_principal("s2", "alice@example.com"),
                Duration::from_secs(60),
            )
            .await
            .expect_err("second session for same principal must fail");

        assert!(matches!(
            err,
            SessionStoreError::PrincipalLimitExceeded { ref principal, pending: 1, max: 1 }
                if principal == "alice@example.com"
        ));
    }

    #[tokio::test]
    async fn test_per_principal_limit_allows_different_principals() {
        let store = Arc::new(InMemoryOidcSessionStore::new_with_principal_limit(
            10,
            1,
            Duration::from_secs(300),
        ));

        store
            .insert(
                "s1".to_string(),
                make_session_for_principal("s1", "alice@example.com"),
                Duration::from_secs(60),
            )
            .await
            .expect("first principal session should succeed");
        store
            .insert(
                "s2".to_string(),
                make_session_for_principal("s2", "bob@example.com"),
                Duration::from_secs(60),
            )
            .await
            .expect("different principal should not consume alice limit");
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
    // RedisOidcSessionStore helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_redis_take_with_status_uses_session_index_to_classify_expired_states() {
        assert!(matches!(
            classify_missing_redis_take(false, true),
            OidcSessionTakeResult::Expired
        ));
        assert!(matches!(
            classify_missing_redis_take(true, false),
            OidcSessionTakeResult::Expired
        ));
        assert!(matches!(
            classify_missing_redis_take(false, false),
            OidcSessionTakeResult::Missing
        ));
    }

    #[test]
    fn test_redis_insert_script_prunes_stale_principal_refs_before_counting_limit() {
        let prune_position = REDIS_INSERT_SCRIPT.find("ZREMRANGEBYSCORE");
        let count_position = REDIS_INSERT_SCRIPT.find("ZCARD");

        assert!(
            prune_position.is_some(),
            "Redis insert script must prune expired principal references"
        );
        assert!(
            count_position.is_some(),
            "Redis insert script must count principal references"
        );
        if let (Some(prune_position), Some(count_position)) = (prune_position, count_position) {
            assert!(
                prune_position < count_position,
                "stale principal references must be pruned before enforcing the limit"
            );
        }
        assert!(
            REDIS_INSERT_SCRIPT.contains("pending >= max_pending"),
            "Redis insert script must enforce the per-principal limit after pruning"
        );
    }

    #[test]
    fn test_redis_stale_principal_refs_do_not_block_limit_after_pruning() {
        let mut principal_refs = vec![
            RedisSessionRefFixture {
                state: "expired".to_string(),
                expires_at_millis: 100,
            },
            RedisSessionRefFixture {
                state: "live".to_string(),
                expires_at_millis: 300,
            },
        ];

        let removed = prune_expired_redis_session_refs(&mut principal_refs, 200);
        let live_count = principal_refs.len();

        assert_eq!(removed, 1, "one stale reference should be removed");
        assert_eq!(live_count, 1, "only live references should remain");
        assert!(
            live_count < 2,
            "one stale and one live reference must not block a limit of two"
        );
        assert!(
            principal_refs
                .iter()
                .all(|session_ref| session_ref.state != "expired"),
            "expired references must not remain in the principal index"
        );
    }

    #[test]
    fn test_redis_key_helpers_use_dedicated_index_namespaces() {
        let prefix = "xzepr:oidc:test";

        assert_eq!(
            redis_session_key(prefix, "session_index"),
            "xzepr:oidc:test:session:session_index"
        );
        assert_eq!(
            redis_session_index_key(prefix),
            "xzepr:oidc:test:session_index"
        );
        assert_ne!(
            redis_session_key(prefix, "session_index"),
            redis_session_index_key(prefix),
            "global session index must not collide with a session state key"
        );
        assert_eq!(
            redis_principal_key(prefix, "alice@example.com"),
            "xzepr:oidc:test:principal_index:alice@example.com"
        );
        assert_eq!(redis_principals_key(prefix), "xzepr:oidc:test:principals");
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
