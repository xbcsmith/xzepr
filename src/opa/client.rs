// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! OPA client for policy evaluation
//!
//! This module provides a client for communicating with Open Policy Agent servers
//! to evaluate authorization policies. It includes caching and circuit breaker
//! support for resilience.

use super::cache::{AuthorizationCache, CacheKey};
use super::circuit_breaker::{CircuitBreaker, CircuitBreakerError};
use super::types::{AuthorizationDecision, OpaConfig, OpaError, OpaInput, OpaRequest, OpaResponse};
use chrono::Duration;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration as StdDuration;

/// OPA client for authorization policy evaluation
///
/// Provides methods to evaluate authorization policies against an OPA server
/// with built-in caching and circuit breaker for resilience.
///
/// # Examples
///
/// ```no_run
/// use xzepr::opa::client::OpaClient;
/// use xzepr::opa::types::{OpaConfig, OpaInput, UserContext, ResourceContext};
///
/// # tokio_test::block_on(async {
/// let config = OpaConfig {
///     enabled: true,
///     url: "http://localhost:8181".to_string(),
///     timeout_seconds: 5,
///     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
///     bundle_url: None,
///     cache_ttl_seconds: 300,
/// };
///
/// let client = OpaClient::new(config);
///
/// let input = OpaInput {
///     user: UserContext {
///         user_id: "user123".to_string(),
///         username: "alice".to_string(),
///         roles: vec!["user".to_string()],
///         groups: vec![],
///     },
///     action: "read".to_string(),
///     resource: ResourceContext {
///         resource_type: "event_receiver".to_string(),
///         resource_id: Some("receiver123".to_string()),
///         owner_id: Some("user123".to_string()),
///         group_id: None,
///         members: vec![],
///         resource_version: 1,
///     },
/// };
///
/// let decision = client.evaluate_with_cache(input, 1).await;
/// # });
/// ```
pub struct OpaClient {
    /// HTTP client for making requests to OPA
    http_client: Client,

    /// OPA configuration
    config: OpaConfig,

    /// Authorization cache
    cache: Arc<AuthorizationCache>,

    /// Circuit breaker for fault tolerance
    circuit_breaker: Arc<CircuitBreaker>,
}

