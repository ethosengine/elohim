//! Signatures — scheme-agile envelopes with an unambiguous signed transcript.
//!
//! Every signed object carries a [`SignatureEnvelope`]: a scheme code, the signer's public key
//! as a [`Multikey`], and the signature bytes. The object supplies two pieces of authenticated
//! context the envelope does not carry: its **domain** (the object type, e.g. `elohim:epr:v1:`)
//! and its **object epoch**. Both are fixed by the verifier, never read from the attacker.
//!
//! # The signed transcript
//!
//! ```text
//! transcript = domain ‖ u16be(epoch) ‖ u16be(scheme) ‖ key ‖ body
//! ```
//!
//! `key` is the canonical multikey bytes (unsigned-varint codec, minimal, then the raw key) and
//! `body` the object's exact signed bytes. Binding scheme and key into the transcript means a
//! signature made under one scheme or key cannot be replayed as another; binding the epoch means
//! an object cannot be reinterpreted under another epoch's rules. For scheme 0 the key is
//! exactly 34 bytes (`ed 01 ‖ 32`), so the key/body boundary is fixed; domains are registered
//! constants ending in `:` and the verifier supplies them.
//!
//! # Outcomes
//!
//! [`SignatureEnvelope::check`] returns [`SignatureCheck`]: `Verified`, `Invalid`, or
//! `Held { scheme }` for a scheme this build does not implement. **Held is not Invalid**: the
//! object is neither authentic nor forged as far as this reader can tell, and it must confer no
//! effect (no custody, acknowledgement, consent or standing) while it may still be retained.
//! Scheme 0 with key material that is not an Ed25519 multikey is `Invalid`.
//!
//! The envelope's key is never the authority on who signed: callers compare it with the signer
//! they established independently ([`SignatureEnvelope::check_for`]).

use std::fmt;

use cid::Cid;
use ed25519_dalek::{Signature as EdSig, Signer, VerifyingKey};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::proof::AgentKeypair;

/// Multicodec of an Ed25519 public key (`ed`), varint-encoded as `ed 01`.
pub const ED25519_PUB_MULTICODEC: u64 = 0xed;
/// Length of an Ed25519 multikey: two varint bytes and the 32-byte key.
pub const ED25519_MULTIKEY_LEN: usize = 34;
/// Length of an Ed25519 signature.
pub const ED25519_SIGNATURE_LEN: usize = 64;

/// The signature schemes this build implements. A scheme code is an immutable, fully
/// specified profile; a new algorithm or composite is a new code.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignatureScheme {
    /// Strict Ed25519 (RFC 8032, `verify_strict`): multikey codec `0xed`, 32-byte key,
    /// 64-byte signature. Epoch 0.
    Ed25519 = 0,
}

impl SignatureScheme {
    /// The scheme's wire code.
    pub fn code(self) -> u16 {
        self as u16
    }

    /// The scheme a wire code names, if this build implements it.
    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            0 => Some(SignatureScheme::Ed25519),
            _ => None,
        }
    }
}

/// Why bytes or text are not a canonical multikey.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MultikeyError {
    /// No bytes at all.
    #[error("multikey is empty")]
    Empty,
    /// The codec varint runs past nine bytes or past the input.
    #[error("multikey codec varint is truncated or too long")]
    Varint,
    /// The codec varint has a redundant trailing zero group: a second spelling of one codec.
    #[error("multikey codec varint is not minimal")]
    NonMinimalVarint,
    /// A codec with no key bytes after it.
    #[error("multikey carries no key")]
    MissingKey,
    /// Text that is not `z` + base58btc.
    #[error("multikey text must be 'z' + base58btc")]
    Text,
}

/// One unsigned varint from the front of `bytes`, requiring minimal encoding.
fn minimal_varint(bytes: &[u8]) -> Result<(u64, usize), MultikeyError> {
    let mut value = 0u64;
    for (i, b) in bytes.iter().enumerate().take(9) {
        value |= u64::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            if i > 0 && *b == 0 {
                return Err(MultikeyError::NonMinimalVarint);
            }
            return Ok((value, i + 1));
        }
    }
    Err(MultikeyError::Varint)
}

