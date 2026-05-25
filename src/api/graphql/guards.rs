// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/graphql/guards.rs

use async_graphql::*;
use std::sync::Arc;

use crate::api::graphql::error_codes;
use crate::api::middleware::jwt::AuthenticatedUser;
use crate::auth::jwt::claims::Claims;
use crate::domain::value_objects::UserId;

/// Authentication check for GraphQL resolvers
///
/// Checks if a user is authenticated by looking for Claims in the context
///
/// # Example
///
/// ```ignore
/// use xzepr::api::graphql::guards::require_auth;
/// use async_graphql::Context;
///
/// async fn my_resolver(ctx: &Context<'_>) -> async_graphql::Result<String> {
///     require_auth(ctx)?;
///     Ok("Protected data".to_string())
/// }
/// ```
pub fn require_auth<'a>(ctx: &Context<'a>) -> Result<&'a Claims> {
    ctx.data_opt::<Claims>()
        .ok_or_else(|| Error::new("Unauthorized: Authentication required"))
}

/// Role-based access control check
///
/// Ensures that an authenticated user has at least one of the required roles
///
/// # Example
///
/// ```ignore
/// use xzepr::api::graphql::guards::require_roles;
/// use async_graphql::Context;
///
/// async fn admin_resolver(ctx: &Context<'_>) -> async_graphql::Result<String> {
///     require_roles(ctx, &["admin"])?;
///     Ok("Admin data".to_string())
/// }
/// ```
pub fn require_roles<'a>(ctx: &Context<'a>, required_roles: &[&str]) -> Result<&'a Claims> {
    let claims = require_auth(ctx)?;

    // If no specific roles required, just check authentication
    if required_roles.is_empty() {
        return Ok(claims);
    }

    // Check if user has any of the required roles
    let has_required_role = required_roles
        .iter()
        .any(|&required| claims.roles.iter().any(|role| role == required));

    if has_required_role {
        Ok(claims)
    } else {
        Err(Error::new(format!(
            "Forbidden: Requires one of these roles: {}",
            required_roles.join(", ")
        )))
    }
}

/// Permission-based access control check
///
/// Ensures that an authenticated user has at least one of the required permissions
///
/// # Example
///
/// ```ignore
/// use xzepr::api::graphql::guards::require_permissions;
/// use async_graphql::Context;
///
/// async fn write_resolver(ctx: &Context<'_>) -> async_graphql::Result<String> {
///     require_permissions(ctx, &["events:write"])?;
///     Ok("Write allowed".to_string())
/// }
/// ```
pub fn require_permissions<'a>(
    ctx: &Context<'a>,
    required_permissions: &[&str],
) -> Result<&'a Claims> {
    let claims = require_auth(ctx)?;

    // Check if user has any of the required permissions
    let has_required_permission = required_permissions
        .iter()
        .any(|&required| claims.permissions.iter().any(|perm| perm == required));

    if has_required_permission {
        Ok(claims)
    } else {
        Err(Error::new(format!(
            "Forbidden: Requires one of these permissions: {}",
            required_permissions.join(", ")
        )))
    }
}

/// Combined role and permission check
///
/// User must have at least one required role AND one required permission
pub fn require_roles_and_permissions<'a>(
    ctx: &Context<'a>,
    required_roles: &[&str],
    required_permissions: &[&str],
) -> Result<&'a Claims> {
    let claims = require_roles(ctx, required_roles)?;
    require_permissions(ctx, required_permissions)?;
    Ok(claims)
}

/// Helper functions for common authorization patterns
pub mod helpers {
    use super::*;

