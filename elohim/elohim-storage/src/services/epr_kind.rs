//! Shared EprKind canonical string helpers and EPR-related domain types for elohim-storage.
//!
//! Also declares [`Reach`] — the reach-scope enum used by the reach-earning gate
//! (Phase 3.5 Light-Up-Graph).
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
        EprKind::AttentionTending => "AttentionTending",
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
// Reach — EPR distribution scope
// ---------------------------------------------------------------------------

/// Distribution scope for an EPR. Used by the reach-earning gate to determine
/// whether an author's standing is sufficient to compose at the requested scope.
///
/// The eight variants map to the `reachThresholds` keys in the standing-policy
/// manifest. Personal/Intimate/Household/Neighborhood map to "any" (floor class;
/// bypass the standing check). The remaining four require minimum standing.
///
/// See: genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md §Components::ReachVerdict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reach {
    Personal,
    Intimate,
    Household,
    Neighborhood,
    Collective,
    Community,
    District,
    Public,
}

impl Reach {
    /// Returns `true` when the manifest's `reachThresholds` maps this reach to
    /// `"any"` — i.e. these reach values bypass standing/floor checks (CID-targeted-
    /// lookup and local-relationship-reach floor classes).
    pub fn is_floor_allowed(self) -> bool {
        matches!(
            self,
            Reach::Personal | Reach::Intimate | Reach::Household | Reach::Neighborhood
        )
    }

    /// Returns the kebab-case identifier matching the manifest key.
    pub fn as_kebab(self) -> &'static str {
        match self {
            Reach::Personal => "personal",
            Reach::Intimate => "intimate",
            Reach::Household => "household",
            Reach::Neighborhood => "neighborhood",
            Reach::Collective => "collective",
            Reach::Community => "community",
            Reach::District => "district",
            Reach::Public => "public",
        }
    }
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
            (EprKind::AttentionTending, "AttentionTending"),
        ];

        for (kind, expected) in cases {
            let got = kind_canonical_str(*kind);
            assert_eq!(
                got, *expected,
                "kind_canonical_str({kind:?}) returned {got:?}, expected {expected:?}"
            );
        }

        // Also assert we covered all 11 variants (update when enum grows).
        assert_eq!(cases.len(), 11, "expected 11 EprKind variants");
    }

    // T11 — Reach::is_floor_allowed tests
    #[test]
    fn floor_reaches_bypass() {
        assert!(Reach::Personal.is_floor_allowed());
        assert!(Reach::Intimate.is_floor_allowed());
        assert!(Reach::Household.is_floor_allowed());
        assert!(Reach::Neighborhood.is_floor_allowed());
    }

    #[test]
    fn non_floor_reaches_do_not_bypass() {
        assert!(!Reach::Collective.is_floor_allowed());
        assert!(!Reach::Community.is_floor_allowed());
        assert!(!Reach::District.is_floor_allowed());
        assert!(!Reach::Public.is_floor_allowed());
    }
}
