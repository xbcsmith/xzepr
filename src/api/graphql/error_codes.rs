// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Stable GraphQL error codes and error-building helpers.
//!
//! All production resolvers must return errors constructed through this module
//! so that API clients can rely on documented, stable extension codes.
//!
//! Each public helper function builds an [`async_graphql::Error`] whose
//! `extensions` map contains a single `"code"` key set to the corresponding
//! stable constant.  Internal error details are **always** sent to the
//! tracing layer and **never** included in the returned message.

use tracing::error;

/// Stable error code returned when a caller is not authenticated.
pub const CODE_UNAUTHENTICATED: &str = "UNAUTHENTICATED";

/// Stable error code returned when a caller lacks permission.
pub const CODE_FORBIDDEN: &str = "FORBIDDEN";

/// Stable error code returned when a requested resource does not exist.
pub const CODE_NOT_FOUND: &str = "NOT_FOUND";

/// Stable error code returned when input fails validation.
pub const CODE_VALIDATION_ERROR: &str = "VALIDATION_ERROR";

/// Stable error code returned when a resource conflict is detected.
pub const CODE_CONFLICT: &str = "CONFLICT";

/// Stable error code returned for unhandled internal errors.
pub const CODE_INTERNAL_ERROR: &str = "INTERNAL_ERROR";

/// Builds an [`async_graphql::Error`] with `extensions.code` set to `code`.
///
/// This is the single implementation point for attaching extension codes;
/// all public helpers delegate here.
fn build_error(msg: &str, code: &str) -> async_graphql::Error {
    let mut err = async_graphql::Error::new(msg);
    err.extensions = Some({
        let mut ext = async_graphql::ErrorExtensionValues::default();
        ext.set("code", code);
        ext
    });
    err
}

/// Builds an UNAUTHENTICATED error with `extensions.code = "UNAUTHENTICATED"`.
///
/// Use this when the caller presents no valid credential (missing or
/// undecodable token).
///
/// # Arguments
///
/// * `msg` - Human-readable message returned to the caller.
///
/// # Returns
///
/// An [`async_graphql::Error`] with `extensions.code = "UNAUTHENTICATED"`.
///
/// # Examples
///
/// ```
/// use xzepr::api::graphql::error_codes::unauthenticated;
///
/// let err = unauthenticated("Authentication required");
/// assert_eq!(err.message, "Authentication required");
/// ```
pub fn unauthenticated(msg: &str) -> async_graphql::Error {
    build_error(msg, CODE_UNAUTHENTICATED)
}

/// Builds a FORBIDDEN error with `extensions.code = "FORBIDDEN"`.
///
/// Use this when the caller is authenticated but lacks the required
/// permission for the requested operation.
///
/// # Arguments
///
/// * `msg` - Human-readable message returned to the caller.
///
/// # Returns
///
/// An [`async_graphql::Error`] with `extensions.code = "FORBIDDEN"`.
///
/// # Examples
///
/// ```
/// use xzepr::api::graphql::error_codes::forbidden;
///
/// let err = forbidden("Permission denied");
/// assert_eq!(err.message, "Permission denied");
/// ```
pub fn forbidden(msg: &str) -> async_graphql::Error {
    build_error(msg, CODE_FORBIDDEN)
}

/// Builds a NOT_FOUND error with `extensions.code = "NOT_FOUND"`.
///
/// # Arguments
///
/// * `resource` - Name or description of the resource that was not found.
///
/// # Returns
///
/// An [`async_graphql::Error`] with `extensions.code = "NOT_FOUND"`.
///
/// # Examples
///
/// ```
/// use xzepr::api::graphql::error_codes::not_found;
///
/// let err = not_found("Resource not found");
/// assert_eq!(err.message, "Resource not found");
/// ```
pub fn not_found(resource: &str) -> async_graphql::Error {
    build_error(resource, CODE_NOT_FOUND)
}

