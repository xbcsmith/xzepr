// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/metrics.rs

use prometheus::{
    CounterVec, Encoder, Gauge, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder,
};
use std::sync::Arc;

/// Prometheus metrics for security and application monitoring
#[derive(Clone)]
pub struct PrometheusMetrics {
    registry: Arc<Registry>,

    // Security metrics
    auth_failures_total: CounterVec,
    auth_success_total: CounterVec,
    rate_limit_rejections_total: CounterVec,
    cors_violations_total: CounterVec,
    validation_errors_total: CounterVec,
    graphql_complexity_violations_total: CounterVec,

    // Application metrics
    http_requests_total: CounterVec,
    http_request_duration_seconds: HistogramVec,
    active_connections: Gauge,

    // System metrics
    uptime_seconds: Gauge,
    #[allow(dead_code)]
    info: GaugeVec,
}

impl PrometheusMetrics {
    /// Creates a new Prometheus metrics instance
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // Security metrics
        let auth_failures_total = CounterVec::new(
            Opts::new(
                "xzepr_auth_failures_total",
                "Total number of authentication failures",
            ),
            &["reason", "client_id"],
        )?;
        registry.register(Box::new(auth_failures_total.clone()))?;

        let auth_success_total = CounterVec::new(
            Opts::new(
                "xzepr_auth_success_total",
                "Total number of successful authentications",
            ),
            &["method", "user_id"],
        )?;
        registry.register(Box::new(auth_success_total.clone()))?;

        let rate_limit_rejections_total = CounterVec::new(
            Opts::new(
                "xzepr_rate_limit_rejections_total",
                "Total number of rate limit rejections",
            ),
            &["endpoint", "client_id"],
        )?;
        registry.register(Box::new(rate_limit_rejections_total.clone()))?;

        let cors_violations_total = CounterVec::new(
            Opts::new(
                "xzepr_cors_violations_total",
                "Total number of CORS policy violations",
            ),
            &["origin", "endpoint"],
        )?;
        registry.register(Box::new(cors_violations_total.clone()))?;

        let validation_errors_total = CounterVec::new(
            Opts::new(
                "xzepr_validation_errors_total",
                "Total number of input validation errors",
            ),
            &["endpoint", "field"],
        )?;
        registry.register(Box::new(validation_errors_total.clone()))?;

        let graphql_complexity_violations_total = CounterVec::new(
            Opts::new(
                "xzepr_graphql_complexity_violations_total",
                "Total number of GraphQL query complexity violations",
            ),
            &["client_id"],
        )?;
        registry.register(Box::new(graphql_complexity_violations_total.clone()))?;

