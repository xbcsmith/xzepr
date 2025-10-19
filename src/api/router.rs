// src/api/router.rs

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::api::graphql::{graphql_handler, graphql_health, graphql_playground};
use crate::api::middleware::rate_limit::RedisRateLimitStore;
use crate::api::middleware::{
    cors::CorsConfig,
    metrics::MetricsMiddlewareState,
    rate_limit::{RateLimitConfig, RateLimiterState},
    security_headers::{security_headers_middleware_with_config, SecurityHeadersConfig},
    validation::body_size_limit_middleware,
};
use crate::api::rest::events::AppState;
use crate::api::rest::events::*;
use crate::infrastructure::{PrometheusMetrics, SecurityConfig, SecurityMonitor};

/// Router configuration
pub struct RouterConfig {
    /// Security configuration
    pub security: SecurityConfig,
    /// Security monitor
    pub monitor: Arc<SecurityMonitor>,
    /// Prometheus metrics
    pub metrics: Option<Arc<PrometheusMetrics>>,
}

impl RouterConfig {
    /// Creates a new router configuration
    pub fn new(security: SecurityConfig, monitor: Arc<SecurityMonitor>) -> Self {
        Self {
            security,
            monitor,
            metrics: None,
        }
    }

    /// Creates a router configuration with Prometheus metrics
    pub fn with_metrics(mut self, metrics: Arc<PrometheusMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Creates a production router configuration
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

    /// Creates a development router configuration
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

/// Builds the application router with all security middleware
///
/// # Middleware Layers (applied in order from outermost to innermost)
///
/// 1. Security Headers - CSP, HSTS, X-Frame-Options, etc.
/// 2. CORS - Origin validation
/// 3. Metrics - Request instrumentation (Prometheus)
/// 4. Rate Limiting - Abuse prevention
/// 5. Body Size Limits - Request size validation
/// 6. Tracing - Request logging
/// 7. Authentication - JWT validation (per-route)
///
/// # Arguments
///
/// * `state` - Application state with handlers
/// * `config` - Router configuration with security settings
///
/// # Returns
///
/// Returns a configured Router with all middleware applied
///
/// # Example
///
/// ```ignore
/// use xzepr::api::router::{build_router, RouterConfig};
/// use xzepr::api::rest::events::AppState;
///
/// # async fn example(state: AppState) {
/// let config = RouterConfig::production().unwrap();
/// let app = build_router(state, config).await;
/// # }
/// ```
pub async fn build_router(state: AppState, config: RouterConfig) -> Router {
    tracing::info!("Building router with security middleware");

    // Create GraphQL schema
    let schema = crate::api::graphql::create_schema(
        Arc::new(state.event_receiver_handler.clone()),
        Arc::new(state.event_receiver_group_handler.clone()),
    );

    // Initialize rate limiter
    let rate_limit_config = RateLimitConfig {
        anonymous_rpm: config.security.rate_limit.anonymous_rpm,
        authenticated_rpm: config.security.rate_limit.authenticated_rpm,
        admin_rpm: config.security.rate_limit.admin_rpm,
        per_endpoint: config.security.rate_limit.per_endpoint.clone(),
        window_size: std::time::Duration::from_secs(60),
    };

    let rate_limiter = if config.security.rate_limit.use_redis {
        // Use Redis-backed rate limiting for distributed deployments
        let redis_url = std::env::var("XZEPR__REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        match RedisRateLimitStore::new(&redis_url).await {
            Ok(redis_store) => {
                tracing::info!("Using Redis-backed rate limiting at {}", redis_url);
                RateLimiterState::new_with_monitor(
                    rate_limit_config.clone(),
                    Arc::new(redis_store),
                    config.monitor.clone(),
                )
            }
            Err(e) => {
                tracing::error!(
                    "Failed to connect to Redis: {}. Falling back to in-memory rate limiting",
                    e
                );
                RateLimiterState::default_with_config(rate_limit_config.clone())
                    .with_monitor(config.monitor.clone())
            }
        }
    } else {
        RateLimiterState::default_with_config(rate_limit_config.clone())
            .with_monitor(config.monitor.clone())
    };

    // Build CORS layer
    let cors_config = CorsConfig {
        allowed_origins: config.security.cors.allowed_origins.clone(),
        allow_credentials: config.security.cors.allow_credentials,
        max_age_seconds: config.security.cors.max_age_seconds,
    };

    let cors_layer = crate::api::middleware::cors::cors_layer(&cors_config);

    // Build security headers config
    let headers_config = SecurityHeadersConfig {
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
    };

    tracing::info!(
        cors_origins = ?cors_config.allowed_origins,
        anonymous_rpm = rate_limit_config.anonymous_rpm,
        authenticated_rpm = rate_limit_config.authenticated_rpm,
        "Security configuration loaded"
    );

    // Create a metrics state for the /metrics endpoint
    let metrics_state = config.metrics.clone().unwrap_or_else(|| {
        tracing::warn!("No Prometheus metrics configured, using default");
        Arc::new(PrometheusMetrics::default())
    });

    // Create metrics middleware state
    let metrics_middleware_state = MetricsMiddlewareState::new(metrics_state.clone());

    // Build the router with all routes
    Router::new()
        // Health check endpoint (public, no auth required)
        .route("/health", get(health_check))
        // Metrics endpoint for Prometheus (separate state)
        .route("/metrics", get(metrics_handler))
        .with_state(metrics_state)
        // GraphQL endpoints
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        .route("/graphql/health", get(graphql_health))
        .with_state(schema.clone())
        // REST API v1 routes
        .route("/api/v1/events", post(create_event))
        .route("/api/v1/events/:id", get(get_event))
        // Event receiver routes
        .route("/api/v1/receivers", post(create_event_receiver))
        .route("/api/v1/receivers", get(list_event_receivers))
        .route("/api/v1/receivers/:id", get(get_event_receiver))
        .route("/api/v1/receivers/:id", post(update_event_receiver))
        .route("/api/v1/receivers/:id", get(delete_event_receiver))
        // Event receiver group routes
        .route("/api/v1/groups", post(create_event_receiver_group))
        .route("/api/v1/groups/:id", get(get_event_receiver_group))
        .route("/api/v1/groups/:id", post(update_event_receiver_group))
        .route("/api/v1/groups/:id", get(delete_event_receiver_group))
        .with_state(state)
        // Apply middleware layers (innermost to outermost)
        // Layer 7: Tracing (request logging)
        .layer(TraceLayer::new_for_http())
        // Layer 6: Body size limits
        .layer(middleware::from_fn(body_size_limit_middleware))
        // Layer 5: Rate limiting
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            crate::api::middleware::rate_limit::rate_limit_middleware,
        ))
        // Layer 4: Metrics (Prometheus instrumentation)
        .layer(middleware::from_fn_with_state(
            metrics_middleware_state,
            crate::api::middleware::metrics::metrics_middleware,
        ))
        // Layer 3: CORS
        .layer(cors_layer)
        // Layer 2: Security headers (outermost)
        .layer(middleware::from_fn(move |req, next| {
            let config = headers_config.clone();
            async move { security_headers_middleware_with_config(config, req, next).await }
        }))
}

/// Metrics handler for Prometheus scraping
async fn metrics_handler(config: axum::extract::State<Arc<PrometheusMetrics>>) -> String {
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
        let config = RouterConfig::production().unwrap();
        assert!(config.security.headers.enable_hsts);
        assert!(config.security.monitoring.metrics_enabled);
        assert!(config.metrics.is_some());
    }

    #[test]
    fn test_router_config_development() {
        let config = RouterConfig::development().unwrap();
        assert_eq!(config.security.cors.allowed_origins, vec!["*"]);
        assert!(!config.security.headers.enable_hsts);
        assert!(config.metrics.is_some());
    }
}
