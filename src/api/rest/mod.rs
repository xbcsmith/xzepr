// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/rest/mod.rs

pub mod auth;
pub mod dtos;
pub mod events;
pub mod group_membership;
pub mod routes;

pub use auth::{AuthState, LoginRequest, LoginResponse, RefreshRequest};
pub use dtos::*;
pub use events::AppState;
pub use group_membership::{
    add_group_member, list_group_members, remove_group_member, GroupMembershipState,
};
pub use routes::build_protected_router;

use axum::http::StatusCode;
use axum::response::Json;

/// Common result type for REST handlers
pub type RestResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Helper function to create a bad request error response
pub fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(
            "bad_request".to_string(),
            message.to_string(),
        )),
    )
}

/// Helper function to create a not found error response
pub fn not_found(resource: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(
            "not_found".to_string(),
            format!("{} not found", resource),
        )),
    )
}

/// Helper function to create an internal server error response
pub fn internal_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new(
            "internal_error".to_string(),
            message.to_string(),
        )),
    )
}

/// Helper function to create a validation error response
pub fn validation_error(field: &str, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::with_field(
            "validation_error".to_string(),
            message.to_string(),
            field.to_string(),
        )),
    )
}

/// Parse a ULID-backed path identifier, returning a typed REST error on failure.
///
/// This helper eliminates the repetitive pattern of parsing path IDs in every
/// REST handler and provides a consistent error response format across all
/// endpoints.
///
/// # Type Parameters
///
/// * `T` - The identifier type to parse into.  Must implement [`std::str::FromStr`].
///
/// # Arguments
///
/// * `id_str` - The raw string taken from the URL path segment.
/// * `entity` - The entity type name used in the error message (e.g., `"Event"`).
///
/// # Returns
///
/// `Ok(T)` if parsing succeeds.
///
/// # Errors
///
/// Returns a `(StatusCode::BAD_REQUEST, Json<ErrorResponse>)` tuple if the
/// supplied string cannot be parsed into `T`.
///
/// # Examples
///
/// ```rust
/// use xzepr::api::rest::parse_path_id;
/// use xzepr::domain::value_objects::EventId;
///
/// // A valid ULID string parses successfully.
/// let id = EventId::new();
/// let result: Result<EventId, _> = parse_path_id(&id.to_string(), "Event");
/// assert!(result.is_ok());
///
/// // An invalid string returns a BAD_REQUEST error.
/// let result: Result<EventId, _> = parse_path_id("not-a-ulid", "Event");
/// assert!(result.is_err());
/// ```
pub fn parse_path_id<T>(id_str: &str, entity: &str) -> Result<T, (StatusCode, Json<ErrorResponse>)>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    id_str.parse::<T>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "invalid_id".to_string(),
                format!("Invalid {} ID format", entity),
            )),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_helpers() {
        let (status, _response) = bad_request("Test message");
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let (status, _response) = not_found("User");
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, _response) = internal_error("Database error");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);

        let (status, _response) = validation_error("name", "Name is required");
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_parse_path_id_with_valid_ulid() {
        use crate::domain::value_objects::EventId;

        let id = EventId::new();
        let result: Result<EventId, _> = parse_path_id(&id.to_string(), "Event");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), id);
    }

    #[test]
    fn test_parse_path_id_with_invalid_string() {
        use crate::domain::value_objects::EventId;

        let result: Result<EventId, _> = parse_path_id("not-a-ulid", "Event");
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
