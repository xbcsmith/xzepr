// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Database infrastructure: PostgreSQL repository implementations.

pub mod postgres_api_key_repo;
pub mod postgres_event_receiver_group_repo;
pub mod postgres_event_receiver_repo;
pub mod postgres_event_repo;
pub mod postgres_user_repo;
pub mod repo_helpers;

pub use postgres_api_key_repo::PostgresApiKeyRepository;
pub use postgres_event_receiver_group_repo::PostgresEventReceiverGroupRepository;
pub use postgres_event_receiver_repo::PostgresEventReceiverRepository;
pub use postgres_event_repo::PostgresEventRepository;
pub use postgres_user_repo::PostgresUserRepository;
pub use repo_helpers::require_entity;
