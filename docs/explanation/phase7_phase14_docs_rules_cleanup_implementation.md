# Phase 7 and Phase 14 Docs Rules Cleanup Implementation

## Scope

This focused cleanup covered documentation, configuration examples, and the
Kafka authentication example listed for the Phase 7 and Phase 14 docs/rules
pass. No source behavior changes were intended beyond removing console status
glyphs from the scoped example.

Changed files:

- `README.md`
- `config/README.md`
- `docker-compose.prod.yaml`
- `docs/explanation/architecture.md`
- `docs/explanation/graphql_playground.md`
- `examples/kafka_with_auth.rs`

## Changes

### JWT configuration examples

Legacy top-level JWT examples were removed from the scoped docs and production
Compose file. The examples now use the current nested `auth.jwt.*` structure and
double-underscore environment variable mapping, including:

- `XZEPR__AUTH__JWT__ALGORITHM`
- `XZEPR__AUTH__JWT__ACCESS_TOKEN_EXPIRATION_SECONDS`
- `XZEPR__AUTH__JWT__REFRESH_TOKEN_EXPIRATION_SECONDS`
- `XZEPR__AUTH__JWT__PRIVATE_KEY_PATH`
- `XZEPR__AUTH__JWT__PUBLIC_KEY_PATH`

The configuration guide now describes production RS256 signing key paths and
local-only HS256 `auth.jwt.secret_key` usage instead of legacy top-level secret
and expiration settings.

### Architecture status corrections

The architecture explanation now describes REST RBAC and OIDC as wired into the
active production routing model: protected REST routes use JWT authentication
with built-in RBAC or OPA authorization, GraphQL execution is JWT-protected, and
OIDC routes are registered when an OIDC client is configured.

The old OIDC placeholder was replaced with documentation that reflects
provider-backed token validation, claim extraction, and user mapping.

### GraphQL playground corrections

The GraphQL playground explanation now reflects implemented query complexity and
depth limits through `ComplexityConfig` and `create_schema_with_config()`. It
also states that `POST /graphql` is protected by JWT authentication in the
production router, while the Playground is controlled by
`graphql.playground_enabled`.

### Rule cleanup

Status glyphs were removed from the scoped architecture examples and the Kafka
authentication example output. The replacements use plain ASCII status prefixes
such as `OK:`, `ERROR:`, and `WARNING:`.

## Final audit checklist

- [x] Scoped legacy JWT config examples removed or replaced with nested
      `auth.jwt.*` names.
- [x] Scoped environment variable examples use double-underscore mapping for
      XZepr configuration.
- [x] Scoped production Compose auth settings use `auth.jwt.*` environment
      variable names.
- [x] Scoped architecture docs describe REST RBAC and OIDC as wired.
- [x] Scoped architecture docs replaced the old OIDC validation placeholder.
- [x] Scoped GraphQL docs describe implemented query complexity limits.
- [x] Scoped files were checked for status glyphs and emoji-style symbols.

## Validation note

Markdown formatting and linting were run as targeted checks when available. The
cleanup intentionally did not run git commands.
