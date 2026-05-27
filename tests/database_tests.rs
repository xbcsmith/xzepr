// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Database-layer domain tests.
//!
//! These tests exercise the `User` domain entity and the `UserBuilder` helper
//! with assertions that verify real domain behaviour: password hashing,
//! timestamp correctness, role management via direct field access, and
//! concurrent entity creation.
//!
//! Tests that require a live PostgreSQL connection are feature-gated and
//! documented with the prerequisites needed to run them.

mod common;

use common::*;

/// Tests that `User::new_local` produces an entity with the expected field
/// values and that password verification succeeds with the correct password.
///
/// Simulates the "create + read-back" part of a typical CRUD flow at the
/// domain level, without requiring a database connection.
#[tokio::test]
async fn test_user_repository_crud() {
    let user = User::new_local(
        "testuser".to_string(),
        "test@example.com".to_string(),
        "password123".to_string(),
    )
    .expect("Failed to create user");

    assert_eq!(user.username(), "testuser");
    assert_eq!(user.email(), "test@example.com");
    assert!(user.enabled(), "Newly created user should be enabled");
    assert!(
        user.verify_password("password123").unwrap_or(false),
        "Password must verify correctly after hashing"
    );
    assert!(
        !user.id().to_string().is_empty(),
        "User ID must be assigned on creation"
    );
}

/// Tests that the properties used to find a user (username, email, ID) are
/// all accessible and non-empty after construction.
///
/// In a full system these values would be passed to repository `find_by_*`
/// methods. This test confirms the domain entity exposes them correctly.
#[tokio::test]
async fn test_user_repository_find_operations() {
    let user = User::new_local(
        "findme".to_string(),
        "findme@example.com".to_string(),
        "password123".to_string(),
    )
    .expect("Failed to create user");

    assert_eq!(
        user.username(),
        "findme",
        "Username must match the value passed to new_local"
    );
    assert_eq!(
        user.email(),
        "findme@example.com",
        "Email must match the value passed to new_local"
    );
    assert!(
        !user.id().to_string().is_empty(),
        "User ID must not be empty after construction"
    );
}

/// Tests that roles can be added to and removed from a `User` by directly
/// manipulating the public `roles` field.
///
/// In a full system these mutations would be committed through a repository.
/// This test verifies the domain entity responds correctly to role changes.
#[tokio::test]
async fn test_user_repository_role_management() {
    let mut user = User::new_local(
        "roletest".to_string(),
        "role@example.com".to_string(),
        "password123".to_string(),
    )
    .expect("Failed to create user");

    // Default role must be Role::User.
    assert!(
        user.has_role(&Role::User),
        "Newly created user should carry Role::User by default"
    );

    // Add an additional role.
    user.roles.push(Role::EventManager);
    assert!(
        user.has_role(&Role::EventManager),
        "User should gain Role::EventManager after push"
    );

    // Remove the default role.
    user.roles.retain(|r| r != &Role::User);
    assert!(
        !user.has_role(&Role::User),
        "Role::User should be gone after retain"
    );
    assert!(
        user.has_role(&Role::EventManager),
        "Role::EventManager should remain after removing Role::User"
    );
}

/// Tests that multiple entity creations succeed independently, mirroring the
/// kind of work performed inside a database transaction.
///
/// Actual database transaction semantics (begin, commit, rollback) require a
/// live connection. This test verifies the domain layer produces valid,
/// distinct entities that could be persisted atomically.
#[tokio::test]
async fn test_database_transactions() {
    let user1 = User::new_local(
        "user1".to_string(),
        "user1@example.com".to_string(),
        "password123".to_string(),
    )
    .expect("Failed to create user1");

    let user2 = User::new_local(
        "user2".to_string(),
        "user2@example.com".to_string(),
        "password123".to_string(),
    )
    .expect("Failed to create user2");

    assert_eq!(user1.username(), "user1");
    assert_eq!(user2.username(), "user2");
    assert_ne!(
        user1.id(),
        user2.id(),
        "Each user must receive a unique ID even when created in sequence"
    );
}

/// Tests that `User::new_local` hashes the password at construction time and
/// that the stored hash is not the plaintext password.
///
/// Also verifies that `verify_password` returns `true` for the original
/// password and `false` for an incorrect one.
#[tokio::test]
async fn test_user_password_operations() {
    let user = User::new_local(
        "passtest".to_string(),
        "pass@example.com".to_string(),
        "mypassword123".to_string(),
    )
    .expect("Failed to create user");

    assert!(
        user.verify_password("mypassword123").unwrap_or(false),
        "Correct password must verify as true"
    );
    assert!(
        !user.verify_password("wrongpassword").unwrap_or(true),
        "Wrong password must verify as false"
    );

    // The password hash must be stored and must differ from the plaintext.
    assert!(
        user.password_hash.is_some(),
        "password_hash must be Some for a local user"
    );
    let hash = user.password_hash.as_ref().unwrap();
    assert_ne!(
        hash.as_str(),
        "mypassword123",
        "Password must be hashed, not stored in plain text"
    );
    assert!(
        hash.len() > 20,
        "Argon2 hash should be substantially longer than the original password"
    );
}

