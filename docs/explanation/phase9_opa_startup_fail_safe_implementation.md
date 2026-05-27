# Phase 9 OPA Startup Fail-Safe Implementation

## Summary

This cleanup makes the canonical startup path fail fast when OPA authorization is
enabled but the OPA middleware state cannot be constructed.

## Changes

- `src/main.rs::build_opa_state` now returns `Result<Option<OpaMiddlewareState>>`.
- Disabled OPA still returns `Ok(None)` so deployments that explicitly disable
  OPA keep the existing RBAC router behavior.
- Enabled OPA now propagates `OpaClient::new` failures with startup context
  instead of silently returning `None` and falling back to RBAC.
- Production `fail_open_development` mode now aborts startup if it reaches this
  path, preventing accidental fail-open authorization in the canonical runtime
  entrypoint.

## Security Impact

Operators can no longer enable OPA in configuration and accidentally run the
production router without OPA because client construction failed. Startup either
builds an OPA middleware state or reports the configuration/construction error
before binding the listener.
