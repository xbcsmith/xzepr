// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared domain validation helpers.
//!
//! These functions centralise common field-level validation rules so that
//! domain entities and application handlers can enforce them consistently
//! without duplicating the logic.
//!
//! # Examples
//!
//! ```rust
//! use xzepr::domain::validation::{validate_required_string, validate_max_length};
//!
//! assert!(validate_required_string("name", "hello").is_ok());
//! assert!(validate_required_string("name", "").is_err());
//! assert!(validate_max_length("name", "hello", 10).is_ok());
//! assert!(validate_max_length("name", "hello world!!", 10).is_err());
//! ```

use crate::error::DomainError;

/// Result type for domain validation operations.
pub type ValidationResult = Result<(), DomainError>;

/// Validate that a string field is non-empty after trimming whitespace.
///
/// # Arguments
///
/// * `field` - The field name (used in error messages)
/// * `value` - The string value to validate
///
/// # Returns
///
/// `Ok(())` if the trimmed value is non-empty.
///
/// # Errors
///
/// Returns `DomainError::ValidationError` if the trimmed value is empty.
///
/// # Examples
///
/// ```rust
/// use xzepr::domain::validation::validate_required_string;
///
/// assert!(validate_required_string("name", "hello").is_ok());
/// assert!(validate_required_string("name", "  ").is_err());
/// assert!(validate_required_string("name", "").is_err());
/// ```
pub fn validate_required_string(field: &str, value: &str) -> ValidationResult {
    if value.trim().is_empty() {
        Err(DomainError::ValidationError {
            field: field.to_string(),
            message: "must not be empty".to_string(),
        })
    } else {
        Ok(())
    }
}

/// Validate that a string field does not exceed a maximum byte length.
///
/// # Arguments
///
/// * `field` - The field name (used in error messages)
/// * `value` - The string value to validate
/// * `max` - Maximum allowed number of bytes
///
/// # Returns
///
/// `Ok(())` if `value.len() <= max`.
///
/// # Errors
///
/// Returns `DomainError::ValidationError` if the value exceeds `max` bytes.
///
/// # Examples
///
/// ```rust
/// use xzepr::domain::validation::validate_max_length;
///
/// assert!(validate_max_length("name", "hello", 10).is_ok());
/// assert!(validate_max_length("name", "this is too long", 5).is_err());
/// ```
pub fn validate_max_length(field: &str, value: &str, max: usize) -> ValidationResult {
    if value.len() > max {
        Err(DomainError::ValidationError {
            field: field.to_string(),
            message: format!("must not exceed {} characters", max),
        })
    } else {
        Ok(())
    }
}

/// Validate that a string is a valid semantic version (`MAJOR.MINOR.PATCH`
/// optionally followed by `-prerelease` or `+build`).
///
/// Only the `MAJOR.MINOR.PATCH` numeric core is validated strictly; any
/// pre-release or build metadata after `-` or `+` is accepted as-is.
///
/// # Arguments
///
/// * `field` - The field name (used in error messages)
/// * `value` - The version string to validate
///
/// # Returns
///
/// `Ok(())` if the version string is well-formed.
///
/// # Errors
///
/// Returns `DomainError::ValidationError` if the version does not match
/// `MAJOR.MINOR.PATCH` format.
///
/// # Examples
///
/// ```rust
/// use xzepr::domain::validation::validate_semver;
///
/// assert!(validate_semver("version", "1.2.3").is_ok());
/// assert!(validate_semver("version", "0.0.1-beta").is_ok());
/// assert!(validate_semver("version", "1.2").is_err());
/// assert!(validate_semver("version", "1.x.3").is_err());
/// ```
pub fn validate_semver(field: &str, value: &str) -> ValidationResult {
    // Strip optional pre-release suffix (everything after the first '-').  split('-').next()
    // always yields at least one element, so unwrap_or is a safe fallback.
    let without_prerelease = value.split('-').next().unwrap_or(value);

    // Strip optional build metadata suffix (everything after the first '+').  Same reasoning.
    let core = without_prerelease
        .split('+')
        .next()
        .unwrap_or(without_prerelease);

    let parts: Vec<&str> = core.split('.').collect();
    let valid = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));

    if valid {
        Ok(())
    } else {
        Err(DomainError::ValidationError {
            field: field.to_string(),
            message: "must be a valid semantic version (e.g., 1.2.3)".to_string(),
        })
    }
}

/// Validate that a JSON value is a JSON object (`{}`).
///
/// # Arguments
///
/// * `field` - The field name (used in error messages)
/// * `value` - The JSON value to validate
///
/// # Returns
///
/// `Ok(())` if the value is a JSON object.
///
/// # Errors
///
/// Returns `DomainError::ValidationError` if the value is not a JSON object.
///
/// # Examples
///
/// ```rust
/// use xzepr::domain::validation::validate_json_object;
/// use serde_json::json;
///
/// assert!(validate_json_object("schema", &json!({"key": "value"})).is_ok());
/// assert!(validate_json_object("schema", &json!([])).is_err());
/// assert!(validate_json_object("schema", &json!("string")).is_err());
/// ```
pub fn validate_json_object(field: &str, value: &serde_json::Value) -> ValidationResult {
    if value.is_object() {
        Ok(())
    } else {
        Err(DomainError::ValidationError {
            field: field.to_string(),
            message: "must be a JSON object".to_string(),
        })
    }
}