/// Builds a VALIDATION_ERROR error with `extensions.code = "VALIDATION_ERROR"`.
///
/// # Arguments
///
/// * `msg` - Human-readable description of the validation failure.
///
/// # Returns
///
/// An [`async_graphql::Error`] with `extensions.code = "VALIDATION_ERROR"`.
///
/// # Examples
///
/// ```
/// use xzepr::api::graphql::error_codes::validation_error;
///
/// let err = validation_error("Validation failed");
/// assert_eq!(err.message, "Validation failed");
/// ```
pub fn validation_error(msg: &str) -> async_graphql::Error {
    build_error(msg, CODE_VALIDATION_ERROR)
}

/// Builds a CONFLICT error with `extensions.code = "CONFLICT"`.
///
/// # Arguments
///
/// * `msg` - Human-readable description of the conflict.
///
/// # Returns
///
/// An [`async_graphql::Error`] with `extensions.code = "CONFLICT"`.
///
/// # Examples
///
/// ```
/// use xzepr::api::graphql::error_codes::conflict;
///
/// let err = conflict("Resource conflict");
/// assert_eq!(err.message, "Resource conflict");
/// ```
pub fn conflict(msg: &str) -> async_graphql::Error {
    build_error(msg, CODE_CONFLICT)
}

/// Builds an INTERNAL_ERROR, logging the internal detail but not exposing it.
///
/// The returned error message is always the generic string
/// `"An internal error occurred"`.  The `msg` argument is recorded at
/// `error` level via `tracing` so that operators can investigate without
/// leaking infrastructure details to callers.
///
/// # Arguments
///
/// * `msg` - Internal description, logged but NOT returned to the caller.
///
/// # Returns
///
/// An [`async_graphql::Error`] with message `"An internal error occurred"` and
/// `extensions.code = "INTERNAL_ERROR"`.
pub fn internal_error(msg: &str) -> async_graphql::Error {
    error!(internal_message = msg, "GraphQL internal error");
    build_error("An internal error occurred", CODE_INTERNAL_ERROR)
}

/// Logs `detail` at error level and returns a generic INTERNAL_ERROR.
///
/// Use this for infrastructure or repository errors so that implementation
/// details never leak into GraphQL responses.
///
/// # Arguments
///
/// * `detail` - Error detail to log (not returned to caller).
/// * `context` - Contextual label for the log entry (e.g., `"resolver"`).
///
/// # Returns
///
/// An [`async_graphql::Error`] with message `"An internal error occurred"` and
/// `extensions.code = "INTERNAL_ERROR"`.
pub fn log_and_internal_error<E: std::fmt::Display>(
    detail: E,
    context: &str,
) -> async_graphql::Error {
    error!(context = context, error = %detail, "GraphQL internal error");
    build_error("An internal error occurred", CODE_INTERNAL_ERROR)
}

