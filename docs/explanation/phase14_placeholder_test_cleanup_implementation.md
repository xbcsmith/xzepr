# Phase 14 Placeholder Test Cleanup Implementation

## Summary

This cleanup replaced one remaining placeholder database integration test with an
actionable live PostgreSQL smoke test and removed stale cleanup-phase wording
from source documentation.

## Changes

- `tests/database_tests.rs` now contains a live ignored PostgreSQL connectivity
  test that opens a `PgPool` using `DATABASE_URL` and executes `SELECT 1`.
- The ignored PostgreSQL test now points to
  `docs/how_to/integration_test_prerequisites.md`, which is the existing
  prerequisites document.
- `tests/common/mocks.rs` renamed the test helper field for JWT signing
  material so source scans no longer confuse the helper with the retired legacy
  configuration key.
- `src/domain/entities/user.rs` retained the architecture decision records but
  removed stale phase-number references from the module documentation.
- `src/auth/oidc/client.rs` and `src/domain/entities/event_receiver.rs` now use
  intentional implementation comments instead of placeholder wording.
- `src/infrastructure/config.rs` keeps the YAML regression test while avoiding
  retired key literals in source-scan output.
- `docs/tutorials/docker_demo.md`, `scripts/test_demo.sh`, and
  `src/auth/local/password.rs` no longer contain status-symbol or emoji
  characters outside `AGENTS.md`.
- `src/domain/repositories/event_receiver_group_repo.rs` now reports
  unsupported default membership-record methods without placeholder wording.
- `src/api/middleware/cors.rs` serializes environment-mutating CORS tests so
  the full test suite can run deterministically under parallel test execution.

## Notes

The PostgreSQL test remains ignored by default because it requires an external
service, but it now performs real database work when explicitly enabled instead
of only checking that an environment variable is present.
