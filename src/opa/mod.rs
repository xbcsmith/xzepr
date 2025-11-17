// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Open Policy Agent (OPA) integration module
//!
//! This module provides integration with Open Policy Agent for fine-grained
//! authorization control. It includes:
//!
//! - Policy evaluation client with HTTP communication
//! - Authorization decision caching with TTL and resource-version invalidation
//! - Circuit breaker for graceful degradation when OPA is unavailable
//! - Type definitions for requests, responses, and configuration
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use xzepr::opa::client::OpaClient;
//! use xzepr::opa::types::{OpaConfig, OpaInput, UserContext, ResourceContext};
//!
//! # tokio_test::block_on(async {
//! let config = OpaConfig {
//!     enabled: true,
//!     url: "http://localhost:8181".to_string(),
//!     timeout_seconds: 5,
//!     policy_path: "/v1/data/xzepr/rbac/allow".to_string(),
//!     bundle_url: None,
//!     cache_ttl_seconds: 300,
//! };
//!
//! let client = OpaClient::new(config);
//!
//! let input = OpaInput {
//!     user: UserContext {
//!         user_id: "user123".to_string(),
//!         username: "alice".to_string(),
//!         roles: vec!["user".to_string()],
//!         groups: vec![],
//!     },
//!     action: "read".to_string(),
//!     resource: ResourceContext {
//!         resource_type: "event_receiver".to_string(),
//!         resource_id: Some("receiver123".to_string()),
//!         owner_id: Some("user123".to_string()),
//!         group_id: None,
//!         members: vec![],
//!         resource_version: 1,
//!     },
//! };
//!
//! let decision = client.evaluate_with_cache(input, 1).await?;
//! println!("Authorization decision: {}", decision.allow);
//! # Ok::<(), xzepr::opa::types::OpaError>(())
//! # });
//! ```
//!
//! ## Cache Invalidation
//!
//! ```
//! use xzepr::opa::cache::{AuthorizationCache, ResourceUpdatedEvent};
//! use chrono::Duration;
//!
//! # tokio_test::block_on(async {
//! let cache = AuthorizationCache::new(Duration::minutes(5));
//!
//! // When a resource is updated, invalidate cache entries
//! let event = ResourceUpdatedEvent::EventReceiverUpdated {
//!     receiver_id: "receiver123".to_string(),
//!     version: 2,
//! };
//!
//! event.apply(&cache).await;
//! # });
//! ```
//!
//! ## Circuit Breaker
//!
//! ```
//! use xzepr::opa::circuit_breaker::CircuitBreaker;
//! use std::time::Duration;
//!
//! # tokio_test::block_on(async {
//! let breaker = CircuitBreaker::new(5, Duration::from_secs(30));
//!
//! let result = breaker.call(|| async {
//!     // Call to OPA or other service
//!     Ok::<_, String>("success")
//! }).await;
//!
//! if breaker.is_open().await {
//!     println!("Circuit breaker is open, service unavailable");
//! }
//! # });
//! ```

pub mod cache;
pub mod circuit_breaker;
pub mod client;
pub mod types;

pub use cache::{AuthorizationCache, CacheEntry, CacheKey, ResourceUpdatedEvent};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerError};
pub use client::OpaClient;
pub use types::{
    AuthorizationDecision, OpaConfig, OpaError, OpaInput, OpaRequest, OpaResponse, ResourceContext,
    UserContext,
};
