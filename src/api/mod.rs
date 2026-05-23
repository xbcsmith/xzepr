// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

pub mod graphql;
pub mod middleware;
pub mod rest;
pub mod router;

pub use router::{build_production_router, RouterConfig};
