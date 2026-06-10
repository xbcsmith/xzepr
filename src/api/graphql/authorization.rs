// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Resource-aware GraphQL authorization helpers backed by OPA.
//!
//! The REST API performs a coarse authorization check at the HTTP request
//! boundary. GraphQL needs additional resolver-level authorization because a
//! single `/graphql` request can contain operations for many resource types.
//! This module builds the same OPA input shape used by REST, evaluates it
//! before resolver handlers execute, and applies the same fail-safe behavior
//! when OPA is unavailable.

use async_graphql::Context;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{error, warn};

use crate::api::graphql::error_codes;
use crate::api::middleware::jwt::AuthenticatedUser;
use crate::api::middleware::opa::ResourceContextBuilders;
use crate::api::middleware::resource_context::ResourceContextBuilder;
use crate::infrastructure::audit::{AuditLogger, AuthorizationDecisionParams};
use crate::infrastructure::metrics::PrometheusMetrics;
use crate::opa::client::OpaClient;
use crate::opa::types::{
    AuthorizationDecision as OpaAuthorizationDecision, OpaDecisionOutcome, OpaError,
    OpaFailSafeMode, OpaInput, ResourceContext, UserContext,
};

/// GraphQL action label for creating a resource.
pub const ACTION_CREATE: &str = "create";

/// GraphQL action label for reading a resource.
pub const ACTION_READ: &str = "read";

/// GraphQL action label for updating a resource.
pub const ACTION_UPDATE: &str = "update";

/// GraphQL action label for managing group membership.
pub const ACTION_MANAGE_MEMBERS: &str = "manage_members";

/// OPA resource type used for events.
pub const RESOURCE_EVENT: &str = "event";

/// OPA resource type used for event receivers.
pub const RESOURCE_EVENT_RECEIVER: &str = "event_receiver";

/// OPA resource type used for event receiver groups.
pub const RESOURCE_EVENT_RECEIVER_GROUP: &str = "event_receiver_group";

/// Evaluates a GraphQL authorization input against a policy engine.
///
/// Production uses [`OpaClientGraphqlEvaluator`], while tests can provide a
/// fake evaluator to verify input construction and fail-safe behavior without
/// a live OPA server.
#[async_trait]
pub trait GraphqlPolicyEvaluator: Send + Sync {
    /// Evaluates `input` for the given resource version.
    ///
    /// # Arguments
    ///
    /// * `input` - OPA input containing user, action, and resource context.
    /// * `resource_version` - Version used for OPA cache invalidation.
    ///
    /// # Errors
    ///
    /// Returns [`OpaError`] when policy evaluation is unavailable or fails.
    async fn evaluate(
        &self,
        input: OpaInput,
        resource_version: i32,
    ) -> Result<OpaAuthorizationDecision, OpaError>;
}

/// [`GraphqlPolicyEvaluator`] implementation backed by [`OpaClient`].
pub struct OpaClientGraphqlEvaluator {
    client: Arc<OpaClient>,
}

impl OpaClientGraphqlEvaluator {
    /// Creates a new evaluator that delegates to `client`.
    ///
    /// # Arguments
    ///
    /// * `client` - OPA client configured for policy evaluation.
    ///
    /// # Returns
    ///
    /// Returns an evaluator suitable for GraphQL resolver authorization.
    pub fn new(client: Arc<OpaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl GraphqlPolicyEvaluator for OpaClientGraphqlEvaluator {
    async fn evaluate(
        &self,
        input: OpaInput,
        resource_version: i32,
    ) -> Result<OpaAuthorizationDecision, OpaError> {
        self.client
            .evaluate_with_circuit_breaker(input, resource_version)
            .await
    }
}

/// Describes the resource and action a GraphQL resolver is about to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphqlAuthorizationOperation {
    /// OPA action label, such as `read`, `create`, or `manage_members`.
    pub action: String,
    /// OPA resource type, such as `event` or `event_receiver_group`.
    pub resource_type: String,
    /// Target resource ID. `None` means the operation is resource agnostic.
    pub resource_id: Option<String>,
}

impl GraphqlAuthorizationOperation {
    /// Creates an operation for a specific event resource.
    ///
    /// # Arguments
    ///
    /// * `action` - OPA action label.
    /// * `event_id` - Target event ID.
    ///
    /// # Returns
    ///
    /// Returns a resource-aware event authorization operation.
    pub fn event(action: impl Into<String>, event_id: impl Into<String>) -> Self {
        Self::resource(RESOURCE_EVENT, action, Some(event_id.into()))
    }

