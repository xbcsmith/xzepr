# Phase 10: Harden GraphQL Authorization - Domain, Application, and Infrastructure Layers

## Summary

This document describes the domain, application, and infrastructure changes made
as part of Phase 10 (Harden GraphQL authorization). The goal is to allow callers
to scope queries by ownership boundaries at every layer of the stack, so that
authenticated users can never accidentally retrieve resources they do not own.

---

## Changes by Layer

### Domain Layer

#### `src/domain/repositories/event_receiver_repo.rs`

`FindEventReceiverCriteria` gained a new optional field:

```src/domain/repositories/event_receiver_repo.rs#L89-90
/// Owner filter; when set, only receivers owned by this user are returned.
pub owner_id: Option<UserId>,
```

A fluent builder method was added:

```src/domain/repositories/event_receiver_repo.rs#L131-134
/// Sets the owner ID filter.
pub fn with_owner_id(mut self, owner_id: UserId) -> Self {
    self.owner_id = Some(owner_id);
    self
}
```

`is_empty()` was extended so that a criteria containing only an `owner_id` is
correctly recognised as non-empty, preventing the default-pagination
short-circuit from discarding the ownership filter.

#### `src/domain/repositories/event_receiver_group_repo.rs`

The same pattern was applied to `FindEventReceiverGroupCriteria`:

- `pub owner_id: Option<UserId>` field added.
- `with_owner_id(owner_id: UserId) -> Self` builder method added.
- `is_empty()` updated to include `&& self.owner_id.is_none()`.

---

### Infrastructure Layer

#### `src/infrastructure/database/postgres_event_receiver_repo.rs`

`build_where_clause` was updated so that:

1. The existing `fingerprint` block now increments `param_count` (it no longer
   sits at the end of the conditions list).
2. A new block appends `owner_id = $N` when `criteria.owner_id` is set.

`find_by_criteria` was updated to bind the `owner_id` string value immediately
after the `fingerprint` binding, preserving positional parameter order.

#### `src/infrastructure/database/postgres_event_receiver_group_repo.rs`

`find_by_criteria` was updated in two places:

1. **Conditions block** - after the `enabled` check, a new block pushes
   `g.owner_id = $N` into the conditions vector and increments `param_count`.
2. **Binding block** - `owner_id` is bound after `enabled` and **before**
   `contains_receiver_id`. This order is mandatory: the positional parameters in
   the generated SQL must match the order in which values are bound.

---

### Application Layer

#### `src/application/handlers/event_receiver_handler.rs`

New public method `find_event_receivers_for_user`:

- Accepts a `FindEventReceiverCriteria` and a `UserId`.
- Unconditionally replaces any caller-supplied `owner_id` on the criteria with
  the authoritative `owner_id` argument.
- Applies default pagination (`limit = 50`, `offset = 0`) when not provided.
- Delegates to `repository.find_by_criteria`.

This prevents callers from bypassing the ownership boundary by supplying an
arbitrary `owner_id` in the criteria.

#### `src/application/handlers/event_receiver_group_handler.rs`

Three new public methods were added:

**`find_event_receiver_groups_for_user`** Owner-scoped version of
`find_by_criteria`. Forces `owner_id` onto the criteria and applies default
pagination before delegating to the group repository.

**`enable_event_receiver_group_for_user`** Calls `group_repository.is_owner`
first. Returns `AuthorizationError::PermissionDenied` immediately if the check
fails, then delegates to `enable_event_receiver_group`.

**`disable_event_receiver_group_for_user`** Identical ownership-gate pattern to
`enable_event_receiver_group_for_user`.

---

## Design Decisions

### Owner ID is always forced, never trusted from caller input

In both `find_event_receivers_for_user` and
`find_event_receiver_groups_for_user`, the `owner_id` argument replaces whatever
the caller may have placed in the criteria struct. This is intentional. A
GraphQL resolver must pass the authenticated user's ID as the `owner_id`
argument; it should not read an `owner_id` out of untrusted input fields and
pass it through to the criteria.

### Binding order matches WHERE clause parameter order

PostgreSQL uses positional parameters (`$1`, `$2`, ...). The binding calls in
`find_by_criteria` must appear in the same order as the corresponding `$N`
placeholders in the generated SQL. For the group repository this means:

```text
id, name, group_type, version, enabled, owner_id, contains_receiver_id
```

Placing the `owner_id` bind call after `enabled` and before
`contains_receiver_id` is not optional.

---

## Test Coverage

| Test                                                                 | Location         | What It Verifies                      |
| -------------------------------------------------------------------- | ---------------- | ------------------------------------- |
| `test_find_event_receiver_criteria_with_owner_id`                    | domain repo      | builder sets `owner_id` field         |
| `test_find_event_receiver_criteria_is_not_empty_with_owner_id`       | domain repo      | `is_empty()` returns `false`          |
| `test_find_event_receiver_group_criteria_with_owner_id`              | domain repo      | builder sets `owner_id` field         |
| `test_find_event_receiver_group_criteria_is_not_empty_with_owner_id` | domain repo      | `is_empty()` returns `false`          |
| `test_find_event_receivers_for_user_scopes_by_owner`                 | receiver handler | method succeeds and returns empty vec |
| `test_find_event_receiver_groups_for_user_scopes_by_owner`           | group handler    | method succeeds and returns empty vec |
| `test_enable_event_receiver_group_for_user_denies_non_owner`         | group handler    | `is_owner = false` yields `Err`       |
| `test_disable_event_receiver_group_for_user_denies_non_owner`        | group handler    | `is_owner = false` yields `Err`       |
