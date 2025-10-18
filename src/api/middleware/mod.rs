// src/api/middleware/mod.rs

//! API middleware components for authentication, rate limiting, CORS, and security
//!
//! This module provides reusable middleware for the XZepr API, including:
//! - CORS configuration and validation
//! - Rate limiting with token bucket algorithm
//! - JWT authentication and validation (TODO)
//! - Request logging and tracing (TODO)
//! - Security headers (TODO)

pub mod cors;
pub mod rate_limit;

// TODO: Implement these modules
// pub mod jwt;
// pub mod logging;
// pub mod security_headers;
// pub mod request_id;

pub use cors::{cors_layer, development_cors_layer, production_cors_layer, CorsConfig};
pub use rate_limit::{
    rate_limit_middleware, InMemoryRateLimitStore, RateLimitConfig, RateLimiterState,
    RateLimitStore,
};

// Re-export for convenience
pub use tower_http::cors::CorsLayer;
