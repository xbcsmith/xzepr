// src/api/middleware/rate_limit.rs

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use redis::{aio::ConnectionManager, Client};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::infrastructure::SecurityMonitor;

/// Rate limit configuration for different user tiers
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per minute for anonymous users
    pub anonymous_rpm: u32,
    /// Requests per minute for authenticated users
    pub authenticated_rpm: u32,
    /// Requests per minute for admin users
    pub admin_rpm: u32,
    /// Per-endpoint rate limits (endpoint path -> RPM)
    pub per_endpoint: HashMap<String, u32>,
    /// Window size for rate limiting
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            anonymous_rpm: 10,
            authenticated_rpm: 100,
            admin_rpm: 1000,
            per_endpoint: HashMap::new(),
            window_size: Duration::from_secs(60),
        }
    }
}

impl RateLimitConfig {
    /// Creates a new rate limit configuration from environment variables
    ///
    /// Environment variables:
    /// - XZEPR__SECURITY__RATE_LIMIT__ANONYMOUS_RPM
    /// - XZEPR__SECURITY__RATE_LIMIT__AUTHENTICATED_RPM
    /// - XZEPR__SECURITY__RATE_LIMIT__ADMIN_RPM
    pub fn from_env() -> Self {
        let anonymous_rpm = std::env::var("XZEPR__SECURITY__RATE_LIMIT__ANONYMOUS_RPM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let authenticated_rpm = std::env::var("XZEPR__SECURITY__RATE_LIMIT__AUTHENTICATED_RPM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let admin_rpm = std::env::var("XZEPR__SECURITY__RATE_LIMIT__ADMIN_RPM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000);

        Self {
            anonymous_rpm,
            authenticated_rpm,
            admin_rpm,
            per_endpoint: HashMap::new(),
            window_size: Duration::from_secs(60),
        }
    }

    /// Creates a permissive configuration for development
    pub fn permissive() -> Self {
        Self {
            anonymous_rpm: 10000,
            authenticated_rpm: 10000,
            admin_rpm: 10000,
            per_endpoint: HashMap::new(),
            window_size: Duration::from_secs(60),
        }
    }

    /// Adds a per-endpoint rate limit
    pub fn with_endpoint_limit(mut self, endpoint: impl Into<String>, rpm: u32) -> Self {
        self.per_endpoint.insert(endpoint.into(), rpm);
        self
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current number of tokens
    tokens: f64,
    /// Maximum number of tokens
    capacity: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    /// Creates a new token bucket
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Refills the bucket based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let tokens_to_add = elapsed * self.refill_rate;

        self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
        self.last_refill = now;
    }

    /// Tries to consume a token
    fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Gets the number of tokens remaining
    fn remaining(&mut self) -> u32 {
        self.refill();
        self.tokens.floor() as u32
    }

    /// Calculates time until next token is available
    fn time_until_refill(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let seconds = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(seconds)
        }
    }
}

/// Storage trait for rate limit state
#[async_trait::async_trait]
pub trait RateLimitStore: Send + Sync {
    /// Checks if the key is rate limited
    async fn check_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window: Duration,
    ) -> Result<RateLimitStatus, String>;
}

/// Rate limit status
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Total limit
    pub limit: u32,
    /// Remaining requests
    pub remaining: u32,
    /// Time until reset
    pub reset_after: Duration,
}

/// In-memory rate limit store using token buckets
#[derive(Clone)]
pub struct InMemoryRateLimitStore {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
}

impl InMemoryRateLimitStore {
    /// Creates a new in-memory rate limit store
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryRateLimitStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RateLimitStore for InMemoryRateLimitStore {
    async fn check_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window: Duration,
    ) -> Result<RateLimitStatus, String> {
        let mut buckets = self.buckets.write().await;

        let refill_rate = limit as f64 / window.as_secs_f64();

        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(limit as f64, refill_rate));

        let allowed = bucket.try_consume();
        let remaining = bucket.remaining();
        let reset_after = bucket.time_until_refill();

        Ok(RateLimitStatus {
            allowed,
            limit,
            remaining,
            reset_after,
        })
    }
}

/// Redis-backed rate limit store using Lua scripts
#[derive(Clone)]
pub struct RedisRateLimitStore {
    client: ConnectionManager,
}