    /// Requires admin role
    pub fn require_admin<'a>(ctx: &Context<'a>) -> Result<&'a Claims> {
        require_roles(ctx, &["admin"])
    }

    /// Requires user role (any authenticated user)
    pub fn require_user<'a>(ctx: &Context<'a>) -> Result<&'a Claims> {
        require_roles(ctx, &["user", "admin"])
    }

    /// Requires read permission for a resource
    pub fn require_read<'a>(ctx: &Context<'a>, resource: &str) -> Result<&'a Claims> {
        let permission = format!("{}:read", resource);
        require_permissions(ctx, &[&permission])
    }

    /// Requires write permission for a resource
    pub fn require_write<'a>(ctx: &Context<'a>, resource: &str) -> Result<&'a Claims> {
        let permission = format!("{}:write", resource);
        require_permissions(ctx, &[&permission])
    }

    /// Requires delete permission for a resource
    pub fn require_delete<'a>(ctx: &Context<'a>, resource: &str) -> Result<&'a Claims> {
        let permission = format!("{}:delete", resource);
        require_permissions(ctx, &[&permission])
    }

    /// Requires admin permission for a resource
    pub fn require_resource_admin<'a>(ctx: &Context<'a>, resource: &str) -> Result<&'a Claims> {
        let permission = format!("{}:admin", resource);
        require_permissions(ctx, &[&permission])
    }
}

/// Extracts the authenticated user from the GraphQL context.
///
/// This is the correct guard to use in production resolvers because
/// the `graphql_handler` injects `AuthenticatedUser`, not `Claims`.
///
/// # Arguments
///
/// * `ctx` - The current resolver context.
///
/// # Returns
///
/// A reference to the [`AuthenticatedUser`] stored in the context.
///
/// # Errors
///
/// Returns `UNAUTHENTICATED` if no `AuthenticatedUser` is present in the
/// context (i.e., the request reached the resolver without a valid JWT).
pub fn require_authenticated_user<'a>(ctx: &Context<'a>) -> Result<&'a AuthenticatedUser> {
    ctx.data_opt::<AuthenticatedUser>()
        .ok_or_else(|| error_codes::unauthenticated("Authentication required"))
}

/// Parses the caller's user ID from an [`AuthenticatedUser`].
///
/// # Arguments
///
/// * `user` - The authenticated user whose `sub` claim is parsed.
///
/// # Returns
///
/// The parsed [`UserId`].
///
/// # Errors
///
/// Returns `VALIDATION_ERROR` if the subject claim is not a valid [`UserId`].
pub fn parse_caller_user_id(user: &AuthenticatedUser) -> Result<UserId> {
    UserId::parse(user.user_id())
        .map_err(|e| error_codes::validation_error(&format!("Invalid user ID in token: {}", e)))
}

/// Asserts that `caller` is the owner of a resource.
///
/// # Arguments
///
/// * `caller` - The [`UserId`] making the request.
/// * `resource_owner` - The [`UserId`] that owns the resource.
/// * `resource` - A human-readable label for the resource type (e.g., `"event"`).
///
/// # Returns
///
/// `Ok(())` when `caller == resource_owner`.
///
/// # Errors
///
/// Returns `FORBIDDEN` if `caller != resource_owner`.
pub fn require_ownership(caller: UserId, resource_owner: UserId, resource: &str) -> Result<()> {
    if caller != resource_owner {
        return Err(error_codes::forbidden(&format!(
            "You do not own this {}",
            resource
        )));
    }
    Ok(())
}

/// Query complexity configuration
#[derive(Debug, Clone)]
pub struct ComplexityConfig {
    /// Maximum query complexity allowed
    pub max_complexity: usize,
    /// Maximum query depth allowed
    pub max_depth: usize,
    /// Whether to enforce complexity limits
    pub enforce: bool,
    /// Whether the GraphQL Playground IDE is exposed.
    ///
    /// Must be `false` in production. Defaults to `false`.
    pub playground_enabled: bool,
}

impl Default for ComplexityConfig {
    fn default() -> Self {
        Self {
            max_complexity: 100,
            max_depth: 10,
            enforce: true,
            playground_enabled: false,
        }
    }
}

impl From<&crate::infrastructure::config::GraphqlConfig> for ComplexityConfig {
    fn from(config: &crate::infrastructure::config::GraphqlConfig) -> Self {
        Self {
            max_complexity: config.max_complexity,
            max_depth: config.max_depth,
            enforce: config.enforce_complexity,
            playground_enabled: config.playground_enabled,
        }
    }
}

