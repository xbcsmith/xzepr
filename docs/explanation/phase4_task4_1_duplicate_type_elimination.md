# Phase 4 Task 4.1: Duplicate Type Elimination Implementation

## Overview

This document describes the changes made in Phase 4, Task 4.1 of the XZepr
cleanup plan. The goal was to eliminate duplicate `UserRepository` trait
definitions, duplicate `PostgresUserRepository` struct definitions, and to place
the `ApiKey` struct in the correct architectural layer (domain).

## Problem Statement

The codebase had three overlapping problems:

### Problem 1: Conflicting UserRepository Traits

Two modules independently declared a `UserRepository` trait with identical names
but different error types:

- `src/domain/repositories/user_repo.rs` - returns `DomainError`
- `src/auth/api_key.rs` - returns `AuthError`

This created a naming collision when both were exported through `lib.rs`, making
it impossible for callers to know which `UserRepository` was intended.

### Problem 2: Conflicting PostgresUserRepository Structs

Two files defined a `PostgresUserRepository` struct:

- `src/infrastructure/database/postgres_user_repo.rs` - implements the domain
  `UserRepository` trait with comprehensive CRUD operations
- `src/infrastructure/database/postgres.rs` - implements the auth-layer
  `UserRepository` trait with simpler auth-focused operations

The `lib.rs` re-exported the wrong one (`postgres::PostgresUserRepository`)
while `infrastructure::database` re-exported the correct domain-oriented one.
This meant `bin/admin.rs` (via `xzepr::PostgresUserRepository`) used a different
struct than `main.rs` (via `infrastructure::database`).

### Problem 3: ApiKey in Wrong Layer

`src/auth/api_key.rs` defined the `ApiKey` struct directly. Because `ApiKey`
references `ApiKeyId` and `UserId` domain value objects and is a core business
concept, it belongs in the domain entity layer, not the auth layer.

## Solution

### Task A: ApiKey Domain Entity

Created `src/domain/entities/api_key.rs` containing:

- `ApiKey` struct with full documentation and accessor methods
- Unit tests covering enabled/disabled state, name, and expiry accessors

The auth layer (`src/auth/api_key.rs`) now re-exports it:

```rust
pub use crate::domain::entities::api_key::ApiKey;
```

This keeps backward compatibility for any code importing `auth::api_key::ApiKey`
while establishing the canonical home in the domain layer.

### Task B: Domain Entities Module

Updated `src/domain/entities/mod.rs` to declare `pub mod api_key` and add
`pub use api_key::ApiKey` for convenient access via `domain::entities::ApiKey`.

### Task C: AuthUserRepository Rename

Renamed `UserRepository` in `src/auth/api_key.rs` to `AuthUserRepository`.

The rename distinguishes the auth-layer user contract (returns `AuthError`,
covers only the operations needed for authentication) from the domain-layer
`UserRepository` (returns `DomainError`, full CRUD surface).

Full documentation was added to every public item in the module. Three unit
tests were added for the key generation and hashing helpers.

### Task D: AuthUserRepository Implementation on PostgresUserRepository

Added a second `impl` block to
`src/infrastructure/database/postgres_user_repo.rs`:

```rust
#[async_trait]
impl crate::auth::api_key::AuthUserRepository for PostgresUserRepository { ... }
```

The implementation bridges the two layers:

- `find_by_id` / `find_by_username` / `find_all` delegate to the domain
  `UserRepository` implementation via fully-qualified syntax
  (`<Self as UserRepository>::method(...)`) and map `DomainError` to
  `AuthError::OidcError`
- `save`, `add_role`, `remove_role` use direct SQL against the `user_roles`
  junction table (matching the schema used by the previous `postgres.rs`
  implementation)

### Task E: Dedicated PostgresApiKeyRepository Module

Created `src/infrastructure/database/postgres_api_key_repo.rs` containing
`PostgresApiKeyRepository` extracted verbatim from `postgres.rs`, now
implementing `auth::api_key::ApiKeyRepository`. All error mapping was updated to
use structured `AuthError::OidcError { message }` instead of bare
`AuthError::InvalidCredentials`.

### Task F: Emptied postgres.rs

`src/infrastructure/database/postgres.rs` was replaced with a stub comment. All
public types that previously lived there (`PostgresUserRepository` and
`PostgresApiKeyRepository`) have been moved to dedicated files.

### Task G: Updated database/mod.rs

Removed `pub mod postgres` and `pub use postgres::PostgresApiKeyRepository`.
Added `pub mod postgres_api_key_repo` and
`pub use postgres_api_key_repo::PostgresApiKeyRepository`.

### Task H: Fixed lib.rs Re-export

Changed:

```rust
pub use infrastructure::database::postgres::{PostgresApiKeyRepository, PostgresUserRepository};
```

to:

```rust
pub use infrastructure::database::{PostgresApiKeyRepository, PostgresUserRepository};
```

This ensures both re-exports now refer to the domain-oriented
`PostgresUserRepository` (from `postgres_user_repo.rs`) rather than the old
auth-oriented one (from the now-empty `postgres.rs`).

### Task I: Updated bin/admin.rs

Changed:

```rust
use xzepr::auth::api_key::UserRepository;
```

to:

```rust
use xzepr::auth::api_key::AuthUserRepository;
```

The trait is imported to bring its methods into scope for method dispatch on
`Arc<PostgresUserRepository>`. No method call sites required changes because the
signatures of `AuthUserRepository` are identical to the old `UserRepository`.

## Architectural Impact

### Layer Dependency After Changes

```text
domain::entities::api_key::ApiKey
    (canonical definition)
        |
        v re-exported by
auth::api_key
    - AuthUserRepository trait  (auth-focused, returns AuthError)
    - ApiKeyRepository trait
    - ApiKeyService

        |
        v implemented by
infrastructure::database::postgres_user_repo::PostgresUserRepository
    - impl domain::repositories::user_repo::UserRepository
    - impl auth::api_key::AuthUserRepository  (NEW, bridges via delegation)

infrastructure::database::postgres_api_key_repo::PostgresApiKeyRepository
    - impl auth::api_key::ApiKeyRepository
```

### Single Canonical PostgresUserRepository

After these changes, there is exactly one `PostgresUserRepository` type in the
codebase, exported canonically through `infrastructure::database` and through
`xzepr` (the crate root). All consumers (`bin/admin.rs`, `main.rs`,
`auth/provisioning.rs`) now refer to the same type.

## Quality Gate Results

All four mandatory quality gates passed:

```text
cargo fmt --all                              -- ok
cargo check --all-targets --all-features     -- ok (0 errors, 0 warnings from project code)
cargo clippy --all-targets --all-features -- -D warnings  -- ok (0 warnings from project code)
cargo test --all-features                    -- ok (667 unit tests + 84 doctests, 0 failures)
```

The only warnings present are from the third-party crate `num-bigint-dig` and
are outside the scope of this task.
