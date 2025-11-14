// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! User Repository Trait
//!
//! This module defines the repository interface for user persistence operations.
//! It follows the repository pattern to abstract data access from business logic.

use async_trait::async_trait;

use crate::domain::entities::user::{AuthProvider, User};
use crate::domain::value_objects::UserId;
use crate::error::DomainError;

/// Result type for user repository operations
pub type UserRepoResult<T> = Result<T, DomainError>;

/// User repository trait for data access operations
///
/// This trait defines the interface for user persistence operations.
/// Implementations should handle database-specific details.
///
/// # Examples
///
/// ```rust,ignore
/// use xzepr::domain::repositories::user_repo::UserRepository;
/// use xzepr::domain::entities::user::User;
///
/// async fn example(repo: &impl UserRepository) -> Result<(), Box<dyn std::error::Error>> {
///     let user = repo.find_by_id(&user_id).await?;
///     if let Some(user) = user {
///         println!("Found user: {}", user.username());
///     }
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Find a user by their ID
    ///
    /// # Arguments
    ///
    /// * `id` - The user ID
    ///
    /// # Returns
    ///
    /// Returns Some(User) if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn find_by_id(&self, id: &UserId) -> UserRepoResult<Option<User>>;

    /// Find a user by username
    ///
    /// # Arguments
    ///
    /// * `username` - The username to search for
    ///
    /// # Returns
    ///
    /// Returns Some(User) if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn find_by_username(&self, username: &str) -> UserRepoResult<Option<User>>;

    /// Find a user by email address
    ///
    /// # Arguments
    ///
    /// * `email` - The email address to search for
    ///
    /// # Returns
    ///
    /// Returns Some(User) if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn find_by_email(&self, email: &str) -> UserRepoResult<Option<User>>;

    /// Find a user by OIDC provider subject
    ///
    /// # Arguments
    ///
    /// * `subject` - The provider subject (sub claim)
    ///
    /// # Returns
    ///
    /// Returns Some(User) if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn find_by_oidc_subject(&self, subject: &str) -> UserRepoResult<Option<User>>;

    /// Create a new user
    ///
    /// # Arguments
    ///
    /// * `user` - The user to create
    ///
    /// # Returns
    ///
    /// Returns the created user with populated ID and timestamps
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Username or email already exists
    /// - Database operation fails
    /// - Validation fails
    async fn create(&self, user: User) -> UserRepoResult<User>;

    /// Update an existing user
    ///
    /// # Arguments
    ///
    /// * `user` - The user with updated fields
    ///
    /// # Returns
    ///
    /// Returns the updated user
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - User not found
    /// - Database operation fails
    /// - Validation fails
    async fn update(&self, user: User) -> UserRepoResult<User>;

    /// Delete a user by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The user ID to delete
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if successful
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - User not found
    /// - Database operation fails
    async fn delete(&self, id: &UserId) -> UserRepoResult<()>;

    /// Check if a username exists
    ///
    /// # Arguments
    ///
    /// * `username` - The username to check
    ///
    /// # Returns
    ///
    /// Returns true if username exists, false otherwise
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn username_exists(&self, username: &str) -> UserRepoResult<bool>;

    /// Check if an email exists
    ///
    /// # Arguments
    ///
    /// * `email` - The email address to check
    ///
    /// # Returns
    ///
    /// Returns true if email exists, false otherwise
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn email_exists(&self, email: &str) -> UserRepoResult<bool>;

    /// Create or update user from OIDC authentication
    ///
    /// This method handles user provisioning from OIDC providers.
    /// If a user with the given subject exists, it updates their information.
    /// Otherwise, it creates a new user.
    ///
    /// # Arguments
    ///
    /// * `subject` - OIDC subject (unique user ID from provider)
    /// * `username` - Preferred username
    /// * `email` - Email address (optional)
    /// * `name` - Full name (optional)
    ///
    /// # Returns
    ///
    /// Returns the created or updated user
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn create_or_update_oidc_user(
        &self,
        subject: String,
        username: String,
        email: Option<String>,
        name: Option<String>,
    ) -> UserRepoResult<User>;

    /// List all users with pagination
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of users to return
    /// * `offset` - Number of users to skip
    ///
    /// # Returns
    ///
    /// Returns a vector of users
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn list(&self, limit: i64, offset: i64) -> UserRepoResult<Vec<User>>;

    /// Count total number of users
    ///
    /// # Returns
    ///
    /// Returns the total count of users
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn count(&self) -> UserRepoResult<i64>;

    /// Find users by authentication provider
    ///
    /// # Arguments
    ///
    /// * `provider` - The authentication provider type
    ///
    /// # Returns
    ///
    /// Returns a vector of users using the specified provider
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    async fn find_by_provider(&self, provider: &AuthProvider) -> UserRepoResult<Vec<User>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_repo_result_ok() {
        let result: UserRepoResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(42));
    }

    #[test]
    fn test_user_repo_result_err() {
        let result: UserRepoResult<i32> = Err(DomainError::NotFound {
            entity: "User".to_string(),
            id: "123".to_string(),
        });
        assert!(result.is_err());
    }
}
