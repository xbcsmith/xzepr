// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/graphql/guards.rs

use async_graphql::*;
use std::sync::Arc;

use crate::auth::jwt::claims::Claims;

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

/// Query complexity configuration
#[derive(Debug, Clone)]
pub struct ComplexityConfig {
    /// Maximum query complexity allowed
    pub max_complexity: usize,
    /// Maximum query depth allowed
    pub max_depth: usize,
    /// Whether to enforce complexity limits
    pub enforce: bool,
}

impl Default for ComplexityConfig {
    fn default() -> Self {
        Self {
            max_complexity: 100,
            max_depth: 10,
            enforce: true,
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
        }
    }

    /// Creates a permissive config for development
    pub fn permissive() -> Self {
        Self {
            max_complexity: 1000,
            max_depth: 20,
            enforce: false,
        }
    }

    /// Creates a strict config for production
    pub fn production() -> Self {
        Self {
            max_complexity: 50,
            max_depth: 8,
            enforce: true,
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
    }

    #[test]
    fn test_complexity_config_permissive() {
        let config = ComplexityConfig::permissive();
        assert_eq!(config.max_complexity, 1000);
        assert_eq!(config.max_depth, 20);
        assert!(!config.enforce);
    }

    #[test]
    fn test_complexity_config_production() {
        let config = ComplexityConfig::production();
        assert_eq!(config.max_complexity, 50);
        assert_eq!(config.max_depth, 8);
        assert!(config.enforce);
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

    // Mock schema for testing
    #[allow(dead_code)]
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
    }
}
