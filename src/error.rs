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
}