/// Maps a [`crate::error::Error`] to a coded [`async_graphql::Error`].
///
/// The mapping is:
///
/// | Application error                                                | GraphQL code      |
/// |------------------------------------------------------------------|-------------------|
/// | `Error::Authorization(_)`                                        | `FORBIDDEN`       |
/// | `Error::Auth(InvalidToken | TokenExpired | MissingToken)`        | `UNAUTHENTICATED` |
/// | `Error::NotFound { .. }`                                         | `NOT_FOUND`       |
/// | `Error::Repository(EntityNotFound { .. })`                       | `NOT_FOUND`       |
/// | `Error::Domain(ReceiverNotFound | GroupNotFound)`                 | `NOT_FOUND`       |
/// | `Error::Repository(ConstraintViolation { .. } | ConcurrencyConflict)` | `CONFLICT`   |
/// | `Error::Validation(_)`                                           | `VALIDATION_ERROR`|
/// | everything else                                                  | `INTERNAL_ERROR`  |
///
/// Internal error details are logged via `tracing` and never exposed in the
/// returned message.
///
/// # Arguments
///
/// * `e` - The application error to map.
///
/// # Returns
///
/// An [`async_graphql::Error`] with an appropriate stable extension code.
pub fn map_app_error(e: crate::error::Error) -> async_graphql::Error {
    use crate::error::{AuthError, DomainError, RepositoryError};

    match e {
        crate::error::Error::Authorization(_) => forbidden("Permission denied"),

        crate::error::Error::Auth(
            AuthError::InvalidToken | AuthError::TokenExpired | AuthError::MissingToken,
        ) => unauthenticated("Authentication required"),

        crate::error::Error::NotFound { .. } => not_found("Resource not found"),

        crate::error::Error::Repository(RepositoryError::EntityNotFound { .. }) => {
            not_found("Resource not found")
        }

        crate::error::Error::Domain(DomainError::ReceiverNotFound | DomainError::GroupNotFound) => {
            not_found("Resource not found")
        }

        crate::error::Error::Repository(
            RepositoryError::ConstraintViolation { .. } | RepositoryError::ConcurrencyConflict,
        ) => conflict("Resource conflict"),

        crate::error::Error::Validation(_) => validation_error("Validation failed"),

        other => log_and_internal_error(other, "resolver"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extracts the `"code"` extension value from an error, if present.
    fn get_code(err: &async_graphql::Error) -> Option<&async_graphql::Value> {
        err.extensions.as_ref().and_then(|ext| ext.get("code"))
    }

    #[test]
    fn test_unauthenticated_has_correct_code() {
        let err = unauthenticated("not logged in");
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(
                CODE_UNAUTHENTICATED.to_string()
            ))
        );
    }

    #[test]
    fn test_forbidden_has_correct_code() {
        let err = forbidden("no permission");
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(CODE_FORBIDDEN.to_string()))
        );
    }

    #[test]
    fn test_not_found_has_correct_code() {
        let err = not_found("Event");
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(CODE_NOT_FOUND.to_string()))
        );
    }

    #[test]
    fn test_validation_error_has_correct_code() {
        let err = validation_error("bad input");
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(
                CODE_VALIDATION_ERROR.to_string()
            ))
        );
    }

    #[test]
    fn test_conflict_has_correct_code() {
        let err = conflict("duplicate key");
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(CODE_CONFLICT.to_string()))
        );
    }

    #[test]
    fn test_internal_error_has_generic_message() {
        let err = internal_error("secret db detail");
        assert_eq!(err.message, "An internal error occurred");
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(
                CODE_INTERNAL_ERROR.to_string()
            ))
        );
    }

    #[test]
    fn test_internal_error_does_not_expose_detail() {
        let detail = "sensitive database credential leak";
        let err = internal_error(detail);
        assert!(
            !err.message.contains(detail),
            "internal detail must not appear in error message: {}",
            err.message
        );
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(
                CODE_INTERNAL_ERROR.to_string()
            ))
        );
    }

    #[test]
    fn test_map_app_error_authorization_is_forbidden() {
        use crate::error::{AuthorizationError, Error};
        let err = map_app_error(Error::Authorization(AuthorizationError::PermissionDenied));
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(CODE_FORBIDDEN.to_string()))
        );
    }

    #[test]
    fn test_map_app_error_not_found_is_not_found() {
        use crate::error::Error;
        let err = map_app_error(Error::NotFound {
            resource: "Event".to_string(),
        });
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(CODE_NOT_FOUND.to_string()))
        );
    }

    #[test]
    fn test_map_app_error_constraint_violation_is_conflict() {
        use crate::error::{Error, RepositoryError};
        let err = map_app_error(Error::Repository(RepositoryError::ConstraintViolation {
            constraint: "unique_email".to_string(),
        }));
        assert_eq!(
            get_code(&err),
            Some(&async_graphql::Value::String(CODE_CONFLICT.to_string()))
        );
    }
}
