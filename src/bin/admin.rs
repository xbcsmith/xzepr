// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use chrono::Utc;
use clap::{Parser, Subcommand};
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use xzepr::auth::api_key::AuthUserRepository;
use xzepr::infrastructure::database::{PostgresApiKeyRepository, PostgresUserRepository};
use xzepr::{ApiKeyId, ApiKeyService, Role, Settings, User};

#[derive(Parser)]
#[command(name = "xzepr-admin")]
#[command(about = "XZEPR administration tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new local user.
    /// The password is read interactively from stdin to avoid exposure in
    /// process listings and shell history.
    CreateUser {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        email: String,
        // password is now read from stdin
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
            role,
        } => {
            // Read password from stdin to avoid exposing it in process listing or shell history
            eprintln!("Enter password for user '{}': ", username);
            let mut password = String::new();
            std::io::stdin()
                .read_line(&mut password)
                .map_err(|e| format!("Failed to read password: {}", e))?;
            let password = password.trim_end_matches(['\n', '\r']).to_string();
            if password.is_empty() {
                return Err("Password cannot be empty".into());
            }

            let role = Role::from_str(&role).map_err(|err| -> Box<dyn std::error::Error> {
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, err))
            })?;

            if let Some(existing_user) = user_repo.find_by_username(&username).await? {
                let existing_roles: Vec<String> = existing_user
                    .roles()
                    .iter()
                    .map(|r| r.to_string())
                    .collect();

                println!("User '{}' already exists.", username);
                println!("  ID: {}", existing_user.id());
                println!("  Email: {}", existing_user.email());
                println!("  Roles: {}", existing_roles.join(", "));

                if role != Role::User && !existing_user.has_role(&role) {
                    user_repo.add_role(existing_user.id(), role).await?;
                    println!("  Added missing role: {}", role);
                }
            } else {
                let user = User::new_local(username, email, password)
                    .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;

                user_repo.save(&user).await?;

                if role != Role::User {
                    user_repo.add_role(user.id(), role).await?;
                }

                println!("[OK] User created successfully.");
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
            println!("[OK] Role '{}' added to user '{}'", role, username);
        }

        Commands::RemoveRole { username, role } => {
            let user = user_repo
                .find_by_username(&username)
                .await?
                .ok_or("User not found")?;
            let role = Role::from_str(&role)?;

            user_repo.remove_role(user.id(), role).await?;
            println!("[OK] Role '{}' removed from user '{}'", role, username);
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

            println!("[OK] API key generated successfully.");
            println!("\nIMPORTANT: Save this key now - it will not be shown again.");
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
            println!("[OK] API key revoked successfully.");
        }
    }

    Ok(())
}
