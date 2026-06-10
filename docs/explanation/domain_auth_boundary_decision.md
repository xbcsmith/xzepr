# Domain and Auth Layer Boundary Decisions

## Overview

The XZepr architecture enforces strict layer boundaries:

- API layer may depend on Application, Domain, Auth, and Infrastructure.
- Application layer may depend on Domain.
- Infrastructure layer may depend on Domain.
- Domain layer must NOT depend on API, Application, or Infrastructure.
- Auth layer is a peer layer providing authentication and RBAC.

Two deliberate exceptions to these rules exist in the domain layer. Both are
documented here with their rationale, risks, and guard tests.

## ADR-1: Password Hashing in the Domain Entity

### Decision

`User::new_local` performs Argon2 password hashing during entity construction.
The functions `hash_password` and `verify_password` are duplicated from
`crate::auth::local::password`.

### Rationale

A `User` value must never hold a plaintext password. Performing hashing at
construction time enforces this invariant without requiring callers to hash
passwords before constructing a `User`. Moving hashing to an auth service would
require every `User::new_local` call site to hold a reference to the auth layer,
coupling the call sites to auth infrastructure.

### Risks

- Argon2 is an infrastructure concern. If the hash algorithm changes, the domain
  entity must be updated.
- The duplication between `domain::entities::user` and `auth::local::password`
  creates a maintenance burden. The two implementations can diverge.

### Long-Term Path

Accept a pre-hashed `PasswordHash` value object at `User::new_local`
construction time. Move all hashing to the auth service caller. This eliminates
the duplicate implementations and removes the Argon2 dependency from the domain.

### Guard Tests

`tests/architecture_boundary_tests.rs::test_domain_user_does_not_import_api_layer`
verifies that `user.rs` contains no direct API or infrastructure imports.

`xzepr/src/domain/entities/user.rs::test_domain_user_boundary_exceptions_are_documented`
verifies that the ADR comments remain present in the source file.

## ADR-2: RBAC Types Imported from the Auth Layer

### Decision

`User` carries `Vec<Role>` and the `has_permission` method, both of which import
`crate::auth::rbac::{permissions::Permission, roles::Role}`. This creates a
domain-to-auth import that inverts the intended layering.

### Rationale

`Role` and `Permission` represent fundamental user identity data. They influence
business rules (for example, which events a user may create) that belong in the
domain. Removing them from `User` would require every consumer of user identity
to reach into the auth layer directly, spreading the layering inversion across
many call sites.

### Risks

- The domain depends on auth-layer types. Changes to `Role` or `Permission`
  require coordinated updates in both the auth and domain layers.
- New auth-specific role concepts can leak into domain code if added to the
  shared `Role` enum without review.

### Long-Term Path

Relocate `Role` and `Permission` to a new `crate::domain::rbac` module. This
makes them a domain concept without any auth-layer dependency. The auth layer
imports from `crate::domain::rbac` instead of defining the types. This
refactoring touches every import site and is deferred to a dedicated cleanup
phase.

### Guard Tests

`tests/architecture_boundary_tests.rs::test_no_auth_specific_user_repository`
verifies that the auth layer does not define its own `UserRepository`
abstraction that could bypass the canonical domain repository.

`tests/architecture_boundary_tests.rs::test_domain_files_do_not_import_api_layer`
scans all domain source files for prohibited API-layer imports.

`tests/architecture_boundary_tests.rs::test_domain_files_do_not_import_infrastructure`
scans all domain source files for prohibited infrastructure-layer imports.

## Canonical User Repository Boundary

`crate::domain::repositories::user_repo::UserRepository` is the single canonical
user persistence abstraction. The auth layer, admin CLI, provisioning service,
and API key service all use this trait. No separate auth-specific user
repository trait exists. Architecture boundary tests enforce this constraint.
