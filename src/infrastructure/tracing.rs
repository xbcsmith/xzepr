// src/infrastructure/tracing.rs

//! Distributed tracing infrastructure for XZepr
//!
//! This module provides comprehensive tracing capabilities using the `tracing` crate
//! with full OpenTelemetry OTLP exporter integration for Jaeger.
//!
//! # Architecture
//!
//! The tracing infrastructure is built on the `tracing` ecosystem:
//! - `tracing` - Core tracing primitives (spans, events)
//! - `tracing-subscriber` - Subscriber implementation and utilities
//! - `tracing-opentelemetry` - OpenTelemetry bridge for tracing
//! - `opentelemetry` - OpenTelemetry API and SDK
//! - `opentelemetry-otlp` - OTLP exporter for Jaeger/Collector
//!
//! # Features
//!
//! - Structured logging with JSON support for production
//! - OpenTelemetry OTLP exporter integration
//! - Jaeger trace collection
//! - Environment-based configuration
//! - Request ID tracking and correlation
//! - Span creation and propagation
//! - Performance measurement
//! - Multi-layer subscriber architecture
//! - Configurable sampling rates

use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{self as sdktrace, Config, RandomIdGenerator, Sampler},
    Resource,
};
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
    /// OTLP endpoint for trace export (e.g., http://jaeger:4317)
    pub otlp_endpoint: Option<String>,
    /// Sample rate (0.0 to 1.0) - controls what percentage of traces are collected
    pub sample_rate: f64,
    /// Environment (production, staging, development)
    pub environment: String,
    /// Enable tracing
    pub enabled: bool,
    /// Enable OTLP export (requires otlp_endpoint)
    pub enable_otlp: bool,
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
            otlp_endpoint: None,
            sample_rate: 1.0,
            environment: "development".to_string(),
            enabled: true,
            enable_otlp: false,
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
            otlp_endpoint: std::env::var("XZEPR__OTLP_ENDPOINT")
                .ok()
                .or_else(|| Some("http://jaeger:4317".to_string())),
            sample_rate: 0.1, // Sample 10% in production
            environment: "production".to_string(),
            enabled: true,
            enable_otlp: true,
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
            otlp_endpoint: std::env::var("XZEPR__OTLP_ENDPOINT")
                .ok()
                .or_else(|| Some("http://localhost:4317".to_string())),
            sample_rate: 1.0, // Sample 100% in development
            environment: "development".to_string(),
            enabled: true,
            enable_otlp: false, // Disabled by default in dev, enable via env var
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
            otlp_endpoint: std::env::var("XZEPR__OTLP_ENDPOINT")
                .ok()
                .or_else(|| Some("http://jaeger:4317".to_string())),
            sample_rate: 0.5, // Sample 50% in staging
            environment: "staging".to_string(),
            enabled: true,
            enable_otlp: true,
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

        if let Ok(enable_otlp) = std::env::var("XZEPR__ENABLE_OTLP") {
            config.enable_otlp = enable_otlp.parse().unwrap_or(config.enable_otlp);
        }

        if let Ok(otlp_endpoint) = std::env::var("XZEPR__OTLP_ENDPOINT") {
            config.otlp_endpoint = Some(otlp_endpoint);
        }

        config
    }
}

/// Initialize OpenTelemetry tracer with OTLP exporter
///
/// Creates and configures an OpenTelemetry tracer that exports spans to an OTLP endpoint.
/// This is typically Jaeger or another OpenTelemetry collector.
///
/// # Arguments
///
/// * `config` - Tracing configuration with OTLP endpoint and sampling
///
/// # Returns
///
/// Returns a tracer provider on success, or an error if initialization fails.
fn init_otlp_tracer(
    config: &TracingConfig,
) -> Result<opentelemetry_sdk::trace::TracerProvider, Box<dyn std::error::Error>> {
    let otlp_endpoint = config
        .otlp_endpoint
        .as_ref()
        .ok_or("OTLP endpoint not configured")?;

    tracing::info!(
        otlp_endpoint = %otlp_endpoint,
        sample_rate = config.sample_rate,
        "Initializing OTLP exporter"
    );

    // Create OTLP exporter using new_pipeline API
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint)
        .build_span_exporter()?;

    // Configure sampler based on sample rate
    let sampler = if config.sample_rate >= 1.0 {
        Sampler::AlwaysOn
    } else if config.sample_rate <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.sample_rate)
    };

    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ]);

    // Build tracer provider
    let tracer_provider = sdktrace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_config(
            Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .build();

    Ok(tracer_provider)
}

