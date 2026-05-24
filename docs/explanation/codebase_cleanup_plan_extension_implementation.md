# Codebase Cleanup Plan Extension Implementation

## Overview

This document summarizes the planning update that extends the codebase cleanup
plan with additional remediation phases. The update was made after a gap review
found that several deliverables from the original phases remained incomplete or
only partially wired into the canonical runtime path.

The implementation changed only planning documentation. It did not modify Rust
source code, runtime configuration, tests, or deployment assets.

## Changes Made

The cleanup plan now includes eight additional phases:

- Phase 7: Make runtime configuration authoritative and fail-fast.
- Phase 8: Complete OIDC runtime integration and session lifecycle.
- Phase 9: Wire OPA authorization and fail-safe resource policies.
- Phase 10: Harden GraphQL authorization and public error contracts.
- Phase 11: Complete typed error and storage boundary normalization.
- Phase 12: Harden persistence transactions and query construction.
- Phase 13: Finish API surface, architecture boundaries, and duplicate model
  cleanup.
- Phase 14: Replace placeholder tests and finalize documentation and rule
  compliance.

The recommended execution order, highest-risk files list, and definition of done
were also expanded so the plan now explicitly covers the previously missed
runtime, security, persistence, testing, documentation, and project-rule
requirements.

## Gap Categories Covered

The added phases cover the following missing deliverable categories:

- Authoritative production configuration loading and validation.
- OIDC startup wiring, session TTL handling, redirect validation, and disabled
  route behavior.
- OPA router integration, resource context enforcement, and production fail-safe
  behavior.
- GraphQL resolver-level authorization, error extensions, and query limits.
- Typed errors for rate limiting, auth, OPA, resource context, repository, and
  SQL failures.
- Transactional association updates and safe dynamic query construction.
- Public API surface pruning and user repository boundary cleanup.
- Replacement of mock-only and ignored tests with real or properly gated test
  coverage.
- Documentation accuracy and repository rule compliance, including emoji removal
  outside `AGENTS.md`.

## Validation

No Rust code was changed, so Rust compilation checks were not required for this
planning-only update. Markdown formatting and linting passed for the changed
documentation files:

- `docs/explanation/codebase_cleanup_plan.md`
- `docs/explanation/codebase_cleanup_plan_extension_implementation.md`
