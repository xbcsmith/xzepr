// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Canonical API router for production and development deployments.
//!
//! This module owns the single supported runtime route composition path. The
//! production builder wires authentication, RBAC, rate limiting, body limits,
//! CORS, security headers, tracing, and metrics consistently across REST and
//! GraphQL endpoints.

use axum::{
    extract::{DefaultBodyLimit, State},
    middleware,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::api::graphql::{create_schema, graphql_handler, graphql_health, graphql_playground};
use crate::api::middleware::rate_limit::RedisRateLimitStore;
use crate::api::middleware::{
    cors::CorsConfig,
    jwt_auth_middleware,
    metrics::MetricsMiddlewareState,
    rate_limit::{RateLimitConfig, RateLimiterState},
    rbac_enforcement_middleware,
    security_headers::{security_headers_middleware_with_config, SecurityHeadersConfig},
    JwtMiddlewareState,
};
use crate::api::rest::auth::{login, logout, oidc_callback, oidc_login, refresh_token, AuthState};
use crate::api::rest::events::{
    create_event, create_event_receiver, create_event_receiver_group, delete_event_receiver,
    delete_event_receiver_group, get_event, get_event_receiver, get_event_receiver_group,
    health_check, list_event_receivers, update_event_receiver, update_event_receiver_group,
    AppState,
};
use crate::domain::repositories::user_repo::UserRepository;
use crate::infrastructure::{PrometheusMetrics, SecurityConfig, SecurityMonitor};

/// Router configuration.
pub struct RouterConfig {
    /// Security configuration.
    pub security: SecurityConfig,
    /// Security monitor.
    pub monitor: Arc<SecurityMonitor>,
    /// Prometheus metrics registry.
    pub metrics: Option<Arc<PrometheusMetrics>>,
}

impl RouterConfig {
    /// Creates a new router configuration.
    pub fn new(security: SecurityConfig, monitor: Arc<SecurityMonitor>) -> Self {
        Self {
            security,
            monitor,
            metrics: None,
        }
    }

    /// Creates a router configuration with Prometheus metrics.
    pub fn with_metrics(mut self, metrics: Arc<PrometheusMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Creates a production router configuration.
    pub fn production() -> Result<Self, prometheus::Error> {
        let security = SecurityConfig::production();
        let metrics = Arc::new(PrometheusMetrics::new()?);
        let monitor = Arc::new(SecurityMonitor::new_with_metrics(metrics.clone()));
        Ok(Self {
            security,
            monitor,
            metrics: Some(metrics),
        })
    }

    /// Creates a development router configuration.
    pub fn development() -> Result<Self, prometheus::Error> {
        let security = SecurityConfig::development();
        let metrics = Arc::new(PrometheusMetrics::new()?);
        let monitor = Arc::new(SecurityMonitor::new_with_metrics(metrics.clone()));
        Ok(Self {
            security,
            monitor,
            metrics: Some(metrics),
        })
    }
}

/// Builds the canonical production router.
///
/// Protected REST routes require JWT authentication and RBAC permissions.
/// GraphQL execution requires JWT authentication. Public routes are limited to
/// health, metrics, status, auth login/refresh, OIDC redirects, GraphQL health,
/// and the GraphQL playground.
///
/// # Arguments
///
/// * `state` - Application event and receiver handlers.
/// * `auth_state` - Authentication state for login, refresh, and logout.
/// * `jwt_state` - JWT middleware state for protected routes.
/// * `config` - Router security and observability configuration.
///
/// # Returns
///
/// Returns an Axum router with the canonical production middleware stack.
pub async fn build_production_router<R>(
    state: AppState,
    auth_state: AuthState<R>,
    jwt_state: JwtMiddlewareState,
    config: RouterConfig,
) -> Router
where
    R: UserRepository + 'static,
{
    tracing::info!("Building canonical production router");

    let schema = create_schema(
        Arc::new(state.event_receiver_handler.clone()),
        Arc::new(state.event_receiver_group_handler.clone()),
    );

    let rate_limiter = build_rate_limiter(&config).await;
    let cors_layer = build_cors_layer(&config);
    let headers_config = build_security_headers_config(&config);
    let metrics_state = config.metrics.clone().unwrap_or_else(|| {
        tracing::warn!("No Prometheus metrics configured, using default registry");
        Arc::new(PrometheusMetrics::default())
    });
    let metrics_middleware_state = MetricsMiddlewareState::new(metrics_state.clone());

    let public_core_routes = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_check))
        .route("/api/v1/status", get(api_status))
        .route("/metrics", get(metrics_handler))
        .with_state(metrics_state)
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            crate::api::middleware::rate_limit::rate_limit_middleware,
        ));

    let public_auth_routes = Router::new()
        .route("/api/v1/auth/login", post(login::<R>))
        .route("/api/v1/auth/refresh", post(refresh_token::<R>))
        .route("/api/v1/auth/oidc/login", get(oidc_login::<R>))
        .route("/api/v1/auth/oidc/callback", get(oidc_callback::<R>))
        .with_state(auth_state.clone())
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            crate::api::middleware::rate_limit::rate_limit_middleware,
        ));

    let protected_auth_routes = Router::new()
        .route("/api/v1/auth/logout", post(logout::<R>))
        .with_state(auth_state)
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            crate::api::middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            jwt_state.clone(),
            jwt_auth_middleware,
        ));

    let public_graphql_routes = Router::new()
        .route("/graphql/playground", get(graphql_playground))
        .route("/graphql/health", get(graphql_health))
        .with_state(schema.clone())
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            crate::api::middleware::rate_limit::rate_limit_middleware,
        ));

    let protected_graphql_routes = Router::new()
        .route("/graphql", post(graphql_handler))
        .with_state(schema)
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            crate::api::middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            jwt_state.clone(),
            jwt_auth_middleware,
        ));

    let protected_api_routes = Router::new()
        .route("/api/v1/events", post(create_event))
        .route("/api/v1/events/:id", get(get_event))
        .route("/api/v1/receivers", post(create_event_receiver))
        .route("/api/v1/receivers", get(list_event_receivers))
        .route("/api/v1/receivers/:id", get(get_event_receiver))
        .route("/api/v1/receivers/:id", put(update_event_receiver))
        .route("/api/v1/receivers/:id", delete(delete_event_receiver))
        .route("/api/v1/groups", post(create_event_receiver_group))
        .route("/api/v1/groups/:id", get(get_event_receiver_group))
        .route("/api/v1/groups/:id", put(update_event_receiver_group))
        .route("/api/v1/groups/:id", delete(delete_event_receiver_group))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            crate::api::middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(middleware::from_fn(rbac_enforcement_middleware))
        .layer(middleware::from_fn_with_state(
            jwt_state,
            jwt_auth_middleware,
        ));

    public_core_routes
        .merge(public_auth_routes)
        .merge(protected_auth_routes)
        .merge(public_graphql_routes)
        .merge(protected_graphql_routes)
        .merge(protected_api_routes)
        .layer(DefaultBodyLimit::max(
            config.security.validation.max_body_size,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            metrics_middleware_state,
            crate::api::middleware::metrics::metrics_middleware,
        ))
        .layer(cors_layer)
        .layer(middleware::from_fn(move |req, next| {
            let config = headers_config.clone();
            async move { security_headers_middleware_with_config(config, req, next).await }
        }))
}

