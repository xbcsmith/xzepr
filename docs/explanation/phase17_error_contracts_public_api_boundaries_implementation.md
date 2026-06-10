# Phase 17: Finish Error Contracts and Public API Boundaries

## Overview

Phase 17 completes the typed-error, GraphQL error-code, public-export, and
architecture-boundary work that was still leaking internal details or
stringifying source errors before the response boundary. Five tasks were
implemented in parallel, each with disjoint write scopes.

## Task 17.1: Replace String-Carrying Auth Endpoint Errors

**File changed:** `src/api/rest/auth.rs`

The module-local `AuthError` enum had five string-carrying variants that
discarded sub-cause identity before reaching `IntoResponse`:

- `Oidc(String)` - not used in any production path; removed
- `Jwt(String)` - replaced with six typed JWT variants
- `Session(String)` - replaced with `SessionStoreFailure`
- `Config(String)` - unused; removed
- `Internal(String)` - replaced with `RepositoryFailure` and
  `TokenGenerationFailed`

### New Typed Variants

| Variant                                  | Inner type                                     | Purpose                                          |
| ---------------------------------------- | ---------------------------------------------- | ------------------------------------------------ |
| `JwtValidation(JwtError)`                | `auth::jwt::error::JwtError`                   | `validate_token` failures                        |
| `JwtRevocationFailed(JwtError)`          | `auth::jwt::error::JwtError`                   | `revoke_token` failures                          |
| `TokenGenerationFailed(JwtError)`        | `auth::jwt::error::JwtError`                   | token signing failures                           |
| `JwtMissingHeader`                       | unit                                           | absent `Authorization` header                    |
| `JwtInvalidHeader`                       | unit                                           | non-UTF-8 header value                           |
| `JwtBearerSchemeMissing`                 | unit                                           | non-`Bearer` auth scheme                         |
| `JwtInvalidSubject`                      | unit                                           | invalid `sub` claim (source logged at call site) |
| `SessionStoreFailure(SessionStoreError)` | `auth::oidc::session_store::SessionStoreError` | OIDC session backend errors                      |
| `RepositoryFailure`                      | unit                                           | user repo errors (source logged at call site)    |

### Key Principle

Sources are preserved as typed values wherever possible. For unit variants
(`JwtInvalidSubject`, `RepositoryFailure`), the caller logs the source error via
`tracing::error!` before returning the typed variant so that `IntoResponse` can
remain free of `format!` conversions.

### IntoResponse Sanitization

All nine new arms log internal context via `tracing::error!` and return only
sanitized public messages. No internal JWT key paths, Redis URLs, or database
error messages ever appear in HTTP responses.

## Task 17.2: Normalize Storage and API-Key Error Identity

**Files changed:** `src/error.rs`,
`src/infrastructure/database/postgres_api_key_repo.rs`,
`src/api/graphql/error_codes.rs`

### New Typed AuthError Variants

Added to `crate::error::AuthError`:

```rust
ApiKeyConstraintViolation { constraint: String },
ApiKeyConcurrencyConflict,
ApiKeyRowDecodeError { column: String, detail: String },
ApiKeyInvalidId { detail: String },
```

These variants preserve SQL constraint type identity (unique, foreign-key,
not-null, check, deadlock) across the auth-repository boundary.

### PostgreSQL API Key Repository

`postgres_api_key_repo.rs` now:

- SQL execution errors go through `classify_sqlx_error` and
  `map_classified_error` which maps `RepositoryError::ConstraintViolation` to
  `ApiKeyConstraintViolation`, `RepositoryError::ConcurrencyConflict` to
  `ApiKeyConcurrencyConflict`, and unknown errors to the existing `StorageError`
  fallback.
- Row decode failures produce `ApiKeyRowDecodeError { column, detail }`.
- ID parse failures produce `ApiKeyInvalidId { detail }`.

The `StorageError { message }` variant is retained for generic SQL failures and
the user-lookup path in `ApiKeyService::verify_api_key`.

### GraphQL Error Mapping

`error_codes::map_app_error` was extended:

- `ApiKeyConstraintViolation` maps to `CONFLICT`
- `ApiKeyRowDecodeError`, `ApiKeyInvalidId`, `StorageError` map to
  `INTERNAL_ERROR` with the `"api_key_storage"` log context