impl RedisRateLimitStore {
    /// Creates a new Redis rate limit store
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self { client: manager })
    }

    /// Creates a new Redis rate limit store from a connection manager
    pub fn from_manager(client: ConnectionManager) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl RateLimitStore for RedisRateLimitStore {
    async fn check_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window: Duration,
    ) -> Result<RateLimitStatus, String> {
        let mut conn = self.client.clone();
        let redis_key = format!("ratelimit:{}", key);
        let window_secs = window.as_secs();

        // Lua script for atomic rate limiting using sliding window
        // Returns: [allowed (0/1), remaining, ttl]
        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local limit = tonumber(ARGV[1])
            local window = tonumber(ARGV[2])
            local now = tonumber(ARGV[3])

            -- Remove old entries outside the window
            redis.call('ZREMRANGEBYSCORE', key, 0, now - window)

            -- Count current requests
            local current = redis.call('ZCARD', key)

            if current < limit then
                -- Add this request
                redis.call('ZADD', key, now, now)
                redis.call('EXPIRE', key, window)
                return {1, limit - current - 1, window}
            else
                -- Get the oldest entry to calculate reset time
                local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
                local reset_at = tonumber(oldest[2]) + window
                local reset_after = math.max(0, reset_at - now)
                return {0, 0, reset_after}
            end
            "#,
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result: Vec<i64> = script
            .key(&redis_key)
            .arg(limit)
            .arg(window_secs)
            .arg(now)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| format!("Redis error: {}", e))?;

        let allowed = result[0] == 1;
        let remaining = result[1] as u32;
        let reset_after = Duration::from_secs(result[2] as u64);

        Ok(RateLimitStatus {
            allowed,
            limit,
            remaining,
            reset_after,
        })
    }
}

/// Rate limiter state
#[derive(Clone)]
pub struct RateLimiterState {
    config: Arc<RateLimitConfig>,
    store: Arc<dyn RateLimitStore>,
    monitor: Option<Arc<SecurityMonitor>>,
}

impl RateLimiterState {
    /// Creates a new rate limiter state
    pub fn new(config: RateLimitConfig, store: Arc<dyn RateLimitStore>) -> Self {
        Self {
            config: Arc::new(config),
            store,
            monitor: None,
        }
    }

    /// Creates a new rate limiter state with monitoring
    pub fn new_with_monitor(
        config: RateLimitConfig,
        store: Arc<dyn RateLimitStore>,
        monitor: Arc<SecurityMonitor>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            store,
            monitor: Some(monitor),
        }
    }

    /// Creates a default rate limiter with in-memory store
    pub fn default_with_config(config: RateLimitConfig) -> Self {
        Self::new(config, Arc::new(InMemoryRateLimitStore::new()))
    }

    /// Sets the security monitor
    pub fn with_monitor(mut self, monitor: Arc<SecurityMonitor>) -> Self {
        self.monitor = Some(monitor);
        self
    }
}

/// Extracts the rate limit key from the request
///
/// Priority:
/// 1. User ID (if authenticated)
/// 2. API Key hash (if using API key)
/// 3. IP address (anonymous)
fn extract_rate_limit_key(headers: &HeaderMap, ip: Option<IpAddr>) -> String {
    // TODO: Extract user ID from authenticated request
    // Check for X-User-Id header or JWT claims

    // Check for API key
    if let Some(api_key) = headers.get("x-api-key") {
        if let Ok(key_str) = api_key.to_str() {
            return format!("apikey:{}", key_str);
        }
    }

    // Fall back to IP address
    if let Some(ip) = ip {
        return format!("ip:{}", ip);
    }

    // Last resort: use a default key (not recommended for production)
    "anonymous".to_string()
}

/// Determines the rate limit for the request based on user tier
fn determine_rate_limit(config: &RateLimitConfig, path: &str, _headers: &HeaderMap) -> u32 {
    // Check for per-endpoint limit first
    if let Some(&limit) = config.per_endpoint.get(path) {
        return limit;
    }

    // TODO: Determine user tier from authentication
    // Check for admin role -> admin_rpm
    // Check for authenticated user -> authenticated_rpm
    // Default to anonymous_rpm

    config.anonymous_rpm
}