async fn build_rate_limiter(config: &RouterConfig) -> RateLimiterState {
    let rate_limit_config = RateLimitConfig {
        anonymous_rpm: config.security.rate_limit.anonymous_rpm,
        authenticated_rpm: config.security.rate_limit.authenticated_rpm,
        admin_rpm: config.security.rate_limit.admin_rpm,
        per_endpoint: config.security.rate_limit.per_endpoint.clone(),
        window_size: std::time::Duration::from_secs(60),
    };

    if config.security.rate_limit.use_redis {
        let redis_url = config
            .security
            .rate_limit
            .redis_url
            .clone()
            .or_else(|| std::env::var("XZEPR__REDIS_URL").ok())
            .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());

        match RedisRateLimitStore::new(&redis_url).await {
            Ok(redis_store) => {
                tracing::info!("Using Redis-backed rate limiting");
                return RateLimiterState::new_with_monitor(
                    rate_limit_config,
                    Arc::new(redis_store),
                    config.monitor.clone(),
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to connect to Redis, falling back to in-memory rate limiting"
                );
            }
        }
    }

    RateLimiterState::default_with_config(rate_limit_config).with_monitor(config.monitor.clone())
}

fn build_cors_layer(config: &RouterConfig) -> tower_http::cors::CorsLayer {
    let cors_config = CorsConfig {
        allowed_origins: config.security.cors.allowed_origins.clone(),
        allow_credentials: config.security.cors.allow_credentials,
        max_age_seconds: config.security.cors.max_age_seconds,
    };

    tracing::info!(
        cors_origins = ?cors_config.allowed_origins,
        anonymous_rpm = config.security.rate_limit.anonymous_rpm,
        authenticated_rpm = config.security.rate_limit.authenticated_rpm,
        "Security configuration loaded"
    );

    crate::api::middleware::cors::cors_layer(&cors_config)
}

