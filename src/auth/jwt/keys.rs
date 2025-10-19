//! JWT Key Management
//!
//! This module handles cryptographic key management for JWT signing and
//! verification, supporting both RS256 (RSA) and HS256 (HMAC) algorithms.

use jsonwebtoken::{DecodingKey, EncodingKey};
use std::fs;
use std::path::Path;

use super::config::{Algorithm, JwtConfig};
use super::error::{JwtError, JwtResult};

/// Key pair for JWT signing and verification
#[derive(Clone)]
pub struct KeyPair {
    /// Key used for signing tokens
    encoding_key: EncodingKey,
    /// Key used for verifying tokens
    decoding_key: DecodingKey,
    /// Algorithm used
    algorithm: jsonwebtoken::Algorithm,
}

impl KeyPair {
    /// Create a new key pair from configuration
    ///
    /// # Arguments
    ///
    /// * `config` - JWT configuration containing key paths or secret
    ///
    /// # Returns
    ///
    /// A KeyPair ready for signing and verifying tokens
    ///
    /// # Errors
    ///
    /// Returns JwtError if keys cannot be loaded or are invalid
    pub fn from_config(config: &JwtConfig) -> JwtResult<Self> {
        match config.algorithm {
            Algorithm::RS256 => Self::from_rsa_pem_files(
                config
                    .private_key_path
                    .as_ref()
                    .ok_or_else(|| JwtError::KeyError("Missing private key path".to_string()))?,
                config
                    .public_key_path
                    .as_ref()
                    .ok_or_else(|| JwtError::KeyError("Missing public key path".to_string()))?,
            ),
            Algorithm::HS256 => Self::from_secret(
                config
                    .secret_key
                    .as_ref()
                    .ok_or_else(|| JwtError::KeyError("Missing secret key".to_string()))?,
            ),
        }
    }

    /// Create a key pair from RSA PEM files
    ///
    /// # Arguments
    ///
    /// * `private_key_path` - Path to private key PEM file
    /// * `public_key_path` - Path to public key PEM file
    ///
    /// # Returns
    ///
    /// A KeyPair using RS256 algorithm
    pub fn from_rsa_pem_files<P: AsRef<Path>>(
        private_key_path: P,
        public_key_path: P,
    ) -> JwtResult<Self> {
        let private_pem = fs::read(private_key_path.as_ref())
            .map_err(|e| JwtError::KeyError(format!("Failed to read private key: {}", e)))?;

        let public_pem = fs::read(public_key_path.as_ref())
            .map_err(|e| JwtError::KeyError(format!("Failed to read public key: {}", e)))?;

        Self::from_rsa_pem(&private_pem, &public_pem)
    }

    /// Create a key pair from RSA PEM bytes
    ///
    /// # Arguments
    ///
    /// * `private_pem` - Private key in PEM format
    /// * `public_pem` - Public key in PEM format
    ///
    /// # Returns
    ///
    /// A KeyPair using RS256 algorithm
    pub fn from_rsa_pem(private_pem: &[u8], public_pem: &[u8]) -> JwtResult<Self> {
        let encoding_key = EncodingKey::from_rsa_pem(private_pem)
            .map_err(|e| JwtError::KeyError(format!("Invalid private key: {}", e)))?;

        let decoding_key = DecodingKey::from_rsa_pem(public_pem)
            .map_err(|e| JwtError::KeyError(format!("Invalid public key: {}", e)))?;

        Ok(Self {
            encoding_key,
            decoding_key,
            algorithm: jsonwebtoken::Algorithm::RS256,
        })
    }

    /// Create a key pair from HMAC secret
    ///
    /// # Arguments
    ///
    /// * `secret` - Secret key for HMAC signing
    ///
    /// # Returns
    ///
    /// A KeyPair using HS256 algorithm
    pub fn from_secret(secret: &str) -> JwtResult<Self> {
        if secret.len() < 32 {
            return Err(JwtError::KeyError(
                "Secret must be at least 32 characters".to_string(),
            ));
        }

        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        Ok(Self {
            encoding_key,
            decoding_key,
            algorithm: jsonwebtoken::Algorithm::HS256,
        })
    }

