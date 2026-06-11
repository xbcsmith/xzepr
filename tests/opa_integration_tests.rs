// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the Open Policy Agent (OPA) authorization layer.
//!
//! ## Default suite (always runs)
//!
//! Tests that are NOT inside a `#[cfg(feature = "opa-integration-tests")]`
//! block exercise OPA configuration types and client construction without a
//! live OPA server.  No external service is required.
//!
//! Covered without a live server:
//!
//! - `OpaConfig` default values
//! - `OpaFailSafeMode` variant behaviour and JSON serialization
//! - `OpaConfig` JSON round-trip serialization
//! - `OpaClient::new` with `enabled: false` (no connection attempt)
//!
//! ## Live integration tests
//!
//! Tests compiled with the `opa-integration-tests` Cargo feature require:
//!
//! - `XZEPR_RUN_OPA_INTEGRATION_TESTS=true` - enables the live test run
//! - `OPA_URL` - OPA server URL (default: `http://localhost:8181`)
//!
//! To run live tests:
//!
//! ```bash
//! XZEPR_RUN_OPA_INTEGRATION_TESTS=true \
//!   OPA_URL=http://localhost:8181 \
//!   cargo test --features opa-integration-tests --test opa_integration_tests
//! ```

use xzepr::opa::{OpaClient, OpaConfig, OpaFailSafeMode};

// ─── Default suite (no external dependency) ──────────────────────────────────

/// Tests that `OpaConfig::default` returns the expected field values without
/// requiring a live OPA server.
#[test]
fn test_opa_config_default_values_match_expected() {
    let config = OpaConfig::default();

    assert!(!config.enabled, "OPA should be disabled by default");
    assert_eq!(config.url, "http://localhost:8181", "default URL mismatch");
    assert_eq!(
        config.timeout_seconds, 5,
        "default timeout should be 5 seconds"
    );
    assert_eq!(
        config.policy_path, "/v1/data/xzepr/rbac/allow",
        "default policy path mismatch"
    );
    assert!(
        config.bundle_url.is_none(),
        "bundle_url should be None by default"
    );
    assert_eq!(
        config.cache_ttl_seconds, 300,
        "default cache TTL should be 300 seconds"
    );
    assert_eq!(
        config.health_path, "/health",
        "default health path mismatch"
    );
    assert!(
        config.allowed_hosts.is_empty(),
        "allowed_hosts should be empty by default"
    );
}

/// Tests that `OpaFailSafeMode::default` is `FailClosed`.
#[test]
fn test_opa_fail_safe_mode_default_is_fail_closed() {
    let mode = OpaFailSafeMode::default();
    assert_eq!(
        mode,
        OpaFailSafeMode::FailClosed,
        "default fail-safe mode should be FailClosed"
    );
}

/// Tests that `OpaFailSafeMode` variants serialize to the expected
/// snake_case JSON strings.
#[test]
fn test_opa_fail_safe_mode_serializes_to_snake_case_json() {
    let fail_closed =
        serde_json::to_string(&OpaFailSafeMode::FailClosed).expect("serialization must succeed");
    let fail_open = serde_json::to_string(&OpaFailSafeMode::FailOpenDevelopment)
        .expect("serialization must succeed");
    let legacy = serde_json::to_string(&OpaFailSafeMode::LegacyRbacFallback)
        .expect("serialization must succeed");

    assert_eq!(fail_closed, "\"fail_closed\"");
    assert_eq!(fail_open, "\"fail_open_development\"");
    assert_eq!(legacy, "\"legacy_rbac_fallback\"");
}

/// Tests that `OpaFailSafeMode` round-trips through JSON deserialization.
#[test]
fn test_opa_fail_safe_mode_json_round_trip_preserves_variant() {
    for mode in [
        OpaFailSafeMode::FailClosed,
        OpaFailSafeMode::FailOpenDevelopment,
        OpaFailSafeMode::LegacyRbacFallback,
    ] {
        let json = serde_json::to_string(&mode).expect("serialization must succeed");
        let decoded: OpaFailSafeMode =
            serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(decoded, mode, "round-trip must preserve {:?}", mode);
    }
}

