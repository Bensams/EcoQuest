//! Opaque QR check-in codes.

use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Generates an opaque 256-bit code. It contains no event id or other metadata.
#[must_use]
pub fn generate_check_in_code() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    STANDARD_NO_PAD.encode(bytes)
}

/// Indexed storage hash for a high-entropy code.
#[must_use]
pub fn hash_check_in_code(code: &str) -> Vec<u8> {
    Sha256::digest(code.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_random_and_hash_to_256_bits() {
        let first = generate_check_in_code();
        assert_ne!(first, generate_check_in_code());
        assert_eq!(hash_check_in_code(&first).len(), 32);
        assert!(!first.contains('-'));
    }
}
