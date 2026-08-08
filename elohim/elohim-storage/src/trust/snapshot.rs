//! `HeadSetSnapshot` — signed transient head-set attestation (Path C, NO DHT
//! entry type). TYPES ONLY: this file declares the wire/domain shape; mint,
//! verify, and delta logic land in T8 (which also wires the
//! `epr_codec::encode_epr_head`-pattern CID and the wire-carry additive
//! fields).
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5.

use seam_contracts::ReasonLabel;

/// One entry in a [`HeadSetSnapshot`] — a content id paired with the
/// head-digest the signer attests to for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadSetEntry {
    pub id: String,
    pub head_digest: String,
}

/// An opaque epoch marker — a token, never a bare numeric type. Two reasons:
/// (1) consistency with [`crate::trust::memo::VerificationMemo`]'s "no
/// numeric field, ever" discipline (§4.2 derived-not-stored) — the same
/// `TrustEpoch` type is reused on both sides of the join so there is exactly
/// one place a regression check (T8/C2) can be written; (2) the clock this
/// token names is derived from the signer's attestation/citation edge set,
/// not an incrementing counter this crate owns or mints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrustEpoch(pub String);

/// A signed, transient attestation of a corpus's head set — evidence, not
/// authority (C5): a receiver re-derives everything it acts on rather than
/// trusting this snapshot's claims outright. Never a DHT entry type; carried
/// on the wire only (T8 adds the additive field).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadSetSnapshot {
    /// The L2 join key — a peer whose own `head_corpus_digest` equals this
    /// value is already in sync with the signer, at zero verification cost.
    pub corpus_digest: String,
    pub entries: Vec<HeadSetEntry>,
    pub signer_agent_cid: String,
    /// Named clock derived from the signer's attestation/citation edge set.
    /// Regression (an epoch that does not advance relative to the last one
    /// verified from this signer) REFUSES — see [`SnapshotRefusal::EpochRegression`].
    pub trust_epoch: TrustEpoch,
    pub edge_set_digest: String,
}

/// Why a snapshot verification refused to accept. Closed vocabulary per
/// `seam_contracts::ReasonLabel` — mint/verify logic that produces these
/// verdicts is T8; this task declares only the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotRefusal {
    /// The receiver's own corpus digest disagrees with the snapshot's
    /// declared digest — evidence-not-authority (C5): the snapshot does not
    /// describe this receiver's corpus.
    CorpusDigestMismatch,
    /// `trust_epoch` failed to advance relative to the last epoch verified
    /// from this signer (C2 — regression REFUSES, never silently accepted).
    EpochRegression,
    /// The signer could not be resolved or its signature could not be
    /// verified.
    UnknownSigner,
}

impl ReasonLabel for SnapshotRefusal {
    const ALL: &'static [Self] = &[
        SnapshotRefusal::CorpusDigestMismatch,
        SnapshotRefusal::EpochRegression,
        SnapshotRefusal::UnknownSigner,
    ];

    fn label(&self) -> &'static str {
        match self {
            SnapshotRefusal::CorpusDigestMismatch => "corpus_digest_mismatch",
            SnapshotRefusal::EpochRegression => "epoch_regression",
            SnapshotRefusal::UnknownSigner => "unknown_signer",
        }
    }
}

/// The outcome of verifying a [`HeadSetSnapshot`] against a receiver's local
/// state. `Accepted` carries nothing beyond the fact itself — the receiver
/// acts on ITS OWN re-derived state, never on the snapshot's claims (C5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotVerdict {
    Accepted,
    Refused(SnapshotRefusal),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_refusal_is_reason_label_conformant() {
        seam_contracts::assert_reason_labels_conformant::<SnapshotRefusal>();
        seam_contracts::assert_reason_labels_discriminating::<SnapshotRefusal>();
        seam_contracts::assert_reason_labels_stable::<SnapshotRefusal>(&[
            "corpus_digest_mismatch",
            "epoch_regression",
            "unknown_signer",
        ]);
    }

    #[test]
    fn verdict_refused_carries_the_reason() {
        let verdict = SnapshotVerdict::Refused(SnapshotRefusal::EpochRegression);
        assert_eq!(
            verdict,
            SnapshotVerdict::Refused(SnapshotRefusal::EpochRegression)
        );
        assert_ne!(verdict, SnapshotVerdict::Accepted);
    }

    #[test]
    fn trust_epoch_is_an_opaque_token_not_a_number() {
        // Type-level assertion: this compiles only because TrustEpoch wraps
        // a String, not an integer/float. If a future edit changes the
        // inner type to a numeric primitive, this construction — and the
        // "no numeric field, ever" discipline it stands in for — breaks
        // loudly at compile time instead of silently at review time.
        let epoch = TrustEpoch("edge-set-digest-derived-token".to_string());
        assert_eq!(epoch.0, "edge-set-digest-derived-token");
    }
}
