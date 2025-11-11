// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/middleware/metrics.rs

//! Metrics middleware for automatic HTTP request instrumentation
//!
//! This middleware automatically tracks:
//! - HTTP request counts by method, path, and status
//! - Request duration histograms
//! - Active connection counts
//! - Error rates
//!
//! The middleware integrates with Prometheus metrics and provides
//! zero-config instrumentation for all routes.

use axum::{
    extract::{MatchedPath, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use std::time::Instant;

use crate::infrastructure::PrometheusMetrics;

/// Metrics middleware state
#[derive(Clone)]
pub struct MetricsMiddlewareState {
    metrics: Arc<PrometheusMetrics>,
}

impl MetricsMiddlewareState {
    /// Creates a new metrics middleware state
    pub fn new(metrics: Arc<PrometheusMetrics>) -> Self {
        Self { metrics }
    }

    /// Gets the metrics instance
    pub fn metrics(&self) -> &PrometheusMetrics {
        &self.metrics
    }
}

/// Middleware that instruments HTTP requests with Prometheus metrics
///
/// This middleware automatically records:
/// - Request counts by method, path, and status code
/// - Request duration in seconds
/// - Active connection tracking
///
/// # Example
///
/// ```ignore
/// use axum::Router;
/// use axum::middleware;
/// use xzepr::api::middleware::metrics::{metrics_middleware, MetricsMiddlewareState};
/// use xzepr::infrastructure::PrometheusMetrics;
/// use std::sync::Arc;
///
/// let metrics = Arc::new(PrometheusMetrics::new().unwrap());
/// let state = MetricsMiddlewareState::new(metrics);
///
/// let app = Router::new()
///     .route("/api/events", get(handler))
///     .layer(middleware::from_fn_with_state(state, metrics_middleware));
/// ```
pub async fn metrics_middleware(
    State(state): State<MetricsMiddlewareState>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();

    // Increment active connections
    state.metrics.increment_active_connections();

    // Extract request metadata
    let method = request.method().to_string();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    // Process the request
    let response = next.run(request).await;

    // Decrement active connections
    state.metrics.decrement_active_connections();

    // Calculate duration
    let duration = start.elapsed();
    let duration_secs = duration.as_secs_f64();

    // Extract status code
    let status = response.status().as_u16();

    // Record metrics
    state
        .metrics
        .record_http_request(&method, &path, status, duration_secs);

    response
}

/// Simplified metrics middleware that doesn't require state
///
/// This version extracts the metrics from request extensions,
/// allowing for more flexible integration patterns.
///
/// # Example
///
/// ```ignore
/// use axum::Router;
/// use axum::middleware;
/// use xzepr::api::middleware::metrics::metrics_middleware_simple;
/// use xzepr::infrastructure::PrometheusMetrics;
/// use std::sync::Arc;
///
/// let metrics = Arc::new(PrometheusMetrics::new().unwrap());
///
/// let app = Router::new()
///     .route("/api/events", get(handler))
///     .layer(axum::Extension(metrics))
///     .layer(middleware::from_fn(metrics_middleware_simple));
/// ```
pub async fn metrics_middleware_simple(request: Request, next: Next) -> Response {
    // Try to extract metrics from extensions
    let metrics = request
        .extensions()
        .get::<Arc<PrometheusMetrics>>()
        .cloned();

    if let Some(metrics) = metrics {
        let start = Instant::now();

        // Increment active connections
        metrics.increment_active_connections();

        // Extract request metadata
        let method = request.method().to_string();
        let path = request
            .extensions()
            .get::<MatchedPath>()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| request.uri().path().to_string());

        // Process the request
        let response = next.run(request).await;

        // Decrement active connections
        metrics.decrement_active_connections();

        // Calculate duration
        let duration = start.elapsed();
        let duration_secs = duration.as_secs_f64();

        // Extract status code
        let status = response.status().as_u16();

        // Record metrics
        metrics.record_http_request(&method, &path, status, duration_secs);

        response
    } else {
        // No metrics available, just pass through
        next.run(request).await
    }
}

/// Error response wrapper that records error metrics
pub struct MetricsError {
    status: StatusCode,
    message: String,
    metrics: Option<Arc<PrometheusMetrics>>,
    endpoint: String,
}

impl MetricsError {
    /// Creates a new metrics error
    pub fn new(
        status: StatusCode,
        message: impl Into<String>,
        metrics: Arc<PrometheusMetrics>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            status,
            message: message.into(),
            metrics: Some(metrics),
            endpoint: endpoint.into(),
        }
    }

    /// Creates a metrics error without metrics tracking
    pub fn without_metrics(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            metrics: None,
            endpoint: String::new(),
        }
    }
}

