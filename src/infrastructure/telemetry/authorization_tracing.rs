// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Authorization Tracing
//!
//! This module provides OpenTelemetry span instrumentation for authorization
//! operations including OPA policy evaluation and legacy RBAC fallback.

use tracing::{debug, info, warn, Span};

/// Creates a span for authorization evaluation
///
/// # Arguments
///
/// * `user_id` - User identifier
/// * `action` - Action being authorized
/// * `resource_type` - Type of resource
/// * `resource_id` - Resource identifier
///
/// # Returns
///
/// Returns a configured tracing span for the authorization operation
///
/// # Examples
///
/// ```
/// use xzepr::infrastructure::telemetry::authorization_tracing::authorization_span;
///
/// let span = authorization_span("user123", "read", "event_receiver", "recv456");
/// let _guard = span.enter();
/// // Perform authorization...
/// ```
pub fn authorization_span(
    user_id: &str,
    action: &str,
    resource_type: &str,
    resource_id: &str,
) -> Span {
    tracing::info_span!(
        "authorization",
        user_id = %user_id,
        action = %action,
        resource_type = %resource_type,
        resource_id = %resource_id,
        otel.kind = "internal",
        otel.status_code = tracing::field::Empty,
    )
}

/// Records authorization decision on the current span
///
/// # Arguments
///
/// * `decision` - Whether access was granted
/// * `duration_ms` - Duration of authorization check in milliseconds
/// * `fallback_used` - Whether fallback to legacy RBAC was used
/// * `policy_version` - OPA policy version (if available)
pub fn record_authorization_decision(
    decision: bool,
    duration_ms: u64,
    fallback_used: bool,
    policy_version: Option<&str>,
) {
    let span = Span::current();

    span.record("authorization.decision", decision);
    span.record("authorization.duration_ms", duration_ms);
    span.record("authorization.fallback_used", fallback_used);

    if let Some(version) = policy_version {
        span.record("authorization.policy_version", version);
    }

    span.record(
        "otel.status_code",
        if decision { "OK" } else { "ERROR" },
    );

    if decision {
        debug!(
            decision = true,
            duration_ms = duration_ms,
            fallback_used = fallback_used,
            "Authorization granted"
        );
    } else {
        warn!(
            decision = false,
            duration_ms = duration_ms,
            fallback_used = fallback_used,
            "Authorization denied"
        );
    }
}

/// Records cache hit/miss on the current span
///
/// # Arguments
///
/// * `cache_hit` - Whether the authorization was served from cache
pub fn record_cache_status(cache_hit: bool) {
    let span = Span::current();
    span.record("authorization.cache_hit", cache_hit);

    debug!(cache_hit = cache_hit, "Authorization cache status");
}

/// Records OPA evaluation details on the current span
///
/// # Arguments
///
/// * `opa_available` - Whether OPA service was available
/// * `opa_duration_ms` - Time taken for OPA evaluation in milliseconds
/// * `opa_error` - Optional error message if OPA evaluation failed
pub fn record_opa_evaluation(
    opa_available: bool,
    opa_duration_ms: Option<u64>,
    opa_error: Option<&str>,
) {
    let span = Span::current();
    span.record("opa.available", opa_available);

    if let Some(duration) = opa_duration_ms {
        span.record("opa.duration_ms", duration);
    }

    if let Some(error) = opa_error {
        span.record("opa.error", error);
        warn!(
            opa_available = opa_available,
            error = error,
            "OPA evaluation failed"
        );
    } else {
        debug!(
            opa_available = opa_available,
            duration_ms = opa_duration_ms,
            "OPA evaluation completed"
        );
    }
}

/// Records legacy RBAC fallback on the current span
///
/// # Arguments
///
/// * `reason` - Reason for fallback (e.g., "opa_unavailable", "opa_timeout")
pub fn record_rbac_fallback(reason: &str) {
    let span = Span::current();
    span.record("rbac.fallback", true);
    span.record("rbac.fallback_reason", reason);

    info!(
        fallback = true,
        reason = reason,
        "Falling back to legacy RBAC"
    );
}

/// Records authorization denial reason on the current span
///
/// # Arguments
///
/// * `reason` - Reason for denial
pub fn record_denial_reason(reason: &str) {
    let span = Span::current();
    span.record("authorization.denial_reason", reason);

    warn!(reason = reason, "Authorization denied");
}

/// Records circuit breaker state on the current span
///
/// # Arguments
///
/// * `state` - Circuit breaker state ("closed", "open", "half_open")
pub fn record_circuit_breaker_state(state: &str) {
    let span = Span::current();
    span.record("circuit_breaker.state", state);

    if state == "open" {
        warn!(state = state, "Circuit breaker is open");
    } else {
        debug!(state = state, "Circuit breaker state");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_span_creation() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        assert_eq!(span.metadata().map(|m| m.name()), Some("authorization"));
    }

    #[test]
    fn test_record_authorization_decision_allowed() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_authorization_decision(true, 25, false, Some("1.0.0"));
    }

    #[test]
    fn test_record_authorization_decision_denied() {
        let span = authorization_span("user123", "write", "event_receiver", "recv456");
        let _guard = span.enter();

        record_authorization_decision(false, 30, false, Some("1.0.0"));
    }

    #[test]
    fn test_record_cache_hit() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_cache_status(true);
    }

    #[test]
    fn test_record_cache_miss() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_cache_status(false);
    }

    #[test]
    fn test_record_opa_evaluation_success() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_opa_evaluation(true, Some(15), None);
    }

    #[test]
    fn test_record_opa_evaluation_failure() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_opa_evaluation(false, None, Some("Connection refused"));
    }

    #[test]
    fn test_record_rbac_fallback() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_rbac_fallback("opa_unavailable");
    }

    #[test]
    fn test_record_denial_reason() {
        let span = authorization_span("user123", "write", "event_receiver", "recv456");
        let _guard = span.enter();

        record_denial_reason("insufficient_permissions");
    }

    #[test]
    fn test_record_circuit_breaker_closed() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_circuit_breaker_state("closed");
    }

    #[test]
    fn test_record_circuit_breaker_open() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_circuit_breaker_state("open");
    }

    #[test]
    fn test_record_circuit_breaker_half_open() {
        let span = authorization_span("user123", "read", "event_receiver", "recv456");
        let _guard = span.enter();

        record_circuit_breaker_state("half_open");
    }
}