## Task 17.3: Finish GraphQL Public Error Extensions

**File changed:** `src/api/graphql/guards.rs`

The legacy Claims-based guard functions (`require_auth`, `require_roles`,
`require_permissions`) used free-form `Error::new(...)` strings without stable
`extensions.code` values.

### Changes

- `require_auth`: error changed from `Error::new("Unauthorized: ...")` to
  `error_codes::unauthenticated("Unauthorized: Authentication required")`
- `require_roles`: forbidden error changed to `error_codes::forbidden(...)`
- `require_permissions`: forbidden error changed to
  `error_codes::forbidden(...)`
- `require_roles_and_permissions` delegates to the above and needed no direct
  change

Doc comments on all three legacy guard functions now include a `# Warning`
section explaining that production resolvers must use
`require_authenticated_user` instead.

### New Tests

Three extension-code tests were added confirming stable codes:

- `test_require_auth_without_claims_returns_unauthenticated_code`
- `test_require_roles_without_required_role_returns_forbidden_code`
- `test_require_permissions_without_required_permission_returns_forbidden_code`

## Task 17.4: Finish Public API Export Pruning

**File changed:** `src/api/graphql/mod.rs`

### Export Pruning

`OpaClientGraphqlEvaluator` was removed from the top-level `graphql::` module
re-exports. It is an infrastructure implementation detail; callers that need
dependency injection should depend on the `GraphqlPolicyEvaluator` trait
instead. The concrete type remains accessible at its full path
`crate::api::graphql::authorization::OpaClientGraphqlEvaluator`.

### Compile-Time API Surface Tests

A new `#[cfg(test)] mod api_surface_tests` block was added to `mod.rs` with four
compile-time guard tests:

- `test_authorization_types_are_exported` - stable authorization types
- `test_error_code_exports_are_stable` - all six `CODE_*` constants
- `test_guard_exports_are_stable` - complexity guard types
- `test_schema_exports_are_stable` - `Schema` type alias

## Task 17.5: Resolve Remaining Architecture Boundary Decisions

**Files created:** `docs/explanation/domain_auth_boundary_decision.md`,
`tests/architecture_boundary_tests.rs`

### Formal ADR Document

`docs/explanation/domain_auth_boundary_decision.md` captures both boundary
exceptions with rationale, risks, long-term migration paths, and pointers to the
guard tests:

- ADR-1: Password hashing in domain entity
- ADR-2: RBAC types imported from auth layer

### Architecture Boundary Tests

`tests/architecture_boundary_tests.rs` adds five architecture guard tests:

| Test                                                       | Enforcement                                                                   |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `test_domain_files_do_not_import_api_layer`                | Scans all `src/domain/**/*.rs` for `use crate::api`                           |
| `test_domain_files_do_not_import_infrastructure`           | Scans all `src/domain/**/*.rs` for `use crate::infrastructure`                |
| `test_no_auth_specific_user_repository`                    | Scans all `src/auth/**/*.rs` for competing `UserRepository` trait definitions |
| `test_canonical_user_repository_is_accessible`             | Compile-time check the canonical trait still exists                           |
| `test_domain_boundary_exceptions_are_documented_in_source` | Confirms ADR-1 and ADR-2 comments remain in `user.rs`                         |

The source-scanning tests use `starts_with("use ")` matching to avoid false
positives from doc example strings.

## Deliverables Satisfied

- Auth, API-key storage, resource-context, and OPA failures preserve typed
  source identity until centralized response mapping.
- GraphQL helpers and resolvers consistently return stable public error
  extensions (`extensions.code`).
- `OpaClientGraphqlEvaluator` removed from top-level `graphql::` module exports.
- Compile-time API surface tests added for the graphql module.
- Domain/auth boundary decisions documented in a formal ADR file.
- Architecture guard tests scan the full domain layer, not only `user.rs`.
- Repository-boundary tests prevent new auth-specific user persistence
  abstractions from bypassing the canonical `UserRepository`.

## Quality Gates

All quality gates pass in order:

```text
cargo fmt --all              -- clean
cargo check --all-targets   -- clean (0 errors, 0 warnings from crate)
cargo clippy -- -D warnings  -- clean (0 warnings)
cargo test --all-features   -- all passed, 0 failed
```
