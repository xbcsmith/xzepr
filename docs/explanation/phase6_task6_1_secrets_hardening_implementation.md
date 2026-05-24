# Phase 6 Task 6.1 - Secrets and Operational Configuration Hardening

## Overview

Task 6.1 hardens secrets handling and operational configuration across the
XZepr server. The changes prevent sensitive values from leaking through log
output or debug traces, add production-readiness validation to the settings
layer, and eliminate the exposure of passwords via CLI process listings.

---

## Sub-task A: `src/infrastructure/secrets.rs`

### Motivation

Rust's `#[derive(Debug)]` will print every field value when a struct is
formatted. If configuration structs hold passwords or signing keys, a single
`tracing::debug!("{:?}", settings)` call can expose secrets in application
logs or error reports.

### Solution: `RedactedSecret<T>`

A generic wrapper type that overrides `fmt::Debug` and `fmt::Display` to
always output the literal string `[REDACTED]`, regardless of the wrapped
value.

```text
RedactedSecret<T: Clone>
  ::new(value: T) -> Self          -- wrap a sensitive value
  ::into_inner(self) -> T          -- explicit, intentional unwrap
  Deref<Target = T>                -- transparent read access
  Clone                            -- cloneable; clone also redacts
  Deserialize<'de>                 -- serde integration for config files
  Debug   -> "[REDACTED]"
  Display -> "[REDACTED]"
```

The type intentionally does not implement `Serialize` to prevent accidental
serialisation of secrets into logs or API responses.

### Memory zeroing note

Generic zeroing (`ZeroizeOnDrop`) requires `T: Zeroize`, which cannot be
asserted for an arbitrary `T: Clone`. Callers that need zeroing must call
`into_inner()` and then apply `zeroize::Zeroize` on the extracted value
directly.

---

## Sub-task B: `src/infrastructure/config.rs`

### Redacted Debug implementations

Three configuration structs that previously derived `Debug` (and therefore
printed every field verbatim) now use manual implementations:

| Struct | Sensitive field | Debug output |
|---|---|---|
| `DatabaseConfig` | password in `url` | `mask_password()` replaces password with `***` |
| `JwtAuthConfig` | `secret_key: Option<String>` | `[REDACTED]` if `Some`, `None` if `None` |
| `KeycloakConfig` | `client_secret: String` | always `[REDACTED]` |

The `mask_password` function, copied from the `mask_password` pattern already
present in `src/main.rs`, extracts the username from the URL and replaces
everything between `:` and `@` with `***`. For example:

```text
postgres://xzepr:hunter2@db.internal:5432/xzepr
         =>
postgres://xzepr:***@db.internal:5432/xzepr
```

### `OidcSessionConfig`

A new struct defining runtime session limits for OIDC-authenticated users.
Both fields default via serde so existing configuration files need no changes:

| Field | Default | Meaning |
|---|---|---|
| `session_ttl_seconds` | `3600` | Session lifetime (1 hour) |
| `max_sessions_per_user` | `10` | Maximum concurrent sessions |

### `Settings::validate_production`

A new method that audits a loaded `Settings` instance for common production
misconfigurations. It is intentionally separate from `Settings::new()` so that
test environments (which load the same struct) are not forced to pass
production checks.

| Check | Severity | Trigger |
|---|---|---|
| HTTPS disabled | Hard error | `RUST_ENV=production` and `enable_https = false` |
| Default database password | Warning (tracing) | URL contains `:password@` |
| HS256 JWT algorithm | Warning (tracing) | `algorithm == "HS256"` |
| OIDC issuer not HTTPS | Hard error | `enable_oidc = true` and issuer URL begins with `http://` |
| OIDC redirect not HTTPS | Hard error | `enable_oidc = true` and redirect URL begins with `http://` |

Hard errors return `Err(String)` with a descriptive message. Warnings are
emitted via `tracing::warn!` so they appear in the application log without
aborting startup.

---

## Sub-task C: `src/infrastructure/mod.rs`

`pub mod secrets;` was added alongside `pub use secrets::RedactedSecret;` so
that downstream code can reference `xzepr::infrastructure::RedactedSecret`
without qualifying the inner module.

---

## Sub-task D: `src/bin/admin.rs`

### Password no longer accepted as a CLI argument

The `CreateUser` subcommand previously accepted `--password` on the command
line. This has two security problems:

1. The password appears in the process listing (`ps aux`) while the command
   runs.
2. The password is recorded in the user's shell history.

The `--password` argument has been removed. The admin tool now prompts on
`stderr` and reads one line from `stdin` before proceeding:

```text
Enter password for user 'alice':
```

`eprintln!` is used for the prompt so that it goes to `stderr` and does not
pollute redirected `stdout` output.

### Emoji removal

All emoji characters (`\u2713`, `\u26a0`, `\ufe0f`) have been replaced with
plain-ASCII equivalents (`[OK]`, `IMPORTANT:`) to comply with the project
rule prohibiting emoji in code and output strings.

---

## Files changed

| File | Change type |
|---|---|
| `src/infrastructure/secrets.rs` | Created |
| `src/infrastructure/config.rs` | Edited |
| `src/infrastructure/mod.rs` | Edited |
| `src/bin/admin.rs` | Edited |

---

## Tests added

| Test | File | Verifies |
|---|---|---|
| `test_redacted_secret_debug_hides_value` | `secrets.rs` | `Debug` always outputs `[REDACTED]` |
| `test_redacted_secret_display_hides_value` | `secrets.rs` | `Display` always outputs `[REDACTED]` |
| `test_redacted_secret_deref_exposes_value` | `secrets.rs` | `Deref` returns inner value |
| `test_redacted_secret_into_inner` | `secrets.rs` | `into_inner` returns original value |
| `test_redacted_secret_clone` | `secrets.rs` | Clone preserves value and redaction |
| `test_debug_database_config_redacts_password` | `config.rs` | Password not present in `DatabaseConfig` debug output |
| `test_debug_jwt_auth_config_redacts_secret_key` | `config.rs` | Secret key shows `[REDACTED]` in `JwtAuthConfig` debug output |
| `test_debug_keycloak_config_redacts_client_secret` | `config.rs` | Client secret shows `[REDACTED]` in `KeycloakConfig` debug output |
| `test_validate_production_rejects_insecure_oidc` | `config.rs` | `validate_production` returns `Err` for http:// OIDC issuer |
| `test_oidc_session_config_defaults` | `config.rs` | `OidcSessionConfig` defaults deserialise correctly from `{}` |
