// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared repository utility helpers.
//!
//! These functions reduce boilerplate in PostgreSQL repository implementations
//! by centralising common patterns such as converting `Option` results to
//! [`RepositoryError::EntityNotFound`] and extracting column values.
//!
//! # Examples
//!
//! ```rust
//! use xzepr::infrastructure::database::repo_helpers::require_entity;
//! use xzepr::error::RepositoryError;
//!
//! let value: Option<i32> = Some(42);
//! assert_eq!(require_entity(value, "Widget").unwrap(), 42);
//!
//! let missing: Option<i32> = None;
//! assert!(matches!(
//!     require_entity(missing, "Widget"),
//!     Err(RepositoryError::EntityNotFound { .. })
//! ));
//! ```

use std::result::Result;

use sqlx::error::{DatabaseError, ErrorKind};

use crate::error::RepositoryError;

/// Convert an optional fetch result to a [`RepositoryError::EntityNotFound`] when absent.
///
/// Repository query methods often return `Option<T>` to indicate that a row
/// was not found.  This helper centralises the conversion so that every
/// repository implementation produces a consistent error variant without
/// duplicating the `ok_or_else` pattern.
///
/// # Arguments
///
/// * `result` - The optional value to check.
/// * `entity` - The entity type name used in the error message (e.g., `"User"`, `"Event"`).
///
/// # Returns
///
/// `Ok(T)` if `result` is `Some(T)`.
///
/// # Errors
///
/// Returns [`RepositoryError::EntityNotFound`] with the supplied `entity` name
/// if `result` is `None`.
///
/// # Examples
///
/// ```rust
/// use xzepr::infrastructure::database::repo_helpers::require_entity;
/// use xzepr::error::RepositoryError;
///
/// let some_value: Option<i32> = Some(42);
/// assert_eq!(require_entity(some_value, "Widget").unwrap(), 42);
///
/// let none_value: Option<i32> = None;
/// assert!(matches!(
///     require_entity(none_value, "Widget"),
///     Err(RepositoryError::EntityNotFound { .. })
/// ));
/// ```
pub fn require_entity<T>(result: Option<T>, entity: &str) -> Result<T, RepositoryError> {
    result.ok_or_else(|| RepositoryError::EntityNotFound {
        entity: entity.to_string(),
    })
}

/// Maps a [`sqlx::Error`] to the most specific application [`crate::error::Error`].
///
/// Database errors indicating unique, foreign-key, not-null, or check constraint
/// violations are mapped to [`crate::error::RepositoryError::ConstraintViolation`].
/// Serialization failures and deadlocks identified by SQLSTATE are mapped to
/// [`crate::error::RepositoryError::ConcurrencyConflict`]. All other errors fall
/// through to [`crate::error::Error::Database`].
///
/// # Arguments
///
/// * `e` - The sqlx error to classify.
///
/// # Returns
///
/// A [`crate::error::Error`] variant representing the failure.
///
/// # Examples
///
/// ```rust
/// use xzepr::infrastructure::database::repo_helpers::classify_sqlx_error;
///
/// let err = sqlx::Error::RowNotFound;
/// let app_err = classify_sqlx_error(err);
/// assert!(matches!(app_err, xzepr::error::Error::Database(_)));
/// ```
pub fn classify_sqlx_error(e: sqlx::Error) -> crate::error::Error {
    if let sqlx::Error::Database(ref db_err) = e {
        if let Some(repo_error) = classify_database_error(db_err.as_ref()) {
            return crate::error::Error::Repository(repo_error);
        }
    }
    crate::error::Error::Database(e)
}

fn classify_database_error(db_err: &dyn DatabaseError) -> Option<crate::error::RepositoryError> {
    let kind_error = match db_err.kind() {
        ErrorKind::UniqueViolation => Some(constraint_violation(db_err, "unique")),
        ErrorKind::ForeignKeyViolation => Some(constraint_violation(db_err, "foreign_key")),
        ErrorKind::NotNullViolation => Some(constraint_violation(db_err, "not_null")),
        ErrorKind::CheckViolation => Some(constraint_violation(db_err, "check")),
        _ => None,
    };

    if let Some(repo_error) = kind_error {
        return Some(repo_error);
    }

    let code = db_err.code()?;
    match code.as_ref() {
        "23505" => Some(constraint_violation(db_err, "unique")),
        "23503" => Some(constraint_violation(db_err, "foreign_key")),
        "23502" => Some(constraint_violation(db_err, "not_null")),
        "23514" => Some(constraint_violation(db_err, "check")),
        "40001" | "40P01" => Some(crate::error::RepositoryError::ConcurrencyConflict),
        _ => None,
    }
}

