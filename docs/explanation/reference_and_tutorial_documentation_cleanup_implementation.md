# Reference and Tutorial Documentation Cleanup Implementation

## Overview

This document summarizes the cleanup performed across the reference and tutorial
documentation in XZepr.

The goal of this work was to align the reference and tutorial materials with the
current application behavior, current Docker Compose workflow, current HTTPS
defaults, current API routes, and the documentation structure that actually
exists in the repository.

Several documents had drifted over time and still referenced outdated routes,
stale ports, old container names, missing documentation pages, or older Docker
workflows. This cleanup focused on making the reference and tutorial documents
accurate, internally consistent, and directly usable.

## Components Delivered

- `docs/reference/api.md` - Updated event receiver endpoint examples and payloads
  to match the current API contract.
- `docs/reference/database_schema.md` - Fixed broken related-documentation links.
- `docs/reference/docker_commands.md` - Cleaned up stale image, port, and
  container examples and aligned them with the current Compose workflow.
- `docs/reference/jwt_api.md` - Updated stale route examples to current `/api/v1`
  paths.
- `docs/reference/observability_quick_reference.md` - Updated stale metric path
  examples to current `/api/v1/events` paths.
- `docs/reference/README.md` - Replaced missing reference entries with links to
  existing reference documents and corrected explanation links.
- `docs/tutorials/README.md` - Removed a missing tutorial entry and corrected the
  explanation link target.
- `docs/tutorials/curl_examples.md` - Updated examples to use HTTPS and fixed a
  malformed group query example.
- `docs/tutorials/docker_demo.md` - Modernized the tutorial to use the current
  Compose workflow, current HTTPS endpoints, current admin CLI usage, and current
  troubleshooting commands.
- `docs/explanation/reference_and_tutorial_documentation_cleanup_implementation.md`
  - This implementation summary.

## Implementation Details

### Problem Statement

The reference and tutorial documentation contained several categories of drift
from the current application and deployment behavior:

1. Some reference examples still used outdated REST routes such as
   `/api/v1/event-receivers`.
2. Some examples used stale local ports such as `8042` or plain HTTP endpoints
   that no longer matched the default local workflow.
3. Some Docker examples still referenced old image names such as `xzepr:demo`
   and old container names such as `xzepr-server`.
4. Some index pages referenced documentation files that no longer existed, such
   as `implementation.md`, `environment_variables.md`, `cli.md`, and
   `first_event_tracker.md`.
5. Some examples used older command styles such as `docker-compose` instead of
   `docker compose`.
6. Some tutorial examples still assumed older standalone container workflows
   instead of the current Compose-first workflow.
7. Some links pointed to missing explanation pages or stale paths.

These issues made the documentation harder to trust and caused avoidable
failures when readers followed the documented steps.

### Cleanup Categories

#### 1. API route and payload corrections

Reference examples were updated to use the current event receiver route:

- `/api/v1/receivers`

instead of the stale:

- `/api/v1/event-receivers`

Where needed, payloads were updated to match the current API contract by using
fields such as:

- `name`
- `type`
- `version`
- `description`
- `schema`

This was especially important in:

- `docs/reference/api.md`

#### 2. HTTPS and port normalization

Tutorial and reference examples were updated to reflect the current local
default of HTTPS on port `8443`.

Examples were changed away from older forms such as:

- `http://localhost:8042`
- `http://localhost:8080`

and aligned with current forms such as:

- `https://localhost:8443`
- `curl -k https://localhost:8443/...`

This was especially important in:

- `docs/tutorials/curl_examples.md`
- `docs/tutorials/docker_demo.md`

#### 3. Docker and container command modernization

Docker examples were updated to reflect the current Compose-first workflow.

Changes included:

- `docker-compose` → `docker compose`
- `xzepr:demo` → current Compose-managed image usage
- `xzepr-server` → `docker compose ... xzepr`
- migration and admin CLI examples updated to use `docker compose run --rm -T`

This was especially important in:

- `docs/reference/docker_commands.md`
- `docs/tutorials/docker_demo.md`

#### 4. Documentation index cleanup

Reference and tutorial index pages were updated to point only to files that
actually exist in the repository.

Examples of removed or replaced stale entries include:

- `implementation.md`
- `environment_variables.md`
- `cli.md`
- `first_event_tracker.md`

This was especially important in:

- `docs/reference/README.md`
- `docs/tutorials/README.md`

#### 5. Link correction

Broken or stale links were updated to point to existing documentation.

Examples include:

- explanation links corrected to `../explanation/...`
- missing explanation README references replaced with a concrete explanation page
- broken related links in schema and reference docs corrected to existing files

This was especially important in:

- `docs/reference/README.md`
- `docs/reference/database_schema.md`
- `docs/tutorials/README.md`

#### 6. Tutorial workflow consistency

The Docker demo tutorial was updated to match the current project workflow:

- start services with `docker compose up -d --build`
- use Compose-managed services instead of ad hoc standalone containers
- use current HTTPS endpoints
- use current admin CLI invocation style
- use current troubleshooting commands

This was especially important in:

- `docs/tutorials/docker_demo.md`

### Why This Approach Was Chosen

This cleanup was intentionally broad because reference and tutorial documents are
often the first materials users consult. If those documents are stale, users can
fail before they ever reach the more detailed how-to or explanation guides.

This approach was chosen because it:

1. improves trust in the documentation set
2. reduces onboarding friction for new users
3. aligns examples with the current application behavior
4. standardizes operational guidance around the current Compose workflow
5. removes ambiguity caused by stale routes, ports, links, and container names

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
- create an event receiver through `/api/v1/receivers`
- create an event through `/api/v1/events`
- access GraphQL Playground at `https://localhost:8443/graphql/playground`
- inspect logs and service status using `docker compose logs` and
  `docker compose ps`

Expected outcomes:

- commands in reference and tutorial docs match the current stack
- links point to existing files
- examples no longer rely on stale routes, ports, or container names
- readers can follow the docs without obvious drift-related failures

## Usage Examples

### Current event receiver creation example

```text
curl -X POST https://localhost:8443/api/v1/receivers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production CI/CD Pipeline",
    "type": "webhook",
    "version": "1.0.0",
    "description": "Receives events from production deployments",
    "schema": {}
  }'
```

### Current Docker admin CLI example

```text
docker compose run --rm -T \
  -e XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr \
  --entrypoint ./admin \
  xzepr \
  create-user \
  --username admin \
  --email admin@xzepr.local \
  --password admin123 \
  --role admin
```

### Current GraphQL Playground access example

```text
https://localhost:8443/graphql/playground
```

### Current health check example

```text
curl -k https://localhost:8443/health
```

## Validation Results

This cleanup is intended to ensure that:

- reference docs use current API routes
- tutorial flows use the current Compose-first workflow
- local examples use the current HTTPS and port defaults
- stale image names and container names are removed
- broken or missing documentation links are corrected
- reference and tutorial indexes point to existing files
- readers can follow the docs without obvious drift-related failures

## References

- `docs/reference/README.md`
- `docs/reference/api.md`
- `docs/reference/database_schema.md`
- `docs/reference/docker_commands.md`
- `docs/reference/jwt_api.md`
- `docs/reference/observability_quick_reference.md`
- `docs/tutorials/README.md`
- `docs/tutorials/curl_examples.md`
- `docs/tutorials/docker_demo.md`
- `docs/tutorials/getting_started.md`
