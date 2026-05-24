# Phase 4: Consolidate Duplicate Abstractions - Implementation Summary

## Overview

Phase 4 of the XZepr codebase cleanup plan addressed five categories of
duplicated or misplaced abstractions that made the codebase harder to maintain
and that violated the intended layered architecture. Tasks 4.1 and 4.3 were
completed in earlier work. This document covers Tasks 4.2, 4.4, and 4.5, plus
the integration and export updates in Task 4.7.

Tasks covered here:

- Task 4.2: Introduce application ports (EventPublisher)
- Task 4.4: Consolidate REST and repository helpers
- Task 4.5: Consolidate roles, permissions, and lifecycle events
- Task 4.7: Deliverables and integration

Previously completed tasks:

- Task 4.1: Unify user and API key persistence (see
  `phase4_task4_1_duplicate_type_elimination.md`)
- Task 4.3: Consolidate ID and validation patterns (see
  `phase4_task4_3_ulid_macro_implementation.md`)

---

## Task 4.2: Introduce Application Ports

### Problem

All three application handlers imported and stored the concrete infrastructure
type `KafkaEventPublisher` directly:

```rust
// Before: Application layer depending on Infrastructure
use crate::infrastructure::messaging::producer::KafkaEventPublisher;

pub struct EventHandler {
    event_publisher: Option<Arc<KafkaEventPublisher>>,
}
```

This violated the architecture constraint that the Application layer must not
depend on Infrastructure. It made unit-testing handlers without Kafka impossible
and locked the application to a single messaging backend.

### Solution

#### New file: `src/domain/repositories/event_publisher.rs`

The `EventPublisher` output port trait is placed in the domain layer so that
both the Application layer (which uses it) and the Infrastructure layer (which
implements it) can reference it without violating any layer dependency rule.

```rust
#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: &Event) -> crate::error::Result<()>;

    async fn publish_with_receiver(
        &self,
        event: &Event,
        receiver: &EventReceiver,
    ) -> crate::error::Result<()> {
        let _ = receiver;
        self.publish(event).await
    }

    async fn publish_with_group(
        &self,
        event: &Event,
        group: &EventReceiverGroup,
    ) -> crate::error::Result<()> {
        let _ = group;
        self.publish(event).await
    }
}
```

The default implementations for `publish_with_receiver` and `publish_with_group`
delegate to `publish`, so a minimal implementation only needs to implement one
method while retaining the ability to override the richer variants.

#### Updated: `src/infrastructure/messaging/producer.rs`

`KafkaEventPublisher` now implements `EventPublisher`. A private
`send_cloud_event_message_internal` helper was extracted to avoid duplication
across the three trait methods. The `publish_with_receiver` and
`publish_with_group` overrides use the corresponding
`CloudEventMessage::from_event_with_*` constructors to embed receiver or group
metadata in the CloudEvents envelope.

#### Updated: all three application handlers

All handler structs changed from `Option<Arc<KafkaEventPublisher>>` to
`Option<Arc<dyn EventPublisher>>`, and all `with_publisher` constructors updated
accordingly:

```rust
// After: Application layer depends only on the domain port
use crate::domain::repositories::event_publisher::EventPublisher;

pub struct EventHandler {
    event_publisher: Option<Arc<dyn EventPublisher>>,
}
```

### Layer dependency after changes

```text
Domain: EventPublisher trait  (new)
    ^
    | implemented by
Infrastructure: KafkaEventPublisher impl EventPublisher

    ^
    | used as Arc<dyn EventPublisher>
Application: EventHandler, EventReceiverHandler, EventReceiverGroupHandler
```

---

## Task 4.4: Consolidate REST and Repository Helpers

### Problem

Several patterns were repeated throughout the codebase with no shared helper:

- Parsing path IDs required inline boilerplate in every REST handler
- Converting `Option<T>` to a `RepositoryError::EntityNotFound` was duplicated
  across repository implementations
- Pagination validation was copy-pasted into `list_events`,
  `list_event_receivers`, and `list_event_receiver_groups`

### Solution

#### New file: `src/infrastructure/database/repo_helpers.rs`

Contains `require_entity<T>`, a generic helper that converts an optional
repository result to `RepositoryError::EntityNotFound`:

```rust
pub fn require_entity<T>(result: Option<T>, entity: &str) -> Result<T, RepositoryError> {
    result.ok_or_else(|| RepositoryError::EntityNotFound {
        entity: entity.to_string(),
    })
}
```

#### Updated: `src/api/rest/mod.rs`

Added `parse_path_id<T: FromStr>`, a generic REST helper that parses a path
segment string into a typed identifier and produces a consistent `BAD_REQUEST`
error response on failure:

```rust
pub fn parse_path_id<T>(id_str: &str, entity: &str)
    -> Result<T, (StatusCode, Json<ErrorResponse>)>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
```

#### Updated: `src/domain/validation.rs`

Added `validate_pagination(limit: usize, max_limit: usize)`, which centralises
the `1 <= limit <= max_limit` guard that was previously copy-pasted into every
paginated list method:

```rust
pub fn validate_pagination(limit: usize, max_limit: usize) -> ValidationResult
```

---

