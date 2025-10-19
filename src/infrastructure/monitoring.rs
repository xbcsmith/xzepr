// src/infrastructure/monitoring.rs

use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

use crate::infrastructure::metrics::PrometheusMetrics;

/// Security monitoring metrics and logging
#[derive(Clone)]
pub struct SecurityMonitor {
    start_time: Instant,
    metrics: Option<Arc<PrometheusMetrics>>,
}

impl SecurityMonitor {
    /// Creates a new security monitor
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            metrics: None,
        }
    }

    /// Creates a new security monitor with Prometheus metrics
    pub fn new_with_metrics(metrics: Arc<PrometheusMetrics>) -> Self {
        Self {
            start_time: Instant::now(),
            metrics: Some(metrics),
        }
    }

    /// Sets the Prometheus metrics
    pub fn with_metrics(mut self, metrics: Arc<PrometheusMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Records a rate limit rejection
    pub fn record_rate_limit_rejection(&self, client_id: &str, endpoint: &str, limit: u32) {
        warn!(
            event = "rate_limit_exceeded",
            client_id = %client_id,
            endpoint = %endpoint,
            limit = %limit,
            "Rate limit exceeded"
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_rate_limit_rejection(endpoint, client_id);
        }
    }

    /// Records an authentication failure
    pub fn record_auth_failure(&self, client_id: &str, reason: &str) {
        warn!(
            event = "authentication_failed",
            client_id = %client_id,
            reason = %reason,
            "Authentication failed"
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_auth_failure(reason, client_id);
        }
    }

    /// Records a CORS violation
    pub fn record_cors_violation(&self, origin: &str, endpoint: &str) {
        warn!(
            event = "cors_violation",
            origin = %origin,
            endpoint = %endpoint,
            "CORS policy violation"
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_cors_violation(origin, endpoint);
        }
    }

    /// Records an input validation error
    pub fn record_validation_error(&self, endpoint: &str, field: &str, error: &str) {
        warn!(
            event = "validation_error",
            endpoint = %endpoint,
            field = %field,
            error = %error,
            "Input validation failed"
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_validation_error(endpoint, field);
        }
    }

    /// Records a GraphQL query complexity violation
    pub fn record_complexity_violation(&self, client_id: &str, complexity: usize, max: usize) {
        warn!(
            event = "query_complexity_violation",
            client_id = %client_id,
            complexity = %complexity,
            max_complexity = %max,
            "GraphQL query complexity exceeded"
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_complexity_violation(client_id);
        }
    }

    /// Records a successful authentication
    pub fn record_auth_success(&self, user_id: &str, method: &str) {
        info!(
            event = "authentication_success",
            user_id = %user_id,
            method = %method,
            "User authenticated successfully"
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_auth_success(method, user_id);
        }
    }

    /// Records a successful request
    pub fn record_request(&self, method: &str, path: &str, status: u16, duration_ms: u64) {
        info!(
            event = "http_request",
            method = %method,
            path = %path,
            status = %status,
            duration_ms = %duration_ms,
            "HTTP request completed"
        );

        if let Some(metrics) = &self.metrics {
            let duration_secs = duration_ms as f64 / 1000.0;
            metrics.record_http_request(method, path, status, duration_secs);
        }
    }

    /// Records a security event
    pub fn record_security_event(&self, event_type: &str, details: &str) {
        warn!(
            event = "security_event",
            event_type = %event_type,
            details = %details,
            "Security event detected"
        );
    }

    /// Records an error
    pub fn record_error(&self, context: &str, error: &str) {
        error!(
            event = "error",
            context = %context,
            error = %error,
            "Error occurred"
        );
    }

    /// Gets the uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        let uptime = self.start_time.elapsed().as_secs();

        // Update Prometheus uptime metric if available
        if let Some(metrics) = &self.metrics {
            metrics.update_uptime(uptime);
        }

        uptime
    }

    /// Gets the Prometheus metrics instance
    pub fn metrics(&self) -> Option<&Arc<PrometheusMetrics>> {
        self.metrics.as_ref()
    }
}

impl Default for SecurityMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics collector for security events
#[derive(Clone)]
pub struct SecurityMetrics {
    monitor: Arc<SecurityMonitor>,
}

impl SecurityMetrics {
    /// Creates a new security metrics collector
    pub fn new() -> Self {
        Self {
            monitor: Arc::new(SecurityMonitor::new()),
        }
    }

    /// Gets the underlying monitor
    pub fn monitor(&self) -> &SecurityMonitor {
        &self.monitor
    }

