// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! User Provisioning Service
//!
//! This module provides user provisioning functionality for OIDC authentication.
//! It handles creating or updating users based on OIDC claims.

use std::sync::Arc;
use tracing::{debug, instrument};

use crate::auth::oidc::callback::OidcUserData;
use crate::domain::entities::user::User;
use crate::domain::repositories::user_repo::UserRepository;
use crate::error::DomainError;

/// User provisioning service
///
/// Handles automatic user creation and updates during OIDC authentication.
pub struct UserProvisioningService<R: UserRepository> {
    user_repository: Arc<R>,
}

/// Result type for provisioning operations
pub type ProvisioningResult<T> = Result<T, ProvisioningError>;

/// Errors that can occur during user provisioning
#[derive(Debug, thiserror::Error)]
pub enum ProvisioningError {
    /// Repository error
    #[error("Repository error: {0}")]
    Repository(#[from] DomainError),

    /// Missing required data
    #[error("Missing required data: {0}")]
    MissingData(String),

    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

impl<R: UserRepository> UserProvisioningService<R> {
    /// Create a new user provisioning service
    ///
    /// # Arguments
    ///
    /// * `user_repository` - User repository for persistence
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::sync::Arc;
    /// use xzepr::auth::provisioning::UserProvisioningService;
    /// use xzepr::infrastructure::database::PostgresUserRepository;
    ///
    /// let repo = Arc::new(PostgresUserRepository::new(pool));
    /// let service = UserProvisioningService::new(repo);
    /// ```
    pub fn new(user_repository: Arc<R>) -> Self {
        Self { user_repository }
    }

    /// Provision a user from OIDC user data
    ///
    /// This method creates a new user if they don't exist, or updates
    /// their information if they do exist (matched by OIDC subject).
    ///
    /// # Arguments
    ///
    /// * `user_data` - User data extracted from OIDC claims
    ///
    /// # Returns
    ///
    /// Returns the provisioned user
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Required data is missing
    /// - Database operation fails
    /// - Data validation fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use xzepr::auth::provisioning::UserProvisioningService;
    /// use xzepr::auth::oidc::callback::OidcUserData;
    /// use xzepr::auth::rbac::Role;
    ///
    /// async fn example(service: &UserProvisioningService<impl UserRepository>) {
    ///     let user_data = OidcUserData {
    ///         sub: "oidc-subject-123".to_string(),
    ///         email: Some("user@example.com".to_string()),
    ///         email_verified: true,
    ///         username: "johndoe".to_string(),
    ///         name: Some("John Doe".to_string()),
    ///         given_name: Some("John".to_string()),
    ///         family_name: Some("Doe".to_string()),
    ///         roles: vec![Role::User],
    ///     };
    ///
    ///     let user = service.provision_user(user_data).await?;
    ///     println!("Provisioned user: {}", user.username());
    /// }
    /// ```
    #[instrument(skip(self, user_data))]
    pub async fn provision_user(&self, user_data: OidcUserData) -> ProvisioningResult<User> {
        debug!("Provisioning user with subject: {}", user_data.sub);

        if let Some(mut existing_user) = self
            .user_repository
            .find_by_oidc_subject(&user_data.sub)
            .await?
        {
            debug!(
                "Found existing user: {} ({})",
                existing_user.username(),
                existing_user.id()
            );

            let updated = self.update_user_from_oidc(&mut existing_user, &user_data)?;

            if updated {
                debug!("Updating user information from OIDC claims");
                let updated_user = self.user_repository.update(existing_user).await?;
                return Ok(updated_user);
            }

            return Ok(existing_user);
        }

        debug!("Creating new user from OIDC data");
        let user = self.create_user_from_oidc(&user_data)?;
        let created_user = self.user_repository.create(user).await?;

        debug!(
            "Created new user: {} ({})",
            created_user.username(),
            created_user.id()
        );

        Ok(created_user)
    }

    /// Create a new user from OIDC data
    fn create_user_from_oidc(&self, user_data: &OidcUserData) -> ProvisioningResult<User> {
        let email = user_data
            .email
            .clone()
            .unwrap_or_else(|| format!("{}@noemail.local", user_data.username));

        let user = User::new_oidc(user_data.username.clone(), email, user_data.sub.clone());

        Ok(user)
    }

    /// Update an existing user with OIDC data
    ///
    /// Returns true if any fields were updated, false otherwise
    fn update_user_from_oidc(
        &self,
        user: &mut User,
        user_data: &OidcUserData,
    ) -> ProvisioningResult<bool> {
        let mut updated = false;

        if user.username != user_data.username {
            user.username = user_data.username.clone();
            updated = true;
        }

        if let Some(ref email) = user_data.email {
            if user.email != *email {
                user.email = email.clone();
                updated = true;
            }
        }

        if user.roles != user_data.roles {
            user.roles = user_data.roles.clone();
            updated = true;
        }

        Ok(updated)
    }

    /// Get user repository reference
    pub fn user_repository(&self) -> &Arc<R> {
        &self.user_repository
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::rbac::Role;
    use crate::domain::value_objects::UserId;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::RwLock;

    struct MockUserRepository {
        users: Arc<RwLock<HashMap<String, User>>>,
    }

    impl MockUserRepository {
        fn new() -> Self {
            Self {
                users: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn find_by_id(&self, _id: &UserId) -> Result<Option<User>, DomainError> {
            Ok(None)
        }

        async fn find_by_username(&self, _username: &str) -> Result<Option<User>, DomainError> {
            Ok(None)
        }

        async fn find_by_email(&self, _email: &str) -> Result<Option<User>, DomainError> {
            Ok(None)
        }

        async fn find_by_oidc_subject(&self, subject: &str) -> Result<Option<User>, DomainError> {
            let users = self.users.read().unwrap();
            Ok(users.get(subject).cloned())
        }

        async fn create(&self, user: User) -> Result<User, DomainError> {
            let subject = match &user.auth_provider {
                crate::domain::entities::user::AuthProvider::Keycloak { subject } => {
                    subject.clone()
                }
                _ => return Err(DomainError::InvalidData("Not a Keycloak user".to_string())),
            };

            let mut users = self.users.write().unwrap();
            users.insert(subject, user.clone());
            Ok(user)
        }

        async fn update(&self, user: User) -> Result<User, DomainError> {
            let subject = match &user.auth_provider {
                crate::domain::entities::user::AuthProvider::Keycloak { subject } => {
                    subject.clone()
                }
                _ => return Err(DomainError::InvalidData("Not a Keycloak user".to_string())),
            };

            let mut users = self.users.write().unwrap();
            users.insert(subject, user.clone());
            Ok(user)
        }

        async fn delete(&self, _id: &UserId) -> Result<(), DomainError> {
            Ok(())
        }

        async fn username_exists(&self, _username: &str) -> Result<bool, DomainError> {
            Ok(false)
        }

        async fn email_exists(&self, _email: &str) -> Result<bool, DomainError> {
            Ok(false)
        }

        async fn create_or_update_oidc_user(
            &self,
            _subject: String,
            _username: String,
            _email: Option<String>,
            _name: Option<String>,
        ) -> Result<User, DomainError> {
            unimplemented!()
        }

        async fn list(&self, _limit: i64, _offset: i64) -> Result<Vec<User>, DomainError> {
            Ok(vec![])
        }

        async fn count(&self) -> Result<i64, DomainError> {
            Ok(0)
        }

        async fn find_by_provider(
            &self,
            _provider: &crate::domain::entities::user::AuthProvider,
        ) -> Result<Vec<User>, DomainError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_provision_new_user() {
        let repo = Arc::new(MockUserRepository::new());
        let service = UserProvisioningService::new(repo);

        let user_data = OidcUserData {
            sub: "oidc-123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: true,
            username: "testuser".to_string(),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            roles: vec![Role::User],
        };

        let result = service.provision_user(user_data).await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.username(), "testuser");
        assert_eq!(user.email(), "test@example.com");
    }

    #[tokio::test]
    async fn test_provision_existing_user() {
        let repo = Arc::new(MockUserRepository::new());
        let service = UserProvisioningService::new(repo.clone());

        let user_data1 = OidcUserData {
            sub: "oidc-123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: true,
            username: "testuser".to_string(),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            roles: vec![Role::User],
        };

        service.provision_user(user_data1).await.unwrap();

        let user_data2 = OidcUserData {
            sub: "oidc-123".to_string(),
            email: Some("updated@example.com".to_string()),
            email_verified: true,
            username: "updateduser".to_string(),
            name: Some("Updated User".to_string()),
            given_name: Some("Updated".to_string()),
            family_name: Some("User".to_string()),
            roles: vec![Role::Admin],
        };

        let result = service.provision_user(user_data2).await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.username(), "updateduser");
        assert_eq!(user.email(), "updated@example.com");
    }

    #[tokio::test]
    async fn test_provision_user_without_email() {
        let repo = Arc::new(MockUserRepository::new());
        let service = UserProvisioningService::new(repo);

        let user_data = OidcUserData {
            sub: "oidc-123".to_string(),
            email: None,
            email_verified: false,
            username: "testuser".to_string(),
            name: None,
            given_name: None,
            family_name: None,
            roles: vec![Role::User],
        };

        let result = service.provision_user(user_data).await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.username(), "testuser");
        assert!(user.email().contains("@noemail.local"));
    }

    #[test]
    fn test_create_user_from_oidc() {
        let repo = Arc::new(MockUserRepository::new());
        let service = UserProvisioningService::new(repo);

        let user_data = OidcUserData {
            sub: "oidc-123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: true,
            username: "testuser".to_string(),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            roles: vec![Role::User],
        };

        let result = service.create_user_from_oidc(&user_data);
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.username(), "testuser");
        assert_eq!(user.email(), "test@example.com");
    }

    #[test]
    fn test_update_user_from_oidc_no_changes() {
        let repo = Arc::new(MockUserRepository::new());
        let service = UserProvisioningService::new(repo);

        let mut user = User::new_oidc(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "oidc-123".to_string(),
        );

        let user_data = OidcUserData {
            sub: "oidc-123".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: true,
            username: "testuser".to_string(),
            name: Some("Test User".to_string()),
            given_name: Some("Test".to_string()),
            family_name: Some("User".to_string()),
            roles: vec![Role::User],
        };

        let result = service.update_user_from_oidc(&mut user, &user_data);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_update_user_from_oidc_with_changes() {
        let repo = Arc::new(MockUserRepository::new());
        let service = UserProvisioningService::new(repo);

        let mut user = User::new_oidc(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "oidc-123".to_string(),
        );

        let user_data = OidcUserData {
            sub: "oidc-123".to_string(),
            email: Some("updated@example.com".to_string()),
            email_verified: true,
            username: "updateduser".to_string(),
            name: Some("Updated User".to_string()),
            given_name: Some("Updated".to_string()),
            family_name: Some("User".to_string()),
            roles: vec![Role::Admin],
        };

        let result = service.update_user_from_oidc(&mut user, &user_data);
        assert!(result.is_ok());
        assert!(result.unwrap());

        assert_eq!(user.username(), "updateduser");
        assert_eq!(user.email(), "updated@example.com");
        assert_eq!(user.roles(), &[Role::Admin]);
    }
}
