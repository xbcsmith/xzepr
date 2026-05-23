# How-To Documentation Cleanup Implementation

## Overview

This document summarizes the cleanup performed across the `docs/how_to/`
documentation set.

The goal of this work was to align the how-to guides with the current XZepr
application behavior, routing, Docker Compose workflow, HTTPS defaults, and
existing documentation structure. Several guides had drifted over time and still
referenced outdated endpoints, stale container names, old ports, missing
documentation pages, or older command styles.

The cleanup focused on making the how-to guides accurate, internally
consistent, and directly usable by someone following the current project setup.

## Components Delivered

- `docs/how_to/configure_kafka_authentication.md` - Updated event receiver
  creation example, corrected route usage, and fixed explanation link path.
- `docs/how_to/configure_redis_rate_limiting.md` - Updated test commands to use
  current HTTPS endpoints and corrected related documentation links.
- `docs/how_to/deployment.md` - Modernized Docker Compose commands, corrected
  container references, updated browser instructions, and fixed documentation
  links.
- `docs/how_to/enable_otlp_tracing.md` - Updated server startup command,
  corrected HTTPS request examples, and removed platform-specific browser
  commands.
- `docs/how_to/jwt_authentication_setup.md` - Updated route examples to current
  `/api/v1/*` paths and removed a broken explanation reference.
- `docs/how_to/migrate_to_kafka_authentication.md` - Updated migration
  verification event example to match the current event API contract.
- `docs/how_to/running_server.md` - Previously updated to use direct Docker
  Compose and Cargo commands and current links.
- `docs/how_to/setup_load_testing.md` - Updated load test examples to use the
  current HTTPS base URL and complete event payload requirements.
- `docs/how_to/setup_monitoring.md` - Updated health and metrics endpoint
  examples and corrected related documentation links.
- `docs/how_to/use_graphql_playground.md` - Updated to use the current server
  binary and HTTPS GraphQL Playground endpoint.
- `docs/how_to/use_rbac.md` - Updated GraphQL endpoint examples, current auth
  status wording, and HTTPS defaults.
- `docs/how_to/verify_event_publication.md` - Updated routes, ports, Redpanda
  commands, and container references to the current stack.
- `docs/explanation/how_to_documentation_cleanup_implementation.md` - This
  implementation summary.

## Implementation Details

### Problem Statement

The how-to guides contained several categories of drift from the current
application and deployment behavior:

1. Some guides still used outdated REST routes such as
   `/api/v1/event-receivers` and `/api/v1/event-receiver-groups`.
2. Some examples used incomplete request payloads that no longer matched the
   current API validation requirements.
3. Several guides still referenced old local ports such as `8042` or plain HTTP
   endpoints on `8080`.
4. Some Docker examples used stale container names such as `xzepr-server`,
   `xzepr-app`, or `xzepr-redpanda`.
5. Some guides still used older command styles such as `docker-compose` instead
   of `docker compose`.
6. Some links pointed to missing documentation pages such as
   `security_configuration.md`, `deploy_production.md`, or removed explanation
   documents.
7. Some browser instructions used platform-specific commands instead of
   portable wording.

These issues made the how-to guides harder to trust and caused avoidable
failures when readers followed the documented steps.

### Cleanup Categories

#### 1. Route and payload corrections

Guides that create or verify resources were updated to use the current API
routes:

- event receivers: `/api/v1/receivers`
- event receiver groups: `/api/v1/groups`
- events: `/api/v1/events`
- login: `/api/v1/auth/login`

Where needed, request payloads were expanded to include required fields such as:

- `type`
- `version`
- `description`
- `schema`
- `payload`
- `event_receiver_id`

This was especially important in:

- `configure_kafka_authentication.md`
- `migrate_to_kafka_authentication.md`
- `verify_event_publication.md`
- `setup_load_testing.md`
- `enable_otlp_tracing.md`

#### 2. HTTPS and port normalization

The current local workflow uses HTTPS on port `8443`, so examples were updated
to reflect that default where appropriate.

Examples were changed from older forms such as:

- `http://localhost:8042`
- `http://localhost:8080`
- `http://127.0.0.1:8443`

to current forms such as:

- `https://localhost:8443`
- `curl -k https://localhost:8443/...`

