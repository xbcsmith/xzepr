# Demo Runtime Cleanup Implementation

## Overview

Phase 2 of the codebase cleanup plan removed stale generated source, replaced
unfinished API behavior, removed source phase references and placeholder
responses, and completed runtime integrity checks that were left as comments.
This work builds on the secured canonical runtime path from Phase 1.

## Current State Analysis

### Existing Infrastructure

The project already had PostgreSQL repositories, event and group application
handlers, GraphQL schema types, REST group membership handlers, OPA resource
context builders, and JWT-authenticated request context. Phase 2 connected these
pieces instead of introducing new runtime subsystems.

### Identified Issues

The cleanup focused on these remaining issues:

- Orphan generated modules remained under `src/`.
- GraphQL event queries and event creation returned placeholder responses.
- Group membership REST and GraphQL responses fabricated usernames, emails, and
  timestamps.
- OPA resource contexts omitted group membership data and contained the last
  source phase reference.
- API key test stubs and test-only `unimplemented!()` scaffolding were visible
  in production source searches.
- Receiver and group delete flows had comments instead of integrity checks.

## Implementation Phases

### Phase 1: Orphan Source Removal

#### Task 1.1 Delete Generated Orphans

Removed unreachable generated modules and stale source fragments:

- `src/mod.rs`
- `src/api/application/`
- `src/infrastructure/telemetry/`
- `src/infrastructure/tls/`

#### Task 1.2 Clean Stale Generated Comments

Removed stale generated-file headers from active modules where they no longer
reflected the maintained source state.

#### Task 1.3 Deliverables

- Unreachable generated source removed from `src/`.
- Active modules no longer carry stale generated-source banners.

#### Task 1.4 Success Criteria

- Deleted paths no longer exist.
- `cargo check --all-targets --all-features` passes without those modules.

### Phase 2: GraphQL Event Completion

#### Task 2.1 Add Event Schema Support

Implemented `EventType` conversion from the domain `Event` and added GraphQL
`EventId` parsing.

#### Task 2.2 Wire Event Handler Into GraphQL

Updated `create_schema` and all schema construction call sites to include
`Arc<EventHandler>`.

#### Task 2.3 Implement Event Resolvers

Implemented:

- `eventsById`
- `events`
- `createEvent`

These resolvers use authenticated `UserId` values and route through application
handlers instead of bypassing ownership checks.

#### Task 2.4 Testing Requirements

Added type conversion and ID parsing coverage for GraphQL events. Updated
GraphQL test schema construction to include the event handler.

#### Task 2.5 Deliverables

- No GraphQL event placeholder fields remain.
- GraphQL event queries and mutations use real application behavior.

#### Task 2.6 Success Criteria

- GraphQL event fields compile and pass tests.
- Event reads and searches are owner scoped.

### Phase 3: Membership Response Cleanup

#### Task 3.1 Add Membership Metadata Records

Added a `GroupMembershipRecord` read model and repository methods for retrieving
persisted membership metadata.

#### Task 3.2 Enrich Member Details

Added application-level `GroupMemberDetails` enrichment using the domain
`UserRepository` so API responses use real user data.

#### Task 3.3 Replace Placeholder Responses

Updated REST and GraphQL group membership paths to return persisted `added_at`,
persisted `added_by`, and user repository `username` and `email` values.

#### Task 3.4 Testing Requirements

Updated test fakes and removed test-only placeholder scaffolding from the REST
group membership module.

#### Task 3.5 Deliverables

- Group member responses no longer fabricate user data or timestamps.
- Target users are validated before membership creation.

#### Task 3.6 Success Criteria

- No placeholder markers remain in group membership source.
- Tests pass with real member-detail mapping paths compiled.

### Phase 4: Resource Context and Integrity Checks

#### Task 4.1 Complete OPA Resource Contexts

Resource context builders now load group IDs and member IDs for receivers,
events, and groups. The stale source phase reference was removed.

#### Task 4.2 Add Delete Integrity Checks

Receiver deletion now checks for existing events and group usage when integrity
repositories are configured. Group deletion now checks whether group receivers
have events when an event repository is configured.

#### Task 4.3 Gate Demo API Key Stub

The API key stub is now test-only and no longer part of production builds.

#### Task 4.4 Testing Requirements

Existing handler tests were preserved and updated where new ownership behavior
required aligned IDs.

#### Task 4.5 Deliverables

- OPA resource contexts contain membership information.
- Delete flows no longer rely on TODO comments.
- Demo API key stub is not compiled into production code.

#### Task 4.6 Success Criteria

- Source search finds no `TODO`, `Phase`, `placeholder`, `demo purposes`, or
  `unimplemented!()` markers under `src/`.
- All quality gates pass.

## Validation Results

The required quality gates were run in order:

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features --quiet`

All commands passed. The final test run reported 624 passed tests and 6 ignored
library tests, plus all integration test groups passing. Kafka-related tests
still print localhost broker connection-refused messages, but those messages do
not indicate failures.

Markdown checks for this document also passed:

- `markdownlint --fix --config .markdownlint.json`
- `prettier --write --parser markdown --prose-wrap always`
- `markdownlint --config .markdownlint.json`

## References

- Plan: `docs/explanation/codebase_cleanup_plan.md`
- GraphQL schema: `src/api/graphql/schema.rs`
- GraphQL types: `src/api/graphql/types.rs`
- Group membership REST handlers: `src/api/rest/group_membership.rs`
- Resource context builders: `src/api/middleware/resource_context.rs`
- Group repository trait: `src/domain/repositories/event_receiver_group_repo.rs`
- PostgreSQL group repository:
  `src/infrastructure/database/postgres_event_receiver_group_repo.rs`
