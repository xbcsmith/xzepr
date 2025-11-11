// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/main.rs
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_telemetry() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "xzepr=debug,tower_http=debug".into())
        ))
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
