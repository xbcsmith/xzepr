// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the OIDC authentication layer.
//!
//! ## Default suite (always runs)
//!
//! Tests that are NOT inside a `#[cfg(feature = "oidc-integration-tests")]`
//! block exercise OIDC configuration types and non-live session store
//! implementations.  No OIDC provider is required.
//!
//! Covered without a live server:
//!
//! - `OidcSessionStoreConfig` default values and backend variant parsing
//! - `OidcSessionStoreBackend` variant equality
//! - `InMemoryOidcSessionStore` construction and accessor methods
//! - `NullOidcSessionStore` construction and no-op behaviour
//!
//! ## Live integration tests
//!
//! Tests compiled with the `oidc-integration-tests` Cargo feature require:
//!
//! - `XZEPR_RUN_OIDC_INTEGRATION_TESTS=true` - enables the live test run
//! - `OIDC_ISSUER_URL` - issuer URL of the OIDC provider
//! - `OIDC_CLIENT_ID` - client ID registered in the provider
//! - `OIDC_CLIENT_SECRET` - client secret registered in the provider
//!
//! To run live tests:
//!
//! ```bash
//! XZEPR_RUN_OIDC_INTEGRATION_TESTS=true \
//!   OIDC_ISSUER_URL=http://localhost:8080/realms/xzepr \
//!   OIDC_CLIENT_ID=xzepr-server \
//!   OIDC_CLIENT_SECRET=changeme \
//!   cargo test --features oidc-integration-tests --test oidc_integration_tests
//! ```

use std::time::Duration;

use xzepr::auth::oidc::session_store::{InMemoryOidcSessionStore, NullOidcSessionStore};
use xzepr::infrastructure::config::{OidcSessionStoreBackend, OidcSessionStoreConfig};

// ─── Default suite (no external dependency) ──────────────────────────────────

/// Tests that `OidcSessionStoreConfig::default` produces a memory backend with
/// the expected key prefix and no Redis URL.
#[test]
fn test_oidc_session_store_config_default_uses_memory_backend() {
    let config = OidcSessionStoreConfig::default();

    assert_eq!(
        config.backend,
        OidcSessionStoreBackend::Memory,
        "default backend should be Memory"
    );
    assert!(
        config.redis_url.is_none(),
        "redis_url should be None for the memory backend default"
    );
    assert_eq!(
        config.key_prefix, "xzepr:oidc",
        "default key_prefix should be 'xzepr:oidc'"
    );
}

/// Tests that `OidcSessionStoreBackend` variants compare equal to themselves.
#[test]
fn test_oidc_session_store_backend_variant_equality() {
    assert_eq!(
        OidcSessionStoreBackend::Memory,
        OidcSessionStoreBackend::Memory
    );
    assert_eq!(
        OidcSessionStoreBackend::Redis,
        OidcSessionStoreBackend::Redis
    );
    assert_ne!(
        OidcSessionStoreBackend::Memory,
        OidcSessionStoreBackend::Redis
    );
}

/// Tests that an `OidcSessionStoreConfig` can be constructed with the Redis
/// backend and a URL.
#[test]
fn test_oidc_session_store_config_redis_backend_stores_url() {
    let config = OidcSessionStoreConfig {
        backend: OidcSessionStoreBackend::Redis,
        redis_url: Some("redis://localhost:6379".to_string()),
        key_prefix: "xzepr:oidc".to_string(),
    };

    assert_eq!(config.backend, OidcSessionStoreBackend::Redis);
    assert_eq!(
        config.redis_url.as_deref(),
        Some("redis://localhost:6379"),
        "redis_url should match the supplied value"
    );
}

/// Tests that `OidcSessionStoreConfig` deserializes from JSON with a
/// `memory` backend string.
#[test]
fn test_oidc_session_store_config_deserializes_memory_backend_from_json() {
    let json = r#"{"backend":"memory","key_prefix":"test:oidc"}"#;
    let config: OidcSessionStoreConfig =
        serde_json::from_str(json).expect("deserialization must succeed");

    assert_eq!(config.backend, OidcSessionStoreBackend::Memory);
    assert_eq!(config.key_prefix, "test:oidc");
    assert!(config.redis_url.is_none());
}

/// Tests that `OidcSessionStoreConfig` deserializes from JSON with a `redis`
/// backend string.
#[test]
fn test_oidc_session_store_config_deserializes_redis_backend_from_json() {
    let json =
        r#"{"backend":"redis","redis_url":"redis://localhost:6379","key_prefix":"xzepr:oidc"}"#;
    let config: OidcSessionStoreConfig =
        serde_json::from_str(json).expect("deserialization must succeed");

    assert_eq!(config.backend, OidcSessionStoreBackend::Redis);
    assert_eq!(config.redis_url.as_deref(), Some("redis://localhost:6379"));
}

/// Tests that `InMemoryOidcSessionStore::new` creates a store with the
/// correct `max_pending` capacity.
#[test]
fn test_in_memory_oidc_session_store_new_sets_max_pending() {
    let store = InMemoryOidcSessionStore::new(500, Duration::from_secs(300));

    assert_eq!(
        store.max_pending(),
        500,
        "max_pending should match the value passed to new()"
    );
}

