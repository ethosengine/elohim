//! Session id: a domain-separated hash of the working version's document id.
//! A lookup key, never an address.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

const DOMAIN: &[u8] = b"elohim/live-session/session-id/v1\0";

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SessionId(pub [u8; 32]);

impl SessionId {
    /// Derive the session id for a working version's document id.
    pub fn for_document(document_id: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(DOMAIN);
        h.update(document_id);
        SessionId(h.finalize().into())
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionId({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_distinct() {
        let a = SessionId::for_document(b"doc-a");
        assert_eq!(a, SessionId::for_document(b"doc-a"));
        assert_ne!(a, SessionId::for_document(b"doc-b"));
    }

    #[test]
    fn domain_separated_from_plain_hash() {
        let plain: [u8; 32] = Sha256::digest(b"doc-a").into();
        assert_ne!(SessionId::for_document(b"doc-a").0, plain);
    }

    #[test]
    fn displays_as_64_hex_chars() {
        let s = SessionId::for_document(b"x").to_string();
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
