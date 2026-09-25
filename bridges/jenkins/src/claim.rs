//! The general external-claim envelope — how anything from OUTSIDE the network enters it.
//!
//! Jenkins is the first user; Requests & Offers (hREA/VF via `bridges/valueflows`) and elohim
//! agents authoring outside a peer runtime are the same shape later. The rules the type carries:
//!
//! - **Legible without permission.** A claim is a spec plus a CID; anyone may read it.
//! - **No standing travels in.** The source is provenance ([`ParticipantRef::Service`] today), never
//!   an actor: it cannot claim, author or attest, and it is never a payee.
//! - **Evaluated on entry, at the lowest reach.** Every claim enters at [`Reach::Private`] and
//!   takes up governance attention only when a participant with standing brings it forward.
//! - **Scoped to the governance that allowed it.** `in_scope_of` is the governing offer's CID, so
//!   every flow traces to the offer a distinct Steward approved.
//!
//! The envelope rides on each event's `classified_as` as prefixed slots AFTER the established
//! tag-then-subject pair, so no existing reader's positional read moves.

use cid::Cid;
use elohim_epr::reach::Reach;
use elohim_epr_rea::{parse_participant_ref, ParticipantRef};

/// Whether the source signed what it claims. `unattested` is honest absence, never a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureStatus {
    /// Signed by the named `did:key` (verification is the reader's, on entry).
    Signed { did: String },
    /// Nothing signed it — the Jenkins archive today.
    Unattested,
}

impl SignatureStatus {
    pub fn slot_value(&self) -> String {
        match self {
            SignatureStatus::Signed { did } => format!("signed:{did}"),
            SignatureStatus::Unattested => "unattested".to_string(),
        }
    }
}

/// The external-claim envelope every observation from outside carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalClaim {
    /// The provenance participant, `service:<name>`.
    source: String,
    /// Where the claim can be re-read at its source (a URL or archive path).
    pub provenance: String,
    pub signature: SignatureStatus,
    /// The governing offer's Intent CID.
    pub in_scope_of: Cid,
}

impl ExternalClaim {
    /// The sole constructor: the source must parse as a SERVICE — a human or agent acting from
    /// outside is a different entry (an authored act under its own claim), not this envelope.
    pub fn new(
        source: &str,
        provenance: &str,
        signature: SignatureStatus,
        in_scope_of: Cid,
    ) -> Result<Self, String> {
        match parse_participant_ref(source).map_err(|e| e.to_string())? {
            ParticipantRef::Service { .. } => {}
            _ => {
                return Err(format!(
                    "external-claim source `{source}` is not a `service:<name>` — this envelope \
                     carries provenance only"
                ))
            }
        }
        if provenance.trim().is_empty() {
            return Err("an external claim needs provenance: where it can be re-read".into());
        }
        Ok(Self {
            source: source.to_string(),
            provenance: provenance.to_string(),
            signature,
            in_scope_of,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// The reach an external claim enters at: always the lowest. Standing is earned inside.
    pub fn reach(&self) -> Reach {
        Reach::Private
    }

    /// The envelope's prefixed `classified_as` slots, in a fixed order.
    pub fn slots(&self) -> Vec<String> {
        let reach = serde_json::to_value(self.reach())
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "private".into());
        vec![
            format!("source:{}", self.source),
            format!("provenance:{}", self.provenance),
            format!("signature:{}", self.signature.slot_value()),
            format!("reach:{reach}"),
            format!("in-scope-of:{}", self.in_scope_of),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> Cid {
        elohim_epr::cid::compute_cid(b"offer")
    }

    #[test]
    fn only_a_service_can_be_an_external_claim_source() {
        assert!(
            ExternalClaim::new("service:jenkins", "u", SignatureStatus::Unattested, scope())
                .is_ok()
        );
        for actor in [
            "human:matthew",
            "agent:scribe@opus-5",
            "repo:ethosengine/elohim",
        ] {
            assert!(
                ExternalClaim::new(actor, "u", SignatureStatus::Unattested, scope()).is_err(),
                "{actor} must not enter as an external claim source"
            );
        }
        assert!(
            ExternalClaim::new("service:jenkins", " ", SignatureStatus::Unattested, scope())
                .is_err()
        );
    }

    #[test]
    fn a_claim_enters_at_the_lowest_reach_with_its_signature_status() {
        let claim = ExternalClaim::new(
            "service:jenkins",
            "https://x/1/",
            SignatureStatus::Signed {
                did: "did:key:z6Mk".into(),
            },
            scope(),
        )
        .unwrap();
        assert_eq!(claim.reach().openness(), 1);
        let slots = claim.slots();
        assert!(slots.contains(&"reach:private".to_string()));
        assert!(slots.contains(&"signature:signed:did:key:z6Mk".to_string()));
        assert!(slots.contains(&format!("in-scope-of:{}", scope())));
    }
}
