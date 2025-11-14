// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Generated from xzepr-architecture-plan.md
// Application-wide error types

use thiserror::Error;

/// Application-wide result type
pub type Result<T> = std::result::Result<T, Error>;

/// Main application error type
#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    #[error("Authorization error: {0}")]
    Authorization(#[from] AuthorizationError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] InfrastructureError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] axum::http::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Bad request: {message}")]
    BadRequest { message: String },
}

/// Authentication-related errors
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Missing token")]
    MissingToken,

    #[error("User not found")]
    UserNotFound,

    #[error("User account disabled")]
    UserDisabled,

    #[error("Password hashing failed: {0}")]
    PasswordHashingFailed(String),

    #[error("Invalid password hash: {0}")]
    InvalidPasswordHash(String),

    #[error("Token generation failed: {0}")]
    TokenGenerationFailed(String),

    #[error("Not a local user")]
    NotLocalUser,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("OIDC error: {message}")]
    OidcError { message: String },

    #[error("API key not found")]
    ApiKeyNotFound,

    #[error("API key expired")]
    ApiKeyExpired,

    #[error("API key disabled")]
    ApiKeyDisabled,
}

/// Authorization-related errors
#[derive(Error, Debug)]
pub enum AuthorizationError {
    #[error("Permission denied")]
    PermissionDenied,

    #[error("Insufficient permissions for action: {action}")]
    InsufficientPermissions { action: String },

    #[error("Role not found: {role}")]
    RoleNotFound { role: String },

    #[error("User does not have required role: {role}")]
    MissingRole { role: String },
}

/// Validation-related errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid input: {field}")]
    InvalidInput { field: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Invalid username format")]
    InvalidUsername,

    #[error("Password too weak")]
    WeakPassword,

    #[error("Invalid UUID format")]
    InvalidUuid,

    #[error("Value out of range: {field}")]
    OutOfRange { field: String },
}

/// Domain-specific errors
#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Event creation failed: {reason}")]
    EventCreationFailed { reason: String },

    #[error("Invalid event payload")]
    InvalidEventPayload,

    #[error("Receiver not found")]
    ReceiverNotFound,

    #[error("Group not found")]
    GroupNotFound,

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Invalid role assignment")]
    InvalidRoleAssignment,

    #[error("Business rule violation: {rule}")]
    BusinessRuleViolation { rule: String },

    #[error("Validation error in field '{field}': {message}")]
    ValidationError { field: String, message: String },

    #[error("Entity not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Entity already exists: {entity} with identifier {identifier}")]
    AlreadyExists { entity: String, identifier: String },

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Infrastructure-related errors
#[derive(Error, Debug)]
pub enum InfrastructureError {
    #[error("Database connection failed")]
    DatabaseConnectionFailed,

    #[error("Kafka producer error: {message}")]
    KafkaProducerError { message: String },

    #[error("Kafka consumer error: {message}")]
    KafkaConsumerError { message: String },

    #[error("TLS configuration error: {message}")]
    TlsConfigError { message: String },

    #[error("Cache error: {message}")]
    CacheError { message: String },

    #[error("External service error: {service}")]
    ExternalServiceError { service: String },
}

/// Repository-related errors
#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Entity not found: {entity}")]
    EntityNotFound { entity: String },

    #[error("Constraint violation: {constraint}")]
    ConstraintViolation { constraint: String },

    #[error("Concurrency conflict")]
    ConcurrencyConflict,

    #[error("Database operation failed: {operation}")]
    OperationFailed { operation: String },
}

impl From<RepositoryError> for Error {
    fn from(_err: RepositoryError) -> Self {
        Error::Infrastructure(InfrastructureError::DatabaseConnectionFailed)
    }
}

// HTTP status code mappings for REST API responses
impl Error {
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;