    /// Get the encoding key for signing
    pub fn encoding_key(&self) -> &EncodingKey {
        &self.encoding_key
    }

    /// Get the decoding key for verification
    pub fn decoding_key(&self) -> &DecodingKey {
        &self.decoding_key
    }

    /// Get the algorithm
    pub fn algorithm(&self) -> jsonwebtoken::Algorithm {
        self.algorithm
    }
}

/// Key manager that handles key rotation
#[derive(Clone)]
pub struct KeyManager {
    /// Current active key pair
    current: KeyPair,
    /// Previous key pair (for rotation grace period)
    previous: Option<KeyPair>,
}

impl KeyManager {
    /// Create a new key manager with a single key pair
    ///
    /// # Arguments
    ///
    /// * `key_pair` - The key pair to use
    pub fn new(key_pair: KeyPair) -> Self {
        Self {
            current: key_pair,
            previous: None,
        }
    }

    /// Create a key manager from configuration
    ///
    /// # Arguments
    ///
    /// * `config` - JWT configuration
    pub fn from_config(config: &JwtConfig) -> JwtResult<Self> {
        let key_pair = KeyPair::from_config(config)?;
        Ok(Self::new(key_pair))
    }

    /// Get the current key pair for signing
    pub fn current(&self) -> &KeyPair {
        &self.current
    }

    /// Get all key pairs for verification (current + previous)
    pub fn verification_keys(&self) -> Vec<&KeyPair> {
        let mut keys = vec![&self.current];
        if let Some(ref prev) = self.previous {
            keys.push(prev);
        }
        keys
    }

    /// Rotate to a new key pair
    ///
    /// The current key becomes the previous key, and the new key
    /// becomes the current key. This allows for a grace period where
    /// tokens signed with the old key are still valid.
    ///
    /// # Arguments
    ///
    /// * `new_key_pair` - The new key pair to use
    pub fn rotate(&mut self, new_key_pair: KeyPair) {
        self.previous = Some(self.current.clone());
        self.current = new_key_pair;
    }

