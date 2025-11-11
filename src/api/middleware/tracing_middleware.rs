// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/middleware/tracing_middleware.rs

//! Distributed tracing middleware for automatic span creation
//!
//! This middleware automatically creates OpenTelemetry spans for all HTTP requests,
//! capturing request metadata, response status, and timing information.

use axum::{
    extract::{MatchedPath, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;
use tracing::{info_span, Instrument};

/// Tracing middleware that creates spans for HTTP requests
///
/// This middleware automatically:
/// - Creates a span for each request
/// - Captures request method, path, and headers
/// - Records response status and duration
/// - Propagates trace context across service boundaries
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware};
/// use xzepr::api::middleware::tracing_middleware::tracing_middleware;
///
/// let app = Router::new()
///     .route("/api/events", get(handler))
///     .layer(middleware::from_fn(tracing_middleware));
/// ```
pub async fn tracing_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();

    // Extract request metadata
    let method = request.method().to_string();
    let uri = request.uri().clone();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());

    // Extract request ID if present
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Extract trace context from headers
    let trace_id = crate::infrastructure::tracing::extract_trace_context(request.headers())
        .unwrap_or_else(|| request_id.to_string());

    // Create span with request metadata
    let span = info_span!(
        "http_request",
        http.method = %method,
        http.route = %path,
        http.target = %uri,
        http.request_id = %request_id,
        http.trace_id = %trace_id,
        http.status_code = tracing::field::Empty,
        http.duration_ms = tracing::field::Empty,
    );

    // Process request with span
    process_request_with_span(request, next, start, span).await
}

/// Process the request within a span
async fn process_request_with_span(
    request: Request,
    next: Next,
    start: Instant,
    span: tracing::Span,
) -> Response {
    async move {
        // Process the request
        let response = next.run(request).await;

        // Record response metadata
        let status = response.status();
        let duration = start.elapsed();

        // Update span with response information
        tracing::Span::current().record("http.status_code", status.as_u16());
        tracing::Span::current().record("http.duration_ms", duration.as_millis() as u64);

        // Log request completion
        if status.is_server_error() {
            tracing::error!(
                status = %status,
                duration_ms = duration.as_millis(),
                "HTTP request failed"
            );
        } else if status.is_client_error() {
            tracing::warn!(
                status = %status,
                duration_ms = duration.as_millis(),
                "HTTP request client error"
            );
        } else {
            tracing::info!(
                status = %status,
                duration_ms = duration.as_millis(),
                "HTTP request completed"
            );
        }

        response
    }
    .instrument(span)
    .await
}

/// Enhanced tracing middleware with custom span attributes
///
/// Allows adding custom attributes to the span from request extensions.
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware};
/// use xzepr::api::middleware::tracing_middleware::enhanced_tracing_middleware;
///
/// let app = Router::new()
///     .route("/api/events", get(handler))
///     .layer(middleware::from_fn(enhanced_tracing_middleware));
/// ```
pub async fn enhanced_tracing_middleware(mut request: Request, next: Next) -> Response {
    let start = Instant::now();

    // Extract request metadata
    let method = request.method().to_string();
    let uri = request.uri().clone();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());

    // Extract request ID or generate one
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(generate_request_id);

    // Extract user information if authenticated
    let user_id = request
        .extensions()
        .get::<crate::api::middleware::jwt::AuthenticatedUser>()
        .map(|u| u.user_id().to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    // Extract trace context
    let trace_id = crate::infrastructure::tracing::extract_trace_context(request.headers())
        .unwrap_or_else(|| request_id.clone());

    // Create comprehensive span
    let span = info_span!(
        "http_request",
        http.method = %method,
        http.route = %path,
        http.target = %uri,
        http.request_id = %request_id,
        http.trace_id = %trace_id,
        http.user_id = %user_id,
        http.status_code = tracing::field::Empty,
        http.duration_ms = tracing::field::Empty,
        http.response_size = tracing::field::Empty,
    );

    // Add request ID to extensions for downstream use
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    // Process request with span
    async move {
        let response = next.run(request).await;
        finalize_span(&tracing::Span::current(), start, &response);
        response
    }
    .instrument(span)
    .await
}

/// Finalize span with response information
fn finalize_span(span: &tracing::Span, start: Instant, response: &Response) {
    let status = response.status();
    let duration = start.elapsed();

    span.record("http.status_code", status.as_u16());
    span.record("http.duration_ms", duration.as_millis() as u64);

    // Try to get response size from content-length header
    if let Some(content_length) = response.headers().get("content-length") {
        if let Ok(size_str) = content_length.to_str() {
            if let Ok(size) = size_str.parse::<u64>() {
                span.record("http.response_size", size);
            }
        }
    }
}

/// Request ID wrapper for extension storage
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// Generate a unique request ID
fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    format!("req-{}-{}", timestamp, counter)
}

/// Middleware that injects request ID into responses
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Get or generate request ID
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(generate_request_id);

    // Store in extensions
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    // Process request
    let mut response = next.run(request).await;

    // Add request ID to response headers
    if let Ok(header_value) = axum::http::HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", header_value);
    }

    response
}

/// Error response that includes tracing context
#[derive(Debug)]
pub struct TracedError {
    status: StatusCode,
    message: String,
    request_id: Option<String>,
}

impl TracedError {
    /// Creates a new traced error
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            request_id: None,
        }
    }

    /// Creates a traced error with request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

impl IntoResponse for TracedError {
    fn into_response(self) -> Response {
        // Log the error with trace context
        tracing::error!(
            status = %self.status,
            message = %self.message,
            request_id = ?self.request_id,
            "Request failed"
        );

        let body = if let Some(request_id) = &self.request_id {
            serde_json::json!({
                "error": self.message,
                "request_id": request_id,
                "status": self.status.as_u16(),
            })
            .to_string()
        } else {
            serde_json::json!({
                "error": self.message,
                "status": self.status.as_u16(),
            })
            .to_string()
        };

        (self.status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body, http::Request, middleware, response::IntoResponse, routing::get, Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> impl IntoResponse {
        (StatusCode::OK, "test response")
    }

    #[tokio::test]
    async fn test_request_id_generation() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("req-"));
        assert!(id2.starts_with("req-"));
    }

    #[tokio::test]
    async fn test_request_id_middleware() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(request_id_middleware));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should have request ID in response headers
        assert!(response.headers().contains_key("x-request-id"));
    }

    #[tokio::test]
    async fn test_request_id_preservation() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(request_id_middleware));

        let custom_id = "custom-request-123";
        let request = Request::builder()
            .uri("/test")
            .header("x-request-id", custom_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should preserve the custom request ID
        let response_id = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap();

        assert_eq!(response_id, custom_id);
    }

    #[test]
    fn test_traced_error_creation() {
        let error = TracedError::new(StatusCode::BAD_REQUEST, "test error");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.message, "test error");
        assert!(error.request_id.is_none());
    }

    #[test]
    fn test_traced_error_with_request_id() {
        let error = TracedError::new(StatusCode::NOT_FOUND, "not found").with_request_id("req-123");

        assert_eq!(error.status, StatusCode::NOT_FOUND);
        assert_eq!(error.request_id, Some("req-123".to_string()));
    }

    #[test]
    fn test_request_id_wrapper() {
        let request_id = RequestId("test-123".to_string());
        let cloned = request_id.clone();
        assert_eq!(request_id.0, cloned.0);
    }
}