fn build_security_headers_config(config: &RouterConfig) -> SecurityHeadersConfig {
    SecurityHeadersConfig {
        enable_csp: config.security.headers.enable_csp,
        csp_directives: config
            .security
            .headers
            .csp_directives
            .clone()
            .unwrap_or_default(),
        enable_hsts: config.security.headers.enable_hsts,
        hsts_max_age: config.security.headers.hsts_max_age,
        hsts_include_subdomains: config.security.headers.hsts_include_subdomains,
        hsts_preload: config.security.headers.hsts_preload,
        enable_frame_options: true,
        frame_options: crate::api::middleware::security_headers::FrameOptions::Deny,
        enable_content_type_options: true,
        enable_xss_protection: true,
        enable_referrer_policy: true,
        referrer_policy:
            crate::api::middleware::security_headers::ReferrerPolicy::StrictOriginWhenCrossOrigin,
        enable_permissions_policy: true,
        permissions_policy: "geolocation=(), microphone=(), camera=()".to_string(),
    }
}

async fn root_handler() -> impl IntoResponse {
    Json(json!({
        "service": "XZepr Event Tracking Server",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "High-performance event tracking with real-time streaming",
        "endpoints": {
            "health": "/health",
            "graphql": "/graphql",
            "graphql_playground": "/graphql/playground",
            "api": "/api/v1"
        }
    }))
}

async fn api_status() -> impl IntoResponse {
    Json(json!({
        "status": "operational",
        "api_version": "v1",
        "features": {
            "authentication": ["local", "oidc", "api_key"],
            "authorization": "rbac",
            "event_tracking": true,
            "graphql": true,
            "real_time_streaming": true
        }
    }))
}

async fn metrics_handler(config: State<Arc<PrometheusMetrics>>) -> String {
    match config.gather() {
        Ok(metrics) => metrics,
        Err(e) => {
            tracing::error!("Failed to gather metrics: {}", e);
            format!("# Error gathering metrics: {}\n", e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_config_production() {
        let config = RouterConfig::production().expect("production router config should build");
        assert!(config.security.headers.enable_hsts);
        assert!(config.security.monitoring.metrics_enabled);
        assert!(config.metrics.is_some());
    }

    #[test]
    fn test_router_config_production_cors_is_not_wildcard() {
        let config = RouterConfig::production().expect("production router config should build");
        assert!(!config
            .security
            .cors
            .allowed_origins
            .iter()
            .any(|origin| origin == "*"));
    }

    #[test]
    fn test_router_config_development() {
        let config = RouterConfig::development().expect("development router config should build");
        assert_eq!(config.security.cors.allowed_origins, vec!["*"]);
        assert!(!config.security.headers.enable_hsts);
        assert!(config.metrics.is_some());
    }
}