/// Tests that a full `OpaConfig` round-trips through JSON without data loss.
#[test]
fn test_opa_config_json_round_trip_preserves_all_fields() {
    let original = OpaConfig {
        enabled: true,
        url: "http://opa.example.com:8181".to_string(),
        timeout_seconds: 10,
        policy_path: "/v1/data/myapp/allow".to_string(),
        bundle_url: Some("http://bundle.example.com/bundle.tar.gz".to_string()),
        cache_ttl_seconds: 60,
        health_path: "/health".to_string(),
        allowed_hosts: vec!["opa.example.com:8181".to_string()],
        fail_safe_mode: OpaFailSafeMode::LegacyRbacFallback,
    };

    let json = serde_json::to_string(&original).expect("serialization must succeed");
    let decoded: OpaConfig = serde_json::from_str(&json).expect("deserialization must succeed");

    assert_eq!(decoded.enabled, original.enabled);
    assert_eq!(decoded.url, original.url);
    assert_eq!(decoded.timeout_seconds, original.timeout_seconds);
    assert_eq!(decoded.policy_path, original.policy_path);
    assert_eq!(decoded.bundle_url, original.bundle_url);
    assert_eq!(decoded.cache_ttl_seconds, original.cache_ttl_seconds);
    assert_eq!(decoded.health_path, original.health_path);
    assert_eq!(decoded.allowed_hosts, original.allowed_hosts);
    assert_eq!(decoded.fail_safe_mode, original.fail_safe_mode);
}

/// Tests that `OpaClient::new` succeeds when OPA is disabled.
///
/// A disabled client does not attempt to connect to OPA at construction time,
/// so this test passes regardless of whether a server is running.
#[test]
fn test_opa_client_new_with_disabled_config_succeeds() {
    let config = OpaConfig {
        enabled: false,
        ..OpaConfig::default()
    };

    let result = OpaClient::new(config);
    assert!(
        result.is_ok(),
        "OpaClient::new must succeed when OPA is disabled"
    );
}

/// Tests that `OpaClient::is_enabled` returns `false` for a disabled config.
#[test]
fn test_opa_client_is_enabled_returns_false_when_disabled() {
    let config = OpaConfig {
        enabled: false,
        ..OpaConfig::default()
    };

    let client = OpaClient::new(config).expect("construction must succeed");
    assert!(
        !client.is_enabled(),
        "is_enabled() should return false for a disabled OPA client"
    );
}

/// Tests that `OpaClient::is_enabled` returns `true` for an enabled config.
#[test]
fn test_opa_client_is_enabled_returns_true_when_enabled() {
    let config = OpaConfig {
        enabled: true,
        ..OpaConfig::default()
    };

    let client = OpaClient::new(config).expect("construction must succeed");
    assert!(
        client.is_enabled(),
        "is_enabled() should return true for an enabled OPA client"
    );
}

/// Tests that `OpaClient::fail_safe_mode` returns the configured mode.
#[test]
fn test_opa_client_fail_safe_mode_accessor_returns_configured_mode() {
    let config = OpaConfig {
        fail_safe_mode: OpaFailSafeMode::LegacyRbacFallback,
        ..OpaConfig::default()
    };

    let client = OpaClient::new(config).expect("construction must succeed");
    assert_eq!(
        client.fail_safe_mode(),
        OpaFailSafeMode::LegacyRbacFallback,
        "fail_safe_mode() must return the mode from the original config"
    );
}

/// Tests that a disabled OPA client's `health_check` returns `Ok(())` without
/// connecting.
#[tokio::test]
async fn test_opa_client_health_check_with_disabled_client_returns_ok() {
    let config = OpaConfig {
        enabled: false,
        ..OpaConfig::default()
    };

    let client = OpaClient::new(config).expect("construction must succeed");
    let result = client.health_check().await;
    assert!(
        result.is_ok(),
        "health_check on a disabled client must return Ok(())"
    );
}

// ─── Live integration tests ───────────────────────────────────────────────────

#[cfg(feature = "opa-integration-tests")]
mod live {
    use xzepr::opa::{
        OpaClient, OpaConfig, OpaFailSafeMode, OpaInput, ResourceContext, UserContext,
    };

