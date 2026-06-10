// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

pub mod authorization;
pub mod error_codes;
pub mod guards;
pub mod handlers;
pub mod schema;
pub mod types;

pub use authorization::{
    authorize_graphql, authorize_graphql_many, GraphqlAuthorizationDecision,
    GraphqlAuthorizationError, GraphqlAuthorizationOperation, GraphqlAuthorizationService,
    GraphqlPolicyEvaluator,
};
pub use error_codes::{
    conflict, forbidden, internal_error, log_and_internal_error, map_app_error, not_found,
    unauthenticated, validation_error, CODE_CONFLICT, CODE_FORBIDDEN, CODE_INTERNAL_ERROR,
    CODE_NOT_FOUND, CODE_UNAUTHENTICATED, CODE_VALIDATION_ERROR,
};
pub use guards::{
    helpers, parse_caller_user_id, require_auth, require_authenticated_user, require_ownership,
    require_permissions, require_roles, require_roles_and_permissions, ComplexityConfig,
    QueryComplexityAnalyzer, QueryComplexityExtension,
};
pub use handlers::{graphql_handler, graphql_health, graphql_playground};
pub use schema::{create_schema_with_config, create_schema_with_config_and_authorization, Schema};

/// Compile-time API surface tests.
///
/// These tests do not execute meaningful logic; they exist to produce compile
/// errors if a deliberately stable public export is accidentally removed.
#[cfg(test)]
mod api_surface_tests {
    use super::*;

    /// Stable authorization types are accessible from the graphql module.
    #[test]
    fn test_authorization_types_are_exported() {
        let _ = std::any::TypeId::of::<GraphqlAuthorizationService>();
        let _ = std::any::TypeId::of::<GraphqlAuthorizationOperation>();
        let _ = std::any::TypeId::of::<GraphqlAuthorizationDecision>();
        let _ = std::any::TypeId::of::<GraphqlAuthorizationError>();
    }

    /// Stable error code constants and helpers are accessible from the graphql module.
    #[test]
    fn test_error_code_exports_are_stable() {
        assert_eq!(CODE_UNAUTHENTICATED, "UNAUTHENTICATED");
        assert_eq!(CODE_FORBIDDEN, "FORBIDDEN");
        assert_eq!(CODE_NOT_FOUND, "NOT_FOUND");
        assert_eq!(CODE_VALIDATION_ERROR, "VALIDATION_ERROR");
        assert_eq!(CODE_CONFLICT, "CONFLICT");
        assert_eq!(CODE_INTERNAL_ERROR, "INTERNAL_ERROR");
    }

    /// Stable guard helpers are accessible from the graphql module.
    #[test]
    fn test_guard_exports_are_stable() {
        let _ = std::any::TypeId::of::<ComplexityConfig>();
        let _ = std::any::TypeId::of::<QueryComplexityAnalyzer>();
        let _ = std::any::TypeId::of::<QueryComplexityExtension>();
    }

    /// Stable schema types are accessible from the graphql module.
    #[test]
    fn test_schema_exports_are_stable() {
        let _ = std::any::TypeId::of::<Schema>();
    }
}
