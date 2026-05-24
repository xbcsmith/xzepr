# Phase 4, Task 4.3: ULID Macro and Domain Validation Implementation

## Summary

This document explains the design and implementation of Phase 4, Task 4.3 of the
XZepr cleanup plan. The goal was to eliminate ~1,000 lines of duplicated
boilerplate across five ULID-backed identifier types and to provide a single,
tested home for shared domain validation logic.

## Problem Statement

The project had five nearly-identical files in `src/domain/value_objects/`:

- `api_key_id.rs`
- `event_id.rs`
- `event_receiver_group_id.rs`
- `event_receiver_id.rs`
- `user_id.rs`

Each file was approximately 200 lines long and contained an identical pattern: a
newtype struct wrapping `ulid::Ulid`, method implementations (`new`,
`from_ulid`, `parse`, `from_string`, `as_ulid`, `as_str`, `timestamp_ms`),
standard trait implementations (`Default`, `Display`, `From`, `FromStr`), SQLx
Postgres trait implementations (`Type`, `Decode`, `Encode`), and a full test
suite. The only variation between files was the struct name and its doc string.

Separately, domain validation logic (non-empty string checks, length limits,
semver validation, JSON object checks) was either absent or scattered, with no
central location for callers to reuse consistent rules.

## Solution Overview

The implementation introduced two new modules:

- `src/domain/value_objects/macros.rs` -- a declarative macro that generates the
  complete ULID-backed identifier implementation from a single invocation
- `src/domain/validation.rs` -- a collection of pure validation helpers that all
  domain entities and application handlers can import

## Design Decisions

### Declarative Macro vs Procedural Macro

A declarative macro (`macro_rules!`) was chosen over a procedural macro
(`proc_macro`) for the following reasons:

- No additional crate dependency required
- The expansion is straightforward and mechanical
- The pattern does not require parsing Rust syntax beyond an identifier and an
  optional string literal
- Compilation overhead is negligible

### Macro Export Strategy

The macro uses `#[macro_export]`, which places it at the crate root under the
path `crate::define_ulid_id`. This means every module within the crate can
invoke it without any use statement. The `#[macro_use]` annotation on
`pub mod macros;` in `value_objects/mod.rs` additionally brings the macro into
scope as a bare identifier within the `value_objects` subtree.

### Fully-Qualified Paths in the Macro Body

All external types referenced inside the macro expansion use `::crate_name::`
absolute paths (for example, `::ulid::Ulid`, `::serde::Serialize`,
`::sqlx::Postgres`). This prevents the macro from accidentally binding to names
that may or may not be in scope at the call site, making it safe to invoke from
any module in the crate.

### Public API Preservation

The refactored ID types expose exactly the same public surface as before:

| Method / Impl                                         | Notes                        |
| ----------------------------------------------------- | ---------------------------- |
| `new() -> Self`                                       | Generates a fresh ULID       |
| `from_ulid(ulid: Ulid) -> Self`                       | Wraps an existing ULID       |
| `parse(s: &str) -> Result<Self, DecodeError>`         | Parses from a string         |
| `from_string(s: String) -> Result<Self, DecodeError>` | Owned-string alias           |
| `as_ulid() -> Ulid`                                   | Returns the inner ULID       |
| `as_str() -> String`                                  | Returns the canonical string |
| `timestamp_ms() -> u64`                               | Millisecond Unix timestamp   |
| `Default`                                             | Calls `new()`                |
| `Display`                                             | Delegates to inner `Ulid`    |
| `From<Ulid>` / `From<Self> for Ulid`                  | Lossless conversion          |
| `FromStr`                                             | Delegates to `parse`         |
| `sqlx::Type`, `Encode`, `Decode`                      | Postgres text column support |

No callers outside `src/domain/value_objects/` required any changes.

### Validation Helper Design

The helpers in `src/domain/validation.rs` operate on primitive types (`&str`,
`usize`, `&serde_json::Value`) so that they can be used in both domain entities
and application-layer handlers without introducing layer-crossing dependencies.
They all return `Result<(), DomainError>` via the `ValidationResult` type alias,
which integrates cleanly with the `?` operator in entity constructors.

The `validate_semver` function deliberately validates only the
`MAJOR.MINOR.PATCH` numeric core. Pre-release identifiers (after `-`) and build
metadata (after `+`) are accepted without further inspection, which matches the
SemVer 2.0 relaxed interpretation used by Cargo.

## Files Changed

| File                                                  | Change                                               |
| ----------------------------------------------------- | ---------------------------------------------------- |
| `src/domain/value_objects/macros.rs`                  | Created: declarative macro + 10 tests                |
| `src/domain/value_objects/user_id.rs`                 | Replaced with macro call + 10 tests                  |
| `src/domain/value_objects/event_id.rs`                | Replaced with macro call + 8 tests                   |
| `src/domain/value_objects/api_key_id.rs`              | Replaced with macro call + 9 tests                   |
| `src/domain/value_objects/event_receiver_id.rs`       | Replaced with macro call + 8 tests                   |
| `src/domain/value_objects/event_receiver_group_id.rs` | Replaced with macro call + 8 tests                   |
| `src/domain/value_objects/mod.rs`                     | Added `#[macro_use] pub mod macros;` and doc comment |
| `src/domain/validation.rs`                            | Created: 4 validation helpers + 18 tests             |
| `src/domain/mod.rs`                                   | Added `pub mod validation;` and doc comment          |

## Line Count Impact

Approximately 1,000 lines of duplicated boilerplate were replaced by roughly 160
lines of macro definition and 5 one-line macro invocations, a reduction of about
84 percent in that area of the codebase.

## Quality Gate Results

All quality gates passed on the library target:

- `cargo fmt --all` -- no formatting changes required after edits
- `cargo check --lib --all-features` -- zero errors
- `cargo clippy --lib --all-features -- -D warnings` -- zero warnings
- `cargo test --lib --all-features domain::` -- 151 tests passed, 0 failed

The pre-existing compilation errors in `src/bin/admin.rs` are unrelated to this
task and were present before these changes.