        match self {
            Error::Auth(auth_err) => match auth_err {
                AuthError::InvalidCredentials
                | AuthError::TokenExpired
                | AuthError::InvalidToken
                | AuthError::MissingToken => StatusCode::UNAUTHORIZED,
                AuthError::UserNotFound => StatusCode::NOT_FOUND,
                AuthError::UserDisabled => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Error::Authorization(_) => StatusCode::FORBIDDEN,
            Error::Validation(_) => StatusCode::BAD_REQUEST,
            Error::NotFound { .. } => StatusCode::NOT_FOUND,
            Error::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Error::Domain(domain_err) => match domain_err {
                DomainError::EventCreationFailed { .. } | DomainError::InvalidEventPayload => {
                    StatusCode::BAD_REQUEST
                }
                DomainError::ReceiverNotFound | DomainError::GroupNotFound => StatusCode::NOT_FOUND,
                DomainError::UserAlreadyExists => StatusCode::CONFLICT,
                _ => StatusCode::BAD_REQUEST,
            },
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            Error::Auth(AuthError::InvalidCredentials).status_code(),
            StatusCode::UNAUTHORIZED
        );

        assert_eq!(
            Error::Authorization(AuthorizationError::PermissionDenied).status_code(),
            StatusCode::FORBIDDEN
        );

        assert_eq!(
            Error::NotFound {
                resource: "user".to_string()
            }
            .status_code(),
            StatusCode::NOT_FOUND
        );

        assert_eq!(
            Error::Validation(ValidationError::InvalidEmail).status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_error_conversion() {
        let repo_err = RepositoryError::EntityNotFound {
            entity: "User".to_string(),
        };
        let app_err: Error = repo_err.into();

        matches!(app_err, Error::Infrastructure(_));
    }

    #[test]
    fn test_auth_error_status_codes() {
        assert_eq!(
            Error::Auth(AuthError::TokenExpired).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            Error::Auth(AuthError::InvalidToken).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            Error::Auth(AuthError::MissingToken).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            Error::Auth(AuthError::UserNotFound).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            Error::Auth(AuthError::UserDisabled).status_code(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn test_domain_error_status_codes() {
        assert_eq!(
            Error::Domain(DomainError::EventCreationFailed {
                reason: "test".to_string()
            })
            .status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            Error::Domain(DomainError::InvalidEventPayload).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            Error::Domain(DomainError::ReceiverNotFound).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            Error::Domain(DomainError::GroupNotFound).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            Error::Domain(DomainError::UserAlreadyExists).status_code(),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn test_bad_request_status_code() {
        assert_eq!(
            Error::BadRequest {
                message: "invalid input".to_string()
            }
            .status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_error_messages() {
        let error = Error::NotFound {
            resource: "user".to_string(),
        };
        assert!(error.message().contains("user"));

        let auth_error = Error::Auth(AuthError::InvalidCredentials);
        assert!(auth_error.message().contains("credentials"));
    }

    #[test]
    fn test_auth_error_display() {
        let error = AuthError::InvalidCredentials;
        assert_eq!(error.to_string(), "Invalid credentials");

        let error = AuthError::TokenExpired;
        assert_eq!(error.to_string(), "Token expired");

        let error = AuthError::UserNotFound;
        assert_eq!(error.to_string(), "User not found");
    }

    #[test]
    fn test_authorization_error_display() {
        let error = AuthorizationError::PermissionDenied;
        assert_eq!(error.to_string(), "Permission denied");

        let error = AuthorizationError::InsufficientPermissions {
            action: "delete".to_string(),
        };
        assert!(error.to_string().contains("delete"));
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::InvalidEmail;
        assert_eq!(error.to_string(), "Invalid email format");

        let error = ValidationError::MissingField {
            field: "username".to_string(),
        };
        assert!(error.to_string().contains("username"));
    }

    #[test]
    fn test_domain_error_display() {
        let error = DomainError::ReceiverNotFound;
        assert_eq!(error.to_string(), "Receiver not found");

        let error = DomainError::ValidationError {
            field: "name".to_string(),
            message: "too short".to_string(),
        };
        assert!(error.to_string().contains("name"));
        assert!(error.to_string().contains("too short"));
    }

    #[test]
    fn test_infrastructure_error_display() {
        let error = InfrastructureError::DatabaseConnectionFailed;
        assert_eq!(error.to_string(), "Database connection failed");

        let error = InfrastructureError::KafkaProducerError {
            message: "connection lost".to_string(),
        };
        assert!(error.to_string().contains("connection lost"));
    }

    #[test]
    fn test_repository_error_display() {
        let error = RepositoryError::EntityNotFound {
            entity: "Event".to_string(),
        };
        assert!(error.to_string().contains("Event"));

        let error = RepositoryError::ConcurrencyConflict;
        assert_eq!(error.to_string(), "Concurrency conflict");
    }

    #[test]
    fn test_error_from_domain_error() {
        let domain_err = DomainError::ReceiverNotFound;
        let app_err: Error = domain_err.into();
        matches!(app_err, Error::Domain(_));
    }

    #[test]
    fn test_error_from_auth_error() {
        let auth_err = AuthError::InvalidCredentials;
        let app_err: Error = auth_err.into();
        matches!(app_err, Error::Auth(_));
    }

    #[test]
    fn test_error_from_validation_error() {
        let val_err = ValidationError::InvalidEmail;
        let app_err: Error = val_err.into();
        matches!(app_err, Error::Validation(_));
    }

    #[test]
    fn test_internal_error_status_code() {
        let error = Error::Internal {
            message: "Something went wrong".to_string(),
        };
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
