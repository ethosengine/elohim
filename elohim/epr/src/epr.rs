//! Epr = Envelope + payload bytes.
//!
//! Construction flow: builder → canonical bytes → CID → sign → assemble.

use crate::{
    cid::compute_cid,
    envelope::Envelope,
    error::{EprError, Result},
    proof::{self, AgentKeypair},
    Coupling, EprKind, Reach, Signature,
};
use chrono::{DateTime, Utc};
use cid::Cid;
use serde::{Deserialize, Serialize};

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
    /// 3. Proof algorithm is "ed25519"
    /// 4. Signature bytes verify under the given public key
    pub fn verify_with_key(&self, public_key: &[u8; 32]) -> Result<()> {
        let canonical = self.envelope.canonical_bytes(&self.payload)?;
        let expected_cid = compute_cid(&canonical);
        if expected_cid != self.envelope.cid {
            return Err(EprError::InvalidCid {
                expected: expected_cid.to_string(),
                actual: self.envelope.cid.to_string(),
            });
        }
        if self.envelope.proof.algorithm != "ed25519" {
            return Err(EprError::Signature(format!(
                "unsupported algorithm: {}",
                self.envelope.proof.algorithm
            )));
        }
        if !proof::verify(public_key, &canonical, &self.envelope.proof.signature) {
            return Err(EprError::Signature("signature verification failed".into()));
        }
        Ok(())
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
            proof: Signature::ed25519(signer_cid, vec![0u8; 64]),
        };

        let canonical = provisional.canonical_bytes(&self.payload)?;
        let cid = compute_cid(&canonical);
        let sig_bytes = proof::sign(kp, &canonical);

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
            proof: Signature::ed25519_checked(signer_cid, sig_bytes)?,
        };

        Ok(Epr {
            envelope,
            payload: self.payload,
        })
    }
}