/// Initialize distributed tracing
///
/// Sets up tracing-subscriber with structured logging and optional OpenTelemetry OTLP export.
/// When OTLP is enabled, traces are exported to the configured endpoint (typically Jaeger).
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
///
/// # OTLP Configuration
///
/// Enable OTLP export with environment variables:
/// - `XZEPR__ENABLE_OTLP=true` - Enable OTLP exporter
/// - `XZEPR__OTLP_ENDPOINT=http://jaeger:4317` - OTLP collector endpoint
/// - `XZEPR__ENVIRONMENT=production` - Set environment (affects sampling)
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

    // Initialize OTLP tracer if enabled and get tracer
    let otlp_tracer = if config.enable_otlp {
        match init_otlp_tracer(&config) {
            Ok(tracer_provider) => {
                tracing::info!(
                    otlp_endpoint = ?config.otlp_endpoint,
                    sample_rate = config.sample_rate,
                    "OTLP exporter initialized successfully"
                );
                // Set global tracer provider
                global::set_tracer_provider(tracer_provider.clone());

                // Create tracer from provider
                Some(tracer_provider.tracer(config.service_name.clone()))
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to initialize OTLP exporter, continuing without it"
                );
                None
            }
        }
    } else {
        None
    };

    // Build subscriber based on log format and OTLP configuration
    // Use a simpler approach that avoids complex layer type constraints
    match (config.json_logs, otlp_tracer) {
        (true, Some(tracer)) => {
            // JSON logs with OTLP
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

            let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            Registry::default()
                .with(env_filter)
                .with(telemetry_layer)
                .with(fmt_layer)
                .init();
        }
        (true, None) => {
            // JSON logs without OTLP
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
        }
        (false, Some(tracer)) => {
            // Human-readable logs with OTLP
            let fmt_layer = fmt::layer()
                .with_target(config.show_target)
                .with_level(true)
                .with_thread_ids(config.show_thread_ids)
                .with_thread_names(config.show_thread_names)
                .with_file(config.show_file_line)
                .with_line_number(config.show_file_line)
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE);

            let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            Registry::default()
                .with(env_filter)
                .with(telemetry_layer)
                .with(fmt_layer)
                .init();
        }
        (false, None) => {
            // Human-readable logs without OTLP
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
    }

    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = %config.environment,
        log_level = %config.log_level,
        json_logs = config.json_logs,
        otlp_enabled = config.enable_otlp,
        "Tracing initialized"
    );

    if config.enable_otlp {
        if let Some(otlp_endpoint) = &config.otlp_endpoint {
            tracing::info!(
                otlp_endpoint = %otlp_endpoint,
                sample_rate = config.sample_rate,
                "OTLP exporter configured and active"
            );
        }
    }

    Ok(())
}

/// Shutdown tracing gracefully
///
/// This ensures all pending spans are flushed to the OTLP collector before shutdown.
/// This is critical to avoid losing traces when the application terminates.
///
/// # Example
///
/// ```ignore
/// use xzepr::infrastructure::tracing::shutdown_tracing;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // ... application code ...
///
///     // Shutdown tracing before exit
///     shutdown_tracing();
///     Ok(())
/// }
/// ```
pub fn shutdown_tracing() {
    tracing::info!("Shutting down tracing and flushing spans");

    // Shutdown OpenTelemetry global tracer provider to flush pending spans
    global::shutdown_tracer_provider();

    tracing::info!("Tracing shutdown complete");
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
        assert!(config.enable_otlp);
        assert!(config.otlp_endpoint.is_some());
    }

    #[test]
    fn test_development_config() {
        let config = TracingConfig::development();
        assert_eq!(config.environment, "development");
        assert_eq!(config.sample_rate, 1.0);
        assert!(!config.json_logs);
        assert_eq!(config.log_level, "debug");
        assert!(config.show_file_line);
        assert!(!config.enable_otlp); // Disabled by default in dev
        assert!(config.otlp_endpoint.is_some()); // But endpoint is configured
    }

    #[test]
    fn test_staging_config() {
        let config = TracingConfig::staging();
        assert_eq!(config.environment, "staging");
        assert_eq!(config.sample_rate, 0.5);
        assert!(config.json_logs);
        assert!(config.enable_otlp);
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