This was especially important in:

- `use_graphql_playground.md`
- `use_rbac.md`
- `setup_monitoring.md`
- `enable_otlp_tracing.md`
- `setup_load_testing.md`

#### 3. Docker and Redpanda command modernization

Operational examples were updated to match the current Compose-first workflow.

Changes included:

- `docker-compose` → `docker compose`
- stale container names → current service/container names
- `xzepr-redpanda` → `redpanda-0`
- `xzepr-server` / `xzepr-app` → `docker compose ... xzepr`

This was especially important in:

- `deployment.md`
- `verify_event_publication.md`
- `enable_otlp_tracing.md`

#### 4. Link correction

Broken or stale links were updated to point to existing documentation.

Examples include replacing missing references with current pages such as:

- `../reference/kafka_security_checklist.md`
- `../how_to/deployment.md`
- `../reference/configuration.md`
- `../explanation/event_publication_implementation.md`

This was especially important in:

- `configure_redis_rate_limiting.md`
- `configure_kafka_authentication.md`
- `setup_monitoring.md`
- `jwt_authentication_setup.md`
- `deployment.md`

#### 5. Platform-neutral browser instructions

Platform-specific commands like `open http://...` were replaced with wording
such as:

- "Open ... in your browser"

This makes the guides more portable across Linux, macOS, and other
environments.

### Why This Approach Was Chosen

This cleanup was done broadly across the how-to guides because these documents
are often used together during setup, deployment, and troubleshooting. Fixing
only one guide would still leave users moving into stale instructions in the
next guide they opened.

This approach was chosen because it:

1. improves trust in the how-to documentation set
2. reduces onboarding and troubleshooting friction
3. aligns examples with the current application behavior
4. standardizes operational guidance around the current Compose workflow
5. removes ambiguity caused by stale routes, ports, and links

## Testing

The intended validation workflow for this documentation cleanup is:

```text
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

In addition to Rust validation, representative documentation examples should be
spot-checked by following the updated commands:

- start the stack with `docker compose up -d --build`
- verify health with `curl -k https://localhost:8443/health`
- create the default admin user with the bundled admin CLI
- log in through `/api/v1/auth/login`
- create an event receiver through `/api/v1/receivers`
- create an event through `/api/v1/events`
- inspect Redpanda with `docker exec redpanda-0 rpk ...`
- access GraphQL Playground at `https://localhost:8443/graphql/playground`

Expected outcomes:

- commands in how-to guides match the current stack
- links point to existing files
- examples no longer rely on stale routes, ports, or container names
- readers can follow the guides without obvious drift-related failures

## Usage Examples

### Current event receiver creation example

```text
curl -X POST https://localhost:8443/api/v1/receivers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Receiver",
    "type": "webhook",
    "version": "1.0.0",
    "description": "Auth test",
    "schema": {}
  }' -k
```

### Current Redpanda inspection example

```text
docker exec redpanda-0 rpk topic list
docker exec redpanda-0 rpk cluster info
docker exec redpanda-0 rpk topic consume xzepr.dev.events
```

### Current GraphQL Playground access example

```text
https://localhost:8443/graphql/playground
```

### Current monitoring endpoint examples

```text
curl -k https://localhost:8443/health
curl -k https://localhost:8443/metrics
```

## Validation Results

This cleanup is intended to ensure that:

- how-to guides use current API routes
- request examples match current validation requirements
- local examples use the current HTTPS and port defaults
- Docker and Redpanda commands reflect the current Compose workflow
- stale container names are removed
- broken or missing documentation links are corrected
- browser instructions are platform-neutral where practical

## References

- `docs/how_to/configure_kafka_authentication.md`
- `docs/how_to/configure_redis_rate_limiting.md`
- `docs/how_to/deployment.md`
- `docs/how_to/enable_otlp_tracing.md`
- `docs/how_to/jwt_authentication_setup.md`
- `docs/how_to/migrate_to_kafka_authentication.md`
- `docs/how_to/running_server.md`
- `docs/how_to/setup_load_testing.md`
- `docs/how_to/setup_monitoring.md`
- `docs/how_to/use_graphql_playground.md`
- `docs/how_to/use_rbac.md`
- `docs/how_to/verify_event_publication.md`