    /// Creates an operation for a specific event receiver resource.
    ///
    /// # Arguments
    ///
    /// * `action` - OPA action label.
    /// * `receiver_id` - Target receiver ID.
    ///
    /// # Returns
    ///
    /// Returns a resource-aware receiver authorization operation.
    pub fn event_receiver(action: impl Into<String>, receiver_id: impl Into<String>) -> Self {
        Self::resource(RESOURCE_EVENT_RECEIVER, action, Some(receiver_id.into()))
    }

    /// Creates an operation for a specific event receiver group resource.
    ///
    /// # Arguments
    ///
    /// * `action` - OPA action label.
    /// * `group_id` - Target group ID.
    ///
    /// # Returns
    ///
    /// Returns a resource-aware group authorization operation.
    pub fn event_receiver_group(action: impl Into<String>, group_id: impl Into<String>) -> Self {
        Self::resource(RESOURCE_EVENT_RECEIVER_GROUP, action, Some(group_id.into()))
    }

    /// Creates an operation for managing members of a group.
    ///
    /// # Arguments
    ///
    /// * `group_id` - Target group ID.
    ///
    /// # Returns
    ///
    /// Returns a resource-aware group membership authorization operation.
    pub fn group_members(group_id: impl Into<String>) -> Self {
        Self::event_receiver_group(ACTION_MANAGE_MEMBERS, group_id)
    }

    /// Creates an operation that is not tied to a specific resource ID.
    ///
    /// Resource-agnostic operations still produce an OPA input, but they do not
    /// call repository-backed context builders because no target ID exists.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - OPA resource type.
    /// * `action` - OPA action label.
    ///
    /// # Returns
    ///
    /// Returns a resource-agnostic authorization operation.
    pub fn resource_agnostic(resource_type: impl Into<String>, action: impl Into<String>) -> Self {
        Self::resource(resource_type, action, None)
    }

    fn resource(
        resource_type: impl Into<String>,
        action: impl Into<String>,
        resource_id: Option<String>,
    ) -> Self {
        Self {
            action: action.into(),
            resource_type: resource_type.into(),
            resource_id,
        }
    }
}

/// Successful resolver-level authorization decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphqlAuthorizationDecision {
    /// Whether the operation was allowed.
    pub allowed: bool,
    /// Outcome label, including OPA/fail-safe/fallback detail.
    pub outcome: OpaDecisionOutcome,
    /// Resource type evaluated by OPA.
    pub resource_type: String,
    /// Optional resource ID evaluated by OPA.
    pub resource_id: Option<String>,
    /// Action evaluated by OPA.
    pub action: String,
}

/// Resolver-level authorization failures.
#[derive(Debug, Error)]
pub enum GraphqlAuthorizationError {
    /// A resource-aware operation could not build the required resource context.
    #[error("authorization resource context unavailable")]
    ResourceContextUnavailable,
    /// Policy evaluation completed or fell back to a denying outcome.
    #[error("permission denied")]
    Denied {
        /// Outcome that denied the operation.
        outcome: OpaDecisionOutcome,
    },
}

impl GraphqlAuthorizationError {
    /// Converts this authorization failure to a coded GraphQL error.
    ///
    /// # Returns
    ///
    /// Returns an [`async_graphql::Error`] with a stable `FORBIDDEN` code.
    pub fn into_graphql_error(self) -> async_graphql::Error {
        match self {
            GraphqlAuthorizationError::ResourceContextUnavailable => {
                error_codes::forbidden("Authorization resource context unavailable")
            }
            GraphqlAuthorizationError::Denied { .. } => error_codes::forbidden("Permission denied"),
        }
    }
}