        // Application metrics
        let http_requests_total = CounterVec::new(
            Opts::new("xzepr_http_requests_total", "Total number of HTTP requests"),
            &["method", "path", "status"],
        )?;
        registry.register(Box::new(http_requests_total.clone()))?;

        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "xzepr_http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
            &["method", "path", "status"],
        )?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;

        let active_connections =
            Gauge::new("xzepr_active_connections", "Number of active connections")?;
        registry.register(Box::new(active_connections.clone()))?;

        // System metrics
        let uptime_seconds = Gauge::new("xzepr_uptime_seconds", "Server uptime in seconds")?;
        registry.register(Box::new(uptime_seconds.clone()))?;

        let info = GaugeVec::new(Opts::new("xzepr_info", "Server information"), &["version"])?;
        registry.register(Box::new(info.clone()))?;

        // Set initial info values
        info.with_label_values(&[env!("CARGO_PKG_VERSION")])
            .set(1.0);

        Ok(Self {
            registry: Arc::new(registry),
            auth_failures_total,
            auth_success_total,
            rate_limit_rejections_total,
            cors_violations_total,
            validation_errors_total,
            graphql_complexity_violations_total,
            http_requests_total,
            http_request_duration_seconds,
            active_connections,
            uptime_seconds,
            info,
        })
    }

    /// Records an authentication failure
    pub fn record_auth_failure(&self, reason: &str, client_id: &str) {
        self.auth_failures_total
            .with_label_values(&[reason, client_id])
            .inc();
    }

    /// Records a successful authentication
    pub fn record_auth_success(&self, method: &str, user_id: &str) {
        self.auth_success_total
            .with_label_values(&[method, user_id])
            .inc();
    }

    /// Records a rate limit rejection
    pub fn record_rate_limit_rejection(&self, endpoint: &str, client_id: &str) {
        self.rate_limit_rejections_total
            .with_label_values(&[endpoint, client_id])
            .inc();
    }

    /// Records a CORS violation
    pub fn record_cors_violation(&self, origin: &str, endpoint: &str) {
        self.cors_violations_total
            .with_label_values(&[origin, endpoint])
            .inc();
    }

    /// Records a validation error
    pub fn record_validation_error(&self, endpoint: &str, field: &str) {
        self.validation_errors_total
            .with_label_values(&[endpoint, field])
            .inc();
    }

    /// Records a GraphQL complexity violation
    pub fn record_complexity_violation(&self, client_id: &str) {
        self.graphql_complexity_violations_total
            .with_label_values(&[client_id])
            .inc();
    }

    /// Records an HTTP request
    pub fn record_http_request(&self, method: &str, path: &str, status: u16, duration_secs: f64) {
        let status_str = status.to_string();

        self.http_requests_total
            .with_label_values(&[method, path, &status_str])
            .inc();

        self.http_request_duration_seconds
            .with_label_values(&[method, path, &status_str])
            .observe(duration_secs);
    }

    /// Sets the number of active connections
    pub fn set_active_connections(&self, count: i64) {
        self.active_connections.set(count as f64);
    }

    /// Increments active connections
    pub fn increment_active_connections(&self) {
        self.active_connections.inc();
    }

    /// Decrements active connections
    pub fn decrement_active_connections(&self) {
        self.active_connections.dec();
    }

    /// Updates the uptime gauge
    pub fn update_uptime(&self, uptime_secs: u64) {
        self.uptime_seconds.set(uptime_secs as f64);
    }

    /// Gathers all metrics and returns them in Prometheus text format
    pub fn gather(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer).map_err(|e| {
            prometheus::Error::Msg(format!("Failed to encode metrics as UTF-8: {}", e))
        })
    }

    /// Gets a reference to the registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create Prometheus metrics")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = PrometheusMetrics::new().unwrap();
        assert!(metrics.gather().is_ok());
    }

    #[test]
    fn test_record_auth_failure() {
        let metrics = PrometheusMetrics::new().unwrap();
        metrics.record_auth_failure("invalid_token", "client123");

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_auth_failures_total"));
    }

    #[test]
    fn test_record_auth_success() {
        let metrics = PrometheusMetrics::new().unwrap();
        metrics.record_auth_success("jwt", "user123");

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_auth_success_total"));
    }

    #[test]
    fn test_record_rate_limit_rejection() {
        let metrics = PrometheusMetrics::new().unwrap();
        metrics.record_rate_limit_rejection("/api/events", "client123");

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_rate_limit_rejections_total"));
    }

    #[test]
    fn test_record_cors_violation() {
        let metrics = PrometheusMetrics::new().unwrap();
        metrics.record_cors_violation("https://evil.com", "/api/events");

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_cors_violations_total"));
    }

    #[test]
    fn test_record_validation_error() {
        let metrics = PrometheusMetrics::new().unwrap();
        metrics.record_validation_error("/api/events", "name");

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_validation_errors_total"));
    }

    #[test]
    fn test_record_http_request() {
        let metrics = PrometheusMetrics::new().unwrap();
        metrics.record_http_request("GET", "/api/events", 200, 0.045);

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_http_requests_total"));
        assert!(output.contains("xzepr_http_request_duration_seconds"));
    }

    #[test]
    fn test_active_connections() {
        let metrics = PrometheusMetrics::new().unwrap();

        metrics.increment_active_connections();
        metrics.increment_active_connections();
        metrics.decrement_active_connections();

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_active_connections"));
    }

    #[test]
    fn test_uptime() {
        let metrics = PrometheusMetrics::new().unwrap();
        metrics.update_uptime(3600);

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_uptime_seconds"));
        assert!(output.contains("3600"));
    }

    #[test]
    fn test_info_metric() {
        let metrics = PrometheusMetrics::new().unwrap();

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_info"));
        assert!(output.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_multiple_recordings() {
        let metrics = PrometheusMetrics::new().unwrap();

        // Record multiple events
        for i in 0..10 {
            metrics.record_http_request("GET", "/api/events", 200, 0.01 * i as f64);
        }

        for i in 0..5 {
            metrics.record_auth_failure("invalid_token", &format!("client{}", i));
        }

        let output = metrics.gather().unwrap();
        assert!(output.contains("xzepr_http_requests_total"));
        assert!(output.contains("xzepr_auth_failures_total"));
    }
}
