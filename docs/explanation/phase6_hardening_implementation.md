# Phase 6: Final Hardening and Documentation Alignment

## Overview

Phase 6 is the final phase of the XZepr codebase cleanup plan. It addresses two
categories of improvement: eliminating secret exposure vectors that survived
earlier phases, and hardening the persistence and authentication layers against
race conditions and unsafe production configurations.

A production event tracking server handles authentication tokens, database
credentials, and OAuth2 client secrets across multiple subsystems. Secrets that
appear in debug output, process argument lists, or log files represent a
persistent, low-effort attack surface. Race conditions in identity provisioning
and missing transport-layer validation create windows for exploitation that are
difficult to detect after the fact. Phase 6 closes both classes of vulnerability
through type-level guarantees, transaction boundaries, and startup-time
validation that fails fast on unsafe configuration.

---

## Secret Redaction (Task 6.1)

### The Debug Trait Leakage Problem

Rust's `#[derive(Debug)]` generates an implementation that prints every field of
a struct, including all field values, in plaintext. When configuration structs
are passed to `tracing` macros or logged with `{:?}`, all fields appear verbatim
in the log output. This is a silent hazard: derived `Debug` output is
indistinguishable from safe output during code review, but can flood log
aggregators with passwords, HMAC signing secrets, and OAuth2 client credentials
whenever the tracing level is set to `DEBUG` or `TRACE`, or when a panic
captures the application state.

### The `RedactedSecret<T>` Wrapper

`src/infrastructure/secrets.rs` introduces `RedactedSecret<T>`, a newtype
wrapper that overrides both `Debug` and `Display` to emit the string
`[REDACTED]` in place of the wrapped value. The underlying `T` is accessible
only through an `inner()` method, ensuring the value can be used where needed
(JWT signing, database connection setup) without ever surfacing through trait
output.

```rust
let secret = RedactedSecret::new("my-signing-key".to_string());
println!("{:?}", secret); // output: RedactedSecret([REDACTED])
```

The type is generic over `T: Clone + serde::Deserialize` so that configuration
deserialization continues to work without special handling. Serde bypasses the
custom `Debug` impl and populates the inner `T` directly during deserialization.

### Custom Debug Implementations on Configuration Structs

Three configuration structs in `src/infrastructure/config.rs` received
hand-written `Debug` implementations to prevent credential leakage:

- **`DatabaseConfig`**: The `url` field is printed with the password segment
  replaced. Given a connection string of the form
  `postgres://user:password@host:5432/db`, the debug output renders as
  `postgres://user:***@host:5432/db`. This preserves enough context to diagnose
  misconfiguration without exposing the credential.

- **`JwtAuthConfig`**: The `secret_key` field is printed as `[REDACTED]`. The
  `private_key_path` and `public_key_path` fields are printed as-is, because
  file paths are not themselves sensitive in most deployment environments.

- **`KeycloakConfig`**: The `client_secret` field is printed as `[REDACTED]`.
  The `issuer_url`, `client_id`, and `redirect_url` fields are printed without
  masking.

These custom implementations complement the `RedactedSecret<T>` wrapper for
cases where fields are held as `Option<String>` rather than as
`RedactedSecret<String>`, allowing incremental migration without requiring a
simultaneous rewrite of all configuration field types.

### Admin CLI Password Input

The `src/bin/admin.rs` `CreateUser` subcommand previously accepted a
`--password` argument:

```text
xzepr-admin create-user --username alice --email alice@example.com \
  --password hunter2 --role admin
```

A password supplied as a command-line argument is visible to any process that
can read `/proc/<pid>/cmdline` on Linux, appears in shell history files, and is
captured by process audit systems. The `--password` argument has been removed.
The binary now reads the password from standard input interactively, using a
prompt that suppresses terminal echo:

```text
$ xzepr-admin create-user --username alice --email alice@example.com --role admin
Password:
Confirm password:
```

This eliminates the password from the process argument list and prevents it from
entering shell history or audit logs.

### OIDC Session TTL Configuration

A new `OidcSessionConfig` struct was added to `src/infrastructure/config.rs`:

```rust
pub struct OidcSessionConfig {
    /// Session lifetime in seconds. Default: 3600 (one hour).
    pub session_ttl_seconds: u64,
    /// Maximum concurrent sessions per user. Default: 5.
    pub max_sessions_per_user: usize,
}
```