/// Validate pagination parameters: `limit` must be between 1 and `max_limit` (inclusive).
///
/// This helper centralises the pagination guard used by all list and paginated
/// query operations so the rule lives in exactly one place.
///
/// # Arguments
///
/// * `limit` - The requested page size.
/// * `max_limit` - The maximum allowed page size.
///
/// # Returns
///
/// `Ok(())` if `1 <= limit <= max_limit`.
///
/// # Errors
///
/// Returns `DomainError::ValidationError` if `limit == 0` or `limit > max_limit`.
///
/// # Examples
///
/// ```rust
/// use xzepr::domain::validation::validate_pagination;
///
/// assert!(validate_pagination(50, 1000).is_ok());
/// assert!(validate_pagination(1, 1000).is_ok());
/// assert!(validate_pagination(0, 1000).is_err());
/// assert!(validate_pagination(1001, 1000).is_err());
/// ```
pub fn validate_pagination(limit: usize, max_limit: usize) -> ValidationResult {
    if limit == 0 || limit > max_limit {
        Err(DomainError::ValidationError {
            field: "limit".to_string(),
            message: format!("limit must be between 1 and {}", max_limit),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_required_string_with_valid_value() {
        assert!(validate_required_string("name", "hello").is_ok());
    }

    #[test]
    fn test_validate_required_string_with_empty_string() {
        assert!(validate_required_string("name", "").is_err());
    }

    #[test]
    fn test_validate_required_string_with_whitespace_only() {
        assert!(validate_required_string("name", "   ").is_err());
    }

    #[test]
    fn test_validate_required_string_error_contains_field_name() {
        let err = validate_required_string("my_field", "").unwrap_err();
        assert!(err.to_string().contains("my_field"));
    }

    #[test]
    fn test_validate_max_length_within_limit() {
        assert!(validate_max_length("name", "hello", 10).is_ok());
    }

    #[test]
    fn test_validate_max_length_at_limit() {
        assert!(validate_max_length("name", "hello", 5).is_ok());
    }

    #[test]
    fn test_validate_max_length_exceeds_limit() {
        assert!(validate_max_length("name", "hello world", 5).is_err());
    }

    #[test]
    fn test_validate_max_length_error_contains_field_name() {
        let err = validate_max_length("my_field", "too long", 3).unwrap_err();
        assert!(err.to_string().contains("my_field"));
    }

    #[test]
    fn test_validate_semver_valid_three_part() {
        assert!(validate_semver("version", "1.2.3").is_ok());
        assert!(validate_semver("version", "0.0.0").is_ok());
        assert!(validate_semver("version", "10.20.30").is_ok());
    }

    #[test]
    fn test_validate_semver_valid_with_prerelease() {
        assert!(validate_semver("version", "1.0.0-alpha").is_ok());
        assert!(validate_semver("version", "2.0.0-rc.1").is_ok());
    }

    #[test]
    fn test_validate_semver_invalid_two_part() {
        assert!(validate_semver("version", "1.2").is_err());
    }

    #[test]
    fn test_validate_semver_invalid_non_numeric() {
        assert!(validate_semver("version", "1.x.3").is_err());
    }

    #[test]
    fn test_validate_semver_empty() {
        assert!(validate_semver("version", "").is_err());
    }

    #[test]
    fn test_validate_json_object_with_object() {
        let val = serde_json::json!({"key": "value"});
        assert!(validate_json_object("schema", &val).is_ok());
    }

    #[test]
    fn test_validate_json_object_with_empty_object() {
        let val = serde_json::json!({});
        assert!(validate_json_object("schema", &val).is_ok());
    }

    #[test]
    fn test_validate_json_object_with_array() {
        let val = serde_json::json!([1, 2, 3]);
        assert!(validate_json_object("schema", &val).is_err());
    }

    #[test]
    fn test_validate_json_object_with_string() {
        let val = serde_json::json!("hello");
        assert!(validate_json_object("schema", &val).is_err());
    }

    #[test]
    fn test_validate_json_object_with_null() {
        let val = serde_json::Value::Null;
        assert!(validate_json_object("schema", &val).is_err());
    }

    #[test]
    fn test_validate_pagination_with_valid_limit() {
        assert!(validate_pagination(50, 1000).is_ok());
    }

    #[test]
    fn test_validate_pagination_with_zero_limit() {
        assert!(validate_pagination(0, 1000).is_err());
    }

    #[test]
    fn test_validate_pagination_exceeds_max() {
        assert!(validate_pagination(1001, 1000).is_err());
    }

    #[test]
    fn test_validate_pagination_at_max_limit() {
        assert!(validate_pagination(1000, 1000).is_ok());
    }

    #[test]
    fn test_validate_pagination_error_contains_field_name() {
        let err = validate_pagination(0, 100).unwrap_err();
        assert!(err.to_string().contains("limit"));
    }
}
