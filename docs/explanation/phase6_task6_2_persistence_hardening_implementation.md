# Phase 6 Task 6.2: Persistence and External Integration Hardening

## Overview

Task 6.2 closes three concrete races and security gaps in the XZepr server:

1. `create_or_update_oidc_user` race condition in `PostgresUserRepository`
2. `AuthUserRepository::save` role sync gap in `PostgresUserRepository`
3. TLS private key permission and path handling in `main.rs`

It also adds production-mode validation to `OidcConfig` and an explicit
redirect-host allowlist.

---

## Sub-task A: Atomic OIDC user provisioning and role sync

### Problem

#### `create_or_update_oidc_user` race

The previous implementation used a two-step find-then-create pattern:

```text
goroutine 1              goroutine 2
 find (no row)            find (no row)
                          INSERT row
 INSERT row  <-- CONFLICT or double-provision
```

Two simultaneous OIDC logins for the same subject could both observe "no
existing user" and both attempt to INSERT, causing a unique-constraint violation
or a duplicated provisioning path.

#### `save` role sync gap

The `AuthUserRepository::save` method issued three sequential statements against
the pool:

1. `INSERT ... ON CONFLICT DO UPDATE` (user row)
2. `DELETE FROM user_roles WHERE user_id = $1`
3. `INSERT INTO user_roles ...` (for each role)

Between step 2 and step 3, a concurrent read could observe the user with no
roles, which could allow privilege escalation or denial depending on how roles
are interpreted.

### Solution

Both methods now open an explicit `sqlx` transaction and perform all their work
inside it.

#### `create_or_update_oidc_user`

```text
BEGIN
  SELECT ... FROM users WHERE auth_provider_subject = $1 FOR UPDATE
  -- row lock prevents concurrent provisioning for the same subject
  UPDATE or INSERT depending on whether the row existed
COMMIT
```

The `FOR UPDATE` advisory lock on the selected row serialises concurrent OIDC
logins at the database level. If a second connection attempts the same
`SELECT ... FOR UPDATE` while the first transaction holds the lock, it will
block until the first transaction commits or rolls back.

The insert branch:

- Extracts the Keycloak subject via `AuthProvider::Keycloak { subject }` pattern
  matching (no method required; the variant exposes the field directly).
- Pre-clones `username_for_error` before the query builder chain to avoid a
  borrow conflict when the closure in `map_err` needs to reference the username
  for a `DomainError::AlreadyExists` return.

#### `AuthUserRepository::save`

All three database operations (user upsert, role DELETE, role INSERT loop) are
now wrapped in a single transaction. The `execute(&mut *tx)` pattern re-borrows
the `Transaction` mutably for each query; the borrow is released at each
`.await` point so the loop compiles correctly.

### Tests added

- `test_create_or_update_oidc_user_logic_is_isolated` - structural compilation
  test that verifies `PostgresUserRepository` satisfies the `UserRepository`
  trait bound (which includes `create_or_update_oidc_user`). No live database is
  required.

---

## Sub-task B: OidcConfig production validation and redirect allowlist

### New fields

| Field                    | Type          | Default | Purpose                                                        |
| ------------------------ | ------------- | ------- | -------------------------------------------------------------- |
| `allowed_redirect_hosts` | `Vec<String>` | `[]`    | Hostname allowlist for redirect URL. Empty disables the check. |
| `session_ttl_seconds`    | `u64`         | `3600`  | Session lifetime in seconds.                                   |

Both fields use `#[serde(default)]` so existing serialised configs that omit
them will deserialise cleanly.

### New error variants

```text
OidcConfigError::HttpIssuerInProduction
OidcConfigError::HttpRedirectInProduction
OidcConfigError::DisallowedRedirectHost { host: String }
```

### `validate()` update

`validate()` now checks the redirect-host allowlist when
`allowed_redirect_hosts` is non-empty. This means the allowlist is enforced in
all environments, not only production:

```text
if !self.allowed_redirect_hosts.is_empty() {
    host = extract_host(redirect_url)
    if host not in allowed_redirect_hosts -> Err(DisallowedRedirectHost)
}
```

### `validate_production()`

Calls `validate()` first (which handles all base checks including the
allowlist), then additionally enforces HTTPS on both issuer URL and redirect
URL. This makes it safe to call `validate_production()` from startup code when
`RUST_ENV=production`.

### `extract_host()` helper

Strips the scheme prefix by finding `://` and then takes the substring up to the
first `/`. Returns the host-and-port string (`host` or `host:port`).

### Tests added

- `test_validate_production_rejects_http_issuer`
- `test_validate_production_rejects_http_redirect`
- `test_validate_production_accepts_https_config`
- `test_validate_redirect_host_allowlist`
- `test_oidc_session_ttl_default`

---

## Sub-task C: TLS path canonicalization and key permission check

### Problem

The previous `load_tls_config` passed user-supplied path strings directly to
`RustlsConfig::from_pem_file`. This had two issues:

1. Relative paths or symlinks could resolve to unintended files depending on the
   working directory.
2. A world-readable private key would be loaded silently even in production.

### Solution

`load_tls_config` now:

1. Calls `std::fs::canonicalize` on both paths, converting them to absolute
   resolved paths and returning a descriptive error if either path does not
   exist.
2. On Unix, calls `check_private_key_permissions` before loading the key.

`check_private_key_permissions` reads the file mode via
`std::os::unix::fs::PermissionsExt` and inspects the group-readable (`0o040`)
and world-readable (`0o004`) bits:

- In `RUST_ENV=production` - returns `Err` and halts startup.
- In any other environment - emits a `warn!` log and continues, allowing
  development with less-restricted key files.

The function is gated with `#[cfg(unix)]` so it compiles to nothing on non-Unix
targets (Windows).

---

## Layer boundaries respected

All changes stay within the infrastructure layer (`src/infrastructure/`) for
database concerns, within the auth layer (`src/auth/`) for OIDC config, and
within the binary entry point (`src/main.rs`) for TLS. No domain layer code was
modified.
