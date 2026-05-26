# Phase 13: API Surface, Architecture Boundaries, and Duplicate Model Cleanup

## Overview

Phase 13 completes public API pruning, unifies the user repository boundary,
documents deliberate domain/auth architecture exceptions, and removes
unjustified suppression attributes introduced before or during earlier cleanup
phases.

## Task 13.1: Crate and Module Export Pruning

### Problem

`src/lib.rs` re-exported infrastructure concrete types at the crate root:

- `PostgresUserRepository` and `PostgresApiKeyRepository` from
  `infrastructure::database`
- `TopicManager` from `infrastructure::messaging`
- Application-internal helpers: `EventHandler`, `EventReceiverGroupHandler`,
  `EventReceiverHandler`, `create_schema`, `Schema`
- Domain-internal validation helpers: `validate_json_object`,
  `validate_max_length`, `validate_pagination`, `validate_required_string`,
  `validate_semver`
- Lifecycle event builders: `build_group_created_event`,
  `build_receiver_created_event`

These exports were convenient during early development but expose implementation
details as stable public API, making future refactors unnecessarily breaking.

`src/api/rest/mod.rs` re-exported the entire `dtos` module with
`pub use dtos::*`, hiding the exact set of public types behind a wildcard.

### Solution

**`src/lib.rs`**: Infrastructure concrete types, application handlers, GraphQL
schema, and validation helpers were removed from the crate root. The stable
public API retained at the root is:

| Export                             | Reason                    |
| ---------------------------------- | ------------------------- |
| `Error`, `Result`                  | Cross-cutting error types |
| `domain::entities::*`              | Core business objects     |
| `domain::value_objects::*`         | Typed identifier types    |
| `auth::api_key::ApiKeyService`     | Auth application service  |
| `auth::rbac::Role`, `Permission`   | Authorization types       |
| `infrastructure::config::Settings` | Runtime configuration     |

Compile-time API surface tests in `lib.rs` enforce that these exports remain
accessible without requiring a live database or server.

**`src/api/rest/mod.rs`**: The `pub use dtos::*` wildcard was replaced with an
explicit named list of all 19 public types and functions from the `dtos` module.
This makes the REST public surface visible without reading `dtos.rs`.

**`src/bin/admin.rs` and `src/main.rs`**: Updated to use full module paths for
the removed root re-exports:

- `xzepr::infrastructure::database::{PostgresApiKeyRepository, PostgresUserRepository}`
- `xzepr::infrastructure::messaging::TopicManager`

## Task 13.2: Unify User Repository Boundaries

### Problem

`PostgresUserRepository` implemented two separate traits with duplicate SQL:

1. `UserRepository` (canonical domain trait)
2. `AuthUserRepository` (auth-layer adapter)

Three methods in `AuthUserRepository` re-implemented the same SQL queries
already present in `UserRepository`:

- `find_by_id` performed a direct SELECT, duplicating
  `UserRepository::find_by_id`
- `find_by_username` performed a direct SELECT, duplicating
  `UserRepository::find_by_username`
- `find_all` performed a direct SELECT, duplicating
  `UserRepository::list(i64::MAX, 0)`

All three methods also used `AuthError::OidcError` to report storage failures -
the wrong error variant for non-OIDC database errors.

### Solution

The three duplicating methods were replaced with delegations to
`UserRepository`:

```xzepr/src/infrastructure/database/postgres_user_repo.rs#L1-1
async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AuthError> {
    <Self as UserRepository>::find_by_id(self, &id)
        .await
        .map_err(|e| AuthError::StorageError { message: ... })
}
```

The `save`, `add_role`, and `remove_role` methods retain direct SQL because they
implement operations that have no direct equivalent in `UserRepository`
(transactional upsert with role sync, role insert, role delete).

All error mappings in the `AuthUserRepository` impl were updated from
`AuthError::OidcError` to `AuthError::StorageError`, which is the correct
variant for database-level failures that have nothing to do with the OIDC
protocol.

### Remaining Work

