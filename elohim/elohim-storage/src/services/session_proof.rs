//! Session proof — the peer-native answer to "did this human prove they hold
//! this key?", with no name, no doorway, and no network involved.
//!
//! # Why this exists
//!
//! Every auth surface in the tree today answers "who is calling?" by reading an
//! *assertion*: `POST /session` deserializes a body naming a `human_id` and an
//! `agent_pub_key` and creates an ACTIVE session from it, and `extract_agent_key`
//! reads `X-Agent-Id` verbatim. Nothing verifies that the caller controls the key
//! they name. That is adequate only while the sole caller is a co-located Tauri
//! shell; it is not adequate for the local-first model this module is the
//! foundation of:
//!
//!   - **Self, own device** — a runtime you own establishes your session because
//!     you proved key control to it. No doorway, no DNS, no redirect.
//!   - **Guest on a trusted peer** — you sign in on a family member's runtime the
//!     way a guest profile works on a shared browser. Trust comes from shared
//!     device custody; the *authorization* still comes from proving the key.
//!
//! Both cases reduce to one predicate, and it is this one. A doorway-issued
//! bearer token cannot serve here, because in the pure-native case there is no
//! doorway to issue one.
//!
//! # What this proves, and what it does not
//!
//! PROVES: the presenter holds the Ed25519 private key whose public half is
//! embedded in `agent_pub_key`, *at the moment they signed this specific nonce*.
//!
//! Does NOT prove:
//!   - that the nonce was fresh, single-use, or issued by this node — nonce
//!     lifecycle is the caller's concern (see `SessionChallenge` in the wiring
//!     slice); this module is a pure function over bytes it is handed.
//!   - that the agent is who they claim socially (that is `HumanityWitness` /
//!     `attestation:*` on the elohim DNA).
//!   - that the key is current in its lineage — a rotated-away key still
//!     verifies here. Lineage currency is `KeyStewardship`'s concern.
//!   - anything about device custody or policy. `DevicePolicy` (imagodei
//!     `stewardship.rs`) governs what a proven guest may then DO.
//!
//! # Encoding discipline (this is the part that has already cost the tree once)
//!
//! The same agent is named by several encodings, and raw-comparing across them
//! is a recurring defect class — `handle_add_portal_host` compares a hex-encoded
//! raw-39 `agent_key_hex()` against a `uhCAk…` base64 identity, a comparison that
//! can never succeed, and it currently reads as an authorization refusal
//! (`503 BROWSER_WRITE_PATH_PENDING`) rather than the encoding bug it is.
//!
//! This module therefore accepts exactly ONE form — the canonical multibase
//! `uhCAk…` agent identity — and decodes it structurally rather than by slicing
//! a string. Anything else is a typed error, never a silent mismatch.
//!
//! A Holochain `AgentPubKey` is multibase `u` (base64url, unpadded) over 39
//! bytes: a 3-byte type prefix, the 32-byte Ed25519 public key, and a 4-byte DHT
//! location. The agent prefix is `0x84 0x20 0x24`, which is why every agent key
//! renders as `uhCAk…`.
//!
//! # Signature strictness
//!
//! Verification uses `verify_strict`, matching `binding_cross_signature.rs` and
//! `shamir_transport.rs`: it rejects signature malleability and small-order
//! public keys. Plain `verify` must not be substituted here.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};

/// Multibase prefix for base64url-unpadded, per the multibase table.
const MULTIBASE_BASE64URL: char = 'u';

/// Holochain `AgentPubKey` type prefix (the bytes that render as `hCAk`).
const AGENT_PREFIX: [u8; 3] = [0x84, 0x20, 0x24];

/// Total decoded length of a Holochain hash: 3 prefix + 32 core + 4 DHT location.
const HOLO_HASH_LEN: usize = 39;

/// Offset of the Ed25519 public key inside a decoded Holochain hash.
const CORE_START: usize = 3;
const CORE_END: usize = 35;

/// Domain separation for a session-establishment challenge.
///
/// Follows the `elohim:<surface>:<claim>:<version>` convention established by
/// `binding_cross_signature.rs`. A signature produced for this domain can never
/// be replayed into the agent/transport binding surfaces, and vice versa —
/// which matters because the same agent key signs on several of them.
pub const SESSION_CHALLENGE_DOMAIN: &[u8] = b"elohim:session:agent-proves-key:v1";

/// Minimum accepted challenge length. A 32-byte nonce is what
/// `signal/mod.rs` issues for the transport-plane challenge; anything shorter
/// is refused rather than trusted, so a caller cannot weaken the proof by
/// handing in a short or empty nonce.
pub const MIN_CHALLENGE_BYTES: usize = 32;

