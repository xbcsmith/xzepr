# Phase 10: GraphQL Authorization Hardening Implementation

## Overview

Phase 10 hardens the GraphQL API surface against unauthorized access. Before
this phase, several resolvers returned data without checking caller identity,
and error responses lacked stable machine-readable codes. This document records
the four sub-tasks completed across three agent passes.

## Changes Made

### Task 10.1: Resolver-Level Authorization

Every query and mutation resolver now enforces authentication before touching
any application handler. The resolvers updated are:

| Resolver                        | Guard added                                           | Handler variant used                       |
| ------------------------------- | ----------------------------------------------------- | ------------------------------------------ |
| `eventReceiversById`            | `require_authenticated_user` + `parse_caller_user_id` | `get_event_receiver_for_user`              |
| `eventReceiverGroupsById`       | `require_authenticated_user` + `parse_caller_user_id` | `get_event_receiver_group_for_user`        |
| `eventReceivers`                | `require_authenticated_user` + `parse_caller_user_id` | `find_event_receivers_for_user`            |
| `eventReceiverGroups`           | `require_authenticated_user` + `parse_caller_user_id` | `find_event_receiver_groups_for_user`      |
| `setEventReceiverGroupEnabled`  | `require_authenticated_user` + `parse_caller_user_id` | `enable_event_receiver_group_for_user`     |
| `setEventReceiverGroupDisabled` | `require_authenticated_user` + `parse_caller_user_id` | `disable_event_receiver_group_for_user`    |
| `eventsById`                    | upgraded from direct `ctx.data` to guards             | `get_event_for_user` (unchanged)           |
| `events`                        | upgraded from direct `ctx.data` to guards             | `find_events_for_user` (unchanged)         |
| `createEvent`                   | upgraded from direct `ctx.data` to guards             | `create_event` (unchanged)                 |
| `createEventReceiver`           | upgraded from direct `ctx.data` to guards             | `create_event_receiver` (unchanged)        |
| `createEventReceiverGroup`      | upgraded from direct `ctx.data` to guards             | `create_event_receiver_group` (unchanged)  |
| `addGroupMember`                | upgraded from direct `ctx.data` to guards             | unchanged; manual ownership check retained |
| `removeGroupMember`             | upgraded from direct `ctx.data` to guards             | unchanged; manual ownership check retained |

The `for_user` handler variants set `owner_id` in the repository criteria so
that the SQL layer filters results to the caller before any data is returned to
the resolver. Callers receive an empty list, not a permission error, when they
request a resource that belongs to a different owner but exists in the database.
This prevents enumeration of other users' resource identifiers.

The `addGroupMember` and `removeGroupMember` resolvers retain an explicit
in-resolver ownership check because they call `find_group_by_id` (unscoped) to
retrieve the group and then verify `group.owner_id() == caller`. The error
messages produced by this check are now routed through `error_codes::forbidden`
instead of bare `Error::new` strings.

### Task 10.2: Aligned GraphQL Guards with Runtime Context

Before Phase 10, some guards operated on `Claims` (extracted from the JWT
middleware chain) while production resolvers relied on `AuthenticatedUser`
injected by `graphql_handler`. The two types are structurally equivalent but are
stored under different context keys.

Agent B introduced two helpers in `src/api/graphql/guards.rs` that work
exclusively with `AuthenticatedUser`:

- `require_authenticated_user(ctx)` - uses `ctx.data_opt::<AuthenticatedUser>()`
  and returns `UNAUTHENTICATED` if the user is absent. This is the correct guard
  for production resolvers because `graphql_handler` injects
  `AuthenticatedUser`, not `Claims`.
- `parse_caller_user_id(user)` - parses `user.user_id()` into a `UserId` ULID
  and returns `VALIDATION_ERROR` if the subject claim is malformed.

Every resolver in `schema.rs` now calls these two helpers as its first two
operations, so authentication and identity parsing are applied uniformly before
any handler or repository is touched.

The legacy `require_auth` / `require_roles` / `require_permissions` guards
(which read `Claims`) remain available for middleware and non-production
contexts; they were not removed.

### Task 10.3: Stable GraphQL Error Extensions

Agent B added `src/api/graphql/error_codes.rs` with six stable codes:

| Constant                | Value                | When to use                               |
| ----------------------- | -------------------- | ----------------------------------------- |
| `CODE_UNAUTHENTICATED`  | `"UNAUTHENTICATED"`  | No valid credential presented             |
| `CODE_FORBIDDEN`        | `"FORBIDDEN"`        | Authenticated but lacks permission        |
| `CODE_NOT_FOUND`        | `"NOT_FOUND"`        | Resource does not exist                   |
| `CODE_VALIDATION_ERROR` | `"VALIDATION_ERROR"` | Input failed validation                   |
| `CODE_CONFLICT`         | `"CONFLICT"`         | Unique constraint or concurrency conflict |
| `CODE_INTERNAL_ERROR`   | `"INTERNAL_ERROR"`   | Unhandled infrastructure error            |