/// Rate limiting middleware
///
/// Applies token bucket rate limiting based on:
/// - User tier (anonymous, authenticated, admin)
/// - Per-endpoint limits
/// - IP address or user ID
///
/// Adds the following headers to responses:
/// - X-RateLimit-Limit: Total limit
/// - X-RateLimit-Remaining: Remaining requests
/// - X-RateLimit-Reset: Seconds until reset
pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiterState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    // TODO: Extract IP address from request
    // Use X-Forwarded-For or X-Real-IP if behind proxy
    let ip: Option<IpAddr> = None;

    let rate_limit_key = extract_rate_limit_key(&headers, ip);
    let limit = determine_rate_limit(&limiter.config, path, &headers);

    let status = limiter
        .store
        .check_rate_limit(&rate_limit_key, limit, limiter.config.window_size)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !status.allowed {
        tracing::warn!(
            key = %rate_limit_key,
            path = %path,
            limit = %limit,
            "Rate limit exceeded"
        );

        // Record rate limit rejection in security monitor
        if let Some(monitor) = &limiter.monitor {
            monitor.record_rate_limit_rejection(&rate_limit_key, path, limit);
        }

        let mut response = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(Body::from("Rate limit exceeded"))
            .unwrap();

        let headers = response.headers_mut();
        headers.insert("X-RateLimit-Limit", status.limit.into());
        headers.insert("X-RateLimit-Remaining", 0.into());
        headers.insert("X-RateLimit-Reset", status.reset_after.as_secs().into());
        headers.insert("Retry-After", status.reset_after.as_secs().into());

        return Ok(response);
    }

    let mut response = next.run(request).await;

    let headers = response.headers_mut();
    headers.insert("X-RateLimit-Limit", status.limit.into());
    headers.insert("X-RateLimit-Remaining", status.remaining.into());
    headers.insert("X-RateLimit-Reset", status.reset_after.as_secs().into());

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(10.0, 1.0);
        assert_eq!(bucket.capacity, 10.0);
        assert_eq!(bucket.tokens, 10.0);
    }

    #[test]
    fn test_token_bucket_consume() {
        let mut bucket = TokenBucket::new(10.0, 1.0);
        assert!(bucket.try_consume());
        assert_eq!(bucket.remaining(), 9);
    }

    #[test]
    fn test_token_bucket_empty() {
        let mut bucket = TokenBucket::new(1.0, 1.0);
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());
        assert_eq!(bucket.remaining(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_store() {
        let store = InMemoryRateLimitStore::new();
        let status = store
            .check_rate_limit("test", 10, Duration::from_secs(60))
            .await
            .unwrap();

        assert!(status.allowed);
        assert_eq!(status.limit, 10);
        assert_eq!(status.remaining, 9);
    }

    #[tokio::test]
    async fn test_rate_limit_enforcement() {
        let store = InMemoryRateLimitStore::new();

        // Consume all tokens
        for _ in 0..10 {
            let status = store
                .check_rate_limit("test", 10, Duration::from_secs(60))
                .await
                .unwrap();
            assert!(status.allowed);
        }

        // Next request should be denied
        let status = store
            .check_rate_limit("test", 10, Duration::from_secs(60))
            .await
            .unwrap();
        assert!(!status.allowed);
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("XZEPR__SECURITY__RATE_LIMIT__ANONYMOUS_RPM", "20");
        std::env::set_var("XZEPR__SECURITY__RATE_LIMIT__AUTHENTICATED_RPM", "200");
        std::env::set_var("XZEPR__SECURITY__RATE_LIMIT__ADMIN_RPM", "2000");

        let config = RateLimitConfig::from_env();
        assert_eq!(config.anonymous_rpm, 20);
        assert_eq!(config.authenticated_rpm, 200);
        assert_eq!(config.admin_rpm, 2000);
    }

    #[test]
    fn test_endpoint_specific_limits() {
        let config = RateLimitConfig::default()
            .with_endpoint_limit("/auth/login", 5)
            .with_endpoint_limit("/auth/register", 3);

        assert_eq!(config.per_endpoint.get("/auth/login"), Some(&5));
        assert_eq!(config.per_endpoint.get("/auth/register"), Some(&3));
    }
}
