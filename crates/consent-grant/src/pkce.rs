//! Proof Key for Code Exchange (RFC 7636), `S256` only.
//!
//! The terminal keeps a random verifier and sends only its hash. Redeeming the
//! code requires the verifier, so the code alone is worth nothing to whoever
//! sees it in a redirect or on a screen. The `plain` method is not offered.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

/// RFC 7636 §4.1: 43 to 128 characters from the unreserved set.
pub fn verifier_is_well_formed(verifier: &str) -> bool {
    (43..=128).contains(&verifier.len())
        && verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
}

/// The challenge a terminal sends for `verifier`.
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// A challenge is the unpadded base64url of a SHA-256 digest: 43 characters.
pub fn challenge_is_well_formed(challenge: &str) -> bool {
    challenge.len() == 43
        && matches!(URL_SAFE_NO_PAD.decode(challenge), Ok(bytes) if bytes.len() == 32)
}

/// Comparison whose duration does not depend on where the inputs differ.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn verifier_matches(verifier: &str, challenge_sent: &str) -> bool {
    verifier_is_well_formed(verifier)
        && constant_time_eq(challenge(verifier).as_bytes(), challenge_sent.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 7636 Appendix B.
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    #[test]
    fn the_rfc_vector_holds() {
        assert_eq!(challenge(VERIFIER), CHALLENGE);
        assert!(verifier_matches(VERIFIER, CHALLENGE));
        assert!(challenge_is_well_formed(CHALLENGE));
    }

    #[test]
    fn another_verifier_does_not_match() {
        let other = "a".repeat(43);
        assert!(!verifier_matches(&other, CHALLENGE));
    }

    #[test]
    fn malformed_verifiers_never_match() {
        assert!(!verifier_is_well_formed(&"a".repeat(42)));
        assert!(!verifier_is_well_formed(&"a".repeat(129)));
        assert!(!verifier_is_well_formed(&format!("{}!", "a".repeat(43))));
        assert!(!verifier_matches("", CHALLENGE));
    }

    #[test]
    fn malformed_challenges_are_recognised() {
        assert!(!challenge_is_well_formed(""));
        assert!(!challenge_is_well_formed(&"=".repeat(43)));
        assert!(!challenge_is_well_formed(&CHALLENGE[..42]));
    }

    #[test]
    fn constant_time_eq_agrees_with_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
