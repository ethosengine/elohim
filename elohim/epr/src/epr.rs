//! Epr = Envelope + payload bytes.
//!
//! Construction flow: builder → canonical bytes → CID → sign → assemble.

use crate::{
    cid::compute_cid,
    envelope::Envelope,
    error::{EprError, Result},
    proof::AgentKeypair,
    signature::{Multikey, SignatureCheck, SignatureEnvelope},
    Coupling, EprKind, Reach, Signature,
};

/// The domain an EPR envelope's signature is made under.
pub const EPR_SIGNATURE_DOMAIN: &[u8] = b"elohim:epr:v1:";
/// The EPR envelope's signature epoch: the transcript rule of [`crate::signature`] at epoch 0.
pub const EPR_SIGNATURE_EPOCH: u16 = 0;
use chrono::{DateTime, Utc};
use cid::Cid;
use serde::{Deserialize, Serialize};

/// The exact bytes an Ed25519 signer of an EPR envelope signs: the scheme-0 transcript (v1)
/// `000e ‖ EPR_SIGNATURE_DOMAIN ‖ u16be(EPR_SIGNATURE_EPOCH) ‖ 0000 ‖ 0022 ‖ (ed 01 ‖ public_key)
/// ‖ canonical` ([`crate::signature::signing_transcript`]).
/// For signers that hold the key elsewhere (a conductor's keystore): sign these bytes, then
/// carry the result as `SignatureEnvelope::ed25519(public_key, sig)`.
pub fn epr_signing_transcript(public_key: &[u8; 32], canonical: &[u8]) -> Vec<u8> {
    crate::signature::signing_transcript(
        EPR_SIGNATURE_DOMAIN,
        EPR_SIGNATURE_EPOCH,
        crate::signature::SignatureScheme::Ed25519.code(),
        Multikey::ed25519(public_key).as_bytes(),
        canonical,
    )
    .expect("the EPR domain and a 34-byte Ed25519 multikey frame in transcript v1")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Epr {
    pub envelope: Envelope,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl Epr {
    pub fn builder() -> EprBuilder {
        EprBuilder::default()
    }

    /// Verify this Epr against a public key.
    ///
    /// Checks (in order):
    /// 1. Re-derive canonical bytes from envelope + payload
    /// 2. CID matches canonical-bytes hash
    /// 3. The proof's scheme is implemented (else [`EprError::SignatureHeld`])
    /// 4. The proof's key is `public_key` and its signature holds over the transcript
    ///    of `(EPR_SIGNATURE_DOMAIN, EPR_SIGNATURE_EPOCH, scheme, key, canonical)` (transcript v1,
    ///    [`crate::signature::signing_transcript`])
    pub fn verify_with_key(&self, public_key: &[u8; 32]) -> Result<()> {
        let canonical = self.envelope.canonical_bytes(&self.payload)?;
        let expected_cid = compute_cid(&canonical);
        if expected_cid != self.envelope.cid {
            return Err(EprError::InvalidCid {
                expected: expected_cid.to_string(),
                actual: self.envelope.cid.to_string(),
            });
        }
        match self.envelope.proof.envelope.check_for(
            &Multikey::ed25519(public_key),
            EPR_SIGNATURE_DOMAIN,
            EPR_SIGNATURE_EPOCH,
            &canonical,
        ) {
            SignatureCheck::Verified => Ok(()),
            SignatureCheck::Held { scheme } => Err(EprError::SignatureHeld(scheme)),
            SignatureCheck::Invalid => {
                Err(EprError::Signature("signature verification failed".into()))
            }
        }
    }
}

#[derive(Default)]
pub struct EprBuilder {
    kind: Option<EprKind>,
    schema_ref: Option<Cid>,
    schema_key: Option<String>,
    reach: Option<Reach>,
    coupling: Coupling,
    claims: Vec<Cid>,
    supersedes: Option<Cid>,
    issued_at: Option<DateTime<Utc>>,
    payload: Vec<u8>,
}

impl EprBuilder {
    pub fn kind(mut self, k: EprKind) -> Self {
        self.kind = Some(k);
        self
    }
    pub fn schema_ref(mut self, c: Cid) -> Self {
        self.schema_ref = Some(c);
        self
    }
    pub fn schema_key<S: Into<String>>(mut self, s: S) -> Self {
        self.schema_key = Some(s.into());
        self
    }
    pub fn reach(mut self, r: Reach) -> Self {
        self.reach = Some(r);
        self
    }
    pub fn coupling(mut self, c: Coupling) -> Self {
        self.coupling = c;
        self
    }
    pub fn claim(mut self, c: Cid) -> Self {
        self.claims.push(c);
        self
    }
    pub fn claims(mut self, c: Vec<Cid>) -> Self {
        self.claims = c;
        self
    }
    pub fn supersedes(mut self, c: Cid) -> Self {
        self.supersedes = Some(c);
        self
    }
    pub fn issued_at(mut self, t: DateTime<Utc>) -> Self {
        self.issued_at = Some(t);
        self
    }
    pub fn payload(mut self, p: Vec<u8>) -> Self {
        self.payload = p;
        self
    }

    /// Assemble canonical bytes, derive CID, sign, and return a complete Epr.
    pub fn sign(self, kp: &AgentKeypair, signer_cid: Cid) -> Result<Epr> {
        let kind = self
            .kind
            .ok_or_else(|| EprError::InvalidEnvelope("kind required".into()))?;
        let schema_ref = self
            .schema_ref
            .ok_or_else(|| EprError::InvalidEnvelope("schema_ref required".into()))?;
        let schema_key = self
            .schema_key
            .ok_or_else(|| EprError::InvalidEnvelope("schema_key required".into()))?;
        let reach = self
            .reach
            .ok_or_else(|| EprError::InvalidEnvelope("reach required".into()))?;
        let issued_at = self
            .issued_at
            .ok_or_else(|| EprError::InvalidEnvelope("issued_at required".into()))?;

        // Build provisional envelope for canonical-bytes derivation.
        // cid and proof are placeholders (not included in canonical bytes anyway).
        let provisional = Envelope {
            cid: compute_cid(&[0]),
            kind,
            schema_ref,
            schema_key: schema_key.clone(),
            reach,
            coupling: self.coupling.clone(),
            claims: self.claims.clone(),
            supersedes: self.supersedes,
            superseded_by: None,
            issued_at,
            proof: Signature::new(
                signer_cid,
                SignatureEnvelope::ed25519(&kp.public_key_bytes(), [0u8; 64]),
            ),
        };

        let canonical = provisional.canonical_bytes(&self.payload)?;
        let cid = compute_cid(&canonical);
        let envelope_sig = SignatureEnvelope::sign_ed25519(
            kp,
            EPR_SIGNATURE_DOMAIN,
            EPR_SIGNATURE_EPOCH,
            &canonical,
        );

        let envelope = Envelope {
            cid,
            kind,
            schema_ref,
            schema_key,
            reach,
            coupling: self.coupling,
            claims: self.claims,
            supersedes: self.supersedes,
            superseded_by: None,
            issued_at,
            proof: Signature::new(signer_cid, envelope_sig),
        };

        Ok(Epr {
            envelope,
            payload: self.payload,
        })
    }
}