/// Why a session proof was refused.
///
/// Every arm is a distinct, actionable cause. In particular an encoding fault is
/// never reported as a signature failure — conflating them is what makes an
/// encoding mismatch masquerade as an authorization decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofError {
    /// The identity string did not carry the `u` multibase prefix.
    NotMultibaseBase64Url,
    /// The base64url body did not decode.
    UndecodableIdentity,
    /// Decoded to something other than 39 bytes.
    WrongIdentityLength(usize),
    /// Decoded, but the 3-byte prefix is not an AgentPubKey's.
    NotAnAgentPubKey([u8; 3]),
    /// The 32-byte core is not a valid Ed25519 public key (or is small-order).
    InvalidPublicKey,
    /// The signature was not 64 bytes.
    WrongSignatureLength(usize),
    /// The challenge was shorter than [`MIN_CHALLENGE_BYTES`].
    ChallengeTooShort(usize),
    /// Decode and shape were fine; the signature does not verify.
    SignatureInvalid,
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotMultibaseBase64Url => write!(
                f,
                "agent identity must be multibase '{MULTIBASE_BASE64URL}' (base64url) — a raw \
                 base64 or hex key is a different encoding, not a different key"
            ),
            Self::UndecodableIdentity => write!(f, "agent identity is not valid base64url"),
            Self::WrongIdentityLength(n) => write!(
                f,
                "agent identity decoded to {n} bytes, expected {HOLO_HASH_LEN}"
            ),
            Self::NotAnAgentPubKey(p) => write!(
                f,
                "identity prefix {p:02x?} is not an AgentPubKey ({:02x?})",
                AGENT_PREFIX
            ),
            Self::InvalidPublicKey => write!(f, "identity core is not a valid Ed25519 public key"),
            Self::WrongSignatureLength(n) => {
                write!(f, "signature is {n} bytes, expected 64")
            }
            Self::ChallengeTooShort(n) => {
                write!(f, "challenge is {n} bytes, minimum {MIN_CHALLENGE_BYTES}")
            }
            Self::SignatureInvalid => write!(f, "signature does not verify for this agent key"),
        }
    }
}

impl std::error::Error for ProofError {}

/// Extract the Ed25519 verifying key from a canonical `uhCAk…` agent identity.
///
/// Structural, not positional: the multibase prefix, decoded length and type
/// prefix are each checked and each has its own error, so a caller can tell
/// "you sent me the wrong encoding" from "you sent me a non-agent hash" from
/// "that is not a usable key".
pub fn verifying_key_from_agent_pub_key(agent_pub_key: &str) -> Result<VerifyingKey, ProofError> {
    let body = agent_pub_key
        .strip_prefix(MULTIBASE_BASE64URL)
        .ok_or(ProofError::NotMultibaseBase64Url)?;

    let decoded = URL_SAFE_NO_PAD
        .decode(body)
        .map_err(|_| ProofError::UndecodableIdentity)?;

    if decoded.len() != HOLO_HASH_LEN {
        return Err(ProofError::WrongIdentityLength(decoded.len()));
    }

    let prefix: [u8; 3] = [decoded[0], decoded[1], decoded[2]];
    if prefix != AGENT_PREFIX {
        return Err(ProofError::NotAnAgentPubKey(prefix));
    }

    let core: [u8; 32] = decoded[CORE_START..CORE_END]
        .try_into()
        .expect("slice is 32 bytes by construction");

    VerifyingKey::from_bytes(&core).map_err(|_| ProofError::InvalidPublicKey)
}

/// The exact bytes an agent signs to establish a session.
///
/// Domain-separated so a signature gathered on another elohim surface cannot be
/// replayed as a session proof.
pub fn challenge_bytes(challenge: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(SESSION_CHALLENGE_DOMAIN.len() + challenge.len());
    msg.extend_from_slice(SESSION_CHALLENGE_DOMAIN);
    msg.extend_from_slice(challenge);
    msg
}

