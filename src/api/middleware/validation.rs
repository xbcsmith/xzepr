// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/api/middleware/validation.rs

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;
use validator::{Validate, ValidationErrors};

/// Maximum request body size in bytes (default: 1MB)
pub const DEFAULT_MAX_BODY_SIZE: usize = 1024 * 1024;

/// Maximum request body size for file uploads (default: 10MB)
pub const MAX_UPLOAD_SIZE: usize = 10 * 1024 * 1024;

/// Configuration for input validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum request body size in bytes
    pub max_body_size: usize,
    /// Maximum string length for text fields
    pub max_string_length: usize,
    /// Maximum array length for list fields
    pub max_array_length: usize,
    /// Enable strict validation mode
    pub strict_mode: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            max_string_length: 10_000,
            max_array_length: 1_000,
            strict_mode: true,
        }
    }
}

impl ValidationConfig {
    /// Creates a validation config from environment variables
    pub fn from_env() -> Self {
        let max_body_size = std::env::var("XZEPR__SECURITY__VALIDATION__MAX_BODY_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_BODY_SIZE);

        let max_string_length = std::env::var("XZEPR__SECURITY__VALIDATION__MAX_STRING_LENGTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000);

        let max_array_length = std::env::var("XZEPR__SECURITY__VALIDATION__MAX_ARRAY_LENGTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1_000);

        let strict_mode = std::env::var("XZEPR__SECURITY__VALIDATION__STRICT_MODE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        Self {
            max_body_size,
            max_string_length,
            max_array_length,
            strict_mode,
        }
    }

    /// Creates a permissive config for development
    pub fn permissive() -> Self {
        Self {
            max_body_size: MAX_UPLOAD_SIZE,
            max_string_length: 100_000,
            max_array_length: 10_000,
            strict_mode: false,
        }
    }

    /// Creates a strict config for production
    pub fn production() -> Self {
        Self {
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            max_string_length: 5_000,
            max_array_length: 500,
            strict_mode: true,
        }
    }
}

/// Validation error response
#[derive(Debug, serde::Serialize)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub field_errors: Vec<FieldError>,
}

/// Individual field error
#[derive(Debug, serde::Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
    pub code: String,
}

impl From<ValidationErrors> for ValidationErrorResponse {
    fn from(errors: ValidationErrors) -> Self {
        let field_errors = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |error| FieldError {
                    field: field.to_string(),
                    message: error
                        .message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| format!("Validation failed for field: {}", field)),
                    code: error.code.to_string(),
                })
            })
            .collect();

        Self {
            error: "Validation failed".to_string(),
            field_errors,
        }
    }
}

impl IntoResponse for ValidationErrorResponse {
    fn into_response(self) -> Response {
        let body = Body::from(serde_json::to_string(&self).unwrap_or_else(|_| {
            json!({
                "error": "Validation failed",
                "message": "Invalid request data"
            })
            .to_string()
        }));

        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap()
    }
}

/// Validates a request payload
///
/// # Arguments
///
/// * `payload` - The payload to validate (must implement Validate trait)
///
/// # Returns
///
/// Returns Ok(()) if validation passes, or a ValidationErrorResponse if it fails
///
/// # Example
///
/// ```rust
/// use validator::Validate;
/// use xzepr::api::middleware::validation::validate_request;
///
/// #[derive(Validate)]
/// struct CreateUserRequest {
///     #[validate(length(min = 3, max = 50))]
///     name: String,
///     #[validate(email)]
///     email: String,
/// }
///
/// let request = CreateUserRequest {
///     name: "John Doe".to_string(),
///     email: "john@example.com".to_string(),
/// };
///
/// validate_request(&request).expect("Valid request");
/// ```
pub fn validate_request<T: Validate>(payload: &T) -> Result<(), ValidationErrorResponse> {
    payload.validate().map_err(ValidationErrorResponse::from)
}

/// Validation state for middleware
#[derive(Clone)]
pub struct ValidationState {
    config: Arc<ValidationConfig>,
}

impl ValidationState {
    /// Creates a new validation state
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Gets the validation config
    pub fn config(&self) -> &ValidationConfig {
        &self.config
    }
}

/// Body size validation middleware
///
/// Checks that the request body size is within configured limits
pub async fn body_size_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check Content-Length header
    if let Some(content_length) = request.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                if length > DEFAULT_MAX_BODY_SIZE {
                    tracing::warn!(
                        content_length = length,
                        max_size = DEFAULT_MAX_BODY_SIZE,
                        "Request body size exceeds limit"
                    );

                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
            }
        }
    }

    Ok(next.run(request).await)
}

/// String sanitization helpers
pub mod sanitize {
    use regex::Regex;

    /// Sanitizes a string by removing control characters and trimming whitespace
    pub fn sanitize_string(input: &str) -> String {
        input
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Sanitizes HTML by escaping special characters
    pub fn sanitize_html(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
            .replace('/', "&#x2F;")
    }

    /// Validates and sanitizes a URL
    pub fn sanitize_url(input: &str) -> Result<String, String> {
        let url = input.trim();

        // Basic URL validation
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("URL must start with http:// or https://".to_string());
        }

        // Check for common injection patterns
        if url.contains("javascript:") || url.contains("data:") || url.contains("vbscript:") {
            return Err("URL contains potentially dangerous protocol".to_string());
        }

        Ok(url.to_string())
    }

