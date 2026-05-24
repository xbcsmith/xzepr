// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// src/infrastructure/messaging/mod.rs

pub mod cloudevents;
pub mod config;
pub mod producer;
pub mod topics;

pub use topics::TopicManager;

/// Re-export the domain `EventPublisher` trait for convenient access via
/// `infrastructure::messaging::EventPublisher`.
pub use crate::domain::repositories::event_publisher::EventPublisher;