impl OpaClient {
    /// Creates a new OPA client
    ///
    /// # Arguments
    ///
    /// * `config` - OPA configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::client::OpaClient;
    /// use xzepr::opa::types::OpaConfig;
    ///
    /// let config = OpaConfig {
    ///     enabled: true,
    ///     url: "http://localhost:8181".to_string(),
    ///     timeout_seconds: 5,
    ///     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
    ///     bundle_url: None,
    ///     cache_ttl_seconds: 300,
    /// };
    ///
    /// let client = OpaClient::new(config);
    /// ```
    pub fn new(config: OpaConfig) -> Self {
        let http_client = Client::builder()
            .timeout(StdDuration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to build HTTP client");

        let cache = Arc::new(AuthorizationCache::new(Duration::seconds(
            config.cache_ttl_seconds as i64,
        )));

        let circuit_breaker = Arc::new(CircuitBreaker::new(5, StdDuration::from_secs(30)));

        Self {
            http_client,
            config,
            cache,
            circuit_breaker,
        }
    }

    /// Evaluates a policy with caching
    ///
    /// Checks the cache first, and if not found, queries OPA and caches the result.
    ///
    /// # Arguments
    ///
    /// * `input` - Policy evaluation input
    /// * `resource_version` - Current version of the resource
    ///
    /// # Returns
    ///
    /// Returns the authorization decision
    ///
    /// # Errors
    ///
    /// Returns `OpaError` if the evaluation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xzepr::opa::client::OpaClient;
    /// use xzepr::opa::types::{OpaConfig, OpaInput, UserContext, ResourceContext};
    ///
    /// # tokio_test::block_on(async {
    /// # let config = OpaConfig {
    /// #     enabled: true,
    /// #     url: "http://localhost:8181".to_string(),
    /// #     timeout_seconds: 5,
    /// #     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
    /// #     bundle_url: None,
    /// #     cache_ttl_seconds: 300,
    /// # };
    /// let client = OpaClient::new(config);
    ///
    /// let input = OpaInput {
    ///     user: UserContext {
    ///         user_id: "user123".to_string(),
    ///         username: "alice".to_string(),
    ///         roles: vec!["user".to_string()],
    ///         groups: vec![],
    ///     },
    ///     action: "read".to_string(),
    ///     resource: ResourceContext {
    ///         resource_type: "event_receiver".to_string(),
    ///         resource_id: Some("receiver123".to_string()),
    ///         owner_id: Some("user123".to_string()),
    ///         group_id: None,
    ///         members: vec![],
    ///         resource_version: 1,
    ///     },
    /// };
    ///
    /// let decision = client.evaluate_with_cache(input, 1).await?;
    /// # Ok::<(), xzepr::opa::types::OpaError>(())
    /// # });
    /// ```
    pub async fn evaluate_with_cache(
        &self,
        input: OpaInput,
        resource_version: i32,
    ) -> Result<AuthorizationDecision, OpaError> {
        // Build cache key
        let cache_key = CacheKey {
            user_id: input.user.user_id.clone(),
            action: input.action.clone(),
            resource_type: input.resource.resource_type.clone(),
            resource_id: input
                .resource
                .resource_id
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            resource_version,
        };

        // Check cache
        if let Some(cached_decision) = self.cache.get(&cache_key).await {
            return Ok(AuthorizationDecision {
                allow: cached_decision,
                reason: Some("Cached decision".to_string()),
                metadata: None,
            });
        }

        // Evaluate policy
        let decision = self.evaluate(input).await?;

        // Cache the result
        self.cache.set(cache_key, decision.allow).await;

        Ok(decision)
    }

    /// Evaluates a policy without caching
    ///
    /// Directly queries OPA without checking or updating the cache.
    ///
    /// # Arguments
    ///
    /// * `input` - Policy evaluation input
    ///
    /// # Returns
    ///
    /// Returns the authorization decision
    ///
    /// # Errors
    ///
    /// Returns `OpaError` if the evaluation fails
    pub async fn evaluate(&self, input: OpaInput) -> Result<AuthorizationDecision, OpaError> {
        let request = OpaRequest { input };

        let url = format!("{}{}", self.config.url, self.config.policy_path);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| OpaError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OpaError::InvalidResponse(format!(
                "OPA returned status: {}",
                response.status()
            )));
        }

        let opa_response: OpaResponse = response
            .json()
            .await
            .map_err(|e| OpaError::InvalidResponse(e.to_string()))?;

