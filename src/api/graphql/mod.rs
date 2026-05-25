// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

pub mod error_codes;
pub mod guards;
pub mod handlers;
pub mod schema;
pub mod types;

pub use error_codes::{
    conflict, forbidden, internal_error, log_and_internal_error, map_app_error, not_found,
    unauthenticated, validation_error, CODE_CONFLICT, CODE_FORBIDDEN, CODE_INTERNAL_ERROR,
    CODE_NOT_FOUND, CODE_UNAUTHENTICATED, CODE_VALIDATION_ERROR,
};
pub use guards::{
    helpers, parse_caller_user_id, require_auth, require_authenticated_user, require_ownership,
    require_permissions, require_roles, require_roles_and_permissions, ComplexityConfig,
    QueryComplexityAnalyzer, QueryComplexityExtension,
};
pub use handlers::{graphql_handler, graphql_health, graphql_playground};
pub use schema::{create_schema, create_schema_with_config, Mutation, Query, Schema};
pub use types::{
    parse_event_id, parse_event_receiver_group_id, parse_event_receiver_id,
    parse_event_receiver_ids, parse_user_id, CreateEventInput, CreateEventReceiverGroupInput,
    CreateEventReceiverInput, EventReceiverGroupType, EventReceiverType, EventType, FindEventInput,
    FindEventReceiverGroupInput, FindEventReceiverInput, GroupMemberType, Time, JSON,
};
