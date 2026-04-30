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
        EprKind::FeedbackSignal => "FeedbackSignal",
    }
}

/// Resolve pillar for an EPR kind via the [`ManifestRegistry`], with a
/// bootstrap fallback to the lowercased canonical kind name when no
/// pillar-projection manifest has been registered yet.
///
/// `standing` is wired through to the registry; Phase 3 returns the same
/// pillar regardless (registry is signal-agnostic). Phase 3.5 lights up
/// gradient-modulated registry lookups.
///
/// The bootstrap fallback preserves the pre-Phase-3 subscriber convention
/// (subscribers match on the lowercased kind name) so tests and existing
/// flows continue to work until pillar-projection manifests are seeded.
pub(crate) fn pillar_for_kind(
    kind: elohim_epr::EprKind,
    registry: &crate::services::manifest_registry::ManifestRegistry,
    standing: crate::services::standing::Standing,
) -> String {
    if let Some(pillar) = registry.pillar_for_kind(kind, standing) {
        return pillar;
    }
    kind_canonical_str(kind).to_lowercase()
}

/// Provisional pillar lookup for an EPR kind.
///
/// Retained as a `#[deprecated]` thin wrapper around the kind→lowercase
/// canonical-name fallback for any callers still in transition. Phase 3.5
/// removes it entirely.
#[deprecated(
    since = "0.3.0",
    note = "use pillar_for_kind with a ManifestRegistry; falls back to the same behavior when registry is empty"
)]
#[allow(dead_code)]
pub(crate) fn pillar_for_kind_provisional(kind: elohim_epr::EprKind) -> String {
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
            (EprKind::FeedbackSignal, "FeedbackSignal"),
        ];

        for (kind, expected) in cases {
            let got = kind_canonical_str(*kind);
            assert_eq!(
                got, *expected,
                "kind_canonical_str({kind:?}) returned {got:?}, expected {expected:?}"
            );
        }

        // Also assert we covered all 10 variants (update when enum grows).
        assert_eq!(cases.len(), 10, "expected 10 EprKind variants");
    }
}
