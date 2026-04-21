//! Signature struct — detached proof on the EPR canonical bytes.

use crate::error::{EprError, Result};
use cid::Cid;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub struct Signature {
    /// CID of the issuer's Agent EPR.
    #[ts(type = "string")]
    pub signer: Cid,
    /// Signing algorithm identifier.
    pub algorithm: String,
    /// Raw signature bytes (64 bytes for Ed25519).
    #[serde(with = "serde_bytes")]
    #[ts(type = "Uint8Array")]
    pub signature: Vec<u8>,
}

impl Signature {
    pub fn ed25519(signer: Cid, signature: Vec<u8>) -> Self {
        Self { signer, algorithm: "ed25519".into(), signature }
    }

    pub fn ed25519_checked(signer: Cid, signature: Vec<u8>) -> Result<Self> {
        if signature.len() != 64 {
            return Err(EprError::Signature(format!(
                "ed25519 signature must be 64 bytes, got {}",
                signature.len()
            )));
        }
        Ok(Self::ed25519(signer, signature))
    }
}