    /// Validates an email address
    pub fn validate_email(email: &str) -> bool {
        let email_regex = Regex::new(
            r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
        ).unwrap();

        email_regex.is_match(email)
    }

    /// Validates that a string contains only alphanumeric characters and common separators
    pub fn validate_alphanumeric_with_separators(input: &str) -> bool {
        let regex = Regex::new(r"^[a-zA-Z0-9\s\-_\.]+$").unwrap();
        regex.is_match(input)
    }

    /// Validates a UUID format
    pub fn validate_uuid(input: &str) -> bool {
        let uuid_regex = Regex::new(
            r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
        )
        .unwrap();

        uuid_regex.is_match(input)
    }

    /// Validates a ULID format
    pub fn validate_ulid(input: &str) -> bool {
        // ULID is 26 characters, case-insensitive base32
        let ulid_regex = Regex::new(r"^[0-9A-HJKMNP-TV-Z]{26}$").unwrap();
        ulid_regex.is_match(&input.to_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[derive(Debug, Validate)]
    struct TestRequest {
        #[validate(length(min = 3, max = 50))]
        name: String,
        #[validate(email)]
        email: String,
        #[validate(range(min = 1, max = 100))]
        age: u8,
    }

    #[test]
    fn test_valid_request() {
        let request = TestRequest {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        };

        assert!(validate_request(&request).is_ok());
    }

    #[test]
    fn test_invalid_name_too_short() {
        let request = TestRequest {
            name: "Jo".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        };

        let result = validate_request(&request);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(!error.field_errors.is_empty());
        assert!(error
            .field_errors
            .iter()
            .any(|e| e.field == "name" && e.code == "length"));
    }

    #[test]
    fn test_invalid_email() {
        let request = TestRequest {
            name: "John Doe".to_string(),
            email: "invalid-email".to_string(),
            age: 30,
        };

        let result = validate_request(&request);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.field_errors.iter().any(|e| e.field == "email"));
    }

    #[test]
    fn test_invalid_age_out_of_range() {
        let request = TestRequest {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 150,
        };

        let result = validate_request(&request);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.field_errors.iter().any(|e| e.field == "age"));
    }

    #[test]
    fn test_config_default() {
        let config = ValidationConfig::default();
        assert_eq!(config.max_body_size, DEFAULT_MAX_BODY_SIZE);
        assert_eq!(config.max_string_length, 10_000);
        assert_eq!(config.max_array_length, 1_000);
        assert!(config.strict_mode);
    }

    #[test]
    fn test_config_permissive() {
        let config = ValidationConfig::permissive();
        assert_eq!(config.max_body_size, MAX_UPLOAD_SIZE);
        assert!(!config.strict_mode);
    }

    #[test]
    fn test_config_production() {
        let config = ValidationConfig::production();
        assert_eq!(config.max_body_size, DEFAULT_MAX_BODY_SIZE);
        assert!(config.strict_mode);
    }

    #[test]
    fn test_sanitize_string() {
        let input = "  Hello\x00World  ";
        let sanitized = sanitize::sanitize_string(input);
        assert_eq!(sanitized, "HelloWorld");
    }

    #[test]
    fn test_sanitize_html() {
        let input = "<script>alert('XSS')</script>";
        let sanitized = sanitize::sanitize_html(input);
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
    }

    #[test]
    fn test_sanitize_url_valid() {
        assert!(sanitize::sanitize_url("https://example.com").is_ok());
        assert!(sanitize::sanitize_url("http://localhost:3000").is_ok());
    }

    #[test]
    fn test_sanitize_url_invalid() {
        assert!(sanitize::sanitize_url("javascript:alert('XSS')").is_err());
        assert!(sanitize::sanitize_url("data:text/html,<script>alert('XSS')</script>").is_err());
        assert!(sanitize::sanitize_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(sanitize::validate_email("user@example.com"));
        assert!(sanitize::validate_email("user.name+tag@example.co.uk"));
        assert!(!sanitize::validate_email("invalid"));
        assert!(!sanitize::validate_email("@example.com"));
        assert!(!sanitize::validate_email("user@"));
    }

    #[test]
    fn test_validate_alphanumeric() {
        assert!(sanitize::validate_alphanumeric_with_separators(
            "test-name_123"
        ));
        assert!(sanitize::validate_alphanumeric_with_separators(
            "Test Name 123"
        ));
        assert!(!sanitize::validate_alphanumeric_with_separators(
            "test<script>"
        ));
        assert!(!sanitize::validate_alphanumeric_with_separators(
            "test@example"
        ));
    }

    #[test]
    fn test_validate_uuid() {
        assert!(sanitize::validate_uuid(
            "550e8400-e29b-41d4-a716-446655440000"
        ));
        assert!(!sanitize::validate_uuid("invalid-uuid"));
        assert!(!sanitize::validate_uuid("550e8400-e29b-41d4-a716"));
    }

    #[test]
    fn test_validate_ulid() {
        assert!(sanitize::validate_ulid("01ARZ3NDEKTSV4RRFFQ69G5FAV"));
        assert!(!sanitize::validate_ulid("invalid-ulid"));
        assert!(!sanitize::validate_ulid("01ARZ3NDEKTSV4RRFFQ69G5FA")); // too short
    }

    #[test]
    fn test_validation_state() {
        let config = ValidationConfig::default();
        let state = ValidationState::new(config);
        assert_eq!(state.config().max_body_size, DEFAULT_MAX_BODY_SIZE);
    }
}