fn constraint_violation(
    db_err: &dyn DatabaseError,
    fallback: &str,
) -> crate::error::RepositoryError {
    crate::error::RepositoryError::ConstraintViolation {
        constraint: db_err.constraint().unwrap_or(fallback).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::borrow::Cow;
    use std::error::Error as StdError;
    use std::fmt;

    #[derive(Debug, Clone, Copy)]
    enum FakeErrorKind {
        Unique,
        ForeignKey,
        NotNull,
        Check,
        Other,
    }

    impl FakeErrorKind {
        fn to_sqlx_kind(self) -> ErrorKind {
            match self {
                Self::Unique => ErrorKind::UniqueViolation,
                Self::ForeignKey => ErrorKind::ForeignKeyViolation,
                Self::NotNull => ErrorKind::NotNullViolation,
                Self::Check => ErrorKind::CheckViolation,
                Self::Other => ErrorKind::Other,
            }
        }
    }

    #[derive(Debug)]
    struct FakeDatabaseError {
        message: &'static str,
        code: Option<&'static str>,
        constraint: Option<&'static str>,
        kind: FakeErrorKind,
    }

    impl fmt::Display for FakeDatabaseError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.message)
        }
    }

    impl StdError for FakeDatabaseError {}

    impl DatabaseError for FakeDatabaseError {
        fn message(&self) -> &str {
            self.message
        }

        fn code(&self) -> Option<Cow<'_, str>> {
            self.code.map(Cow::Borrowed)
        }

        fn as_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn StdError + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn StdError + Send + Sync + 'static> {
            self
        }

        fn constraint(&self) -> Option<&str> {
            self.constraint
        }

        fn kind(&self) -> ErrorKind {
            self.kind.to_sqlx_kind()
        }
    }

    fn fake_sqlx_database_error(
        kind: FakeErrorKind,
        code: Option<&'static str>,
        constraint: Option<&'static str>,
    ) -> sqlx::Error {
        sqlx::Error::Database(Box::new(FakeDatabaseError {
            message: "fake database error",
            code,
            constraint,
            kind,
        }))
    }

    fn assert_constraint_violation(error: crate::error::Error, expected_constraint: &str) {
        match error {
            crate::error::Error::Repository(RepositoryError::ConstraintViolation {
                constraint,
            }) => assert_eq!(constraint, expected_constraint),
            other => panic!("expected constraint violation, got {other:?}"),
        }
    }

    fn assert_concurrency_conflict(error: crate::error::Error) {
        assert!(
            matches!(
                error,
                crate::error::Error::Repository(RepositoryError::ConcurrencyConflict)
            ),
            "expected concurrency conflict"
        );
    }

    #[test]
    fn test_require_entity_with_some_returns_value() {
        let result: Result<i32, RepositoryError> = require_entity(Some(42), "Widget");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_require_entity_with_none_returns_not_found() {
        let result: Result<i32, RepositoryError> = require_entity(None, "Widget");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RepositoryError::EntityNotFound { .. }
        ));
    }

    #[test]
    fn test_require_entity_error_contains_entity_name() {
        let result: Result<String, RepositoryError> = require_entity(None, "MyEntity");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("MyEntity"));
    }

    #[test]
    fn test_classify_sqlx_error_row_not_found_passes_through() {
        let err = sqlx::Error::RowNotFound;
        let result = classify_sqlx_error(err);
        assert!(
            matches!(result, crate::error::Error::Database(_)),
            "RowNotFound must map to Error::Database"
        );
    }

    #[test]
    fn test_classify_sqlx_error_unique_violation_maps_to_constraint_violation() {
        let err = fake_sqlx_database_error(
            FakeErrorKind::Unique,
            Some("23505"),
            Some("users_email_key"),
        );
        let result = classify_sqlx_error(err);
        assert_constraint_violation(result, "users_email_key");
    }

    #[test]
    fn test_classify_sqlx_error_foreign_key_violation_maps_to_constraint_violation() {
        let err = fake_sqlx_database_error(
            FakeErrorKind::ForeignKey,
            Some("23503"),
            Some("events_receiver_id_fkey"),
        );
        let result = classify_sqlx_error(err);
        assert_constraint_violation(result, "events_receiver_id_fkey");
    }

    #[test]
    fn test_classify_sqlx_error_not_null_violation_maps_to_constraint_violation() {
        let err = fake_sqlx_database_error(FakeErrorKind::NotNull, Some("23502"), None);
        let result = classify_sqlx_error(err);
        assert_constraint_violation(result, "not_null");
    }

    #[test]
    fn test_classify_sqlx_error_check_violation_maps_to_constraint_violation() {
        let err = fake_sqlx_database_error(
            FakeErrorKind::Check,
            Some("23514"),
            Some("events_payload_object_check"),
        );
        let result = classify_sqlx_error(err);
        assert_constraint_violation(result, "events_payload_object_check");
    }

    #[test]
    fn test_classify_sqlx_error_not_null_sqlstate_maps_to_constraint_violation() {
        let err = fake_sqlx_database_error(FakeErrorKind::Other, Some("23502"), None);
        let result = classify_sqlx_error(err);
        assert_constraint_violation(result, "not_null");
    }

    #[test]
    fn test_classify_sqlx_error_check_sqlstate_maps_to_constraint_violation() {
        let err = fake_sqlx_database_error(FakeErrorKind::Other, Some("23514"), None);
        let result = classify_sqlx_error(err);
        assert_constraint_violation(result, "check");
    }

    #[test]
    fn test_classify_sqlx_error_serialization_sqlstate_maps_to_concurrency_conflict() {
        let err = fake_sqlx_database_error(FakeErrorKind::Other, Some("40001"), None);
        let result = classify_sqlx_error(err);
        assert_concurrency_conflict(result);
    }

    #[test]
    fn test_classify_sqlx_error_deadlock_sqlstate_maps_to_concurrency_conflict() {
        let err = fake_sqlx_database_error(FakeErrorKind::Other, Some("40P01"), None);
        let result = classify_sqlx_error(err);
        assert_concurrency_conflict(result);
    }

    #[test]
    fn test_classify_sqlx_error_other_database_error_passes_through() {
        let err = fake_sqlx_database_error(FakeErrorKind::Other, Some("99999"), None);
        let result = classify_sqlx_error(err);
        assert!(
            matches!(result, crate::error::Error::Database(_)),
            "unclassified database errors must remain Error::Database"
        );
    }

    #[test]
    fn test_classify_sqlx_error_return_type() {
        fn _assert_error(_e: crate::error::Error) {}
        let err = sqlx::Error::RowNotFound;
        _assert_error(classify_sqlx_error(err));
    }
}
