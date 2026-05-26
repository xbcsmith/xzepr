// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/middleware/mod.rs

//! API middleware components.
//!
//! This module re-exports the public API surface of every middleware component.
//! All items listed here are intentionally stable and may be used by integration
//! code and tests outside this module.
//!
//! ## Components
//!
//! - [`cors`]: CORS configuration and origin validation
//! - [`jwt`]: JWT authentication and user extraction
//! - [`metrics`]: Request metrics and Prometheus integration
//! - [`opa`]: OPA policy authorization middleware
//! - [`rate_limit`]: Token-bucket rate limiting
//! - [`rbac`]: Role-based access control enforcement
//! - [`rbac_helpers`]: Permission-to-route mapping utilities
//! - [`resource_context`]: Resource context builders for OPA input
//! - [`security_headers`]: CSP, HSTS, and security header injection
//! - [`tracing_middleware`]: Request ID generation and tracing
//! - [`validation`]: Input validation, sanitization, and body-size limits

pub mod cors;
pub mod jwt;
pub mod metrics;
pub mod opa;
pub mod rate_limit;
pub mod rbac;
pub mod rbac_helpers;
pub mod resource_context;
pub mod security_headers;
pub mod tracing_middleware;
pub mod validation;

pub use cors::{
    cors_layer, development_cors_layer, production_cors_layer, CorsConfig, CorsConfigError,
};
pub use jwt::{
    jwt_auth_middleware, optional_jwt_auth_middleware, require_permissions, require_roles,
    AuthError, AuthenticatedUser, JwtMiddlewareState,
};
pub use metrics::{
    extract_path_for_metrics, metrics_middleware, metrics_middleware_simple, record_error_metric,
    MetricsError, MetricsMiddlewareState,
};
pub use opa::{
    opa_authorize_middleware, AuthorizationDecision, AuthorizationError, OpaMiddlewareState,
    ResourceContextBuilders,
};
pub use rate_limit::{
    rate_limit_middleware, InMemoryRateLimitStore, RateLimitConfig, RateLimitStore,
    RateLimiterState,
};
pub use rbac::{
    rbac_enforcement_middleware, rbac_enforcement_middleware_with_state, RbacError,
    RbacMiddlewareState,
};
pub use rbac_helpers::{
    extract_resource_id, get_resource_permissions, is_public_route, route_to_permission,
};
pub use resource_context::{
    EventContextBuilder, EventReceiverContextBuilder, EventReceiverGroupContextBuilder,
    ResourceContextBuilder, ResourceContextError,
};
pub use security_headers::{
    security_headers_middleware, security_headers_middleware_with_config, FrameOptions,
    ReferrerPolicy, SecurityHeadersConfig,
};
pub use tracing_middleware::{
    enhanced_tracing_middleware, request_id_middleware, tracing_middleware, RequestId, TracedError,
};
pub use validation::{
    body_size_limit_middleware, sanitize, validate_request, FieldError, ValidationConfig,
    ValidationErrorResponse, ValidationState, DEFAULT_MAX_BODY_SIZE, MAX_UPLOAD_SIZE,
};

// Re-export for convenience
pub use tower_http::cors::CorsLayer;
