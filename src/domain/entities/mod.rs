// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Domain entity types.
//!
//! Each sub-module contains a single aggregate root or value-rich entity.

pub mod api_key;
pub mod event;
pub mod event_receiver;
pub mod event_receiver_group;
pub mod event_receiver_group_membership;
pub mod user;

pub use api_key::ApiKey;
