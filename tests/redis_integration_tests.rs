// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Redis rate-limiting infrastructure.
//!
//! ## Default suite (always runs)
//!
//! Tests that are NOT inside a `#[cfg(feature = "redis-integration-tests")]`
//! block exercise `RateLimitSecurityConfig` with non-live assertions: config
//! construction, default value validation, and field-level invariants.  No
//! Redis instance is required.
//!
//! ## Live integration tests
//!
//! Tests compiled with the `redis-integration-tests` Cargo feature require:
//!
//! - `XZEPR_RUN_REDIS_INTEGRATION_TESTS=true` - enables the live test run
//! - `REDIS_URL` - Redis connection URL (default: `redis://127.0.0.1:6379`)
//!
//! To run live tests:
//!
//! ```bash
//! XZEPR_RUN_REDIS_INTEGRATION_TESTS=true \
//!   REDIS_URL=redis://localhost:6379 \
//!   cargo test --features redis-integration-tests --test redis_integration_tests
//! ```

use xzepr::infrastructure::RateLimitSecurityConfig;

// ─── Default suite (no external dependency) ──────────────────────────────────

/// Tests that `RateLimitSecurityConfig::default` returns the expected
/// anonymous, authenticated, and admin request-per-minute values.
#[test]
fn test_rate_limit_security_config_default_values_match_expected() {
    let config = RateLimitSecurityConfig::default();

    assert_eq!(config.anonymous_rpm, 10);
    assert_eq!(config.authenticated_rpm, 100);
    assert_eq!(config.admin_rpm, 1000);
    assert!(
        !config.use_redis,
        "Redis backend should be disabled by default"
    );
    assert!(
        config.redis_url.is_none(),
        "Redis URL should be None by default"
    );
    assert!(
        config.per_endpoint.is_empty(),
        "Per-endpoint limits should be empty by default"
    );
}

/// Tests that a `RateLimitSecurityConfig` can be constructed with Redis
/// explicitly enabled and a URL supplied.
#[test]
fn test_rate_limit_security_config_with_redis_enabled_stores_url() {
    let config = RateLimitSecurityConfig {
        use_redis: true,
        redis_url: Some("redis://localhost:6379".to_string()),
        ..RateLimitSecurityConfig::default()
    };

    assert!(
        config.use_redis,
        "use_redis should be true when explicitly set"
    );
    assert_eq!(
        config.redis_url.as_deref(),
        Some("redis://localhost:6379"),
        "redis_url should match the supplied value"
    );
}

/// Tests that per-endpoint rate-limit overrides are stored and retrieved
/// correctly.
#[test]
fn test_rate_limit_security_config_per_endpoint_override_stores_values() {
    let mut config = RateLimitSecurityConfig::default();
    config.per_endpoint.insert("/api/v1/events".to_string(), 50);
    config.per_endpoint.insert("/api/v1/admin".to_string(), 200);

    assert_eq!(
        config.per_endpoint.get("/api/v1/events"),
        Some(&50),
        "events endpoint limit should be 50"
    );
    assert_eq!(
        config.per_endpoint.get("/api/v1/admin"),
        Some(&200),
        "admin endpoint limit should be 200"
    );
    assert_eq!(
        config.per_endpoint.get("/unknown"),
        None,
        "unknown endpoint should return None"
    );
}

/// Tests that all default rate limits are strictly positive.
#[test]
fn test_rate_limit_security_config_default_rpm_values_are_positive() {
    let config = RateLimitSecurityConfig::default();

    assert!(config.anonymous_rpm > 0, "anonymous_rpm must be positive");
    assert!(
        config.authenticated_rpm > 0,
        "authenticated_rpm must be positive"
    );
    assert!(config.admin_rpm > 0, "admin_rpm must be positive");
}

/// Tests that the default rate hierarchy is: admin > authenticated > anonymous.
#[test]
fn test_rate_limit_security_config_default_rate_hierarchy_is_ascending() {
    let config = RateLimitSecurityConfig::default();

    assert!(
        config.admin_rpm > config.authenticated_rpm,
        "admin_rpm ({}) should exceed authenticated_rpm ({})",
        config.admin_rpm,
        config.authenticated_rpm
    );
    assert!(
        config.authenticated_rpm > config.anonymous_rpm,
        "authenticated_rpm ({}) should exceed anonymous_rpm ({})",
        config.authenticated_rpm,
        config.anonymous_rpm
    );
}

