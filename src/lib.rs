// src/lib.rs

pub mod auth;
pub mod domain;
pub mod error;
pub mod infrastructure;

// Re-exports for convenience
pub use auth::api_key::ApiKeyService;
pub use auth::rbac::roles::Role;
pub use domain::entities::user::User;
pub use domain::value_objects::{ApiKeyId, UserId};
pub use error::{Error, Result};
pub use infrastructure::config::Settings;
pub use infrastructure::database::postgres::PostgresUserRepository;

// Common types
pub use chrono::{DateTime, Utc};
pub use std::sync::Arc;
pub use uuid::Uuid;
