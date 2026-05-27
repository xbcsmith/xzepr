# Phase 3: Lifecycle Event Builder Cleanup

## Overview

This focused Phase 3 cleanup removes the remaining production panic path from
application lifecycle event construction. The receiver and group lifecycle event
builders now return typed domain errors instead of calling `expect` when
`Event::new` rejects generated event data.

The cleanup keeps lifecycle publishing best-effort. Creating a receiver or group
still succeeds after the entity is saved even if lifecycle event construction or
publishing fails. The failure is logged with the affected entity ID for operator
visibility.

## Changes Made

### Typed builder results

`build_receiver_created_event` and `build_group_created_event` now return
`Result<Event, DomainError>`. Each function delegates directly to `Event::new`,
so any event validation failure is preserved as the original typed domain error.

The public documentation for both builders now describes the error behavior and
includes compiling examples that use `?` rather than panic-oriented unwraps.

### Caller handling

The receiver and group create handlers now match on lifecycle event builder
results before publishing. On success they publish the event as before. On
failure they log the typed `DomainError` and continue returning the persisted
entity ID, matching the existing best-effort publishing behavior.

### Tests

The lifecycle event unit tests were updated for the new `Result` return type.
They continue to verify event names, owners, receiver IDs, and payload fields
for receiver and group creation events.

## Files Modified

- `src/application/lifecycle_events.rs`
- `src/application/handlers/event_receiver_handler.rs`
- `src/application/handlers/event_receiver_group_handler.rs`
- `docs/explanation/phase3_lifecycle_event_builder_cleanup_implementation.md`

## Validation

The following validation was run for this focused cleanup:

1. `cargo fmt --all --check`
2. `cargo test lifecycle_events --all-features`
3. `cargo test --doc --all-features lifecycle_events`
4. `markdownlint --fix --config .markdownlint.json docs/explanation/phase3_lifecycle_event_builder_cleanup_implementation.md`
5. `prettier --write --parser markdown --prose-wrap always docs/explanation/phase3_lifecycle_event_builder_cleanup_implementation.md`