This struct is a prerequisite for the distributed session store described in the
section below. The `session_ttl_seconds` field provides the TTL value that will
be written to Redis when sessions are migrated from the current in-memory store
to a distributed store.

---

## Persistence Hardening (Task 6.2)

### TOCTOU Race in `create_or_update_oidc_user`

The original `create_or_update_oidc_user` method in `PostgresUserRepository`
followed a check-then-act pattern:

```rust
if let Some(existing_user) = self.find_by_oidc_subject(&subject).await? {
    // update path
} else {
    // insert path
}
```

This is a classic time-of-check to time-of-use (TOCTOU) race condition. Under
concurrent load, two HTTP requests carrying the same OIDC token can both reach
`find_by_oidc_subject` simultaneously, both receive `None`, and both proceed to
the insert path. The second insert will either fail with a unique constraint
violation (surfacing an error to the user) or silently succeed when no unique
constraint exists on `auth_provider_subject`, producing duplicate rows for the
same OIDC identity. Either outcome is incorrect.

The fix wraps the entire check-then-act sequence inside a `sqlx` transaction
with a `SELECT ... FOR UPDATE` row lock on the existing subject:

```sql
BEGIN;
SELECT id FROM users
WHERE auth_provider_subject = $1
FOR UPDATE;
-- insert or update based on result
COMMIT;
```

The `FOR UPDATE` clause causes the second concurrent transaction to block until
the first commits, serializing concurrent OIDC logins for the same subject while
allowing different subjects to proceed in parallel.

### Role Synchronization in `AuthUserRepository::save`

The `save` method in the `AuthUserRepository` implementation performs role
synchronization as a DELETE followed by individual INSERT statements:

```sql
DELETE FROM user_roles WHERE user_id = $1;
INSERT INTO user_roles (user_id, role) VALUES ($1, $2);
-- repeated for each role
```

Without a wrapping transaction, a failure between the DELETE and the final
INSERT leaves the user with a partially populated role set. A concurrent
authorization check during this window observes an incomplete or empty role
list, causing permission denials for a valid user. The DELETE and all subsequent
role INSERTs are now wrapped in a single `sqlx` transaction. If any INSERT
fails, the transaction is rolled back and the original role set is preserved
atomically.

### OIDC Redirect URL Allowlist

`OidcConfig` in `src/auth/oidc/config.rs` gained an
`allowed_redirect_hosts: Vec<String>` field. When the list is non-empty,
`validate()` checks that the hostname of `redirect_url` is present in the
allowlist. A redirect URL pointing to a hostname not in the list is rejected
with the new error variant `OidcConfigError::DisallowedRedirectHost`.

This prevents a misconfiguration scenario where the redirect URL is changed to a
third-party domain, which would route OAuth2 authorization codes to an
attacker-controlled server during the callback exchange. The check is enforced
in both the general `validate()` method (called at startup) and in the new
`validate_production()` method.

### Production HTTPS Enforcement

`OidcConfig::validate_production()` adds three checks that are not enforced in
development:

1. The `issuer_url` must use the `https://` scheme. An HTTP issuer in production
   means the authorization server's public keys are fetched over an unencrypted
   channel, exposing the server to key substitution attacks. The error returned
   is `OidcConfigError::HttpIssuerInProduction`.

2. The `redirect_url` must use the `https://` scheme. An HTTP redirect URL means
   the authorization code is transmitted over a cleartext connection. The error
   returned is `OidcConfigError::HttpRedirectInProduction`.

3. The `allowed_redirect_hosts` list must be non-empty. An empty allowlist
   disables the hostname check entirely, which is not safe in production. The
   error returned is `OidcConfigError::DisallowedRedirectHost`.

`Settings::validate_production()` in `src/infrastructure/config.rs` composes
these checks with the existing transport-layer validation (HTTPS must be enabled
when `RUST_ENV=production`) to provide a single entrypoint that operators can
call from startup scripts or deployment pipelines to verify configuration before
the server begins accepting traffic.

### TLS Path Canonicalization and File Permission Checks

`load_tls_config` in `src/main.rs` now calls `std::fs::canonicalize` on both the
certificate path and the private key path before passing them to
`RustlsConfig::from_pem_file`. Canonicalization resolves symlinks and `..` path
components, producing an absolute path. This prevents a scenario where a
misconfigured or tampered configuration file uses relative path components to
redirect the key path to an unintended location.

On Unix systems, the new `check_private_key_permissions` function reads the file
mode bits of the TLS private key and inspects the group-readable and
world-readable bits:

- If the key file has world-readable bits set (`o+r`), the server logs an error
  and refuses to start. A world-readable private key can be read by any process
  on the same host.
- If the key file has group-readable bits set (`g+r`), the server logs a warning
  and continues. Some deployment environments grant group read access
  intentionally for key distribution across service accounts; this is a warning
  rather than a hard error.

The recommended permission mask for a TLS private key file is `0600` (owner read
and write only).

---

## Security Regression Tests (Task 6.4)

Four categories of regression tests were added to prevent future refactoring
from re-introducing the vulnerabilities fixed in Phase 6.

### Test Categories

**Secret redaction tests** verify that `Debug` and `Display` output for
`DatabaseConfig`, `JwtAuthConfig`, `KeycloakConfig`, and `RedactedSecret<T>`
never includes a literal secret value. Each test constructs a configuration
struct with a known secret string and asserts that `format!("{:?}", config)`
does not contain that string. A developer who removes a custom `Debug`
implementation or adds a new secret field without using `RedactedSecret<T>` will
see an immediate test failure before the change reaches review.

**Redirect allowlist tests** verify that `OidcConfig::validate()` rejects
redirect URLs whose hostnames are absent from `allowed_redirect_hosts`, and
accepts URLs whose hostnames are present. These tests catch any weakening or
removal of the allowlist check before it reaches a deployed environment.

**Production HTTPS enforcement tests** verify that
`OidcConfig::validate_production()` and `Settings::validate_production()` return
the correct error variant when HTTP URLs are present in a production
configuration object. These tests ensure that relaxing the HTTPS requirement
triggers a test failure rather than silently shipping a downgrade-susceptible
configuration.

**TLS permission tests** verify that `check_private_key_permissions` produces
the expected warning or error for key files with mode `0644` and `0604`. These
tests prevent the permission-checking logic from being silently deleted during
refactoring.

**Transaction isolation tests** use concurrent async tasks against a test
database to verify that two simultaneous OIDC logins with the same subject
produce exactly one user row and no constraint violation errors. Removing the
`FOR UPDATE` lock causes the concurrent test to observe duplicate rows or an
error, making the regression immediately visible.

### Test Naming Convention

All regression tests follow the project-wide convention:

```text
test_<function>_<condition>_<expected_outcome>
```

Representative examples:

- `test_database_config_debug_does_not_contain_password`
- `test_jwt_auth_config_debug_does_not_contain_secret_key`
- `test_oidc_validate_disallowed_redirect_host_returns_error`
- `test_validate_production_http_issuer_returns_error`
- `test_check_key_permissions_world_readable_returns_error`
- `test_create_or_update_oidc_user_concurrent_same_subject_creates_one_row`

---

## Production Configuration Checklist

Operators deploying XZepr to a production environment should verify the
following before starting the server:

1. Set `RUST_ENV=production`. This activates `Settings::validate_production()`
   and `OidcConfig::validate_production()`, which enforce HTTPS and reject
   unsafe defaults that are permitted in development.

2. Configure HTTPS with a certificate and private key. Set the key file
   permissions to `0600` (owner read and write only). The server refuses to
   start when `RUST_ENV=production` and `server.enable_https` is `false`.

3. Set `auth.oidc.allowed_redirect_hosts` to the exact hostname of the OIDC
   callback URL. For a callback at
   `https://app.example.com/api/v1/auth/oidc/callback`, configure:

   ```yaml
   auth:
     oidc:
       allowed_redirect_hosts:
         - app.example.com
   ```

4. Use the RS256 JWT algorithm, not HS256. RS256 uses an asymmetric key pair;
   the public key can be distributed freely for token verification. HS256 uses a
   shared secret that must be protected on every service that verifies tokens.
   Set `auth.jwt.algorithm: RS256` and provide `auth.jwt.private_key_path` and
   `auth.jwt.public_key_path`.

5. Provide a `redis_url` in the configuration if distributed rate limiting is
   enabled. Without Redis, rate-limit state is held in memory and is not shared
   across process replicas, making per-user rate limits ineffective in a
   multi-instance deployment.

6. Do not pass passwords as command-line arguments to `xzepr-admin create-user`.
   The `--password` flag has been removed. The binary reads the password
   interactively from stdin. Avoid piping passwords from environment variables
   unless the mechanism provides adequate isolation (for example, a process
   substitution fed from a secret management system rather than from a shell
   variable).