    /// Records a metric value
    pub fn record_metric(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let labels_str: Vec<String> = labels.iter().map(|(k, v)| format!("{}={}", k, v)).collect();

        info!(
            metric = %name,
            value = %value,
            labels = %labels_str.join(","),
            "Metric recorded"
        );
    }

    /// Increments a counter
    pub fn increment_counter(&self, name: &str, labels: &[(&str, &str)]) {
        self.record_metric(name, 1.0, labels);
    }

    /// Records a histogram value
    pub fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        self.record_metric(name, value, labels);
    }

    /// Records a gauge value
    pub fn record_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        self.record_metric(name, value, labels);
    }
}

impl Default for SecurityMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// System is healthy
    Healthy,
    /// System is degraded but operational
    Degraded,
    /// System is unhealthy
    Unhealthy,
}

/// Component health information
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Health status
    pub status: HealthStatus,
    /// Optional message
    pub message: Option<String>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Overall status
    pub status: HealthStatus,
    /// System version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Component health checks
    pub components: Vec<ComponentHealth>,
}

impl HealthCheck {
    /// Creates a new health check result
    pub fn new(version: String, uptime_seconds: u64) -> Self {
        Self {
            status: HealthStatus::Healthy,
            version,
            uptime_seconds,
            components: Vec::new(),
        }
    }

    /// Adds a component health check
    pub fn add_component(&mut self, component: ComponentHealth) {
        // Update overall status if component is unhealthy
        if component.status == HealthStatus::Unhealthy {
            self.status = HealthStatus::Unhealthy;
        } else if component.status == HealthStatus::Degraded && self.status == HealthStatus::Healthy
        {
            self.status = HealthStatus::Degraded;
        }
        self.components.push(component);
    }

    /// Checks if the system is healthy
    pub fn is_healthy(&self) -> bool {
        self.status == HealthStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_monitor_creation() {
        let monitor = SecurityMonitor::new();
        assert!(monitor.uptime_seconds() < 1);
    }

    #[test]
    fn test_security_metrics_creation() {
        let metrics = SecurityMetrics::new();
        assert!(metrics.monitor().uptime_seconds() < 1);
    }

    #[test]
    fn test_health_check_healthy() {
        let check = HealthCheck::new("1.0.0".to_string(), 100);
        assert!(check.is_healthy());
        assert_eq!(check.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_check_degraded() {
        let mut check = HealthCheck::new("1.0.0".to_string(), 100);
        check.add_component(ComponentHealth {
            name: "database".to_string(),
            status: HealthStatus::Degraded,
            message: Some("High latency".to_string()),
            response_time_ms: Some(500),
        });
        assert_eq!(check.status, HealthStatus::Degraded);
        assert!(!check.is_healthy());
    }

    #[test]
    fn test_health_check_unhealthy() {
        let mut check = HealthCheck::new("1.0.0".to_string(), 100);
        check.add_component(ComponentHealth {
            name: "database".to_string(),
            status: HealthStatus::Unhealthy,
            message: Some("Connection failed".to_string()),
            response_time_ms: None,
        });
        assert_eq!(check.status, HealthStatus::Unhealthy);
        assert!(!check.is_healthy());
    }

    #[test]
    fn test_health_check_multiple_components() {
        let mut check = HealthCheck::new("1.0.0".to_string(), 100);

        check.add_component(ComponentHealth {
            name: "database".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            response_time_ms: Some(10),
        });

        check.add_component(ComponentHealth {
            name: "redis".to_string(),
            status: HealthStatus::Healthy,
            message: None,
            response_time_ms: Some(5),
        });

        assert_eq!(check.status, HealthStatus::Healthy);
        assert_eq!(check.components.len(), 2);
    }

    #[test]
    fn test_monitor_records_events() {
        let monitor = SecurityMonitor::new();

        // These should not panic
        monitor.record_rate_limit_rejection("client1", "/api/events", 10);
        monitor.record_auth_failure("client2", "Invalid token");
        monitor.record_cors_violation("https://evil.com", "/api/events");
        monitor.record_validation_error("/api/events", "name", "too short");
        monitor.record_complexity_violation("client3", 150, 100);
        monitor.record_auth_success("user123", "jwt");
        monitor.record_request("GET", "/api/events", 200, 45);
        monitor.record_security_event("suspicious_activity", "Multiple failed logins");
        monitor.record_error("database", "Connection timeout");
    }

    #[test]
    fn test_metrics_collector() {
        let metrics = SecurityMetrics::new();

        // These should not panic
        metrics.increment_counter("requests_total", &[("method", "GET"), ("status", "200")]);
        metrics.record_histogram(
            "request_duration_seconds",
            0.045,
            &[("endpoint", "/api/events")],
        );
        metrics.record_gauge("active_connections", 42.0, &[]);
    }
}