/// A public key as `varint(codec) ‖ key bytes` — the multikey whose `z…` text is the did:key
/// method-specific identifier. Equality is by decoded codec and key; construction admits only
/// the minimal varint, so two spellings of one key cannot exist.
#[derive(Debug, Clone, Eq)]
pub struct Multikey(Vec<u8>);

impl Multikey {
    /// Admit canonical multikey bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Multikey, MultikeyError> {
        if bytes.is_empty() {
            return Err(MultikeyError::Empty);
        }
        let (_, n) = minimal_varint(&bytes)?;
        if n == bytes.len() {
            return Err(MultikeyError::MissingKey);
        }
        Ok(Multikey(bytes))
    }

    /// The Ed25519 multikey (`ed 01 ‖ key`).
    pub fn ed25519(public_key: &[u8; 32]) -> Multikey {
        let mut bytes = Vec::with_capacity(ED25519_MULTIKEY_LEN);
        bytes.extend_from_slice(&[0xed, 0x01]);
        bytes.extend_from_slice(public_key);
        Multikey(bytes)
    }

    /// Parse `z` + base58btc text (a did:key method-specific identifier).
    pub fn parse(text: &str) -> Result<Multikey, MultikeyError> {
        let body = text.strip_prefix('z').ok_or(MultikeyError::Text)?;
        let bytes = bs58::decode(body)
            .into_vec()
            .map_err(|_| MultikeyError::Text)?;
        Multikey::from_bytes(bytes)
    }

    /// The decoded codec.
    pub fn codec(&self) -> u64 {
        self.split().0
    }

    /// The key bytes after the codec.
    pub fn key_bytes(&self) -> &[u8] {
        self.split().1
    }

    /// The canonical bytes (`varint(codec) ‖ key`).
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The raw Ed25519 key, when this is an Ed25519 multikey of the right length.
    pub fn ed25519_key(&self) -> Option<[u8; 32]> {
        if self.codec() != ED25519_PUB_MULTICODEC {
            return None;
        }
        self.key_bytes().try_into().ok()
    }

    fn split(&self) -> (u64, &[u8]) {
        let (codec, n) = minimal_varint(&self.0).expect("a Multikey holds a minimal varint");
        (codec, &self.0[n..])
    }
}

impl PartialEq for Multikey {
    fn eq(&self, other: &Self) -> bool {
        self.split() == other.split()
    }
}

impl std::hash::Hash for Multikey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.split().hash(state);
    }
}

impl fmt::Display for Multikey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "z{}", bs58::encode(&self.0).into_string())
    }
}

/// The bytes a signer signs and a verifier checks:
/// `domain ‖ u16be(epoch) ‖ u16be(scheme) ‖ key ‖ body`.
pub fn signing_transcript(
    domain: &[u8],
    epoch: u16,
    scheme: u16,
    key: &[u8],
    body: &[u8],
) -> Vec<u8> {
    let mut t = Vec::with_capacity(domain.len() + 4 + key.len() + body.len());
    t.extend_from_slice(domain);
    t.extend_from_slice(&epoch.to_be_bytes());
    t.extend_from_slice(&scheme.to_be_bytes());
    t.extend_from_slice(key);
    t.extend_from_slice(body);
    t
}

/// The outcome of checking an envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureCheck {
    /// The signature holds over the transcript under a known scheme.
    Verified,
    /// A known scheme, and the signature, key material or signer does not hold.
    Invalid,
    /// A scheme this build does not implement: neither authentic nor forged here. Confers no
    /// effect; may be retained under a bounded policy.
    Held {
        /// The scheme code carried.
        scheme: u16,
    },
}

/// A signature with its scheme and the signer's key, on the wire as
/// `{ scheme: u16, key: bytes, sig: bytes }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct SignatureEnvelope {
    /// Scheme code ([`SignatureScheme`]); unknown codes are carried and checked as `Held`.
    pub scheme: u16,
    /// The signer's public key as canonical multikey bytes.
    #[serde(with = "serde_bytes")]
    #[ts(type = "Uint8Array")]
    pub key: Vec<u8>,
    /// The signature bytes.
    #[serde(with = "serde_bytes")]
    #[ts(type = "Uint8Array")]
    pub sig: Vec<u8>,
}

