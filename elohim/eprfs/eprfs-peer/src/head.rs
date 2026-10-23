//! A device's signed channel head: what it gossips on the steward topic (spec §3.3).
//!
//! `ChannelHead { device, root, seq, at, folded, signature }`. `seq` strictly increases per
//! device; `folded` is the device's acknowledgement vector, `{device → highest seq folded}`;
//! `at` is the announcing device's own clock in unix milliseconds, carried for display only (no
//! decision compares two devices' clocks). The signature is the device's did:key signature over
//! a domain tag and the canonical dag-cbor of every other field, `folded` included, so an
//! acknowledgement cannot be forged or altered in transit.

use std::collections::BTreeMap;

use elohim_epr_rea::SignatureVerifier;
use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};

use crate::error::PeerError;

/// The most bytes one head may take on the wire: iroh-gossip's default message cap. A head
/// acknowledging a few dozen devices fits.
pub const MAX_HEAD_BYTES: usize = 4096;

/// What signs a head: the device key's signature over a message. epr-cli supplies its device key;
/// this crate never holds one.
pub type HeadSigner = dyn Fn(&[u8]) -> Vec<u8> + Send + Sync;

/// One device's announcement of its channel root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelHead {
    /// The announcing device's did:key.
    pub device: String,
    /// The root of its channel tree.
    pub root: BlobCid,
    /// Strictly increasing per device.
    pub seq: u64,
    /// The announcing device's clock, unix milliseconds. Display only.
    pub at: u64,
    /// For each other device, the highest `seq` this device has folded from it.
    pub folded: BTreeMap<String, u64>,
    /// Lowercase hex of the device's ed25519 signature over [`Self::signing_message`].
    pub signature: String,
}

/// The signed body: every field but the signature, in canonical dag-cbor.
#[derive(Serialize)]
struct Body<'h> {
    device: &'h str,
    root: &'h BlobCid,
    seq: u64,
    at: u64,
    folded: &'h BTreeMap<String, u64>,
}

impl ChannelHead {
    /// The domain tag a head's signature covers, ahead of its body.
    pub const SIGNING_DOMAIN: &'static str = "elohim:steward-channel-head:v1:";

    /// Sign a head as `device` with `signer` (the device key behind `device`).
    ///
    /// # Errors
    ///
    /// [`PeerError::HeadFrame`] when the body does not encode.
    pub fn sign(
        device: String,
        root: BlobCid,
        seq: u64,
        at: u64,
        folded: BTreeMap<String, u64>,
        signer: &HeadSigner,
    ) -> Result<ChannelHead, PeerError> {
        let mut head = ChannelHead {
            device,
            root,
            seq,
            at,
            folded,
            signature: String::new(),
        };
        head.signature = hex(&signer(&head.signing_message()?));
        Ok(head)
    }

    /// The bytes the signature covers: [`Self::SIGNING_DOMAIN`], then the canonical dag-cbor of
    /// `{device, root, seq, at, folded}`.
    ///
    /// # Errors
    ///
    /// [`PeerError::HeadFrame`] when the body does not encode.
    pub fn signing_message(&self) -> Result<Vec<u8>, PeerError> {
        let body = serde_ipld_dagcbor::to_vec(&Body {
            device: &self.device,
            root: &self.root,
            seq: self.seq,
            at: self.at,
            folded: &self.folded,
        })
        .map_err(|e| frame(e.to_string()))?;
        let mut message = Self::SIGNING_DOMAIN.as_bytes().to_vec();
        message.extend_from_slice(&body);
        Ok(message)
    }

    /// Check that this head is `device_did`'s, signed by the key behind it.
    ///
    /// # Errors
    ///
    /// [`PeerError::HeadSignature`] when the head names another device, its signature is not
    /// hex, or it does not verify; [`PeerError::HeadFrame`] when the body does not encode.
    pub fn verify(
        &self,
        device_did: &str,
        verifier: &dyn SignatureVerifier,
    ) -> Result<(), PeerError> {
        let refused = || PeerError::HeadSignature {
            device: device_did.to_string(),
        };
        if self.device != device_did {
            return Err(refused());
        }
        let signature = unhex(&self.signature).ok_or_else(refused)?;
        if verifier.verify(device_did, &self.signing_message()?, &signature) {
            Ok(())
        } else {
            Err(refused())
        }
    }

    /// The head as one gossip message: canonical dag-cbor, at most [`MAX_HEAD_BYTES`].
    ///
    /// # Errors
    ///
    /// [`PeerError::HeadFrame`] when it does not encode or exceeds the cap.
    pub fn to_wire(&self) -> Result<Vec<u8>, PeerError> {
        let bytes = serde_ipld_dagcbor::to_vec(self).map_err(|e| frame(e.to_string()))?;
        if bytes.len() > MAX_HEAD_BYTES {
            return Err(frame(format!(
                "a head of {} bytes exceeds the {MAX_HEAD_BYTES}-byte cap",
                bytes.len()
            )));
        }
        Ok(bytes)
    }

    /// A head read from one gossip message. Decoding is not verifying: the caller still checks
    /// the signature against the admitted device.
    ///
    /// # Errors
    ///
    /// [`PeerError::HeadFrame`] when it exceeds the cap or does not decode.
    pub fn from_wire(bytes: &[u8]) -> Result<ChannelHead, PeerError> {
        if bytes.len() > MAX_HEAD_BYTES {
            return Err(frame(format!(
                "a head of {} bytes exceeds the {MAX_HEAD_BYTES}-byte cap",
                bytes.len()
            )));
        }
        serde_ipld_dagcbor::from_slice(bytes).map_err(|e| frame(e.to_string()))
    }
}

fn frame(reason: String) -> PeerError {
    PeerError::HeadFrame { reason }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| {
            text.get(i..i + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
        })
        .collect()
}