impl ComplexityConfig {
    /// Creates a complexity config from environment variables
    pub fn from_env() -> Self {
        let max_complexity = std::env::var("XZEPR__GRAPHQL__MAX_COMPLEXITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let max_depth = std::env::var("XZEPR__GRAPHQL__MAX_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let enforce = std::env::var("XZEPR__GRAPHQL__ENFORCE_COMPLEXITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        Self {
            max_complexity,
            max_depth,
            enforce,
            playground_enabled: false,
        }
    }

    /// Creates a permissive config for development.
    ///
    /// Playground is enabled in dev mode only.
    pub fn permissive() -> Self {
        Self {
            max_complexity: 1000,
            max_depth: 20,
            enforce: false,
            playground_enabled: true,
        }
    }

    /// Creates a strict config for production.
    ///
    /// Playground is disabled; all complexity limits are enforced.
    pub fn production() -> Self {
        Self {
            max_complexity: 50,
            max_depth: 8,
            enforce: true,
            playground_enabled: false,
        }
    }
}

/// Query complexity analyzer
///
/// Analyzes and limits query complexity to prevent resource exhaustion attacks
pub struct QueryComplexityAnalyzer {
    config: Arc<ComplexityConfig>,
}

impl QueryComplexityAnalyzer {
    /// Creates a new query complexity analyzer
    pub fn new(config: ComplexityConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Gets the maximum allowed complexity
    pub fn max_complexity(&self) -> usize {
        self.config.max_complexity
    }

    /// Gets the maximum allowed depth
    pub fn max_depth(&self) -> usize {
        self.config.max_depth
    }

    /// Checks if enforcement is enabled
    pub fn is_enforced(&self) -> bool {
        self.config.enforce
    }
}

/// Extension for query complexity limiting
pub struct QueryComplexityExtension {
    analyzer: QueryComplexityAnalyzer,
}

impl QueryComplexityExtension {
    /// Creates a new query complexity extension
    pub fn new(config: ComplexityConfig) -> Self {
        Self {
            analyzer: QueryComplexityAnalyzer::new(config),
        }
    }