impl SignatureEnvelope {
    /// A scheme-0 envelope from an Ed25519 verifying key and signature.
    pub fn ed25519(vk: &[u8; 32], sig: [u8; 64]) -> Self {
        SignatureEnvelope {
            scheme: SignatureScheme::Ed25519.code(),
            key: Multikey::ed25519(vk).0,
            sig: sig.to_vec(),
        }
    }

    /// Sign `body` under `domain` and `epoch` with scheme 0.
    pub fn sign_ed25519(kp: &AgentKeypair, domain: &[u8], epoch: u16, body: &[u8]) -> Self {
        let key = Multikey::ed25519(&kp.public_key_bytes());
        let transcript = signing_transcript(
            domain,
            epoch,
            SignatureScheme::Ed25519.code(),
            key.as_bytes(),
            body,
        );
        let sig = kp.signing_key().sign(&transcript).to_bytes();
        SignatureEnvelope::ed25519(&kp.public_key_bytes(), sig)
    }

    /// This envelope's transcript for `body` under `domain` and `epoch`.
    pub fn transcript(&self, domain: &[u8], epoch: u16, body: &[u8]) -> Vec<u8> {
        signing_transcript(domain, epoch, self.scheme, &self.key, body)
    }

    /// The envelope's key as a [`Multikey`].
    pub fn multikey(&self) -> Result<Multikey, MultikeyError> {
        Multikey::from_bytes(self.key.clone())
    }

    /// Check the signature over `domain ‖ u16be(epoch) ‖ u16be(scheme) ‖ key ‖ body`.
    ///
    /// Unknown scheme → `Held`; scheme 0 with a key that is not a canonical 34-byte Ed25519
    /// multikey, a signature that is not 64 bytes, or a failing strict verification → `Invalid`.
    /// This proves only that the envelope's own key signed; use [`Self::check_for`] to bind
    /// the key to the signer the caller expects.
    pub fn check(&self, domain: &[u8], epoch: u16, body: &[u8]) -> SignatureCheck {
        match SignatureScheme::from_code(self.scheme) {
            None => SignatureCheck::Held {
                scheme: self.scheme,
            },
            Some(SignatureScheme::Ed25519) => {
                let Some(raw) = self
                    .multikey()
                    .ok()
                    .filter(|_| self.key.len() == ED25519_MULTIKEY_LEN)
                    .and_then(|k| k.ed25519_key())
                else {
                    return SignatureCheck::Invalid;
                };
                let (Ok(vk), Ok(sig)) =
                    (VerifyingKey::from_bytes(&raw), EdSig::from_slice(&self.sig))
                else {
                    return SignatureCheck::Invalid;
                };
                if vk
                    .verify_strict(&self.transcript(domain, epoch, body), &sig)
                    .is_ok()
                {
                    SignatureCheck::Verified
                } else {
                    SignatureCheck::Invalid
                }
            }
        }
    }

    /// [`Self::check`], additionally requiring the envelope's key to equal `expected` — the
    /// signer the caller established independently (from the claimed signer, never from the
    /// envelope). An unknown scheme stays `Held`; a key mismatch is `Invalid`.
    pub fn check_for(
        &self,
        expected: &Multikey,
        domain: &[u8],
        epoch: u16,
        body: &[u8],
    ) -> SignatureCheck {
        if SignatureScheme::from_code(self.scheme).is_none() {
            return SignatureCheck::Held {
                scheme: self.scheme,
            };
        }
        match self.multikey() {
            Ok(key) if key == *expected => self.check(domain, epoch, body),
            _ => SignatureCheck::Invalid,
        }
    }
}

/// A detached signature: who signed (the CID of the issuer's Agent EPR) and the envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct Signature {
    /// CID of the issuer's Agent EPR.
    #[ts(type = "string")]
    pub signer: Cid,
    /// Scheme, key and signature bytes.
    pub envelope: SignatureEnvelope,
}

impl Signature {
    /// A signature by `signer` carried in `envelope`.
    pub fn new(signer: Cid, envelope: SignatureEnvelope) -> Self {
        Signature { signer, envelope }
    }
}
