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
/// Database errors indicating a unique-constraint or foreign-key violation are
/// mapped to [`crate::error::RepositoryError::ConstraintViolation`] so that
/// callers can distinguish conflict responses (HTTP 409) from generic failures.
/// All other errors fall through to [`crate::error::Error::Database`].
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
        if db_err.is_unique_violation() {
            return crate::error::Error::Repository(
                crate::error::RepositoryError::ConstraintViolation {
                    constraint: db_err.constraint().unwrap_or("unique").to_string(),
                },
            );
        }
        if db_err.is_foreign_key_violation() {
            return crate::error::Error::Repository(
                crate::error::RepositoryError::ConstraintViolation {
                    constraint: db_err.constraint().unwrap_or("foreign_key").to_string(),
                },
            );
        }
    }
    crate::error::Error::Database(e)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_classify_sqlx_error_return_type() {
        fn _assert_error(_e: crate::error::Error) {}
        let err = sqlx::Error::RowNotFound;
        _assert_error(classify_sqlx_error(err));
    }
}
