// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/auth/local/password.rs

use crate::error::AuthError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::PasswordHashingFailed(e.to_string()))?
        .to_string();

    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| AuthError::InvalidPasswordHash(e.to_string()))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_success() {
        let password = "TestPassword123!";
        let result = hash_password(password);

        assert!(result.is_ok());
        let hash = result.unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password);
    }

    #[test]
    fn test_hash_password_generates_unique_hashes() {
        let password = "TestPassword123!";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Same password should generate different hashes due to different salts
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "MySecurePassword123!";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "CorrectPassword123!";
        let hash = hash_password(password).unwrap();

        let result = verify_password("WrongPassword123!", &hash);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_password_with_invalid_hash() {
        let result = verify_password("anypassword", "not-a-valid-hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_password_with_empty_hash() {
        let result = verify_password("password", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_empty_password() {
        let result = hash_password("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_long_password() {
        let long_password = "a".repeat(1000);
        let result = hash_password(&long_password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_special_characters() {
        let password = "P@ssw0rd!#$%^&*()_+-=[]{}|;:',.<>?/~`";
        let result = hash_password(password);
        assert!(result.is_ok());

        let hash = result.unwrap();
        let verify_result = verify_password(password, &hash);
        assert!(verify_result.is_ok());
        assert!(verify_result.unwrap());
    }

    #[test]
    fn test_hash_unicode_password() {
        let password = "пароль密码🔒";
        let result = hash_password(password);
        assert!(result.is_ok());

        let hash = result.unwrap();
        let verify_result = verify_password(password, &hash);
        assert!(verify_result.is_ok());
        assert!(verify_result.unwrap());
    }

    #[test]
    fn test_verify_password_case_sensitive() {
        let password = "Password123!";
        let hash = hash_password(password).unwrap();

        let result = verify_password("password123!", &hash);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_hash_password_whitespace() {
        let password = "password with spaces";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
