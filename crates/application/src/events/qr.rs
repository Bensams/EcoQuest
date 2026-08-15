//! Opaque QR check-in codes.
//!
//! Codes are read aloud and typed by hand at events, so they use a Crockford
//! base32 alphabet: no `I`, `L`, `O` or `U`, and the usual misreadings are
//! folded back on input. The code carries no event id or other metadata.

use rand::RngCore;
use sha2::{Digest, Sha256};

/// Crockford base32. Exactly 32 symbols, so a byte masked to 5 bits selects one
/// without modulo bias.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Symbols per code. 32^8 is about 1.1e12 possibilities.
const CODE_LEN: usize = 8;

/// Where the readability separator goes, as in `XK4T-9MPQ`.
const GROUP: usize = 4;

/// Generates a code such as `XK4T-9MPQ`.
///
/// The hyphen is presentation only; [`normalize_check_in_code`] removes it, so
/// a player may type the code with or without it.
#[must_use]
pub fn generate_check_in_code() -> String {
    let mut bytes = [0_u8; CODE_LEN];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut out = String::with_capacity(CODE_LEN + 1);
    for (i, byte) in bytes.iter().enumerate() {
        if i == GROUP {
            out.push('-');
        }
        // The alphabet is exactly 32 symbols, so the low 5 bits are uniform.
        out.push(ALPHABET[usize::from(byte & 0x1f)] as char);
    }
    out
}

/// Canonicalises a typed or scanned code: upper-cases it, folds the characters
/// Crockford treats as interchangeable, and drops separators and spaces.
///
/// Returns the bare symbols with no hyphen, which is the form that is hashed.
#[must_use]
pub fn normalize_check_in_code(code: &str) -> String {
    code.chars()
        .filter_map(|c| match c.to_ascii_uppercase() {
            // Readers and typists confuse these with digits; Crockford folds them.
            'O' => Some('0'),
            'I' | 'L' => Some('1'),
            c if ALPHABET.contains(&(c as u8)) => Some(c),
            // Hyphens, spaces and anything else are separators, not symbols.
            _ => None,
        })
        .collect()
}

/// Indexed storage hash for a code.
///
/// Normalises first so a code hashes the same whether it arrived from the
/// generator, a QR scan, or a player typing it in lower case without the hyphen.
#[must_use]
pub fn hash_check_in_code(code: &str) -> Vec<u8> {
    Sha256::digest(normalize_check_in_code(code).as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn codes_are_grouped_and_use_the_unambiguous_alphabet() {
        let code = generate_check_in_code();
        assert_eq!(code.len(), CODE_LEN + 1, "{code}");
        assert_eq!(code.as_bytes()[GROUP], b'-', "{code}");
        for c in code.chars().filter(|c| *c != '-') {
            assert!(ALPHABET.contains(&(c as u8)), "{c} is not in the alphabet");
        }
        assert!(
            !code.contains(['I', 'L', 'O', 'U']),
            "ambiguous letter in {code}"
        );
    }

    #[test]
    fn codes_are_random() {
        let codes: HashSet<String> = (0..500).map(|_| generate_check_in_code()).collect();
        assert_eq!(codes.len(), 500, "generator repeated a code");
    }

    #[test]
    fn typed_variants_hash_to_the_same_code() {
        let code = "XK4T-9MPQ";
        let expected = hash_check_in_code(code);
        for variant in ["xk4t-9mpq", "XK4T9MPQ", "  xk4t 9mpq  ", "XK4T–9MPQ"] {
            assert_eq!(hash_check_in_code(variant), expected, "variant {variant}");
        }
    }

    #[test]
    fn confusable_characters_are_folded() {
        // O reads as zero, I and L read as one.
        assert_eq!(normalize_check_in_code("OIL5"), "0115");
        assert_eq!(hash_check_in_code("0115"), hash_check_in_code("oil5"));
    }

    #[test]
    fn distinct_codes_do_not_collide() {
        assert_ne!(
            hash_check_in_code("XK4T9MPQ"),
            hash_check_in_code("XK4T9MPR")
        );
        assert_eq!(hash_check_in_code("XK4T9MPQ").len(), 32);
    }
}
