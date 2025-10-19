// src/infrastructure/tracing.rs

//! Distributed tracing infrastructure for XZepr
//!
//! This module provides comprehensive tracing capabilities using the `tracing` crate
//! with support for structured logging, span creation, and Jaeger integration readiness.
//!
//! # Architecture
//!
//! The tracing infrastructure is built on the `tracing` ecosystem:
//! - `tracing` - Core tracing primitives (spans, events)
//! - `tracing-subscriber` - Subscriber implementation and utilities
//! - OpenTelemetry integration ready (add `tracing-opentelemetry` for Jaeger)
//!
//! # Features
//!
//! - Structured logging with JSON support for production
//! - Environment-based configuration
//! - Request ID tracking and correlation
//! - Span creation and propagation
//! - Performance measurement
//! - Multi-layer subscriber architecture

use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name for tracing
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Jaeger endpoint (for future OpenTelemetry integration)
    pub jaeger_endpoint: Option<String>,
    /// Sample rate (0.0 to 1.0) - reserved for future use
    pub sample_rate: f64,
    /// Environment (production, staging, development)
    pub environment: String,
    /// Enable tracing
    pub enabled: bool,
    /// Log level filter
    pub log_level: String,
    /// Use JSON formatting for logs
    pub json_logs: bool,
    /// Show target in logs
    pub show_target: bool,
    /// Show thread IDs in logs
    pub show_thread_ids: bool,
    /// Show thread names in logs
    pub show_thread_names: bool,
    /// Show file and line numbers
    pub show_file_line: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "xzepr".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            jaeger_endpoint: None,
            sample_rate: 1.0,
            environment: "development".to_string(),
            enabled: true,
            log_level: "info".to_string(),
            json_logs: false,
            show_target: true,
            show_thread_ids: false,
            show_thread_names: false,
            show_file_line: false,
        }
    }
}

impl TracingConfig {
    /// Creates a production tracing configuration
    pub fn production() -> Self {
        Self {
            service_name: "xzepr".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            jaeger_endpoint: std::env::var("XZEPR__JAEGER_ENDPOINT")
                .ok()
                .or_else(|| Some("http://jaeger:4317".to_string())),
            sample_rate: 0.1, // Sample 10% in production
            environment: "production".to_string(),
            enabled: true,
            log_level: "info".to_string(),
            json_logs: true,
            show_target: true,
            show_thread_ids: true,
            show_thread_names: true,
            show_file_line: false,
        }
    }

    /// Creates a development tracing configuration
    pub fn development() -> Self {
        Self {
            service_name: "xzepr".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            jaeger_endpoint: Some("http://localhost:4317".to_string()),
            sample_rate: 1.0, // Sample 100% in development
            environment: "development".to_string(),
            enabled: true,
            log_level: "debug".to_string(),
            json_logs: false,
            show_target: true,
            show_thread_ids: false,
            show_thread_names: false,
            show_file_line: true,
        }
    }

    /// Creates a staging tracing configuration
    pub fn staging() -> Self {
        Self {
            service_name: "xzepr".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            jaeger_endpoint: std::env::var("XZEPR__JAEGER_ENDPOINT")
                .ok()
                .or_else(|| Some("http://jaeger:4317".to_string())),
            sample_rate: 0.5, // Sample 50% in staging
            environment: "staging".to_string(),
            enabled: true,
            log_level: "info".to_string(),
            json_logs: true,
            show_target: true,
            show_thread_ids: true,
            show_thread_names: false,
            show_file_line: false,
        }
    }

    /// Creates configuration from environment variables
    pub fn from_env() -> Self {
        let environment =
            std::env::var("XZEPR__ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

        let mut config = match environment.as_str() {
            "production" => Self::production(),
            "staging" => Self::staging(),
            _ => Self::development(),
        };

        // Override with environment variables if set
        if let Ok(log_level) = std::env::var("XZEPR__LOG_LEVEL") {
            config.log_level = log_level;
        }

        if let Ok(json_logs) = std::env::var("XZEPR__JSON_LOGS") {
            config.json_logs = json_logs.parse().unwrap_or(config.json_logs);
        }

        config
    }
}

/// Initialize distributed tracing
///
/// Sets up tracing-subscriber with appropriate formatting and filtering.
/// This provides structured logging and prepares for OpenTelemetry integration.
///
/// # Arguments
///
/// * `config` - Tracing configuration
///
/// # Example
///
/// ```ignore
/// use xzepr::infrastructure::tracing::{init_tracing, TracingConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TracingConfig::production();
///     init_tracing(config)?;
///
///     tracing::info!("Application started");
///     Ok(())
/// }
/// ```
pub fn init_tracing(config: TracingConfig) -> Result<(), Box<dyn std::error::Error>> {
    if !config.enabled {
        tracing::warn!("Tracing is disabled");
        return Ok(());
    }

    // Create environment filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "{}={},tower_http=debug,axum=debug",
            config.service_name, config.log_level
        ))
    });

    // Build subscriber based on log format
    if config.json_logs {
        // JSON formatted logs for production
        let fmt_layer = fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_target(config.show_target)
            .with_level(true)
            .with_thread_ids(config.show_thread_ids)
            .with_thread_names(config.show_thread_names)
            .with_file(config.show_file_line)
            .with_line_number(config.show_file_line)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE);

        Registry::default().with(env_filter).with(fmt_layer).init();
    } else {
        // Human-readable logs for development
        let fmt_layer = fmt::layer()
            .with_target(config.show_target)
            .with_level(true)
            .with_thread_ids(config.show_thread_ids)
            .with_thread_names(config.show_thread_names)
            .with_file(config.show_file_line)
            .with_line_number(config.show_file_line)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE);

        Registry::default().with(env_filter).with(fmt_layer).init();
    }

    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = %config.environment,
        log_level = %config.log_level,
        json_logs = config.json_logs,
        "Tracing initialized"
    );

    if let Some(jaeger_endpoint) = &config.jaeger_endpoint {
        tracing::info!(
            jaeger_endpoint = %jaeger_endpoint,
            "Jaeger endpoint configured (OpenTelemetry integration ready)"
        );
    }

    Ok(())
}