/// Resource-aware GraphQL OPA authorization service.
///
/// The service is designed to be stored in async-graphql schema data and used
/// by resolver helpers before application handlers execute.
pub struct GraphqlAuthorizationService {
    evaluator: Arc<dyn GraphqlPolicyEvaluator>,
    context_builders: ResourceContextBuilders,
    fail_safe_mode: OpaFailSafeMode,
    is_production: bool,
    audit_logger: Option<Arc<AuditLogger>>,
    metrics: Option<Arc<PrometheusMetrics>>,
}

impl GraphqlAuthorizationService {
    /// Creates a new GraphQL authorization service.
    ///
    /// # Arguments
    ///
    /// * `evaluator` - Policy evaluator, typically backed by OPA.
    /// * `context_builders` - Resource context builders shared with REST OPA.
    /// * `fail_safe_mode` - Behavior when OPA is unavailable.
    /// * `is_production` - Whether fail-open development mode must be blocked.
    ///
    /// # Returns
    ///
    /// Returns a service ready for insertion into GraphQL schema data.
    pub fn new(
        evaluator: Arc<dyn GraphqlPolicyEvaluator>,
        context_builders: ResourceContextBuilders,
        fail_safe_mode: OpaFailSafeMode,
        is_production: bool,
    ) -> Self {
        Self {
            evaluator,
            context_builders,
            fail_safe_mode,
            is_production,
            audit_logger: None,
            metrics: None,
        }
    }

    /// Adds audit and metrics sinks to the authorization service.
    ///
    /// # Arguments
    ///
    /// * `audit_logger` - Audit logger used to record authorization decisions.
    /// * `metrics` - Prometheus metrics registry used for OPA decision labels.
    ///
    /// # Returns
    ///
    /// Returns `self` with GraphQL OPA observability enabled.
    pub fn with_observability(
        mut self,
        audit_logger: Arc<AuditLogger>,
        metrics: Arc<PrometheusMetrics>,
    ) -> Self {
        self.audit_logger = Some(audit_logger);
        self.metrics = Some(metrics);
        self
    }

    /// Creates a GraphQL authorization service from REST OPA middleware state.
    ///
    /// # Arguments
    ///
    /// * `state` - REST OPA middleware state used as the source of OPA client,
    ///   context builders, and fail-safe settings.
    ///
    /// # Returns
    ///
    /// Returns a resolver-level authorization service using the same OPA
    /// fail-safe concepts as REST.
    pub fn from_opa_middleware_state(
        state: &crate::api::middleware::opa::OpaMiddlewareState,
    ) -> Self {
        Self::new(
            Arc::new(OpaClientGraphqlEvaluator::new(state.opa_client.clone())),
            state.context_builders.clone(),
            state.fail_safe_mode,
            state.is_production,
        )
        .with_observability(state.audit_logger.clone(), state.metrics.clone())
    }

    /// Authorizes a single GraphQL operation for `user`.
    ///
    /// # Arguments
    ///
    /// * `user` - Authenticated GraphQL caller.
    /// * `operation` - Resource and action the resolver is about to perform.
    ///
    /// # Returns
    ///
    /// Returns the allowed decision and outcome metadata.
    ///
    /// # Errors
    ///
    /// Returns [`GraphqlAuthorizationError`] when resource context cannot be
    /// built, OPA denies, or fail-safe handling denies.
    pub async fn authorize(
        &self,
        user: &AuthenticatedUser,
        operation: GraphqlAuthorizationOperation,
    ) -> Result<GraphqlAuthorizationDecision, GraphqlAuthorizationError> {
        let started_at = Instant::now();
        let resource_context = match self.build_resource_context(&operation).await {
            Ok(context) => context,
            Err(err) => {
                self.record_resource_context_failure(user, &operation, started_at.elapsed());
                return Err(err);
            }
        };
        let resource_version = i32::try_from(resource_context.resource_version).unwrap_or(i32::MAX);
        let input = OpaInput {
            user: UserContext {
                user_id: user.user_id().to_string(),
                username: user.claims.sub.clone(),
                roles: user.claims.roles.clone(),
                groups: Vec::new(),
            },
            action: operation.action.clone(),
            resource: resource_context.clone(),
        };

        let outcome = match self.evaluator.evaluate(input, resource_version).await {
            Ok(decision) if decision.allow => OpaDecisionOutcome::OpaAllow,
            Ok(_) => OpaDecisionOutcome::OpaDeny,
            Err(e) => {
                error!(error = %e, "GraphQL OPA policy evaluation failed");
                self.fail_safe_outcome(user, &operation.action, &resource_context)
            }
        };

        self.record_authorization_outcome(
            user,
            &operation.action,
            &resource_context,
            outcome,
            started_at.elapsed(),
        );

        if outcome.is_allowed() {
            Ok(GraphqlAuthorizationDecision {
                allowed: true,
                outcome,
                resource_type: resource_context.resource_type,
                resource_id: resource_context.resource_id,
                action: operation.action,
            })
        } else {
            warn!(
                user_id = %user.user_id(),
                action = %operation.action,
                resource_type = %resource_context.resource_type,
                outcome = %outcome.as_metric_label(),
                "GraphQL authorization denied"
            );
            Err(GraphqlAuthorizationError::Denied { outcome })
        }
    }