        opa_response
            .result
            .ok_or_else(|| OpaError::EvaluationError("No result in OPA response".to_string()))
    }

    /// Evaluates a policy with circuit breaker protection
    ///
    /// Uses the circuit breaker to prevent cascading failures when OPA is unavailable.
    ///
    /// # Arguments
    ///
    /// * `input` - Policy evaluation input
    /// * `resource_version` - Current version of the resource
    ///
    /// # Returns
    ///
    /// Returns the authorization decision, or an error if circuit is open
    ///
    /// # Errors
    ///
    /// Returns `OpaError` if the evaluation fails or circuit is open
    pub async fn evaluate_with_circuit_breaker(
        &self,
        input: OpaInput,
        resource_version: i32,
    ) -> Result<AuthorizationDecision, OpaError> {
        let input_clone = input.clone();

        self.circuit_breaker
            .call(|| async move {
                self.evaluate_with_cache(input_clone, resource_version)
                    .await
            })
            .await
            .map_err(|e| match e {
                CircuitBreakerError::CircuitOpen => OpaError::CircuitOpen,
                CircuitBreakerError::CallFailed(opa_error) => opa_error,
            })
    }

    /// Gets the authorization cache
    ///
    /// Returns a reference to the internal cache for manual invalidation.
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::client::OpaClient;
    /// use xzepr::opa::types::OpaConfig;
    ///
    /// # tokio_test::block_on(async {
    /// # let config = OpaConfig {
    /// #     enabled: true,
    /// #     url: "http://localhost:8181".to_string(),
    /// #     timeout_seconds: 5,
    /// #     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
    /// #     bundle_url: None,
    /// #     cache_ttl_seconds: 300,
    /// # };
    /// let client = OpaClient::new(config);
    ///
    /// client.cache().invalidate_resource("event_receiver", "receiver123").await;
    /// # });
    /// ```
    pub fn cache(&self) -> &Arc<AuthorizationCache> {
        &self.cache
    }

    /// Gets the circuit breaker
    ///
    /// Returns a reference to the internal circuit breaker for status checks.
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::client::OpaClient;
    /// use xzepr::opa::types::OpaConfig;
    ///
    /// # tokio_test::block_on(async {
    /// # let config = OpaConfig {
    /// #     enabled: true,
    /// #     url: "http://localhost:8181".to_string(),
    /// #     timeout_seconds: 5,
    /// #     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
    /// #     bundle_url: None,
    /// #     cache_ttl_seconds: 300,
    /// # };
    /// let client = OpaClient::new(config);
    ///
    /// let is_open = client.circuit_breaker().is_open().await;
    /// # });
    /// ```
    pub fn circuit_breaker(&self) -> &Arc<CircuitBreaker> {
        &self.circuit_breaker
    }

    /// Checks if OPA is enabled in the configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use xzepr::opa::client::OpaClient;
    /// use xzepr::opa::types::OpaConfig;
    ///
    /// let config = OpaConfig {
    ///     enabled: true,
    ///     url: "http://localhost:8181".to_string(),
    ///     timeout_seconds: 5,
    ///     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
    ///     bundle_url: None,
    ///     cache_ttl_seconds: 300,
    /// };
    ///
    /// let client = OpaClient::new(config);
    /// assert!(client.is_enabled());
    /// ```
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opa::types::{ResourceContext, UserContext};

    #[test]
    fn test_opa_client_creation() {
        let config = OpaConfig {
            enabled: true,
            url: "http://localhost:8181".to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
        };

        let client = OpaClient::new(config);
        assert!(client.is_enabled());
    }

    #[tokio::test]
    async fn test_cache_access() {
        let config = OpaConfig {
            enabled: true,
            url: "http://localhost:8181".to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
        };

        let client = OpaClient::new(config);
        assert!(client.cache().is_empty().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_access() {
        let config = OpaConfig {
            enabled: true,
            url: "http://localhost:8181".to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
        };

        let client = OpaClient::new(config);
        assert!(!client.circuit_breaker().is_open().await);
    }

    #[test]
    fn test_opa_client_disabled() {
        let config = OpaConfig {
            enabled: false,
            url: "http://localhost:8181".to_string(),
            timeout_seconds: 5,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
        };

        let client = OpaClient::new(config);
        assert!(!client.is_enabled());
    }

    #[tokio::test]
    async fn test_evaluate_with_unreachable_server() {
        let config = OpaConfig {
            enabled: true,
            url: "http://localhost:9999".to_string(),
            timeout_seconds: 1,
            policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
            bundle_url: None,
            cache_ttl_seconds: 300,
        };

        let client = OpaClient::new(config);

        let input = OpaInput {
            user: UserContext {
                user_id: "user123".to_string(),
                username: "alice".to_string(),
                roles: vec!["user".to_string()],
                groups: vec![],
            },
            action: "read".to_string(),
            resource: ResourceContext {
                resource_type: "event_receiver".to_string(),
                resource_id: Some("receiver123".to_string()),
                owner_id: Some("owner456".to_string()),
                group_id: None,
                members: vec![],
                resource_version: 1,
            },
        };

        let result = client.evaluate(input).await;
        assert!(result.is_err());
    }
}