Every public helper (`unauthenticated`, `forbidden`, `not_found`,
`validation_error`, `conflict`, `internal_error`) attaches the code to
`error.extensions["code"]`. The `map_app_error` function maps
`crate::error::Error` variants to the correct code; internal error details are
logged via `tracing` and never included in the returned message.

All `Error::new(format!("..."))` calls in `schema.rs` were replaced:

- Auth failures use `require_authenticated_user` / `parse_caller_user_id` which
  call `error_codes::unauthenticated` and `error_codes::validation_error`
  internally.
- Handler errors use `error_codes::map_app_error(e)`.
- Ownership check failures use `error_codes::forbidden(...)`.
- Resource not found uses `error_codes::not_found("group")`.

### Task 10.4: GraphQL Security Limits

#### Playground gating

`ComplexityConfig` gained a `playground_enabled: bool` field (default `false`).
The production config sets it to `false`; the permissive development config sets
it to `true`. The infrastructure `GraphqlConfig` carries the same field and its
`validate_production` method rejects `playground_enabled = true` with
`GraphqlConfigError::PlaygroundEnabledInProduction`.

`src/api/router.rs` now reads `config.graphql.playground_enabled` and registers
one of two handlers on `/graphql/playground`:

- When enabled: the existing `graphql_playground` handler serving the
  interactive IDE.
- When disabled: `graphql_playground_disabled` which returns `403 Forbidden`
  with a JSON body
  `{"error": "GraphQL Playground is disabled", "code": "FORBIDDEN"}`.

Both branches call `.with_state(schema.clone())` where needed to keep the
`Router<()>` type consistent, allowing the same `.layer(...)` chain to be
applied unconditionally after the if/else block.

#### Complexity and depth limits

Complexity and depth enforcement were already wired in
`create_schema_with_config` through `complexity_config.enforce`,
`limit_complexity`, and `limit_depth`. Phase 10 confirms these remain active and
are not bypassed by the authorization changes.

## Architecture Notes

The authorization boundary is at the resolver layer (API), not the application
or domain layer. This is intentional: the application handler `for_user` methods
act as a second defence-in-depth layer by setting `owner_id` in repository
criteria, but the resolver is the primary enforcement point.

Layer dependency rules from `AGENTS.md` are preserved:

- `schema.rs` (API layer) calls `error_codes` and `guards` (also API layer) and
  `application::handlers` (Application layer).
- `guards.rs` (API layer) calls `error_codes` (API layer) and
  `domain::value_objects` (Domain layer). No Infrastructure imports.
- `schema.rs` does not import anything from `infrastructure::`.

## Security Guarantees

After Phase 10 the following guarantees hold for every GraphQL request:

1. Any resolver that returns owner-scoped data requires a valid JWT access
   token. Requests without a token receive `UNAUTHENTICATED`.
2. Tokens whose `sub` claim is not a valid ULID are rejected with
   `VALIDATION_ERROR` before any repository is queried.
3. Owner-scoped queries (`eventReceivers`, `eventReceiverGroups`, and their
   `ById` variants) return data belonging only to the authenticated caller. A
   request for a resource owned by a different user returns an empty list, not
   an error, preventing resource enumeration.
4. Mutation operations that modify a group (`enable`, `disable`,
   `addGroupMember`, `removeGroupMember`) require the caller to own the group.
   Non-owners receive `FORBIDDEN`.
5. The GraphQL Playground IDE is disabled in production configurations. Requests
   to `/graphql/playground` return `403 Forbidden` instead of the IDE HTML.
6. All error responses include `extensions.code` set to one of the six stable
   values, enabling clients to handle errors programmatically without parsing
   human-readable message strings.

## Testing

### New tests in `src/api/graphql/schema.rs`

Six authorization tests were added to the `schema.rs` test module. Each test
builds a schema backed by no-op mock repositories and executes a query or
mutation without inserting an `AuthenticatedUser` into the request context.
Tests assert that:

- `response.errors` is non-empty.
- The first error's `extensions["code"]` equals `"UNAUTHENTICATED"`.

Tests cover:

- `test_event_receivers_by_id_requires_auth`
- `test_event_receiver_groups_by_id_requires_auth`
- `test_event_receivers_requires_auth`
- `test_event_receiver_groups_requires_auth`
- `test_set_event_receiver_group_enabled_requires_auth`
- `test_set_event_receiver_group_disabled_requires_auth`

### New test in `src/api/graphql/handlers.rs`

`test_graphql_playground_disabled_returns_403` calls
`graphql_playground_disabled` directly and asserts HTTP 403 and
`application/json` content type.

### New test in `src/api/router.rs`

`test_router_config_playground_disabled_by_default` asserts that
`RouterConfig::production()` sets `graphql.playground_enabled` to `false`.

### Test helper fix

`create_test_authenticated_user` in `handlers.rs` was updated to generate a
valid ULID for the user subject claim using `UserId::new().to_string()`. The
previous value `"test-user"` was not a valid ULID and would have caused
`parse_caller_user_id` to return a `VALIDATION_ERROR` in tests that exercise
authenticated resolvers.
