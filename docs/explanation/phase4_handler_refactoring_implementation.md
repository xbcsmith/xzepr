# Phase 4 Handler Refactoring Implementation

## Overview

This document describes the Phase 4 refactoring of the three application
handlers to use the new `EventPublisher` port trait and the centralised
`lifecycle_events` module instead of concrete infrastructure types and
duplicated private methods.

## What Changed

### New Modules

#### `src/domain/repositories/event_publisher.rs`

Defines the `EventPublisher` port trait in the domain layer. The trait is
object-safe (`Send + Sync`) and exposes three methods:

- `publish(&Event)` - required, no default
- `publish_with_receiver(&Event, &EventReceiver)` - default delegates to
  `publish`
- `publish_with_group(&Event, &EventReceiverGroup)` - default delegates to
  `publish`

Placing the trait in the domain layer respects the layered architecture rule
that the domain must never import infrastructure code.

#### `src/application/lifecycle_events.rs`

Provides two standalone builder functions that were previously duplicated as
private methods inside their respective handler structs:

- `build_receiver_created_event(receiver: &EventReceiver) -> Event`
- `build_group_created_event(group: &EventReceiverGroup) -> Event`

Centralising the builders ensures payload shape and event names stay consistent
across the codebase and removes the duplication between the two handlers.

### Modified Files

#### `src/application/handlers/event_handler.rs`

| Location               | Before                                    | After                                     |
| ---------------------- | ----------------------------------------- | ----------------------------------------- |
| Import                 | `KafkaEventPublisher` from infrastructure | `EventPublisher` from domain repositories |
| Field type             | `Option<Arc<KafkaEventPublisher>>`        | `Option<Arc<dyn EventPublisher>>`         |
| `with_publisher` param | `Arc<KafkaEventPublisher>`                | `Arc<dyn EventPublisher>`                 |

The `create_event` method body was unchanged because it already called
`publisher.publish(&event)`, which matches the trait method signature exactly.

#### `src/application/handlers/event_receiver_handler.rs`

| Location               | Before                                                                         | After                                                             |
| ---------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------- |
| Imports removed        | `CloudEventMessage`, `KafkaEventPublisher`                                     | -                                                                 |
| Imports added          | `EventPublisher`, `build_receiver_created_event`                               | -                                                                 |
| Field type             | `Option<Arc<KafkaEventPublisher>>`                                             | `Option<Arc<dyn EventPublisher>>`                                 |
| `with_publisher` param | `Arc<KafkaEventPublisher>`                                                     | `Arc<dyn EventPublisher>`                                         |
| Publish call           | `publisher.publish_message(&CloudEventMessage::from_event_with_receiver(...))` | `publisher.publish_with_receiver(&system_event, &event_receiver)` |
| Private method         | `fn create_receiver_created_event` existed                                     | Removed (replaced by `lifecycle_events` module)                   |

The unused `use crate::domain::entities::event::Event` import was also removed
after the private helper method was deleted.

#### `src/application/handlers/event_receiver_group_handler.rs`

| Location               | Before                                                                      | After                                                                |
| ---------------------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Imports removed        | `CloudEventMessage`, `KafkaEventPublisher`                                  | -                                                                    |
| Imports added          | `EventPublisher`, `build_group_created_event`                               | -                                                                    |
| Field type             | `Option<Arc<KafkaEventPublisher>>`                                          | `Option<Arc<dyn EventPublisher>>`                                    |
| `with_publisher` param | `Arc<KafkaEventPublisher>`                                                  | `Arc<dyn EventPublisher>`                                            |
| Publish call           | `publisher.publish_message(&CloudEventMessage::from_event_with_group(...))` | `publisher.publish_with_group(&system_event, &event_receiver_group)` |
| Private method         | `fn create_group_created_event` existed                                     | Removed (replaced by `lifecycle_events` module)                      |

The unused `use crate::domain::entities::event::Event` import was also removed
after the private helper method was deleted.

## Architecture Compliance

The changes enforce the layered dependency rules defined in `AGENTS.md`:

```text
API -> Application -> Domain
Infrastructure -> Domain (for implementation)
Domain -> Infrastructure: NEVER
```

Before this refactoring the application layer imported
`crate::infrastructure::messaging::producer::KafkaEventPublisher` directly,
creating a forbidden downward dependency from application into infrastructure.
After the refactoring the application layer depends only on
`crate::domain::repositories::event_publisher::EventPublisher`, which is a
domain-layer trait. Concrete infrastructure types are injected at startup by the
binary crates (`server`, `admin`), never referenced from application code.

## Testing

All existing handler tests pass unchanged because the test mocks implement the
repository traits, not the publisher. The `lifecycle_events` module carries its
own unit test suite with nine tests covering both builders:

- Correct event name
- Owner ID propagates from the source entity
- Receiver ID field matches the entity
- Payload contains expected keys
- Edge case: empty group synthesises a receiver ID from the group ID

## Quality Gate Results

```text
cargo fmt --all              -- ok (no changes)
cargo check --all-targets    -- ok, 0 errors, 0 warnings (our code)
cargo clippy -D warnings     -- ok, 0 warnings
cargo test --lib application -- ok, 21 passed
```

The 4 pre-existing failures in `api::middleware::rbac::tests` are unrelated to
this change set and were present before this work began.