/// Verify that `signature` is `agent_pub_key`'s signature over `challenge`.
///
/// This is the whole predicate. `Ok(())` means the caller demonstrably holds the
/// private key for the named agent identity. It says nothing about nonce
/// freshness, which the caller owns.
pub fn verify_session_proof(
    agent_pub_key: &str,
    challenge: &[u8],
    signature: &[u8],
) -> Result<(), ProofError> {
    if challenge.len() < MIN_CHALLENGE_BYTES {
        return Err(ProofError::ChallengeTooShort(challenge.len()));
    }
    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| ProofError::WrongSignatureLength(signature.len()))?;

    let vk = verifying_key_from_agent_pub_key(agent_pub_key)?;
    let sig = Signature::from_bytes(&sig_bytes);

    vk.verify_strict(&challenge_bytes(challenge), &sig)
        .map_err(|_| ProofError::SignatureInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// Render a signing key as the canonical `uhCAk…` identity, the way a
    /// conductor would. Mirrors the decode under test rather than reusing it, so
    /// a bug in the decoder cannot hide behind a matching encoder.
    fn agent_identity_for(sk: &SigningKey) -> String {
        let mut raw = Vec::with_capacity(HOLO_HASH_LEN);
        raw.extend_from_slice(&AGENT_PREFIX);
        raw.extend_from_slice(sk.verifying_key().as_bytes());
        // DHT location — 4 bytes, opaque to verification.
        raw.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        format!("{MULTIBASE_BASE64URL}{}", URL_SAFE_NO_PAD.encode(&raw))
    }

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn nonce(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    #[test]
    fn a_real_signature_over_the_challenge_verifies() {
        let sk = key(1);
        let id = agent_identity_for(&sk);
        let n = nonce(9);
        let sig = sk.sign(&challenge_bytes(&n));
        assert_eq!(
            verify_session_proof(&id, &n, &sig.to_bytes()),
            Ok(()),
            "a genuine proof must verify"
        );
    }

    #[test]
    fn the_encoded_identity_renders_as_uhcak() {
        // Guards the prefix constant: if AGENT_PREFIX drifts, every agent
        // identity in the tree stops matching and this is the cheapest place to
        // notice.
        assert!(
            agent_identity_for(&key(3)).starts_with("uhCAk"),
            "agent identities must render as uhCAk…"
        );
    }

    #[test]
    fn another_agents_signature_is_refused() {
        let signer = key(1);
        let impostor_id = agent_identity_for(&key(2));
        let n = nonce(9);
        let sig = signer.sign(&challenge_bytes(&n));
        assert_eq!(
            verify_session_proof(&impostor_id, &n, &sig.to_bytes()),
            Err(ProofError::SignatureInvalid),
            "holding SOME key must not establish a session as ANOTHER agent"
        );
    }

    #[test]
    fn a_signature_over_a_different_nonce_is_refused() {
        let sk = key(1);
        let id = agent_identity_for(&sk);
        let sig = sk.sign(&challenge_bytes(&nonce(1)));
        assert_eq!(
            verify_session_proof(&id, &nonce(2), &sig.to_bytes()),
            Err(ProofError::SignatureInvalid),
            "a captured proof must not replay onto a different challenge"
        );
    }

    /// Domain separation is the whole defence against cross-surface replay: the
    /// same agent key also signs transport bindings. A signature over the bare
    /// nonce (no domain) must not pass here.
    #[test]
    fn a_signature_missing_the_domain_prefix_is_refused() {
        let sk = key(1);
        let id = agent_identity_for(&sk);
        let n = nonce(9);
        let undomained = sk.sign(&n);
        assert_eq!(
            verify_session_proof(&id, &n, &undomained.to_bytes()),
            Err(ProofError::SignatureInvalid),
            "an un-domain-separated signature must not establish a session"
        );
    }

    /// A signature made for the agent↔transport binding surface must not be
    /// replayable as a session proof even though the signing key is the same.
    #[test]
    fn a_signature_from_another_elohim_domain_is_refused() {
        let sk = key(1);
        let id = agent_identity_for(&sk);
        let n = nonce(9);
        let mut other = Vec::new();
        other.extend_from_slice(b"elohim:apb:agent-attests-transport:v1");
        other.extend_from_slice(&n);
        let foreign = sk.sign(&other);
        assert_eq!(
            verify_session_proof(&id, &n, &foreign.to_bytes()),
            Err(ProofError::SignatureInvalid),
            "cross-domain replay must be refused"
        );
    }

    #[test]
    fn a_short_challenge_is_refused_before_any_crypto() {
        let sk = key(1);
        let id = agent_identity_for(&sk);
        let short = [7u8; 8];
        let sig = sk.sign(&challenge_bytes(&short));
        assert_eq!(
            verify_session_proof(&id, &short, &sig.to_bytes()),
            Err(ProofError::ChallengeTooShort(8)),
            "a caller must not be able to weaken the proof with a short nonce"
        );
    }

    #[test]
    fn a_truncated_signature_is_a_length_error_not_a_verify_failure() {
        let sk = key(1);
        let id = agent_identity_for(&sk);
        let n = nonce(9);
        let sig = sk.sign(&challenge_bytes(&n));
        let truncated = &sig.to_bytes()[..63];
        assert_eq!(
            verify_session_proof(&id, &n, truncated),
            Err(ProofError::WrongSignatureLength(63))
        );
    }

    // ---- encoding discipline: each fault gets its OWN error --------------

    #[test]
    fn a_raw_base64_key_is_refused_as_an_encoding_fault() {
        // This is the shape the doorway JWT carries (standard base64 of the raw
        // 32 bytes). It names the same agent but is NOT the canonical identity.
        // It must fail as an ENCODING error, never as a signature failure — the
        // distinction is what stops an encoding bug reading as an auth refusal.
        let sk = key(1);
        let raw_b64 =
            base64::engine::general_purpose::STANDARD.encode(sk.verifying_key().as_bytes());
        assert_eq!(
            verify_session_proof(&raw_b64, &nonce(9), &[0u8; 64]),
            Err(ProofError::NotMultibaseBase64Url)
        );
    }

    #[test]
    fn a_hex_raw39_key_is_refused_as_an_encoding_fault() {
        // The `agent_key_hex()` form whose mismatch against `uhCAk…` currently
        // presents as 503 BROWSER_WRITE_PATH_PENDING in handle_add_portal_host.
        let mut raw = Vec::new();
        raw.extend_from_slice(&AGENT_PREFIX);
        raw.extend_from_slice(key(1).verifying_key().as_bytes());
        raw.extend_from_slice(&[0, 0, 0, 0]);
        let hexed: String = raw.iter().map(|b| format!("{b:02x}")).collect();
        assert!(matches!(
            verify_session_proof(&hexed, &nonce(9), &[0u8; 64]),
            Err(ProofError::NotMultibaseBase64Url)
        ));
    }

    #[test]
    fn a_non_agent_holochain_hash_is_refused() {
        // uhCEk… is an EntryHash: right length, right multibase, wrong prefix.
        let mut raw = Vec::with_capacity(HOLO_HASH_LEN);
        raw.extend_from_slice(&[0x84, 0x21, 0x24]);
        raw.extend_from_slice(key(1).verifying_key().as_bytes());
        raw.extend_from_slice(&[0, 0, 0, 0]);
        let id = format!("u{}", URL_SAFE_NO_PAD.encode(&raw));
        assert_eq!(
            verify_session_proof(&id, &nonce(9), &[0u8; 64]),
            Err(ProofError::NotAnAgentPubKey([0x84, 0x21, 0x24]))
        );
    }

    #[test]
    fn a_wrong_length_identity_is_refused() {
        let id = format!("u{}", URL_SAFE_NO_PAD.encode([0x84, 0x20, 0x24, 0x01]));
        assert_eq!(
            verify_session_proof(&id, &nonce(9), &[0u8; 64]),
            Err(ProofError::WrongIdentityLength(4))
        );
    }

    #[test]
    fn an_undecodable_identity_is_refused() {
        assert_eq!(
            verify_session_proof("u!!!not base64!!!", &nonce(9), &[0u8; 64]),
            Err(ProofError::UndecodableIdentity)
        );
    }

    #[test]
    fn an_empty_identity_is_refused() {
        assert_eq!(
            verify_session_proof("", &nonce(9), &[0u8; 64]),
            Err(ProofError::NotMultibaseBase64Url)
        );
    }

    /// `verify_strict` (not `verify`) is required: it rejects small-order public
    /// keys. An all-zero core is the canonical small-order case.
    #[test]
    fn a_small_order_public_key_is_refused() {
        let mut raw = Vec::with_capacity(HOLO_HASH_LEN);
        raw.extend_from_slice(&AGENT_PREFIX);
        raw.extend_from_slice(&[0u8; 32]);
        raw.extend_from_slice(&[0, 0, 0, 0]);
        let id = format!("u{}", URL_SAFE_NO_PAD.encode(&raw));
        // Rejected either at key parse or at verify_strict — never Ok.
        assert!(
            verify_session_proof(&id, &nonce(9), &[0u8; 64]).is_err(),
            "a small-order key must never establish a session"
        );
    }

    #[test]
    fn round_trip_decode_recovers_the_public_key() {
        let sk = key(5);
        let id = agent_identity_for(&sk);
        let vk = verifying_key_from_agent_pub_key(&id).expect("decodes");
        assert_eq!(vk.as_bytes(), sk.verifying_key().as_bytes());
    }

    #[test]
    fn the_dht_location_suffix_does_not_affect_verification() {
        // Two identities for the SAME key differing only in the trailing 4-byte
        // DHT location must both verify — the location is not key material.
        let sk = key(6);
        let n = nonce(2);
        let sig = sk.sign(&challenge_bytes(&n)).to_bytes();
        for loc in [[0u8, 0, 0, 0], [0xFF, 0xFF, 0xFF, 0xFF]] {
            let mut raw = Vec::with_capacity(HOLO_HASH_LEN);
            raw.extend_from_slice(&AGENT_PREFIX);
            raw.extend_from_slice(sk.verifying_key().as_bytes());
            raw.extend_from_slice(&loc);
            let id = format!("u{}", URL_SAFE_NO_PAD.encode(&raw));
            assert_eq!(verify_session_proof(&id, &n, &sig), Ok(()));
        }
    }
}