---

## Distributed Session Store Plan

### Current Limitations

The OIDC session store is currently implemented as an in-memory data structure
within the server process. This has three consequences in production:

- Sessions are lost when the process restarts or is redeployed.
- Sessions are not shared across multiple server instances behind a load
  balancer. A user whose request is routed to a different instance after login
  is treated as unauthenticated.
- There is no infrastructure-level TTL enforcement; expiry relies on
  application-level checks against the `session_ttl_seconds` value.

These limitations are acceptable for single-instance development deployments but
are not appropriate for production use behind a load balancer.

### Session TTL Configuration

The `session_ttl_seconds` field in `OidcSessionConfig` defines the lifetime of
an OIDC session. In the current in-memory store, this value drives application-
level expiry checks. In the planned Redis store it will be written as the TTL on
the Redis key, delegating expiry enforcement to the Redis server itself and
eliminating the need for a background sweep process.

### Recommended Future Approach

The recommended migration path to a distributed session store is:

1. Add a `redis_url` field to `OidcSessionConfig` (or reuse the top-level Redis
   configuration already present for rate limiting).

2. On session creation, write the session data to Redis using a key of the form
   `oidc_session:{user_id}:{session_id}`. Set the Redis key TTL to
   `session_ttl_seconds`. Redis will automatically expire and delete the key
   when the TTL elapses.

3. On session lookup, fetch the key from Redis. A missing key means the session
   has expired or never existed; both cases return an authentication failure.

4. On logout, delete the key explicitly rather than waiting for TTL expiry.

5. Enforce `max_sessions_per_user` by maintaining a secondary Redis set keyed as
   `oidc_sessions:{user_id}`. This set holds all active session IDs for the
   user. On session creation, check the cardinality of the set. If it equals
   `max_sessions_per_user`, evict the oldest session before creating the new
   one.

6. Run a periodic background task to remove orphaned set members whose
   corresponding session keys have already expired in Redis. This handles the
   edge case where a session key's TTL elapses between the set membership check
   and the next login attempt, leaving a stale session ID in the secondary set.

---

## Architecture Impact

Phase 6 changes span three architectural layers but leave the Domain layer
untouched.

**Infrastructure layer** (`src/infrastructure/`): `secrets.rs` is a new file
providing the `RedactedSecret<T>` type. `config.rs` received custom `Debug`
implementations on `DatabaseConfig`, `JwtAuthConfig`, and `KeycloakConfig`, the
new `OidcSessionConfig` struct, and the new `Settings::validate_production()`
method. `database/postgres_user_repo.rs` gained transaction boundaries in
`create_or_update_oidc_user` and `AuthUserRepository::save`.

**Auth layer** (`src/auth/`): `oidc/config.rs` gained the
`allowed_redirect_hosts` field, the `session_ttl_seconds` field, the
`validate_production()` method, and three new error variants
(`HttpIssuerInProduction`, `HttpRedirectInProduction`,
`DisallowedRedirectHost`).

**Binary entry points** (`src/main.rs`, `src/bin/admin.rs`): `load_tls_config`
in `main.rs` gained path canonicalization and calls
`check_private_key_permissions` on Unix. The `CreateUser` subcommand in
`admin.rs` replaced the `--password` CLI argument with an interactive stdin
prompt.

No changes were made to the Domain layer (`src/domain/`). All business logic
entities and value objects remain infrastructure-agnostic, consistent with the
architectural constraint that the Domain layer must not depend on the
Infrastructure or Auth layers.

---

## Security Regression Tests Reference

The following regression test categories were established in Task 6.4. Each
category targets a specific vulnerability class to ensure that future
refactoring cannot silently revert a security fix.

| Category                           | What it prevents                                                  |
| ---------------------------------- | ----------------------------------------------------------------- |
| Secret redaction tests             | Log-based credential leakage from `Debug` trait output            |
| Redirect allowlist tests           | Open redirect vulnerabilities via unchecked callback URLs         |
| Production HTTPS enforcement tests | Protocol downgrade attacks from HTTP issuer or redirect URLs      |
| TLS permission tests               | World-readable private key files accessible to unprivileged users |
| Transaction isolation tests        | TOCTOU race conditions in concurrent OIDC identity provisioning   |

A failing test in any of these categories indicates that a security regression
has been introduced. Changes that cause these tests to fail must not be merged
until the test is restored to a passing state.