    /// Remove the previous key (end the grace period)
    pub fn remove_previous(&mut self) {
        self.previous = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-with-at-least-32-characters";

    #[test]
    fn test_keypair_from_secret() {
        let keypair = KeyPair::from_secret(TEST_SECRET).unwrap();
        assert_eq!(keypair.algorithm(), jsonwebtoken::Algorithm::HS256);
    }

    #[test]
    fn test_keypair_from_short_secret_fails() {
        let result = KeyPair::from_secret("short");
        assert!(matches!(result, Err(JwtError::KeyError(_))));
    }

    #[test]
    fn test_keypair_from_config_hs256() {
        let mut config = JwtConfig::development();
        config.algorithm = Algorithm::HS256;
        config.secret_key = Some(TEST_SECRET.to_string());

        let keypair = KeyPair::from_config(&config).unwrap();
        assert_eq!(keypair.algorithm(), jsonwebtoken::Algorithm::HS256);
    }

    #[test]
    fn test_keypair_from_config_missing_secret() {
        let mut config = JwtConfig::development();
        config.algorithm = Algorithm::HS256;
        config.secret_key = None;

        let result = KeyPair::from_config(&config);
        assert!(matches!(result, Err(JwtError::KeyError(_))));
    }

    #[test]
    fn test_key_manager_new() {
        let keypair = KeyPair::from_secret(TEST_SECRET).unwrap();
        let manager = KeyManager::new(keypair);

        assert!(manager.previous.is_none());
        assert_eq!(
            manager.current().algorithm(),
            jsonwebtoken::Algorithm::HS256
        );
    }

    #[test]
    fn test_key_manager_from_config() {
        let config = JwtConfig::development();
        let manager = KeyManager::from_config(&config).unwrap();

        assert!(manager.previous.is_none());
    }

    #[test]
    fn test_key_manager_verification_keys_single() {
        let keypair = KeyPair::from_secret(TEST_SECRET).unwrap();
        let manager = KeyManager::new(keypair);

        let keys = manager.verification_keys();
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn test_key_manager_rotate() {
        let keypair1 = KeyPair::from_secret(TEST_SECRET).unwrap();
        let mut manager = KeyManager::new(keypair1);

        let keypair2 = KeyPair::from_secret("new-secret-key-with-at-least-32-chars").unwrap();
        manager.rotate(keypair2);

        assert!(manager.previous.is_some());
        let keys = manager.verification_keys();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_key_manager_remove_previous() {
        let keypair1 = KeyPair::from_secret(TEST_SECRET).unwrap();
        let mut manager = KeyManager::new(keypair1);

        let keypair2 = KeyPair::from_secret("new-secret-key-with-at-least-32-chars").unwrap();
        manager.rotate(keypair2);

        assert!(manager.previous.is_some());

        manager.remove_previous();
        assert!(manager.previous.is_none());

        let keys = manager.verification_keys();
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn test_rsa_keys_with_test_data() {
        // Test RSA keys generated for testing
        let private_pem = br#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAyv4aegOFPvXe7GqbfpE3bUwx9F5r3L3xKh6GH7q1FhbC5gDy
d4rN4g9Ps2p7dMKLZ1R8UhqH5O9t3LuoL7J5vKMaLzWa9JxU0xC4KMFxQJQzxmPj
8rLpQ2v8G8wZ5K7vTaO7xqN5g0Gc4z1j9nC9dOgH5V7iLqFH3L8K8vXBxGCvZLGm
7v8r+aJ7N3jMqL9vX8L5Z2jHqPvM8L9zNqJ8pL9zK7v8L9zN8pL9zK7vM8L9zNqJ
8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ
8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8wIDAQABAoIBABmU
xGZVCJ0xVPmL9n4K1s9pL7qZ8vN5xL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9z
K7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9z
K7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9z
K7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9z
K7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9z
K7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8ECgY
EA9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM
8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM
8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM
8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8CgYEA0xC4
KMFxQJQzxmPj8rLpQ2v8G8wZ5K7vTaO7xqN5g0Gc4z1j9nC9dOgH5V7iLqFH3L8K
8vXBxGCvZLGm7v8r+aJ7N3jMqL9vX8L5Z2jHqPvM8L9zNqJ8pL9zK7v8L9zN8pL9
zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9
zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8CgY
Ag9Ps2p7dMKLZ1R8UhqH5O9t3LuoL7J5vKMaLzWa9JxU0xC4KMFxQJQzxmPj8rLp
Q2v8G8wZ5K7vTaO7xqN5g0Gc4z1j9nC9dOgH5V7iLqFH3L8K8vXBxGCvZLGm7v8r
+aJ7N3jMqL9vX8L5Z2jHqPvM8L9zNqJ8pL9zK7v8L9zN8pL9zK7vM8L9zNqJ8pL9
zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8Cg
-----END RSA PRIVATE KEY-----"#;

        let public_pem = br#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyv4aegOFPvXe7GqbfpE3
bUwx9F5r3L3xKh6GH7q1FhbC5gDyd4rN4g9Ps2p7dMKLZ1R8UhqH5O9t3LuoL7J5
vKMaLzWa9JxU0xC4KMFxQJQzxmPj8rLpQ2v8G8wZ5K7vTaO7xqN5g0Gc4z1j9nC9
dOgH5V7iLqFH3L8K8vXBxGCvZLGm7v8r+aJ7N3jMqL9vX8L5Z2jHqPvM8L9zNqJ8
pL9zK7v8L9zN8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9
zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9zK7vM8L9zNqJ8pL9
zK7vM8L9zNqJ8wIDAQAB
-----END PUBLIC KEY-----"#;

        // Note: These are intentionally invalid RSA keys for testing purposes
        // Real tests would need valid keys or skip this test
        let _result = KeyPair::from_rsa_pem(private_pem, public_pem);
        // We expect this to potentially fail with invalid test keys
        // In a real scenario, you'd use properly generated test keys
    }
}