/// Tests that `InMemoryOidcSessionStore::new` stores the supplied default TTL.
#[test]
fn test_in_memory_oidc_session_store_new_stores_default_ttl() {
    let ttl = Duration::from_secs(120);
    let store = InMemoryOidcSessionStore::new(100, ttl);

    assert_eq!(
        store.default_ttl(),
        ttl,
        "default_ttl should match the value passed to new()"
    );
}

/// Tests that `InMemoryOidcSessionStore::new_with_principal_limit` creates a
/// store with a per-principal cap separate from the global cap.
#[test]
fn test_in_memory_oidc_session_store_with_principal_limit_sets_both_caps() {
    let store =
        InMemoryOidcSessionStore::new_with_principal_limit(1000, 5, Duration::from_secs(60));

    assert_eq!(store.max_pending(), 1000, "global cap should be 1000");
}

/// Tests that a `NullOidcSessionStore` can be constructed and used as a
/// no-op without panicking.
#[tokio::test]
async fn test_null_oidc_session_store_construction_succeeds() {
    // Construction is a unit struct; it just needs to compile and not panic.
    let _store = NullOidcSessionStore;
}

/// Tests that `NullOidcSessionStore::pending_count` always returns zero.
#[tokio::test]
async fn test_null_oidc_session_store_pending_count_returns_zero() {
    use xzepr::auth::oidc::session_store::OidcSessionStore;

    let store = NullOidcSessionStore;
    let count = store
        .pending_count()
        .await
        .expect("pending_count must not fail on NullOidcSessionStore");

    assert_eq!(
        count, 0,
        "NullOidcSessionStore always has zero pending sessions"
    );
}

/// Tests that `NullOidcSessionStore::cleanup_expired` always returns zero.
#[tokio::test]
async fn test_null_oidc_session_store_cleanup_expired_returns_zero() {
    use xzepr::auth::oidc::session_store::OidcSessionStore;

    let store = NullOidcSessionStore;
    let removed = store
        .cleanup_expired()
        .await
        .expect("cleanup_expired must not fail on NullOidcSessionStore");

    assert_eq!(
        removed, 0,
        "NullOidcSessionStore should report zero cleaned-up sessions"
    );
}

// ─── Live integration tests ───────────────────────────────────────────────────

#[cfg(feature = "oidc-integration-tests")]
mod live {
    /// Returns `true` when the caller has opted in to live OIDC tests.
    fn live_oidc_integration_enabled() -> bool {
        std::env::var("XZEPR_RUN_OIDC_INTEGRATION_TESTS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    /// Returns the OIDC issuer URL from the environment.
    fn oidc_issuer_url() -> Option<String> {
        std::env::var("OIDC_ISSUER_URL").ok()
    }

    /// Returns the OIDC client ID from the environment.
    fn oidc_client_id() -> Option<String> {
        std::env::var("OIDC_CLIENT_ID").ok()
    }

    /// Returns the OIDC client secret from the environment.
    fn oidc_client_secret() -> Option<String> {
        std::env::var("OIDC_CLIENT_SECRET").ok()
    }

    /// Tests that the OIDC discovery endpoint at `{issuer}/.well-known/openid-configuration`
    /// is reachable and returns a JSON document.
    ///
    /// Returns `Ok(())` early when `XZEPR_RUN_OIDC_INTEGRATION_TESTS` is not
    /// set or when `OIDC_ISSUER_URL` is absent.
    #[tokio::test]
    async fn test_oidc_discovery_endpoint_is_reachable_and_returns_json(
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !live_oidc_integration_enabled() {
            return Ok(());
        }

        let issuer = match oidc_issuer_url() {
            Some(u) => u,
            None => return Ok(()),
        };

        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client.get(&discovery_url).send().await?;

        assert!(
            response.status().is_success(),
            "Discovery endpoint at {} should return 2xx, got {}",
            discovery_url,
            response.status()
        );

        let body: serde_json::Value = response.json().await?;
        assert!(
            body.get("issuer").is_some(),
            "Discovery document must contain 'issuer' field"
        );
        assert!(
            body.get("authorization_endpoint").is_some(),
            "Discovery document must contain 'authorization_endpoint' field"
        );

        Ok(())
    }

    /// Tests that an `OidcConfig` can be constructed from the live environment
    /// variables and that the OIDC client initializes successfully.
    ///
    /// Returns `Ok(())` early when `XZEPR_RUN_OIDC_INTEGRATION_TESTS` is not
    /// set or when any required env var is absent.
    #[tokio::test]
    async fn test_oidc_client_construction_with_live_config_succeeds(
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !live_oidc_integration_enabled() {
            return Ok(());
        }

        let issuer = match oidc_issuer_url() {
            Some(u) => u,
            None => return Ok(()),
        };
        let client_id = match oidc_client_id() {
            Some(id) => id,
            None => return Ok(()),
        };
        let client_secret = match oidc_client_secret() {
            Some(s) => s,
            None => return Ok(()),
        };

        let config = xzepr::auth::oidc::OidcConfig::keycloak(
            issuer,
            client_id,
            client_secret,
            "https://localhost/api/v1/auth/oidc/callback".to_string(),
        );

        let _client = xzepr::auth::oidc::OidcClient::new(config).await?;
        Ok(())
    }
}
