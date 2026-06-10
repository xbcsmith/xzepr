# Phase 17.3 and 17.4: GraphQL Guard Stable Error Codes and Module API Surface

## Overview

Tasks 17.3 and 17.4 complete the stabilisation of the GraphQL error-code
contract and tighten the public API surface of the `src/api/graphql` module.

## Task 17.3: Stable extension codes in legacy guard functions

### Problem

The legacy Claims-based guard functions (`require_auth`, `require_roles`,
`require_permissions`) returned `async_graphql::Error` values created with plain
`Error::new(...)`. These errors carried no `extensions.code` field, so API
clients could not programmatically distinguish authentication failures from
authorisation failures without parsing human-readable strings.

### Solution

All error returns in the three affected functions now delegate to the
`error_codes` helpers introduced in earlier phases:

| Guard                 | Old error                               | New error                           |
| --------------------- | --------------------------------------- | ----------------------------------- |
| `require_auth`        | `Error::new("Unauthorized: ...")`       | `error_codes::unauthenticated(...)` |
| `require_roles`       | `Error::new(format!("Forbidden: ..."))` | `error_codes::forbidden(...)`       |
| `require_permissions` | `Error::new(format!("Forbidden: ..."))` | `error_codes::forbidden(...)`       |

The human-readable messages are unchanged, so existing tests that assert on
message text continue to pass.

### Doc comment warnings

Each of the three functions now carries a `# Warning` section in its doc comment
explaining that the function reads `Claims` from the GraphQL context. Because
the production `graphql_handler` injects `AuthenticatedUser` rather than
`Claims`, production resolvers must call `require_authenticated_user` instead.
These guards remain correct and intentional for test schemas and internal
helpers.

### New tests

Three new `#[tokio::test]` cases were added to `guards.rs`:

- `test_require_auth_without_claims_returns_unauthenticated_code` - asserts that
  the `UNAUTHENTICATED` extension code is present in the error response.
- `test_require_roles_without_required_role_returns_forbidden_code` - asserts
  that the `FORBIDDEN` extension code is present.
- `test_require_permissions_without_required_permission_returns_forbidden_code`
  - asserts that the `FORBIDDEN` extension code is present.

These tests reuse the `server_error_code` helper and the existing
`build_test_schema_with_claims` / `QueryRoot` test infrastructure already
present in the module.

## Task 17.4: Prune public re-exports and add API surface tests

### Removed re-export

`OpaClientGraphqlEvaluator` was removed from the top-level
`pub use authorization::{...}` list in `src/api/graphql/mod.rs`. It is an
infrastructure implementation detail (the concrete OPA HTTP client). Callers
that must construct it directly can use the full path
`crate::api::graphql::authorization::OpaClientGraphqlEvaluator`; the module
itself is still `pub` so this is not a breaking change to internal code.
Production resolvers depend on the `GraphqlPolicyEvaluator` trait and are
unaffected.

### Compile-time API surface tests

A new `#[cfg(test)] mod api_surface_tests` block was added at the bottom of
`mod.rs` containing four tests:

| Test                                    | What it guards                                                                                                              |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `test_authorization_types_are_exported` | `GraphqlAuthorizationService`, `GraphqlAuthorizationOperation`, `GraphqlAuthorizationDecision`, `GraphqlAuthorizationError` |
| `test_error_code_exports_are_stable`    | All six `CODE_*` string constants                                                                                           |
| `test_guard_exports_are_stable`         | `ComplexityConfig`, `QueryComplexityAnalyzer`, `QueryComplexityExtension`                                                   |
| `test_schema_exports_are_stable`        | `Schema` type alias                                                                                                         |

These tests use `std::any::TypeId::of` for types and direct constant comparisons
for string constants. They produce compile errors immediately if a stable export
is accidentally removed, with no runtime cost.

## Files changed

- `src/api/graphql/guards.rs` - updated three guard functions and their doc
  comments; added three new extension-code assertion tests.
- `src/api/graphql/mod.rs` - removed `OpaClientGraphqlEvaluator` re-export;
  added `api_surface_tests` module.

## Validation

```bash
cargo fmt --all                         # clean
cargo check --lib --all-features        # clean
cargo clippy --lib --all-features -- -D warnings  # clean
cargo test --lib --all-features         # 937 passed
```

All 65 tests in `api::graphql::*` pass, including the three new guard
extension-code tests and the four new surface tests.