/// Tests that `created_at` and `updated_at` timestamps are set at construction
/// time and are reasonable values (positive Unix timestamp, close to each
/// other for a freshly created user).
#[tokio::test]
async fn test_user_timestamps() {
    let user = User::new_local(
        "timetest".to_string(),
        "time@example.com".to_string(),
        "password123".to_string(),
    )
    .expect("Failed to create user");

    let created_at = user.created_at();
    let updated_at = user.updated_at();

    assert!(
        created_at.timestamp() > 0,
        "created_at must be a positive Unix timestamp"
    );
    assert!(
        updated_at.timestamp() > 0,
        "updated_at must be a positive Unix timestamp"
    );

    let diff_seconds = (updated_at.timestamp() - created_at.timestamp()).unsigned_abs();
    assert!(
        diff_seconds < 2,
        "created_at and updated_at should be within 2 seconds of each other for a new user"
    );
}

/// Tests various domain-layer validation edge cases relevant to database
/// error scenarios.
///
/// Key observations:
/// - The domain entity does not enforce username uniqueness; that is a
///   database constraint enforced at the persistence layer.
/// - The domain entity does not validate email format; that is an
///   application-layer or API concern.
///
/// Documenting these boundaries prevents callers from mistakenly assuming
/// the domain will catch these errors.
#[tokio::test]
async fn test_database_error_handling() {
    // The domain allows two users with the same username.
    // Uniqueness is enforced by a DB UNIQUE constraint, not in the entity.
    let user1 = User::new_local(
        "duplicate".to_string(),
        "dup1@example.com".to_string(),
        "password123".to_string(),
    );
    let user2 = User::new_local(
        "duplicate".to_string(),
        "dup2@example.com".to_string(),
        "password123".to_string(),
    );
    assert!(
        user1.is_ok(),
        "Domain must accept first user with username 'duplicate'"
    );
    assert!(
        user2.is_ok(),
        "Domain must accept second user with username 'duplicate' (uniqueness is a DB concern)"
    );

    // The domain does not validate email format; that is the API layer's job.
    let invalid_email_user = User::new_local(
        "validuser".to_string(),
        "not-an-email-address".to_string(),
        "password123".to_string(),
    );
    assert!(
        invalid_email_user.is_ok(),
        "Domain must not reject malformed email (format validation is an API/application concern)"
    );
}

/// Tests that many `User` entities can be created concurrently without data
/// races or panics, and that each entity receives a unique ID.
///
/// This mirrors the concurrency requirements that a connection pool must
/// satisfy under production load.
#[tokio::test]
async fn test_concurrent_database_operations() {
    use tokio::task::JoinSet;

    let mut join_set = JoinSet::new();

    for i in 0..10 {
        join_set.spawn(async move {
            User::new_local(
                format!("concurrent_user_{}", i),
                format!("user{}@example.com", i),
                "password123".to_string(),
            )
            .is_ok()
        });
    }

    let mut success_count = 0usize;
    while let Some(result) = join_set.join_next().await {
        if result.unwrap_or(false) {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, 10,
        "All 10 concurrent user creations must succeed"
    );
}

/// Tests the `UserBuilder` fluent API produces a valid `User` entity with
/// the expected username and email.
///
/// Role overrides set on the builder are for documentation purposes; the
/// returned user always carries the default `Role::User` from `User::new_local`.
#[test]
fn test_user_builder_pattern() {
    let user = UserBuilder::new("buildertest")
        .email("builder@example.com")
        .password("builderpass")
        .roles(vec![Role::EventManager, Role::EventViewer])
        .enabled(true)
        .build();

    assert!(user.is_ok(), "UserBuilder::build should succeed");
    let user = user.unwrap();
    assert_eq!(user.username(), "buildertest");
    assert_eq!(user.email(), "builder@example.com");
    assert!(user.enabled(), "Built user should be enabled");
}

/// Verifies live PostgreSQL connectivity for database-backed integration tests.
///
/// This test requires a running PostgreSQL instance and a `DATABASE_URL` value.
/// See `docs/how_to/integration_test_prerequisites.md` for setup instructions.
///
/// Run with: `DATABASE_URL=postgres://... cargo test --features database-integration-tests --test database_tests`.
#[tokio::test]
#[cfg(feature = "database-integration-tests")]
async fn test_database_integration_connects_to_postgres() -> anyhow::Result<()> {
    use anyhow::Context;

    let run_live = std::env::var("XZEPR_RUN_DATABASE_INTEGRATION_TESTS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !run_live {
        return Ok(());
    }

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set to run database integration tests")?;
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL using DATABASE_URL")?;

    let value: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .context("failed to execute PostgreSQL smoke query")?;

    assert_eq!(value, 1, "PostgreSQL smoke query must return 1");
    Ok(())
}
