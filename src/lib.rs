// src/lib.rs

pub mod api;
pub mod application;
pub mod auth;
pub mod domain;
pub mod error;
pub mod infrastructure;

// Re-exports for convenience
pub use auth::api_key::ApiKeyService;
pub use auth::rbac::roles::Role;
pub use domain::entities::{
    event::Event, event_receiver::EventReceiver, event_receiver_group::EventReceiverGroup,
    user::User,
};
pub use domain::value_objects::{ApiKeyId, EventId, EventReceiverGroupId, EventReceiverId, UserId};
pub use error::{Error, Result};
pub use infrastructure::config::Settings;
pub use infrastructure::database::postgres::{PostgresApiKeyRepository, PostgresUserRepository};
pub use infrastructure::messaging::TopicManager;

// Application services
pub use application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};

// GraphQL
pub use api::graphql::{create_schema, Schema};

// Common types
pub use chrono::{DateTime, Utc};
pub use serde_json::Value as JsonValue;
pub use std::sync::Arc;
pub use ulid::Ulid;
