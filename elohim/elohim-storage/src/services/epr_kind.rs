//! Shared EprKind canonical string helpers for elohim-storage.
//!
//! `kind_canonical_str` is the single in-crate source of truth for
//! `EprKind → &'static str` mapping.  It must be kept in sync with the
//! private `kind_canonical` function in `elohim/epr/src/envelope.rs` (which
//! lives in a separate crate and returns `String` rather than `&'static str`;
//! a cross-crate API change is not worth the coupling).
//!
//! If a new `EprKind` variant is added, the compiler will flag the `match` here
//! (non-exhaustive), and the test below will catch any string drift.

use elohim_epr::EprKind;

/// Map an [`EprKind`] to its canonical protocol string.
///
/// The returned strings are load-bearing — they appear in Envelope headers,
/// fanout routing keys, and SQLite projections.  Do **not** change them
/// without a coordinated protocol version bump.
pub(crate) fn kind_canonical_str(k: EprKind) -> &'static str {
    match k {
        EprKind::Content => "Content",
        EprKind::Agent => "Agent",
        EprKind::Manifest => "Manifest",
        EprKind::Claim => "Claim",
        EprKind::Observation => "Observation",
        EprKind::EconomicEvent => "EconomicEvent",
        EprKind::Commitment => "Commitment",
        EprKind::Attestation => "Attestation",
        EprKind::Delegation => "Delegation",
    }
}

/// Provisional pillar lookup for an EPR kind.
///
/// FIXME(phase-3): replace with a `ManifestRegistry` lookup once Phase 3's
/// Manifest-EPR resolver exists. Until then, return the kind name lowercased
/// — sufficient for D.8's integration tests because subscribers match on the
/// same provisional name. **Subscribers written against this naming MUST be
/// updated when Phase 3 lands.** Find this function by grepping for
/// `pillar_for_kind_provisional` or `FIXME(phase-3)`.
pub(crate) fn pillar_for_kind_provisional(kind: EprKind) -> String {
    kind_canonical_str(kind).to_lowercase()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Variant-coverage guard: every EprKind must round-trip through
    /// `kind_canonical_str` and return a non-empty, expected string.
    /// Add a row here whenever a new variant is introduced so drift is caught
    /// before it reaches a running node.
    #[test]
    fn kind_canonical_str_covers_all_variants() {
        let cases: &[(EprKind, &str)] = &[
            (EprKind::Content, "Content"),
            (EprKind::Agent, "Agent"),
            (EprKind::Manifest, "Manifest"),
            (EprKind::Claim, "Claim"),
            (EprKind::Observation, "Observation"),
            (EprKind::EconomicEvent, "EconomicEvent"),
            (EprKind::Commitment, "Commitment"),
            (EprKind::Attestation, "Attestation"),
            (EprKind::Delegation, "Delegation"),
        ];

        for (kind, expected) in cases {
            let got = kind_canonical_str(*kind);
            assert_eq!(
                got, *expected,
                "kind_canonical_str({kind:?}) returned {got:?}, expected {expected:?}"
            );
        }

        // Also assert we covered all 9 variants (update when enum grows).
        assert_eq!(cases.len(), 9, "expected 9 EprKind variants");
    }
}