`save`, `add_role`, and `remove_role` still contain SQL that partially overlaps
with `UserRepository`. The full unification path requires adding a generic
`save` (upsert) and role-management methods to the `UserRepository` trait. This
is deferred because:

1. Adding role-management methods to the domain trait introduces auth types
   (`Role`) into the domain layer boundary, conflicting with ADR-2.
2. A separate cleanup phase should first relocate `Role` and `Permission` to the
   domain layer before adding them to domain trait signatures.

## Task 13.3: Domain/Auth Boundary Decisions

### Identified Violations

`domain/entities/user.rs` contains two layering violations:

**Violation 1 - Password hashing in domain**: The `hash_password` and
`verify_password` functions use Argon2 directly. Identical functions exist in
`auth::local::password`. The domain entity calls these during `User::new_local`
construction.

**Violation 2 - RBAC types from auth layer**: `User` holds `Vec<Role>` and
implements `has_permission`. Both `Role` and `Permission` are defined in
`auth::rbac`, creating a domain-to-auth import.

### Decisions (ADR-1 and ADR-2)

**ADR-1 (Password hashing)**: Documented as a deliberate exception. Entity
construction needs password hashing so that no `User` value ever holds
plaintext. Moving hashing to the auth service layer requires accepting a
pre-hashed credential at construction time, which changes the public API of
`User::new_local`. Deferred.

**ADR-2 (RBAC types)**: Documented as a deliberate exception. `Role` and
`Permission` are core user identity data - they are a domain concept living in
the wrong module. The fix is to relocate `Role`/`Permission` to `domain::rbac`.
This touches every import site in the codebase and is deferred to a dedicated
cleanup phase.

Both ADRs are written as inline doc comments in `user.rs` and are verified by
architecture guard tests that fail if the documentation is removed without
resolving the underlying issue.

## Task 13.4: Suppression and Compatibility Cleanup

### `#[allow(dead_code)]` on `hash_api_key` in `auth/api_key.rs`

Phase 12 added a backward-compatibility shim `fn hash_api_key` that simply
called `hash_api_key_v1`. No production code called it; only the test
`test_hash_api_key_backward_compat` did. The function was removed along with its
test. V1 hash semantics are covered by `test_hash_api_key_v1_deterministic` and
`test_hash_api_key_with_config_v1`.

### `#[allow(dead_code)]` on `build_where_clause` in `postgres_event_receiver_repo.rs`

This suppression is narrow and justified: `build_where_clause` is no longer
called by production code (Phase 12 replaced it with `QueryBuilder`) but is
retained for diagnostic use and direct unit testing. An inline comment was added
to document this intent.

## Files Modified

| File                                                          | Change                                                                                         |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                                  | Removed infrastructure/application/GraphQL/validation root re-exports; added API surface tests |
| `src/api/rest/mod.rs`                                         | Replaced `pub use dtos::*` with explicit named exports                                         |
| `src/api/middleware/mod.rs`                                   | Added documentation for the intentional stable middleware API                                  |
| `src/bin/admin.rs`                                            | Updated imports to use full module paths                                                       |
| `src/main.rs`                                                 | Updated `TopicManager` import to use full module path                                          |
| `src/infrastructure/database/postgres_user_repo.rs`           | Delegated 3 methods; fixed OidcError to StorageError                                           |
| `src/domain/entities/user.rs`                                 | Added ADR doc comments and architecture guard tests                                            |
| `src/auth/api_key.rs`                                         | Removed dead `hash_api_key` wrapper and redundant test                                         |
| `src/infrastructure/database/postgres_event_receiver_repo.rs` | Narrowed suppression comment on `build_where_clause`                                           |

## Success Criteria

- Crate public API at root reflects only deliberately stable exports.
- Explicit `pub use dtos::{...}` list shows the complete REST DTO surface.
- User persistence SQL exists in one place (UserRepository impl) for read
  operations; write operations delegated as much as possible.
- Boundary exceptions are documented with ADR numbers and architecture guard
  tests that enforce the documentation is present.
- No unjustified `#[allow(dead_code)]` attributes in production source.
