# Phase 14: Placeholder Tests and Documentation Implementation

## Overview

Phase 14 covers two task groups:

- Tasks 14.1 through 14.3: Replace placeholder tests with real domain and router
  tests, add security regression tests, and gate all external-service tests with
  `#[ignore]`.
- Tasks 14.4 and 14.5: Documentation alignment and repository rule enforcement.

This document summarizes the work performed and the design decisions made.

## Motivation

Earlier phases introduced placeholder test bodies (functions that compiled but
did not assert any real behaviour). These tests gave false confidence: the test
count appeared healthy while large sections of the codebase had no actual
coverage. Phase 14 addressed this by replacing each placeholder with a real test
against domain logic or the HTTP router layer.

Additionally, documentation files accumulated emojis in section headings and
inline text. The project rules in `AGENTS.md` prohibit emojis outside
`AGENTS.md` itself, so all affected files were cleaned up.

## Test Strategy Change

### Before Phase 14

Many integration test functions followed this pattern:

```rust
#[tokio::test]
async fn test_something() {
    // TODO: implement
    assert!(true);
}
```

These functions satisfied the compiler and appeared in test output, but they did
not verify any behaviour.

### After Phase 14

Every test function now exercises real logic:

- Domain tests construct entities using public constructors and assert
  invariants on the returned values.
- Router tests build an `axum::Router` in-process with a test JWT service, issue
  requests via `tower::ServiceExt::oneshot`, and assert HTTP status codes and
  response bodies.
- Auth tests exercise the full JWT token lifecycle (issue, validate, reject
  expired or malformed tokens).

No test depends on a running external service unless it is explicitly gated with
`#[ignore]`.

## Security Regression Tests

The file `tests/rbac_rest_integration.rs` contains security regression tests
that verify the RBAC middleware cannot be bypassed at the HTTP layer. Key
scenarios covered:

- Unauthenticated requests receive `401 Unauthorized`.
- Authenticated requests from users lacking the required permission receive
  `403 Forbidden`.
- Authenticated requests from users with the correct role and permission receive
  the expected `2xx` response.
- Public health-check routes remain accessible without a token.
- Tokens signed with a wrong secret are rejected.
- Expired tokens are rejected.
- Tokens missing required claims are rejected.

These tests use an in-process router built with the production middleware stack,
so they exercise the same code path that runs in production without requiring a
database or network.

## External Integration Tests Gating

All tests that require running infrastructure use the `#[ignore]` attribute with
a descriptive reason string. This means:

- `cargo test --all-features` always passes without any external services.
- Infrastructure-dependent tests must be opted into explicitly with
  `-- --ignored`.

Current gated test categories:

| Test file                               | Gated reason                              |
| --------------------------------------- | ----------------------------------------- |
| `tests/kafka_auth_integration_tests.rs` | Requires a Kafka or Redpanda broker       |
| Database live-query tests               | Require a PostgreSQL instance             |
| OIDC flow tests                         | Require a Keycloak or compatible provider |
| OPA policy evaluation tests             | Require a running OPA server              |

See `docs/how_to/integration_test_prerequisites.md` for full instructions on
running these tests.

## Documentation Improvements

### Emoji removal

The following files contained emojis in section headings or inline text. All
emojis were removed and the surrounding text was adjusted to remain clear:

- `docs/explanation/architecture.md`
- `docs/explanation/otlp_integration_validation.md`
- `docs/how-to/opa_rbac_next_steps.md`
- `docs/tutorials/getting_started.md`
- `docs/reference/otlp_quick_reference.md`
- `README.md`

### Stale content in architecture.md

The `docs/explanation/architecture.md` file contained two roadmap sections with
phase headings that included week-based timeline projections (for example,
`### Phase 1: Foundation (Week 1-2)`). These projections were accurate during
early development but became misleading once the project matured. The following
changes were made:

- Timeline markers (`(Week X-Y)`, `(Week X)`) were removed from all phase
  headings in both roadmap sections.
- Checkboxes for completed items were updated from `[ ]` to `[x]` to reflect
  actual project state.
- A note was added to the Implementation Roadmap section stating that all phases
  are now complete.
- A broken line that merged a list item with a heading (`...you need!## Title`)
  was corrected to have the heading on its own line.

### New documentation files

Two new files were created in this phase:

- `docs/how_to/integration_test_prerequisites.md`: Describes all external
  services required by gated tests, lists environment variables, and provides
  Docker Compose start commands.
- `docs/explanation/phase14_placeholder_tests_and_documentation_implementation.md`:
  This document.

## Quality Gates

All changes passed the mandatory quality gate sequence:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Markdown files were additionally processed with:

```bash
markdownlint --fix --config .markdownlint.json <file>
prettier --write --parser markdown --prose-wrap always <file>
```

## Files Changed

| File                                                                             | Change type                          |
| -------------------------------------------------------------------------------- | ------------------------------------ |
| `docs/explanation/architecture.md`                                               | Emoji removal, stale roadmap updates |
| `docs/explanation/otlp_integration_validation.md`                                | Emoji removal                        |
| `docs/how-to/opa_rbac_next_steps.md`                                             | Emoji removal                        |
| `docs/tutorials/getting_started.md`                                              | Emoji removal                        |
| `docs/reference/otlp_quick_reference.md`                                         | Emoji removal                        |
| `README.md`                                                                      | Emoji removal                        |
| `docs/how_to/integration_test_prerequisites.md`                                  | New file                             |
| `docs/explanation/phase14_placeholder_tests_and_documentation_implementation.md` | New file                             |

## Layer Boundaries

All test code respects the established layer boundaries:

- Domain tests import only from `xzepr::domain`.
- Router tests import from `xzepr::api` and `xzepr::auth` but not from
  `xzepr::infrastructure`.
- Infrastructure tests (gated) import from `xzepr::infrastructure`.

No test crosses a boundary in the direction that is prohibited by the
architecture (for example, domain code must not import infrastructure types).

## Running the Test Suite

### Default suite (no external services)

```bash
cargo test --all-features
```

### Kafka integration tests

```bash
docker compose up -d redpanda-0
cargo test --test kafka_auth_integration_tests -- --ignored --test-threads=1
```

### Full suite including all gated tests

```bash
docker compose up -d

DATABASE_URL=postgres://xzepr:password@localhost:5432/xzepr \
  REDIS_URL=redis://localhost:6379 \
  OPA_URL=http://localhost:8181 \
  cargo test -- --ignored --test-threads=1
```
