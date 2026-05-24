// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! XZepr event tracking server library.
//!
//! This crate provides the core XZepr functionality including domain entities,
//! application services, authentication, and infrastructure adapters.
//!
//! # Public API
//!
//! The canonical public API is re-exported at the crate root for convenience:
//!
//! - Domain entities: [`domain::entities`]
//! - Value objects: [`domain::value_objects`]
//! - Application handlers: [`application::handlers`]
//! - Configuration: [`infrastructure::config::Settings`]
//! - Error types: [`error::Error`], [`error::Result`]
//! - GraphQL schema: [`api::graphql::create_schema`]

pub mod api;
pub mod application;
pub mod auth;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod opa;

// Re-exports for convenience
pub use application::{build_group_created_event, build_receiver_created_event};
pub use auth::api_key::ApiKeyService;
pub use auth::rbac::permissions::{Permission, PermissionParseError};
pub use auth::rbac::roles::{Role, RoleParseError};
pub use domain::entities::{
    event::Event, event_receiver::EventReceiver, event_receiver_group::EventReceiverGroup,
    user::User,
};
pub use domain::repositories::event_publisher::EventPublisher;
pub use domain::validation::{
    validate_json_object, validate_max_length, validate_pagination, validate_required_string,
    validate_semver,
};
pub use domain::value_objects::{ApiKeyId, EventId, EventReceiverGroupId, EventReceiverId, UserId};
pub use error::{Error, Result};
pub use infrastructure::config::Settings;
pub use infrastructure::database::{PostgresApiKeyRepository, PostgresUserRepository};
pub use infrastructure::messaging::TopicManager;

// Application services
pub use application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};

// GraphQL
pub use api::graphql::{create_schema, Schema};