/// Shutdown tracing gracefully
///
/// This ensures all pending spans are flushed before shutdown.
/// Currently a no-op but reserved for future OpenTelemetry integration.
pub fn shutdown_tracing() {
    tracing::info!("Shutting down tracing");
    // Future: Flush OpenTelemetry spans
}

/// Extract trace context from HTTP headers
///
/// Used to continue traces across service boundaries.
/// Returns a trace ID if present in standard headers.
pub fn extract_trace_context(headers: &axum::http::HeaderMap) -> Option<String> {
    // Check for traceparent header (W3C Trace Context)
    if let Some(traceparent) = headers.get("traceparent") {
        return traceparent.to_str().ok().map(|s| s.to_string());
    }

    // Check for X-Request-ID as fallback
    if let Some(request_id) = headers.get("x-request-id") {
        return request_id.to_str().ok().map(|s| s.to_string());
    }

    None
}

/// Inject trace context into HTTP headers
///
/// Used to propagate traces to downstream services.
pub fn inject_trace_context(headers: &mut axum::http::HeaderMap, trace_id: &str) {
    // Add X-Request-ID header
    if let Ok(header_value) = axum::http::HeaderValue::from_str(trace_id) {
        headers.insert("x-request-id", header_value);
    }

    // Future: Add traceparent header for W3C Trace Context
    // Format: version-trace_id-parent_id-trace_flags
    // Example: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
}

/// Generate a unique trace ID
///
/// Creates a trace ID compatible with distributed tracing systems.
pub fn generate_trace_id() -> String {
    use std::time::SystemTime;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_micros();

    // Generate a unique ID using timestamp and random component
    format!("{:016x}{:016x}", timestamp, rand::random::<u64>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "xzepr");
        assert_eq!(config.sample_rate, 1.0);
        assert_eq!(config.environment, "development");
        assert!(config.enabled);
        assert!(!config.json_logs);
    }

    #[test]
    fn test_production_config() {
        let config = TracingConfig::production();
        assert_eq!(config.environment, "production");
        assert_eq!(config.sample_rate, 0.1);
        assert!(config.json_logs);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_development_config() {
        let config = TracingConfig::development();
        assert_eq!(config.environment, "development");
        assert_eq!(config.sample_rate, 1.0);
        assert!(!config.json_logs);
        assert_eq!(config.log_level, "debug");
        assert!(config.show_file_line);
    }

    #[test]
    fn test_staging_config() {
        let config = TracingConfig::staging();
        assert_eq!(config.environment, "staging");
        assert_eq!(config.sample_rate, 0.5);
        assert!(config.json_logs);
    }

    #[test]
    fn test_config_clone() {
        let config = TracingConfig::default();
        let cloned = config.clone();
        assert_eq!(config.service_name, cloned.service_name);
        assert_eq!(config.sample_rate, cloned.sample_rate);
    }

    #[test]
    fn test_generate_trace_id() {
        let id1 = generate_trace_id();
        let id2 = generate_trace_id();

        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 32); // 16 bytes * 2 hex chars
        assert_eq!(id2.len(), 32);
    }

    #[test]
    fn test_trace_id_uniqueness() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let id = generate_trace_id();
            assert!(ids.insert(id), "Generated duplicate trace ID");
        }
    }

    #[test]
    fn test_extract_trace_context() {
        use axum::http::HeaderMap;

        let mut headers = HeaderMap::new();

        // Test with traceparent
        headers.insert(
            "traceparent",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
                .parse()
                .unwrap(),
        );
        let trace_id = extract_trace_context(&headers);
        assert!(trace_id.is_some());

        // Test with x-request-id
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "req-123-456".parse().unwrap());
        let trace_id = extract_trace_context(&headers);
        assert_eq!(trace_id, Some("req-123-456".to_string()));

        // Test with no headers
        let headers = HeaderMap::new();
        let trace_id = extract_trace_context(&headers);
        assert!(trace_id.is_none());
    }

    #[test]
    fn test_inject_trace_context() {
        use axum::http::HeaderMap;

        let mut headers = HeaderMap::new();
        inject_trace_context(&mut headers, "test-trace-123");

        assert!(headers.contains_key("x-request-id"));
        assert_eq!(
            headers.get("x-request-id").unwrap().to_str().unwrap(),
            "test-trace-123"
        );
    }
}
