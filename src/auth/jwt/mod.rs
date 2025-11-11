// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! JWT Authentication Module
//!
//! This module provides comprehensive JWT (JSON Web Token) authentication
//! functionality including token generation, validation, refresh, and revocation.
//!
//! # Features
//!
//! - RS256 (RSA) and HS256 (HMAC) signing algorithms
//! - Access and refresh token generation
//! - Token validation with expiration and signature verification
//! - Token blacklist for revocation
//! - Key rotation support
//! - Configurable token lifetimes
//!
//! # Example
//!
//! ```rust,ignore
//! use xzepr::auth::jwt::{JwtConfig, JwtService};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create configuration
//! let config = JwtConfig::production_template();
//!
//! // Create JWT service
//! let service = JwtService::from_config(config)?;
//!
//! // Generate token pair
//! let token_pair = service.generate_token_pair(
//!     "user123".to_string(),
//!     vec!["admin".to_string()],
//!     vec!["read".to_string(), "write".to_string()],
//! )?;
//!
//! // Validate token
//! let claims = service.validate_token(&token_pair.access_token).await?;
//! println!("User ID: {}", claims.sub);
//! # Ok(())
//! # }
//! ```

pub mod blacklist;
pub mod claims;
pub mod config;
pub mod error;
pub mod keys;
pub mod service;

pub use blacklist::{Blacklist, TokenBlacklist};
pub use claims::{Claims, TokenType};
pub use config::{Algorithm, JwtConfig};
pub use error::{JwtError, JwtResult};
pub use keys::{KeyManager, KeyPair};
pub use service::{JwtService, TokenPair};
