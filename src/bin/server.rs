// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Retired duplicate server binary.
//!
//! The production server entrypoint is `src/main.rs`, which delegates to the
//! canonical router in `api::router`. This binary remains only to provide a
//! clear failure mode for callers that still invoke `cargo run --bin server`.

fn main() {
    eprintln!("The `server` binary has been retired. Use the default `xzepr` binary instead.");
    std::process::exit(1);
}