## Task 4.5: Consolidate Roles, Permissions, and Lifecycle Events

### Problem 1: Duplicated lifecycle event builders

Both `EventReceiverHandler` and `EventReceiverGroupHandler` contained private
`create_*_event` methods with identical structure:

```rust
// Duplicated in EventReceiverHandler
fn create_receiver_created_event(&self, receiver: &EventReceiver) -> Event { ... }

// Duplicated in EventReceiverGroupHandler
fn create_group_created_event(&self, group: &EventReceiverGroup) -> Event { ... }
```

These also coupled the application handlers to infrastructure concerns because
they built `CloudEventMessage` objects directly.

### Solution: `src/application/lifecycle_events.rs`

Two standalone public functions replace the duplicated private methods:

```rust
pub fn build_receiver_created_event(receiver: &EventReceiver) -> Event;
pub fn build_group_created_event(group: &EventReceiverGroup) -> Event;
```

Both return pure domain `Event` objects. The infrastructure conversion to
`CloudEventMessage` is delegated entirely to `EventPublisher` implementations.
The handlers now use these functions and call `publish_with_receiver` or
`publish_with_group` on the publisher:

```rust
// After: clean Application -> Domain dependency only
let system_event = build_receiver_created_event(&event_receiver);
publisher.publish_with_receiver(&system_event, &event_receiver).await?;
```

### Problem 2: Permission canonical string inconsistency

`Permission::Display` previously delegated to the `Debug` representation
(`"EventCreate"`, `"EventRead"`, etc.), which produced PascalCase output
inconsistent with `Role::Display` (which uses `"event_manager"`, `"user"`,
etc.).

### Solution: updated `src/auth/rbac/permissions.rs`

`Permission` now has a consistent lowercase snake_case `Display`
(`"event_create"`, `"receiver_read"`, etc.) that matches the style used by
`Role`. A new `PermissionParseError` and `FromStr` implementation make the
permission canonical string bidirectionally parseable:

```rust
impl std::str::FromStr for Permission {
    type Err = PermissionParseError;
    // Accepts snake_case, hyphen-separated, and camelCase aliases.
}
```

The Serde-derived serialization (`"EventCreate"`) is intentionally preserved for
wire and database compatibility.

---

## Files Changed

### New files

| File                                          | Purpose                                  |
| --------------------------------------------- | ---------------------------------------- |
| `src/domain/repositories/event_publisher.rs`  | EventPublisher output port trait         |
| `src/application/lifecycle_events.rs`         | Shared lifecycle event builder functions |
| `src/infrastructure/database/repo_helpers.rs` | Repository utility helpers               |

### Modified files

| File                                                       | Change                                                      |
| ---------------------------------------------------------- | ----------------------------------------------------------- |
| `src/domain/repositories/mod.rs`                           | Added `event_publisher` module and re-export                |
| `src/application/mod.rs`                                   | Added `lifecycle_events` module and re-exports              |
| `src/application/handlers/event_handler.rs`                | Port-based publisher dependency                             |
| `src/application/handlers/event_receiver_handler.rs`       | Port + lifecycle events                                     |
| `src/application/handlers/event_receiver_group_handler.rs` | Port + lifecycle events                                     |
| `src/infrastructure/messaging/producer.rs`                 | Implements EventPublisher trait                             |
| `src/infrastructure/messaging/mod.rs`                      | Re-exports EventPublisher from domain                       |
| `src/infrastructure/database/mod.rs`                       | Added `repo_helpers` module                                 |
| `src/domain/validation.rs`                                 | Added `validate_pagination` helper                          |
| `src/api/rest/mod.rs`                                      | Added `parse_path_id` helper                                |
| `src/auth/rbac/permissions.rs`                             | Added `PermissionParseError`, `FromStr`, snake_case Display |
| `src/auth/rbac/mod.rs`                                     | Re-exports `PermissionParseError`                           |
| `src/lib.rs`                                               | Canonical re-exports for all new public items               |

---

## Task 4.8 Success Criteria Assessment

| Criterion                                                  | Result                                                                     |
| ---------------------------------------------------------- | -------------------------------------------------------------------------- |
| Changes to ID behavior happen in one place                 | Met (Tasks 4.1, 4.3 prior work)                                            |
| Changes to validation happen in one place                  | Met (`domain/validation.rs`)                                               |
| Changes to error mapping happen in one place               | Met (`error.rs`)                                                           |
| Changes to permission strings happen in one place          | Met (`permissions.rs`)                                                     |
| Layer dependencies match intended architecture             | Met (Application no longer imports Infrastructure)                         |
| Duplicate logic replaced by documented reusable components | Met (lifecycle_events, parse_path_id, require_entity, validate_pagination) |

---

## Quality Gate Results

All four mandatory quality gates pass with zero project errors or warnings:

```text
cargo fmt --all                                           -- ok
cargo check --all-targets --all-features                 -- ok (0 errors)
cargo clippy --all-targets --all-features -- -D warnings -- ok (0 warnings)
cargo test --all-features                                -- ok (706 unit tests,
                                                              0 failures,
                                                              9 ignored)
```

The only diagnostic present is a third-party `num-bigint-dig` future-
incompatibility notice that is outside the scope of this project.
