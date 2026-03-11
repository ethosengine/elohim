//! Password hashing and verification using Argon2
//!
//! Uses argon2id variant with recommended parameters for password hashing.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::types::DoorwayError;

/// Hash a password using Argon2id
///
/// Returns the PHC-formatted hash string that includes the salt and parameters.
pub fn hash_password(password: &str) -> Result<String, DoorwayError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| DoorwayError::Auth(format!("Failed to hash password: {e}")))
}

/// Verify a password against a stored hash
///
/// Returns true if the password matches the hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, DoorwayError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| DoorwayError::Auth(format!("Invalid password hash format: {e}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "correct-horse-battery-staple";
        let hash = hash_password(password).unwrap();

        // Hash should be in PHC format
        assert!(hash.starts_with("$argon2"));

        // Correct password should verify
        assert!(verify_password(password, &hash).unwrap());

        // Wrong password should not verify
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_different_salts() {
        let password = "same-password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Same password should produce different hashes (different salts)
        assert_ne!(hash1, hash2);

        // Both should verify
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_invalid_hash_format() {
        let result = verify_password("password", "not-a-valid-hash");
        assert!(result.is_err());
    }
}
