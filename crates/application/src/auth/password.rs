//! Argon2id password hashing.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};

use crate::{AppError, AppResult};

/// Argon2id hasher with explicit, documented parameters.
#[derive(Clone)]
pub struct PasswordHasherService {
    argon2: Argon2<'static>,
}

impl PasswordHasherService {
    /// OWASP baseline for Argon2id: 19 MiB memory, 2 iterations, 1 lane.
    const MEMORY_KIB: u32 = 19 * 1024;
    const ITERATIONS: u32 = 2;
    const PARALLELISM: u32 = 1;

    /// Builds the hasher with production parameters.
    ///
    /// # Panics
    /// Panics only if the compiled-in constants are invalid, which is a bug.
    #[must_use]
    pub fn new() -> Self {
        let params = Params::new(Self::MEMORY_KIB, Self::ITERATIONS, Self::PARALLELISM, None)
            .expect("argon2 parameters are valid");
        Self {
            argon2: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
        }
    }

    /// Builds a deliberately weak hasher for tests, so suites stay fast.
    ///
    /// # Panics
    /// Panics only if the compiled-in constants are invalid, which is a bug.
    #[must_use]
    pub fn for_tests() -> Self {
        let params = Params::new(Params::MIN_M_COST, Params::MIN_T_COST, 1, None)
            .expect("argon2 test parameters are valid");
        Self {
            argon2: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
        }
    }

    /// Hashes a plaintext password into a PHC string.
    ///
    /// # Errors
    /// Returns [`AppError::Infrastructure`] when hashing fails.
    pub fn hash(&self, plaintext: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        self.argon2
            .hash_password(plaintext.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AppError::Infrastructure(format!("password hashing failed: {e}")))
    }

    /// Verifies a candidate password against a stored PHC string.
    ///
    /// A malformed stored hash yields `false` rather than an error, so a single
    /// corrupt row cannot be distinguished from a wrong password by an attacker.
    #[must_use]
    pub fn verify(&self, plaintext: &str, stored_hash: &str) -> bool {
        match PasswordHash::new(stored_hash) {
            Ok(parsed) => self
                .argon2
                .verify_password(plaintext.as_bytes(), &parsed)
                .is_ok(),
            Err(err) => {
                tracing::error!(error = %err, "stored password hash is malformed");
                false
            }
        }
    }
}

impl Default for PasswordHasherService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_verify_and_are_salted() {
        let hasher = PasswordHasherService::for_tests();
        let hash = hasher.hash("correct-horse-1").expect("hash");
        assert!(hash.starts_with("$argon2id$"));
        assert!(hasher.verify("correct-horse-1", &hash));
        assert!(!hasher.verify("wrong-horse-1", &hash));

        let second = hasher.hash("correct-horse-1").expect("hash");
        assert_ne!(hash, second, "identical passwords must not share a salt");
    }

    #[test]
    fn malformed_stored_hash_fails_closed() {
        let hasher = PasswordHasherService::for_tests();
        assert!(!hasher.verify("correct-horse-1", "not-a-phc-string"));
    }
}
