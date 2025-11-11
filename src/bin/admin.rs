// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/bin/admin.rs
use chrono::Utc;
use clap::{Parser, Subcommand};
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use xzepr::auth::api_key::UserRepository;
use xzepr::{
    ApiKeyId, ApiKeyService, PostgresApiKeyRepository, PostgresUserRepository, Role, Settings, User,
};

#[derive(Parser)]
#[command(name = "xzepr-admin")]
#[command(about = "XZEPR administration tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new local user
    CreateUser {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        email: String,
        #[arg(short, long)]
        password: String,
        #[arg(short, long)]
        role: String,
    },
    /// List all users
    ListUsers,
    /// Add role to user
    AddRole {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        role: String,
    },
    /// Remove role from user
    RemoveRole {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        role: String,
    },
    /// Generate API key for user
    GenerateApiKey {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        expires_days: Option<i64>,
    },
    /// List user's API keys
    ListApiKeys {
        #[arg(short, long)]
        username: String,
    },
    /// Revoke API key
    RevokeApiKey {
        #[arg(short, long)]
        key_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load configuration
    let settings = Settings::new()?;

    // Connect to database
    let pool = PgPool::connect(&settings.database.url).await?;

    // Initialize repositories and services
    let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
    let api_key_repo = Arc::new(PostgresApiKeyRepository::new(pool.clone()));
    let api_key_service = Arc::new(ApiKeyService::new(user_repo.clone(), api_key_repo));

    match cli.command {
        Commands::CreateUser {
            username,
            email,
            password,
            role,
        } => {
            let user = User::new_local(username, email, password)?;

            // Parse and add the requested role
            let role = Role::from_str(&role)?;

            // If the requested role is different from the default User role, add it
            if role != Role::User {
                // Get mutable reference to roles and add the new role
                // Note: User struct doesn't expose roles mutably, so we need to use add_role after save
                user_repo.save(&user).await?;
                user_repo.add_role(user.id(), role).await?;
            } else {
                // Just save with default role
                user_repo.save(&user).await?;
            }

            println!("✓ User created successfully!");
            println!("  ID: {}", user.id());
            println!("  Username: {}", user.username());
            println!(
                "  Roles: user{}",
                if role != Role::User {
                    format!(", {}", role)
                } else {
                    String::new()
                }
            );
        }

        Commands::ListUsers => {
            let users = user_repo.find_all().await?;
            println!(
                "\n{:<36} {:<20} {:<30} {:<15}",
                "ID", "Username", "Email", "Roles"
            );
            println!("{}", "-".repeat(100));

            for user in users {
                let roles: Vec<String> = user.roles().iter().map(|r| r.to_string()).collect();
                println!(
                    "{:<36} {:<20} {:<30} {:<15}",
                    user.id(),
                    user.username(),
                    user.email(),
                    roles.join(", ")
                );
            }
        }

        Commands::AddRole { username, role } => {
            let user = user_repo
                .find_by_username(&username)
                .await?
                .ok_or("User not found")?;
            let role = Role::from_str(&role)?;

            user_repo.add_role(user.id(), role).await?;
            println!("✓ Role '{}' added to user '{}'", role, username);
        }

        Commands::RemoveRole { username, role } => {
            let user = user_repo
                .find_by_username(&username)
                .await?
                .ok_or("User not found")?;
            let role = Role::from_str(&role)?;

            user_repo.remove_role(user.id(), role).await?;
            println!("✓ Role '{}' removed from user '{}'", role, username);
        }

        Commands::GenerateApiKey {
            username,
            name,
            expires_days,
        } => {
            let user = user_repo
                .find_by_username(&username)
                .await?
                .ok_or("User not found")?;

            let expires_at = expires_days.map(|days| Utc::now() + chrono::Duration::days(days));

            let (key, api_key) = api_key_service
                .generate_api_key(*user.id(), name, expires_at)
                .await?;

            println!("✓ API key generated successfully!");
            println!("\n⚠️  IMPORTANT: Save this key now - it won't be shown again!");
            println!("\n  API Key: {}", key);
            println!("  Key ID:  {}", api_key.id());
            println!("  Name:    {}", api_key.name());
            if let Some(expires) = api_key.expires_at() {
                println!("  Expires: {}", expires);
            }
        }

        Commands::ListApiKeys { username } => {
            let user = user_repo
                .find_by_username(&username)
                .await?
                .ok_or("User not found")?;

            let keys = api_key_service.list_user_keys(user.id()).await?;

            println!("\nAPI Keys for user '{}':", username);
            println!(
                "{:<36} {:<20} {:<12} {:<20}",
                "ID", "Name", "Status", "Expires"
            );
            println!("{}", "-".repeat(90));

            for key in keys {
                let status = if key.enabled() { "Active" } else { "Disabled" };
                let expires = key
                    .expires_at()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Never".to_string());

                println!(
                    "{:<36} {:<20} {:<12} {:<20}",
                    key.id(),
                    key.name(),
                    status,
                    expires
                );
            }
        }

        Commands::RevokeApiKey { key_id } => {
            let key_id = ApiKeyId::parse(&key_id)?;
            api_key_service.revoke_key(key_id).await?;
            println!("✓ API key revoked successfully");
        }
    }

    Ok(())
}