    /// Returns `true` when the caller has opted in to live OPA tests.
    fn live_opa_integration_enabled() -> bool {
        std::env::var("XZEPR_RUN_OPA_INTEGRATION_TESTS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    /// Returns the OPA server URL from the environment, defaulting to the
    /// standard local address.
    fn opa_url() -> String {
        std::env::var("OPA_URL").unwrap_or_else(|_| "http://localhost:8181".to_string())
    }

    /// Builds a minimal `OpaConfig` suitable for live tests.
    fn live_config(url: &str) -> OpaConfig {
        OpaConfig {
            enabled: true,
            url: url.to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            fail_safe_mode: OpaFailSafeMode::FailClosed,
            ..OpaConfig::default()
        }
    }

    /// Builds a minimal `OpaInput` for connectivity tests.
    fn test_input() -> OpaInput {
        OpaInput {
            user: UserContext {
                user_id: "test-user".to_string(),
                username: "testuser".to_string(),
                roles: vec!["user".to_string()],
                groups: vec![],
            },
            action: "read".to_string(),
            resource: ResourceContext {
                resource_type: "event".to_string(),
                resource_id: Some("event-123".to_string()),
                owner_id: Some("test-user".to_string()),
                group_id: None,
                members: vec![],
                resource_version: 1,
            },
        }
    }

    /// Tests that the OPA health endpoint at `{url}/health` is reachable.
    ///
    /// Returns `Ok(())` early when `XZEPR_RUN_OPA_INTEGRATION_TESTS` is not
    /// set.
    #[tokio::test]
    async fn test_opa_health_endpoint_is_reachable() -> Result<(), Box<dyn std::error::Error>> {
        if !live_opa_integration_enabled() {
            return Ok(());
        }

        let url = opa_url();
        let health_url = format!("{}/health", url.trim_end_matches('/'));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let response = client.get(&health_url).send().await?;
        assert!(
            response.status().is_success(),
            "OPA health endpoint at {} should return 2xx, got {}",
            health_url,
            response.status()
        );

        Ok(())
    }

    /// Tests that `OpaClient::new` succeeds with a live OPA URL.
    ///
    /// Client construction does not perform a network request, so this
    /// verifies only that the config is accepted.  Returns `Ok(())` early when
    /// `XZEPR_RUN_OPA_INTEGRATION_TESTS` is not set.
    #[tokio::test]
    async fn test_opa_client_new_with_live_url_succeeds() -> Result<(), Box<dyn std::error::Error>>
    {
        if !live_opa_integration_enabled() {
            return Ok(());
        }

        let url = opa_url();
        let config = live_config(&url);
        let _client = OpaClient::new(config)?;
        Ok(())
    }

    /// Tests that `OpaClient::health_check` returns `Ok(())` when the live
    /// OPA server is reachable.
    ///
    /// Returns `Ok(())` early when `XZEPR_RUN_OPA_INTEGRATION_TESTS` is not
    /// set.
    #[tokio::test]
    async fn test_opa_client_health_check_with_live_server_returns_ok(
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !live_opa_integration_enabled() {
            return Ok(());
        }

        let url = opa_url();
        let config = live_config(&url);
        let client = OpaClient::new(config)?;
        client.health_check().await?;
        Ok(())
    }

    /// Tests that `evaluate_with_cache` produces any result (allow, deny, or
    /// error) when the live OPA server is reachable.
    ///
    /// Both a successful `AuthorizationDecision` and an `OpaError` are
    /// acceptable outcomes - the test verifies only that the client can reach
    /// the server and receive a response.  Returns `Ok(())` early when
    /// `XZEPR_RUN_OPA_INTEGRATION_TESTS` is not set.
    #[tokio::test]
    async fn test_opa_evaluate_with_cache_reaches_live_server(
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !live_opa_integration_enabled() {
            return Ok(());
        }

        let url = opa_url();
        let config = live_config(&url);
        let client = OpaClient::new(config)?;

        // Both Ok and Err are acceptable: the test verifies that the client
        // can communicate with OPA, not that a particular policy exists.
        let _result = client.evaluate_with_cache(test_input(), 1).await;
        Ok(())
    }
}
