// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Architecture boundary tests.
//!
//! These tests enforce layer dependency rules by scanning source files for
//! prohibited import patterns. They complement the per-file guard tests in
//! `src/domain/entities/user.rs` with broader coverage across the domain layer.

use std::path::Path;

/// Collects all `.rs` source file paths under `base_dir` recursively.
fn collect_rs_files(base_dir: &Path) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                paths.extend(collect_rs_files(&p));
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                paths.push(p);
            }
        }
    }
    paths
}

/// Returns `true` when `line` is a real `use` import statement (not a comment
/// or string literal) that contains `needle`.
fn is_import_line(line: &str, needle: &str) -> bool {
    let trimmed = line.trim_start();
    // Only match real `use` statements; ignore comments and doc examples.
    trimmed.starts_with("use ") && !trimmed.starts_with("//") && trimmed.contains(needle)
}

/// Scans a directory for source files that contain real import lines matching
/// `prohibited_needle`. Returns a list of `(file_path, line_number, line)` triples.
fn scan_for_prohibited_imports(
    dir: &Path,
    prohibited_needle: &str,
) -> Vec<(std::path::PathBuf, usize, String)> {
    let mut violations = Vec::new();
    for path in collect_rs_files(dir) {
        if let Ok(source) = std::fs::read_to_string(&path) {
            for (i, line) in source.lines().enumerate() {
                if is_import_line(line, prohibited_needle) {
                    violations.push((path.clone(), i + 1, line.to_string()));
                }
            }
        }
    }
    violations
}

/// The domain layer must not import from the API layer.
///
/// A violation here means domain entities or repositories depend on HTTP
/// routing, REST DTOs, or GraphQL schema types, which must never happen.
#[test]
fn test_domain_files_do_not_import_api_layer() {
    let domain_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain");
    let violations = scan_for_prohibited_imports(&domain_dir, "crate::api");
    assert!(
        violations.is_empty(),
        "Domain source files must not import from the API layer.\n\
         Violations found:\n{}",
        violations
            .iter()
            .map(|(p, l, s)| format!("  {}:{}: {}", p.display(), l, s.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The domain layer must not import from the infrastructure layer.
///
/// A violation here means domain entities or repositories depend on database
/// drivers, Kafka, Redis, or other external-service types, which must never happen.
#[test]
fn test_domain_files_do_not_import_infrastructure() {
    let domain_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain");
    let violations = scan_for_prohibited_imports(&domain_dir, "crate::infrastructure");
    assert!(
        violations.is_empty(),
        "Domain source files must not import from the infrastructure layer.\n\
         Violations found:\n{}",
        violations
            .iter()
            .map(|(p, l, s)| format!("  {}:{}: {}", p.display(), l, s.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The auth layer must not define a separate user repository abstraction.
///
/// All user persistence must flow through `crate::domain::repositories::user_repo::UserRepository`.
/// A separate auth-specific repository would bypass the canonical boundary,
/// allowing the auth and domain user models to diverge silently.
#[test]
fn test_no_auth_specific_user_repository() {
    let auth_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/auth");
    let rs_files = collect_rs_files(&auth_dir);

    for path in &rs_files {
        if let Ok(source) = std::fs::read_to_string(path) {
            // Check that no file in the auth layer defines its own UserRepository trait.
            // The canonical trait is only in domain/repositories/user_repo.rs.
            let defines_user_repo_trait = source.lines().any(|line| {
                let t = line.trim_start();
                // Matches: `pub trait UserRepository`, `trait UserRepository`, etc.
                (t.starts_with("pub trait UserRepository") || t.starts_with("trait UserRepository"))
                    && !t.starts_with("//")
            });

            assert!(
                !defines_user_repo_trait,
                "Auth layer must not define its own UserRepository trait.\n\
                 Found in: {}\n\
                 All user persistence must go through \
                 `crate::domain::repositories::user_repo::UserRepository`.",
                path.display()
            );
        }
    }
}

/// The canonical user repository is in the domain layer.
///
/// Compile-time check: verifies `UserRepository` from the domain is accessible,
/// confirming the canonical boundary has not been accidentally removed.
#[test]
fn test_canonical_user_repository_is_accessible() {
    // This is a compile-time existence check. If UserRepository is removed or
    // renamed from the domain, this line fails to compile.
    let _: Option<Box<dyn xzepr::domain::repositories::user_repo::UserRepository>> = None;
}

/// The domain boundary exceptions are documented in the user entity source.
///
/// If the ADR comments (ADR-1, ADR-2) are removed from user.rs without
/// resolving the underlying issues, this test fails.
#[test]
fn test_domain_boundary_exceptions_are_documented_in_source() {
    let user_rs = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain/entities/user.rs");
    let source =
        std::fs::read_to_string(&user_rs).expect("user.rs must be readable for architecture audit");
    assert!(
        source.contains("ADR-1"),
        "user.rs must document the password-hashing boundary exception with ADR-1"
    );
    assert!(
        source.contains("ADR-2"),
        "user.rs must document the RBAC boundary exception with ADR-2"
    );
}
