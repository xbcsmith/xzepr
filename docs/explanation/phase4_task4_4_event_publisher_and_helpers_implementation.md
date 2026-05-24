# Phase 4 Task 4.4: EventPublisher Trait, Repository Helpers, Domain Validation, REST Helpers, and Permission FromStr

## Overview

This document describes the implementation of Phase 4, Task 4.4 of the XZepr
codebase cleanup plan. The changes deliver five distinct improvements:

1. `EventPublisher` trait implementation for `KafkaEventPublisher`
2. Repository utility helpers (`require_entity`)
3. Extended domain validation (`validate_pagination`)
4. REST path parsing helper (`parse_path_id`)
5. `FromStr` and improved `Display` for `Permission`

---

## 1. EventPublisher Trait and KafkaEventPublisher Implementation

### Files affected

- `src/domain/repositories/event_publisher.rs` (created/updated)
- `src/domain/repositories/mod.rs` (updated)
- `src/infrastructure/messaging/producer.rs` (updated)
- `src/infrastructure/messaging/mod.rs` (updated)

### Design

The `EventPublisher` trait lives in the domain layer:

```rust
#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: &Event) -> crate::error::Result<()>;
    async fn publish_with_receiver(
        &self, event: &Event, receiver: &EventReceiver,
    ) -> crate::error::Result<()>;
    async fn publish_with_group(
        &self, event: &Event, group: &EventReceiverGroup,
    ) -> crate::error::Result<()>;
}
```

The infrastructure layer implementation in `KafkaEventPublisher` follows the
Dependency Inversion Principle: the domain defines the interface, and
infrastructure provides the concrete transport.

### Name-conflict resolution

Both the trait and the `KafkaEventPublisher` inherent impl originally had a
`publish` method with the same signature. Rust permits this: inherent methods
take priority in dot-call resolution, so there is no infinite recursion. The
`impl EventPublisher` block delegates explicitly:

```rust
async fn publish(&self, event: &Event) -> crate::error::Result<()> {
    // Explicit call to the inherent method to avoid ambiguity.
    KafkaEventPublisher::publish(self, event).await
}
```

### Single Kafka I/O path

A private helper `send_cloud_event_message_internal` centralises all Kafka
sending logic. Every public method (inherent and trait) funnels through this
single site, which eliminates duplication and makes future transport changes
trivial.

```text
publish (inherent)          -> from_event          -> send_cloud_event_message_internal
publish_message (inherent)  -> (direct)            -> send_cloud_event_message_internal
EventPublisher::publish     -> from_event          -> send_cloud_event_message_internal
publish_with_receiver       -> from_event_with_receiver -> send_cloud_event_message_internal
publish_with_group          -> from_event_with_group    -> send_cloud_event_message_internal
```

### Layer compliance

- `EventPublisher` trait: `src/domain/repositories/` (domain layer)
- `KafkaEventPublisher` impl: `src/infrastructure/messaging/` (infrastructure
  layer)
- Infrastructure imports domain types; domain never imports infrastructure

---

## 2. Repository Utility Helpers

### File

`src/infrastructure/database/repo_helpers.rs`

### Motivation

Every PostgreSQL repository function that queries by primary key repeats:

```rust
result.ok_or_else(|| RepositoryError::EntityNotFound {
    entity: "Foo".to_string(),
})
```

The `require_entity` helper centralises this pattern:

```rust
pub fn require_entity<T>(result: Option<T>, entity: &str) -> Result<T, RepositoryError> {
    result.ok_or_else(|| RepositoryError::EntityNotFound {
        entity: entity.to_string(),
    })
}
```

This is re-exported from `src/infrastructure/database/mod.rs` as
`crate::infrastructure::database::require_entity`.

---

## 3. Domain Validation: validate_pagination

### File

`src/domain/validation.rs`

### Motivation

Pagination parameters appear in every list-query handler. Placing the guard in
the domain validation module means there is a single source of truth for the
rule `1 <= limit <= max_limit`.

```rust
pub fn validate_pagination(limit: usize, max_limit: usize) -> ValidationResult {
    if limit == 0 || limit > max_limit {
        Err(DomainError::ValidationError {
            field: "limit".to_string(),
            message: format!("limit must be between 1 and {}", max_limit),
        })
    } else {
        Ok(())
    }
}
```

---

## 4. REST Path Parsing Helper

### File

`src/api/rest/mod.rs`

### Motivation

Every REST handler that accepts an ID path segment contains a near-identical
parse-or-return-400 block. The `parse_path_id` generic helper removes this
boilerplate:

```rust
pub fn parse_path_id<T>(id_str: &str, entity: &str)
    -> Result<T, (StatusCode, Json<ErrorResponse>)>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    id_str.parse::<T>().map_err(|_| (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(
            "invalid_id".to_string(),
            format!("Invalid {} ID format", entity),
        )),
    ))
}
```

Because all domain identifier types generated by `define_ulid_id!` implement
`FromStr`, this helper works for `EventId`, `EventReceiverId`,
`EventReceiverGroupId`, `UserId`, and `ApiKeyId` without any extra code.

---

## 5. Permission Display, FromStr, and PermissionParseError

### File

`src/auth/rbac/permissions.rs`

### Motivation

The original `Display` implementation used `{:?}` (Rust Debug format), producing
PascalCase strings such as `"EventCreate"`. This created two problems:

- The string form was identical to the Serde representation, making it
  impossible to distinguish display output from wire format in logs or tests.
- There was no `FromStr` implementation, preventing round-trip parsing.

### Solution

`Display` now uses explicit arms returning lowercase snake_case:

```rust
Permission::EventCreate => write!(f, "event_create"),
```

`FromStr` accepts snake_case, hyphenated variants, camelCase aliases, and is
case-insensitive:

```rust
"event_create" | "eventcreate" => Ok(Permission::EventCreate),
```

`PermissionParseError` follows the same pattern as `RoleParseError`.

### Serde compatibility

The `derive(Serialize, Deserialize)` attribute is retained unchanged, so the
wire/storage representation remains PascalCase (`"EventCreate"`). Only the
`Display` and `FromStr` implementations use snake_case.

### RBAC middleware update

The RBAC enforcement middleware converts a `Permission` to a string via
`to_string()` and compares it against JWT claim permission strings. Because
`Display` now produces snake_case, all JWT claims (in both unit and integration
tests) were updated to use snake_case format (e.g., `"event_read"` instead of
`"EventRead"`).

---

## Quality Gate Results

All four mandatory gates pass with zero errors or warnings:

```text
cargo fmt --all                                        # no changes
cargo check --all-targets --all-features               # 0 errors, 0 warnings
cargo clippy --all-targets --all-features -- -D warnings  # 0 warnings
cargo test --all-features                              # 0 failures
```

Test totals: 697 lib tests + 14 integration tests + 90 doc tests, all passing.
