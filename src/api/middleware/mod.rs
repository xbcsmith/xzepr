// src/api/middleware/mod.rs

//! API middleware components for authentication, rate limiting, CORS, and security
//!
//! This module provides reusable middleware for the XZepr API, including:
//! - CORS configuration and validation
//! - Rate limiting with token bucket algorithm
//! - JWT authentication and validation
//! - Input validation and sanitization
//! - Security headers (CSP, HSTS, etc.)

pub mod cors;
pub mod jwt;
pub mod rate_limit;
pub mod security_headers;
pub mod validation;

// TODO: Implement these modules
// pub mod logging;
// pub mod request_id;

pub use cors::{cors_layer, development_cors_layer, production_cors_layer, CorsConfig};
pub use jwt::{
    jwt_auth_middleware, optional_jwt_auth_middleware, require_permissions, require_roles,
    AuthError, AuthenticatedUser, JwtMiddlewareState,
};
pub use rate_limit::{
    rate_limit_middleware, InMemoryRateLimitStore, RateLimitConfig, RateLimitStore,
    RateLimiterState,
};
pub use security_headers::{
    security_headers_middleware, security_headers_middleware_with_config, FrameOptions,
    ReferrerPolicy, SecurityHeadersConfig,
};
pub use validation::{
    body_size_limit_middleware, sanitize, validate_request, FieldError, ValidationConfig,
    ValidationErrorResponse, ValidationState, DEFAULT_MAX_BODY_SIZE, MAX_UPLOAD_SIZE,
};

// Re-export for convenience
pub use tower_http::cors::CorsLayer;