/// Tests that Redis can be disabled after being enabled by overwriting the
/// `use_redis` field.
#[test]
fn test_rate_limit_security_config_redis_disabled_clears_use_redis_flag() {
    let mut config = RateLimitSecurityConfig {
        use_redis: true,
        redis_url: Some("redis://localhost:6379".to_string()),
        ..RateLimitSecurityConfig::default()
    };
    config.use_redis = false;

    assert!(
        !config.use_redis,
        "use_redis should be false after disabling"
    );
}

// ─── Live integration tests ───────────────────────────────────────────────────

#[cfg(feature = "redis-integration-tests")]
mod live {
    /// Returns `true` when the caller has opted in to live Redis tests.
    fn live_redis_integration_enabled() -> bool {
        std::env::var("XZEPR_RUN_REDIS_INTEGRATION_TESTS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    /// Returns the Redis URL from the environment, defaulting to the loopback
    /// address.
    fn redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
    }

    /// Tests that `redis::Client::open` succeeds for the configured URL.
    ///
    /// Returns `Ok(())` early when `XZEPR_RUN_REDIS_INTEGRATION_TESTS` is not
    /// set to `true` or `1`.
    #[tokio::test]
    async fn test_redis_client_open_with_configured_url_succeeds(
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !live_redis_integration_enabled() {
            return Ok(());
        }

        let url = redis_url();
        let _client = redis::Client::open(url.as_str())?;
        Ok(())
    }

    /// Tests connectivity to a live Redis server by issuing a PING command.
    ///
    /// Returns `Ok(())` early when `XZEPR_RUN_REDIS_INTEGRATION_TESTS` is not
    /// set.
    #[tokio::test]
    async fn test_redis_ping_returns_pong() -> Result<(), Box<dyn std::error::Error>> {
        if !live_redis_integration_enabled() {
            return Ok(());
        }

        let url = redis_url();
        let client = redis::Client::open(url.as_str())?;
        let mut conn = client.get_connection_manager().await?;
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        assert_eq!(pong, "PONG", "PING must return PONG");
        Ok(())
    }

    /// Tests a SET/GET round trip through a live Redis `ConnectionManager`.
    ///
    /// Writes a value under a test key, reads it back, and cleans up the key
    /// before returning.  Returns `Ok(())` early when
    /// `XZEPR_RUN_REDIS_INTEGRATION_TESTS` is not set.
    #[tokio::test]
    async fn test_redis_set_get_round_trip_returns_stored_value(
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !live_redis_integration_enabled() {
            return Ok(());
        }

        use redis::AsyncCommands;

        let url = redis_url();
        let client = redis::Client::open(url.as_str())?;
        let mut conn = client.get_connection_manager().await?;
        let key = "xzepr:test:set_get_round_trip";
        let expected = "integration_test_value";

        let _: () = conn.set(key, expected).await?;
        let actual: String = conn.get(key).await?;
        let _: () = conn.del(key).await?;

        assert_eq!(
            actual, expected,
            "retrieved value must match what was stored"
        );
        Ok(())
    }

    /// Tests that a `redis::aio::ConnectionManager` can be constructed for the
    /// configured URL.
    ///
    /// This validates that the connection infrastructure required by the
    /// `RedisRateLimitStore` (an internal implementation detail of the
    /// rate-limiting middleware, not part of the public API surface) is
    /// reachable.  Returns `Ok(())` early when
    /// `XZEPR_RUN_REDIS_INTEGRATION_TESTS` is not set.
    #[tokio::test]
    async fn test_redis_connection_manager_construction_succeeds(
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !live_redis_integration_enabled() {
            return Ok(());
        }

        let url = redis_url();
        let client = redis::Client::open(url.as_str())?;
        let _manager = client.get_connection_manager().await?;
        Ok(())
    }
}