    async fn build_resource_context(
        &self,
        operation: &GraphqlAuthorizationOperation,
    ) -> Result<ResourceContext, GraphqlAuthorizationError> {
        let Some(resource_id) = operation.resource_id.as_deref() else {
            return Ok(ResourceContext {
                resource_type: operation.resource_type.clone(),
                resource_id: None,
                owner_id: None,
                group_id: None,
                members: Vec::new(),
                resource_version: 1,
            });
        };

        let builder = self.builder_for(&operation.resource_type)?;
        builder.build_context(resource_id).await.map_err(|e| {
            warn!(
                resource_type = %operation.resource_type,
                resource_id = %resource_id,
                error = %e,
                "Failed to build GraphQL authorization resource context"
            );
            GraphqlAuthorizationError::ResourceContextUnavailable
        })
    }

    fn builder_for(
        &self,
        resource_type: &str,
    ) -> Result<&Arc<dyn ResourceContextBuilder>, GraphqlAuthorizationError> {
        match resource_type {
            RESOURCE_EVENT => Ok(&self.context_builders.event),
            RESOURCE_EVENT_RECEIVER => Ok(&self.context_builders.receiver),
            RESOURCE_EVENT_RECEIVER_GROUP => Ok(&self.context_builders.group),
            _ => Err(GraphqlAuthorizationError::ResourceContextUnavailable),
        }
    }

    fn fail_safe_outcome(
        &self,
        user: &AuthenticatedUser,
        action: &str,
        resource: &ResourceContext,
    ) -> OpaDecisionOutcome {
        match self.fail_safe_mode {
            OpaFailSafeMode::FailClosed => OpaDecisionOutcome::UnavailableFailClosed,
            OpaFailSafeMode::FailOpenDevelopment => {
                if self.is_production {
                    warn!(
                        user_id = %user.user_id(),
                        "GraphQL FailOpenDevelopment requested in production; overriding to FailClosed"
                    );
                    OpaDecisionOutcome::UnavailableFailClosed
                } else {
                    OpaDecisionOutcome::UnavailableFailOpenDevelopment
                }
            }
            OpaFailSafeMode::LegacyRbacFallback => {
                if legacy_rbac_check(user, action, resource) {
                    OpaDecisionOutcome::UnavailableLegacyRbacAllow
                } else {
                    OpaDecisionOutcome::UnavailableLegacyRbacDeny
                }
            }
        }
    }

    fn record_resource_context_failure(
        &self,
        user: &AuthenticatedUser,
        operation: &GraphqlAuthorizationOperation,
        duration: Duration,
    ) {
        let resource = ResourceContext {
            resource_type: operation.resource_type.clone(),
            resource_id: operation.resource_id.clone(),
            owner_id: None,
            group_id: None,
            members: Vec::new(),
            resource_version: 1,
        };
        self.record_authorization_outcome(
            user,
            &operation.action,
            &resource,
            OpaDecisionOutcome::UnavailableFailClosed,
            duration,
        );
    }