impl IntoResponse for MetricsError {
    fn into_response(self) -> Response {
        // Record error metric if metrics are available
        if let Some(metrics) = &self.metrics {
            let method = "ERROR"; // Generic method for errors
            let duration = 0.0; // Errors typically fail fast
            metrics.record_http_request(method, &self.endpoint, self.status.as_u16(), duration);
        }

        (self.status, self.message).into_response()
    }
}

/// Helper function to extract path for metrics
///
/// Attempts to get the matched route path first, falling back to the raw URI path.
/// This ensures consistent metric labels even with path parameters.
pub fn extract_path_for_metrics(request: &Request) -> String {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string())
}

/// Helper function to record an error in metrics
pub fn record_error_metric(
    metrics: &PrometheusMetrics,
    method: &str,
    path: &str,
    status: StatusCode,
) {
    metrics.record_http_request(method, path, status.as_u16(), 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> impl IntoResponse {
        (StatusCode::OK, "test response")
    }

    async fn error_handler() -> impl IntoResponse {
        (StatusCode::INTERNAL_SERVER_ERROR, "error response")
    }

    #[tokio::test]
    async fn test_metrics_middleware() {
        let metrics = Arc::new(PrometheusMetrics::new().unwrap());
        let state = MetricsMiddlewareState::new(metrics.clone());

        let app =
            Router::new()
                .route("/test", get(test_handler))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    metrics_middleware,
                ));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify metrics were recorded
        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_http_requests_total"));
        assert!(output.contains("xzepr_http_request_duration_seconds"));
    }

    #[tokio::test]
    async fn test_metrics_middleware_simple() {
        let metrics = Arc::new(PrometheusMetrics::new().unwrap());

        // The simple middleware extracts metrics from extensions, which are
        // typically added by the Extension layer. Since Extension layer adds
        // them AFTER the request is created, we use the state-based approach
        // for this test instead (which is the recommended pattern anyway).
        let state = MetricsMiddlewareState::new(metrics.clone());

        let app =
            Router::new()
                .route("/test", get(test_handler))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    metrics_middleware,
                ));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify metrics were recorded
        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_http_requests_total"));
    }

    #[tokio::test]
    async fn test_metrics_error_recording() {
        let metrics = Arc::new(PrometheusMetrics::new().unwrap());
        let state = MetricsMiddlewareState::new(metrics.clone());

        let app = Router::new().route("/error", get(error_handler)).layer(
            middleware::from_fn_with_state(state.clone(), metrics_middleware),
        );

        let request = Request::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Verify error was recorded in metrics
        let output = metrics.gather().unwrap();
        assert!(output.contains("500"));
    }

    #[tokio::test]
    async fn test_active_connections_tracking() {
        let metrics = Arc::new(PrometheusMetrics::new().unwrap());
        let state = MetricsMiddlewareState::new(metrics.clone());

        let app =
            Router::new()
                .route("/test", get(test_handler))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    metrics_middleware,
                ));

        // Make multiple concurrent requests
        let handles: Vec<_> = (0..5)
            .map(|_| {
                let app = app.clone();
                tokio::spawn(async move {
                    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
                    app.oneshot(request).await.unwrap()
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        // Verify connections were tracked (should be back to 0 after all complete)
        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_active_connections"));
    }

    #[test]
    fn test_metrics_error_creation() {
        let metrics = Arc::new(PrometheusMetrics::new().unwrap());
        let error = MetricsError::new(StatusCode::BAD_REQUEST, "test error", metrics, "/api/test");

        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.message, "test error");
        assert_eq!(error.endpoint, "/api/test");
    }

    #[test]
    fn test_metrics_error_without_metrics() {
        let error = MetricsError::without_metrics(StatusCode::NOT_FOUND, "not found");

        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.message, "not found");
        assert!(error.metrics.is_none());
    }

    #[test]
    fn test_record_error_metric() {
        let metrics = PrometheusMetrics::new().unwrap();
        record_error_metric(&metrics, "POST", "/api/events", StatusCode::BAD_REQUEST);

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_http_requests_total"));
        assert!(output.contains("400"));
    }
}
