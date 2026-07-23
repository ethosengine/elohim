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
        EprKind::WitnessedInteraction => "WitnessedInteraction",
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

/// Canonical declared-reach vocabulary — re-exported from the protocol crate.
/// The schema (elohim/sdk/schemas/v1/enums/reach.schema.json) is the source of
/// record; the local kebab-8 (personal…district/public) is RETIRED
/// (spec: reach-ontology-vocabulary-split §1). Legacy manifest keys parse via
/// `parse_reach_key` and map per the spec's pinned table.
pub use elohim_epr::Reach;

/// Standing/floor semantics for the standing-policy manifest's `reachThresholds`.
pub trait ReachStandingExt {
    /// `true` ⇒ manifest floor class "any": bypasses the standing check
    /// (CID-targeted lookup + local-relationship floor classes).
    fn is_floor_allowed(&self) -> bool;
    /// Canonical manifest key (the serde/schema string).
    fn as_manifest_key(&self) -> &'static str;
}

impl ReachStandingExt for Reach {
    fn is_floor_allowed(&self) -> bool {
        matches!(
            self,
            Reach::Private | Reach::SelfScope | Reach::Intimate | Reach::Trusted | Reach::Familiar
        )
    }
    fn as_manifest_key(&self) -> &'static str {
        match self {
            Reach::Private => "private",
            Reach::SelfScope => "self",
            Reach::Intimate => "intimate",
            Reach::Trusted => "trusted",
            Reach::Familiar => "familiar",
            Reach::Community => "community",
            Reach::Public => "public",
            Reach::Commons => "commons",
        }
    }
}

/// Parse a manifest/config reach key: canonical schema-8 keys plus the retired
/// kebab-8 legacy aliases (data-aware migration — spec §7.5: no value removed
/// while live rows/manifests still carry it).
pub fn parse_reach_key(key: &str) -> Option<Reach> {
    Some(match key {
        "private" => Reach::Private,
        "self" | "personal" => Reach::SelfScope,
        "intimate" => Reach::Intimate,
        "trusted" | "household" => Reach::Trusted,
        "familiar" | "neighborhood" => Reach::Familiar,
        "community" | "collective" => Reach::Community,
        // legacy "district" sat below legacy "public"(top). district→public.
        "public" | "district" => Reach::Public,
        "commons" => Reach::Commons,
        _ => return None,
    })
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
            (EprKind::WitnessedInteraction, "WitnessedInteraction"),
        ];

        for (kind, expected) in cases {
            let got = kind_canonical_str(*kind);
            assert_eq!(
                got, *expected,
                "kind_canonical_str({kind:?}) returned {got:?}, expected {expected:?}"
            );
        }

        // Also assert we covered all 12 variants (update when enum grows).
        assert_eq!(cases.len(), 12, "expected 12 EprKind variants");
    }

    #[test]
    fn canonical_reach_floor_split() {
        use super::ReachStandingExt;
        for r in [
            Reach::Private,
            Reach::SelfScope,
            Reach::Intimate,
            Reach::Trusted,
            Reach::Familiar,
        ] {
            assert!(r.is_floor_allowed(), "{r:?} must be floor-allowed");
        }
        for r in [Reach::Community, Reach::Public, Reach::Commons] {
            assert!(!r.is_floor_allowed(), "{r:?} must require standing");
        }
    }

    #[test]
    fn legacy_manifest_keys_still_parse() {
        // Data-aware migration (spec §7.5): live standing-policy manifests may
        // still carry the retired kebab-8 keys. They map, never 404.
        for (legacy, canonical) in [
            ("personal", Reach::SelfScope),
            ("household", Reach::Trusted),
            ("neighborhood", Reach::Familiar),
            ("collective", Reach::Community),
            ("district", Reach::Public),
        ] {
            assert_eq!(parse_reach_key(legacy), Some(canonical));
        }
        // Legacy "public" meant the top rung → commons; canonical "public" stays public.
        assert_eq!(parse_reach_key("commons"), Some(Reach::Commons));
        assert_eq!(parse_reach_key("self"), Some(Reach::SelfScope));
        assert_eq!(parse_reach_key("nonsense"), None);
    }
}