    fn record_authorization_outcome(
        &self,
        user: &AuthenticatedUser,
        action: &str,
        resource: &ResourceContext,
        outcome: OpaDecisionOutcome,
        duration: Duration,
    ) {
        let fallback_used = matches!(
            outcome,
            OpaDecisionOutcome::UnavailableLegacyRbacAllow
                | OpaDecisionOutcome::UnavailableLegacyRbacDeny
        );

        if let Some(metrics) = &self.metrics {
            metrics.record_authorization_decision(
                outcome.is_allowed(),
                &resource.resource_type,
                action,
                duration.as_secs_f64(),
                fallback_used,
            );
        }

        if let Some(audit_logger) = &self.audit_logger {
            let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
            audit_logger.log_authorization_decision(&AuthorizationDecisionParams {
                user_id: user.user_id().to_string(),
                action: action.to_string(),
                resource_type: resource.resource_type.clone(),
                resource_id: resource
                    .resource_id
                    .clone()
                    .unwrap_or_else(|| "*".to_string()),
                decision: outcome.is_allowed(),
                duration_ms,
                fallback_used,
                policy_version: None,
                reason: if outcome.is_allowed() {
                    None
                } else {
                    Some(outcome.as_metric_label().to_string())
                },
                request_id: None,
            });
        }
    }
}

/// Authorizes a resolver operation when a GraphQL authorization service exists.
///
/// Schemas created without a [`GraphqlAuthorizationService`] keep the existing
/// coarse `/graphql` middleware behavior. Schemas that include the service
/// enforce resource-aware authorization before resolver handlers execute.
///
/// # Arguments
///
/// * `ctx` - GraphQL resolver context.
/// * `user` - Authenticated caller.
/// * `operation` - Resource and action to authorize.
///
/// # Returns
///
/// Returns `Ok(None)` when no service is configured, or `Ok(Some(decision))`
/// after an allowed decision.
///
/// # Errors
///
/// Returns a coded GraphQL `FORBIDDEN` error when authorization denies.
pub async fn authorize_graphql(
    ctx: &Context<'_>,
    user: &AuthenticatedUser,
    operation: GraphqlAuthorizationOperation,
) -> async_graphql::Result<Option<GraphqlAuthorizationDecision>> {
    let Some(service) = ctx.data_opt::<Arc<GraphqlAuthorizationService>>() else {
        return Ok(None);
    };

    service
        .authorize(user, operation)
        .await
        .map(Some)
        .map_err(GraphqlAuthorizationError::into_graphql_error)
}

/// Authorizes multiple resolver operations in order.
///
/// This is used for mutations such as group creation where the new group is
/// resource agnostic but referenced receivers must also pass resource-aware
/// checks before handler execution.
///
/// # Arguments
///
/// * `ctx` - GraphQL resolver context.
/// * `user` - Authenticated caller.
/// * `operations` - Operations to evaluate sequentially.
///
/// # Returns
///
/// Returns all decisions when a service is configured, or an empty vector when
/// no service is configured.
///
/// # Errors
///
/// Returns a coded GraphQL `FORBIDDEN` error on the first denying operation.
pub async fn authorize_graphql_many(
    ctx: &Context<'_>,
    user: &AuthenticatedUser,
    operations: Vec<GraphqlAuthorizationOperation>,
) -> async_graphql::Result<Vec<GraphqlAuthorizationDecision>> {
    let Some(service) = ctx.data_opt::<Arc<GraphqlAuthorizationService>>() else {
        return Ok(Vec::new());
    };

    let mut decisions = Vec::with_capacity(operations.len());
    for operation in operations {
        let decision = service
            .authorize(user, operation)
            .await
            .map_err(GraphqlAuthorizationError::into_graphql_error)?;
        decisions.push(decision);
    }
    Ok(decisions)
}