    /// Gets the analyzer
    pub fn analyzer(&self) -> &QueryComplexityAnalyzer {
        &self.analyzer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt::claims::TokenType;
    use chrono::Utc;

    fn create_test_claims(roles: Vec<String>, permissions: Vec<String>) -> Claims {
        Claims {
            sub: "user123".to_string(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp(),
            iat: Utc::now().timestamp(),
            nbf: Utc::now().timestamp(),
            jti: "test-jti".to_string(),
            iss: "xzepr".to_string(),
            aud: "xzepr-api".to_string(),
            token_type: TokenType::Access,
            roles,
            permissions,
        }
    }

    #[test]
    fn test_complexity_config_default() {
        let config = ComplexityConfig::default();
        assert_eq!(config.max_complexity, 100);
        assert_eq!(config.max_depth, 10);
        assert!(config.enforce);
        assert!(!config.playground_enabled);
    }

    #[test]
    fn test_complexity_config_permissive() {
        let config = ComplexityConfig::permissive();
        assert_eq!(config.max_complexity, 1000);
        assert_eq!(config.max_depth, 20);
        assert!(!config.enforce);
        assert!(
            config.playground_enabled,
            "permissive config should enable playground"
        );
    }

    #[test]
    fn test_complexity_config_production() {
        let config = ComplexityConfig::production();
        assert_eq!(config.max_complexity, 50);
        assert_eq!(config.max_depth, 8);
        assert!(config.enforce);
        assert!(
            !config.playground_enabled,
            "production config must disable playground"
        );
    }

    #[test]
    fn test_query_complexity_analyzer() {
        let config = ComplexityConfig::default();
        let analyzer = QueryComplexityAnalyzer::new(config.clone());

        assert_eq!(analyzer.max_complexity(), config.max_complexity);
        assert_eq!(analyzer.max_depth(), config.max_depth);
        assert_eq!(analyzer.is_enforced(), config.enforce);
    }

    #[test]
    fn test_query_complexity_extension() {
        let config = ComplexityConfig::default();
        let extension = QueryComplexityExtension::new(config.clone());

        assert_eq!(extension.analyzer().max_complexity(), config.max_complexity);
    }

    #[test]
    fn test_claims_with_roles() {
        let claims =
            create_test_claims(vec!["admin".to_string()], vec!["events:write".to_string()]);

        assert_eq!(claims.sub, "user123");
        assert!(claims.roles.contains(&"admin".to_string()));
        assert!(claims.permissions.contains(&"events:write".to_string()));
    }

    fn build_test_schema_with_claims(
        claims: Option<Claims>,
    ) -> async_graphql::Schema<
        QueryRoot,
        async_graphql::EmptyMutation,
        async_graphql::EmptySubscription,
    > {
        let mut builder = async_graphql::Schema::build(
            QueryRoot,
            async_graphql::EmptyMutation,
            async_graphql::EmptySubscription,
        );
        if let Some(c) = claims {
            builder = builder.data(c);
        }
        builder.finish()
    }

    #[tokio::test]
    async fn test_require_auth_with_valid_claims_returns_data() {
        let claims = create_test_claims(vec![], vec![]);
        let schema = build_test_schema_with_claims(Some(claims));
        let response = schema.execute("{ protectedField }").await;
        assert!(
            response.errors.is_empty(),
            "expected no errors, got: {:?}",
            response.errors
        );
        assert_eq!(
            response.data.into_json().unwrap()["protectedField"],
            serde_json::json!("Protected data")
        );
    }

    #[tokio::test]
    async fn test_require_auth_without_claims_returns_error() {
        let schema = build_test_schema_with_claims(None);
        let response = schema.execute("{ protectedField }").await;
        assert!(
            !response.errors.is_empty(),
            "expected an authentication error"
        );
        let error_msg = response.errors[0].message.to_lowercase();
        assert!(
            error_msg.contains("unauthorized") || error_msg.contains("authentication"),
            "unexpected error message: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_require_roles_with_admin_role_returns_data() {
        let claims = create_test_claims(vec!["admin".to_string()], vec![]);
        let schema = build_test_schema_with_claims(Some(claims));
        let response = schema.execute("{ adminField }").await;
        assert!(
            response.errors.is_empty(),
            "expected no errors, got: {:?}",
            response.errors
        );
        assert_eq!(
            response.data.into_json().unwrap()["adminField"],
            serde_json::json!("Admin data")
        );
    }

    #[tokio::test]
    async fn test_require_roles_without_admin_role_returns_error() {
        let claims = create_test_claims(vec!["user".to_string()], vec![]);
        let schema = build_test_schema_with_claims(Some(claims));
        let response = schema.execute("{ adminField }").await;
        assert!(!response.errors.is_empty(), "expected a role error");
    }

    #[tokio::test]
    async fn test_require_permissions_with_valid_permission_returns_data() {
        let claims = create_test_claims(vec![], vec!["events:write".to_string()]);
        let schema = build_test_schema_with_claims(Some(claims));
        let response = schema.execute("{ eventsWriteField }").await;
        assert!(
            response.errors.is_empty(),
            "expected no errors, got: {:?}",
            response.errors
        );
        assert_eq!(
            response.data.into_json().unwrap()["eventsWriteField"],
            serde_json::json!("Events data")
        );
    }

    #[tokio::test]
    async fn test_require_permissions_without_permission_returns_error() {
        let claims = create_test_claims(vec![], vec![]);
        let schema = build_test_schema_with_claims(Some(claims));
        let response = schema.execute("{ eventsWriteField }").await;
        assert!(!response.errors.is_empty(), "expected a permission error");
    }

    #[tokio::test]
    async fn test_public_field_without_auth_returns_data() {
        let schema = build_test_schema_with_claims(None);
        let response = schema.execute("{ publicField }").await;
        assert!(
            response.errors.is_empty(),
            "expected no errors, got: {:?}",
            response.errors
        );
        assert_eq!(
            response.data.into_json().unwrap()["publicField"],
            serde_json::json!("Public data")
        );
    }

    /// Builds a test schema that injects the given `AuthenticatedUser` into context.
    fn build_test_schema_with_auth_user(
        user: Option<AuthenticatedUser>,
    ) -> async_graphql::Schema<
        QueryRoot,
        async_graphql::EmptyMutation,
        async_graphql::EmptySubscription,
    > {
        let mut builder = async_graphql::Schema::build(
            QueryRoot,
            async_graphql::EmptyMutation,
            async_graphql::EmptySubscription,
        );
        if let Some(u) = user {
            builder = builder.data(u);
        }
        builder.finish()
    }

    /// Creates an `AuthenticatedUser` with a custom `sub` claim.
    fn create_user_with_sub(sub: impl Into<String>) -> AuthenticatedUser {
        let base = create_test_claims(vec![], vec![]);
        AuthenticatedUser {
            claims: Claims {
                sub: sub.into(),
                ..base
            },
        }
    }

    /// Extracts the extension code string from a `ServerError`, if present.
    fn server_error_code(err: &async_graphql::ServerError) -> Option<String> {
        err.extensions
            .as_ref()
            .and_then(|ext| ext.get("code"))
            .and_then(|v| {
                if let async_graphql::Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
    }

    #[tokio::test]
    async fn test_require_authenticated_user_with_user_returns_user() {
        let user = create_user_with_sub("test-user");
        let schema = build_test_schema_with_auth_user(Some(user));
        let response = schema.execute("{ authUserField }").await;
        assert!(
            response.errors.is_empty(),
            "expected no errors, got: {:?}",
            response.errors
        );
    }

    #[tokio::test]
    async fn test_require_authenticated_user_without_user_returns_unauthenticated() {
        let schema = build_test_schema_with_auth_user(None);
        let response = schema.execute("{ authUserField }").await;
        assert!(
            !response.errors.is_empty(),
            "expected an authentication error"
        );
        assert_eq!(
            server_error_code(&response.errors[0]),
            Some(crate::api::graphql::error_codes::CODE_UNAUTHENTICATED.to_string()),
            "expected UNAUTHENTICATED extension code"
        );
    }

    #[test]
    fn test_parse_caller_user_id_with_valid_id_returns_id() {
        let valid_id = UserId::new().to_string();
        let user = create_user_with_sub(&valid_id);
        let result = parse_caller_user_id(&user);
        assert!(result.is_ok(), "valid ULID should parse: {:?}", result);
        assert_eq!(result.unwrap().to_string(), valid_id);
    }

    #[test]
    fn test_parse_caller_user_id_with_invalid_id_returns_error() {
        let user = create_user_with_sub("not-a-ulid");
        let result = parse_caller_user_id(&user);
        assert!(result.is_err(), "invalid ULID should fail");
        let err = result.unwrap_err();
        assert_eq!(
            err.extensions
                .as_ref()
                .and_then(|e| e.get("code"))
                .and_then(|v| {
                    if let async_graphql::Value::String(s) = v {
                        Some(s.as_str())
                    } else {
                        None
                    }
                }),
            Some(crate::api::graphql::error_codes::CODE_VALIDATION_ERROR),
            "expected VALIDATION_ERROR extension code"
        );
    }

    #[test]
    fn test_require_ownership_same_owner_returns_ok() {
        let id = UserId::new();
        let result = require_ownership(id, id, "event");
        assert!(result.is_ok(), "same owner should succeed");
    }

    #[test]
    fn test_require_ownership_different_owner_returns_forbidden() {
        let caller = UserId::new();
        let owner = UserId::new();
        let result = require_ownership(caller, owner, "event");
        assert!(result.is_err(), "different owner should fail");
        let err = result.unwrap_err();
        assert_eq!(
            err.extensions
                .as_ref()
                .and_then(|e| e.get("code"))
                .and_then(|v| {
                    if let async_graphql::Value::String(s) = v {
                        Some(s.as_str())
                    } else {
                        None
                    }
                }),
            Some(crate::api::graphql::error_codes::CODE_FORBIDDEN),
            "expected FORBIDDEN extension code"
        );
    }

    // Mock schema for testing
    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        async fn public_field(&self) -> String {
            "Public data".to_string()
        }

        async fn protected_field(&self, ctx: &Context<'_>) -> Result<String> {
            require_auth(ctx)?;
            Ok("Protected data".to_string())
        }

        async fn admin_field(&self, ctx: &Context<'_>) -> Result<String> {
            require_roles(ctx, &["admin"])?;
            Ok("Admin data".to_string())
        }

        async fn events_write_field(&self, ctx: &Context<'_>) -> Result<String> {
            require_permissions(ctx, &["events:write"])?;
            Ok("Events data".to_string())
        }

        /// Resolver used to test `require_authenticated_user`.
        async fn auth_user_field(&self, ctx: &Context<'_>) -> Result<String> {
            let user = require_authenticated_user(ctx)?;
            Ok(format!("Authenticated as: {}", user.user_id()))
        }
    }
}
