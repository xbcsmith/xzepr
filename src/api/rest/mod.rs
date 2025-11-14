// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/rest/mod.rs

pub mod auth;
pub mod dtos;
pub mod events;
pub mod routes;

pub use auth::{AuthState, LoginRequest, LoginResponse, RefreshRequest};
pub use dtos::*;
pub use events::AppState;
pub use routes::{build_protected_router, build_router};

/// Re-export common types for convenience
pub use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};

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
}