fn legacy_rbac_check(user: &AuthenticatedUser, action: &str, resource: &ResourceContext) -> bool {
    if user.has_role("admin") {
        return true;
    }

    if resource.owner_id.as_deref() == Some(user.user_id()) {
        return true;
    }

    if action == ACTION_READ
        && resource.group_id.is_some()
        && resource
            .members
            .iter()
            .any(|member| member == user.user_id())
    {
        return true;
    }

    let permission = format!("{}:{}", resource.resource_type, action);
    user.has_permission(&permission)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::middleware::resource_context::ResourceContextError;
    use crate::auth::jwt::claims::TokenType;
    use crate::auth::jwt::Claims;
    use std::collections::HashMap;
    use std::sync::{Mutex, MutexGuard};

    fn lock_test_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        // SAFETY: A poisoned test mutex means an earlier assertion already failed;
        // failing the current test with a clear message is the correct behavior.
        mutex.lock().expect("test mutex should not be poisoned")
    }

    #[derive(Clone)]
    struct FakeResourceContextBuilder {
        contexts: Arc<Mutex<HashMap<String, ResourceContext>>>,
        fail_missing: bool,
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl FakeResourceContextBuilder {
        fn new(contexts: Vec<ResourceContext>) -> Self {
            let mut map = HashMap::new();
            for context in contexts {
                if let Some(resource_id) = &context.resource_id {
                    map.insert(resource_id.clone(), context);
                }
            }
            Self {
                contexts: Arc::new(Mutex::new(map)),
                fail_missing: true,
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn empty() -> Self {
            Self::new(Vec::new())
        }

        fn call_count(&self) -> usize {
            lock_test_mutex(&self.calls).len()
        }
    }

    #[async_trait]
    impl ResourceContextBuilder for FakeResourceContextBuilder {
        async fn build_context(
            &self,
            resource_id: &str,
        ) -> Result<ResourceContext, ResourceContextError> {
            lock_test_mutex(&self.calls).push(resource_id.to_string());
            if let Some(context) = lock_test_mutex(&self.contexts).get(resource_id).cloned() {
                return Ok(context);
            }

            if self.fail_missing {
                Err(ResourceContextError::NotFound {
                    resource_type: "test".to_string(),
                    id: resource_id.to_string(),
                })
            } else {
                Ok(ResourceContext {
                    resource_type: "test".to_string(),
                    resource_id: Some(resource_id.to_string()),
                    owner_id: None,
                    group_id: None,
                    members: Vec::new(),
                    resource_version: 1,
                })
            }
        }
    }

    enum FakeEvaluatorMode {
        Allow,
        Deny,
        Error,
        OwnerOnly,
    }

    struct FakeEvaluator {
        mode: FakeEvaluatorMode,
        inputs: Arc<Mutex<Vec<OpaInput>>>,
    }

    impl FakeEvaluator {
        fn new(mode: FakeEvaluatorMode) -> Self {
            Self {
                mode,
                inputs: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn inputs(&self) -> Vec<OpaInput> {
            lock_test_mutex(&self.inputs).clone()
        }
    }

    #[async_trait]
    impl GraphqlPolicyEvaluator for FakeEvaluator {
        async fn evaluate(
            &self,
            input: OpaInput,
            _resource_version: i32,
        ) -> Result<OpaAuthorizationDecision, OpaError> {
            let allow = match self.mode {
                FakeEvaluatorMode::Allow => true,
                FakeEvaluatorMode::Deny => false,
                FakeEvaluatorMode::Error => {
                    return Err(OpaError::CircuitOpen);
                }
                FakeEvaluatorMode::OwnerOnly => {
                    input.resource.owner_id.as_deref() == Some(input.user.user_id.as_str())
                }
            };
            lock_test_mutex(&self.inputs).push(input);
            Ok(OpaAuthorizationDecision {
                allow,
                reason: None,
                metadata: None,
            })
        }
    }

    fn user_with(user_id: &str, roles: Vec<&str>, permissions: Vec<&str>) -> AuthenticatedUser {
        AuthenticatedUser::new(Claims {
            sub: user_id.to_string(),
            roles: roles.into_iter().map(str::to_string).collect(),
            permissions: permissions.into_iter().map(str::to_string).collect(),
            exp: 9_999_999_999,
            iat: 0,
            nbf: 0,
            jti: "graphql-authz-test".to_string(),
            iss: "xzepr".to_string(),
            aud: "xzepr-api".to_string(),
            token_type: TokenType::Access,
        })
    }

    fn context(resource_type: &str, resource_id: &str, owner_id: &str) -> ResourceContext {
        ResourceContext {
            resource_type: resource_type.to_string(),
            resource_id: Some(resource_id.to_string()),
            owner_id: Some(owner_id.to_string()),
            group_id: None,
            members: Vec::new(),
            resource_version: 7,
        }
    }

    fn service_with(
        evaluator: Arc<dyn GraphqlPolicyEvaluator>,
        event_builder: FakeResourceContextBuilder,
        receiver_builder: FakeResourceContextBuilder,
        group_builder: FakeResourceContextBuilder,
        fail_safe_mode: OpaFailSafeMode,
        is_production: bool,
    ) -> GraphqlAuthorizationService {
        GraphqlAuthorizationService::new(
            evaluator,
            ResourceContextBuilders {
                event: Arc::new(event_builder),
                receiver: Arc::new(receiver_builder),
                group: Arc::new(group_builder),
            },
            fail_safe_mode,
            is_production,
        )
    }

    #[tokio::test]
    async fn test_authorize_allow_builds_event_opa_input() {
        let evaluator = Arc::new(FakeEvaluator::new(FakeEvaluatorMode::Allow));
        let service = service_with(
            evaluator.clone(),
            FakeResourceContextBuilder::new(vec![context(RESOURCE_EVENT, "event-1", "user-1")]),
            FakeResourceContextBuilder::empty(),
            FakeResourceContextBuilder::empty(),
            OpaFailSafeMode::FailClosed,
            false,
        );

        let decision = service
            .authorize(
                &user_with("user-1", vec!["user"], vec![]),
                GraphqlAuthorizationOperation::event(ACTION_READ, "event-1"),
            )
            .await;

        assert!(decision.is_ok(), "OPA allow should authorize the operation");
        let inputs = evaluator.inputs();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].action, ACTION_READ);
        assert_eq!(inputs[0].resource.resource_type, RESOURCE_EVENT);
        assert_eq!(inputs[0].resource.resource_id.as_deref(), Some("event-1"));
        assert_eq!(inputs[0].resource.owner_id.as_deref(), Some("user-1"));
    }

    #[tokio::test]
    async fn test_authorize_denies_when_opa_denies() {
        let evaluator = Arc::new(FakeEvaluator::new(FakeEvaluatorMode::Deny));
        let service = service_with(
            evaluator,
            FakeResourceContextBuilder::empty(),
            FakeResourceContextBuilder::new(vec![context(
                RESOURCE_EVENT_RECEIVER,
                "receiver-1",
                "user-1",
            )]),
            FakeResourceContextBuilder::empty(),
            OpaFailSafeMode::FailClosed,
            false,
        );

        let result = service
            .authorize(
                &user_with("user-1", vec!["user"], vec![]),
                GraphqlAuthorizationOperation::event_receiver(ACTION_READ, "receiver-1"),
            )
            .await;

        assert!(matches!(
            result,
            Err(GraphqlAuthorizationError::Denied {
                outcome: OpaDecisionOutcome::OpaDeny
            })
        ));
    }

    #[tokio::test]
    async fn test_authorize_fail_closed_denies_when_opa_unavailable() {
        let evaluator = Arc::new(FakeEvaluator::new(FakeEvaluatorMode::Error));
        let service = service_with(
            evaluator,
            FakeResourceContextBuilder::empty(),
            FakeResourceContextBuilder::empty(),
            FakeResourceContextBuilder::new(vec![context(
                RESOURCE_EVENT_RECEIVER_GROUP,
                "group-1",
                "user-1",
            )]),
            OpaFailSafeMode::FailClosed,
            false,
        );

        let result = service
            .authorize(
                &user_with("user-1", vec!["user"], vec![]),
                GraphqlAuthorizationOperation::event_receiver_group(ACTION_UPDATE, "group-1"),
            )
            .await;

        assert!(matches!(
            result,
            Err(GraphqlAuthorizationError::Denied {
                outcome: OpaDecisionOutcome::UnavailableFailClosed
            })
        ));
    }

    #[tokio::test]
    async fn test_authorize_legacy_fallback_allows_owner_without_opa() {
        let evaluator = Arc::new(FakeEvaluator::new(FakeEvaluatorMode::Error));
        let service = service_with(
            evaluator,
            FakeResourceContextBuilder::new(vec![context(RESOURCE_EVENT, "event-1", "user-1")]),
            FakeResourceContextBuilder::empty(),
            FakeResourceContextBuilder::empty(),
            OpaFailSafeMode::LegacyRbacFallback,
            false,
        );

        let decision = service
            .authorize(
                &user_with("user-1", vec!["user"], vec![]),
                GraphqlAuthorizationOperation::event(ACTION_UPDATE, "event-1"),
            )
            .await;

        match decision {
            Ok(decision) => assert_eq!(
                decision.outcome,
                OpaDecisionOutcome::UnavailableLegacyRbacAllow
            ),
            Err(err) => panic!("owner legacy RBAC fallback should allow: {}", err),
        }
    }

    #[tokio::test]
    async fn test_authorize_resource_context_missing_fails_closed_without_evaluator_call() {
        let evaluator = Arc::new(FakeEvaluator::new(FakeEvaluatorMode::Allow));
        let event_builder = FakeResourceContextBuilder::empty();
        let service = service_with(
            evaluator.clone(),
            event_builder.clone(),
            FakeResourceContextBuilder::empty(),
            FakeResourceContextBuilder::empty(),
            OpaFailSafeMode::FailOpenDevelopment,
            false,
        );

        let result = service
            .authorize(
                &user_with("user-1", vec!["user"], vec![]),
                GraphqlAuthorizationOperation::event(ACTION_READ, "missing-event"),
            )
            .await;

        assert!(matches!(
            result,
            Err(GraphqlAuthorizationError::ResourceContextUnavailable)
        ));
        assert_eq!(event_builder.call_count(), 1);
        assert!(
            evaluator.inputs().is_empty(),
            "OPA must not be called without resource context"
        );
    }

    #[tokio::test]
    async fn test_authorize_resource_agnostic_operation_does_not_build_context() {
        let evaluator = Arc::new(FakeEvaluator::new(FakeEvaluatorMode::Allow));
        let event_builder = FakeResourceContextBuilder::empty();
        let service = service_with(
            evaluator.clone(),
            event_builder.clone(),
            FakeResourceContextBuilder::empty(),
            FakeResourceContextBuilder::empty(),
            OpaFailSafeMode::FailClosed,
            false,
        );

        let decision = service
            .authorize(
                &user_with("user-1", vec!["admin"], vec![]),
                GraphqlAuthorizationOperation::resource_agnostic(RESOURCE_EVENT, ACTION_CREATE),
            )
            .await;

        assert!(decision.is_ok(), "resource-agnostic allow should succeed");
        assert_eq!(event_builder.call_count(), 0);
        let inputs = evaluator.inputs();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].resource.resource_id, None);
        assert_eq!(inputs[0].resource.owner_id, None);
    }

    #[tokio::test]
    async fn test_authorize_cross_owner_receiver_context_denies() {
        let evaluator = Arc::new(FakeEvaluator::new(FakeEvaluatorMode::OwnerOnly));
        let service = service_with(
            evaluator,
            FakeResourceContextBuilder::empty(),
            FakeResourceContextBuilder::new(vec![context(
                RESOURCE_EVENT_RECEIVER,
                "receiver-1",
                "owner-1",
            )]),
            FakeResourceContextBuilder::empty(),
            OpaFailSafeMode::FailClosed,
            false,
        );

        let result = service
            .authorize(
                &user_with("other-user", vec!["user"], vec![]),
                GraphqlAuthorizationOperation::event_receiver(ACTION_CREATE, "receiver-1"),
            )
            .await;

        assert!(matches!(
            result,
            Err(GraphqlAuthorizationError::Denied {
                outcome: OpaDecisionOutcome::OpaDeny
            })
        ));
    }
}
