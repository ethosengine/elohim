//! Floor protections — non-negotiable minimums that cannot be eroded by the
//! standing gradient. These are mishpat-DNA-notarized in Phase 3.5; Phase 3
//! ships with the predicate scaffolding so gradient-modulated paths bypass
//! their normal logic when the floor applies.
//!
//! See: brainstorm §2.8 (constitutional floors) and §3.2 (per-layer floor protection column).

use elohim_epr::EprKind;

/// Constitutional kinds — full per-message validation, never amortized.
/// Phase 3.5 expands this list when mishpat-DNA-notarized rules land.
pub fn is_constitutional_kind(kind: EprKind) -> bool {
    matches!(
        kind,
        EprKind::Manifest | EprKind::Attestation | EprKind::Delegation
    )
}

/// Protocol-load-bearing schemaRef types — DNA-notarized manifest schemas
/// always resolvable at full depth, regardless of standing arg.
pub fn is_protocol_load_bearing_schemaref(kind: EprKind) -> bool {
    matches!(kind, EprKind::Manifest)
}

/// Reach floor — local relationship reach is unconditional. Phase 3.5
/// lights up the topology check; Phase 3 placeholder treats `Reach::Private`
/// as the local-relationship indicator.
pub fn is_local_relationship_reach(reach: &elohim_epr::Reach) -> bool {
    matches!(reach, elohim_epr::Reach::Private)
}

/// The [`crate::trust::pricer::FloorClass`] that applies to ONE content-head
/// adoption decision, derived from the local projection row's `reach`.
///
/// `None` means **unclassifiable** — the row's reach string does not parse
/// through the canonical [`crate::services::epr_kind::parse_reach_key`]. The
/// caller must then decline to price the id at all (full verification), because
/// a floor that cannot be evaluated is not a floor that can be shown absent.
/// Never invent a `FloorClass::None` for an unreadable reach: that is exactly
/// the fail-open the typed `Reach` on `PricingInput` exists to prevent.
///
/// Which floors are reachable from the content head plane:
///
/// - **LocalRelationship** — REAL and reachable: a `reach = "private"` row.
///   [`is_local_relationship_reach`] is the single predicate, reused not
///   restated.
/// - **Constitutional** — structurally NOT reachable here. Manifests,
///   attestations and delegations are not `content` rows (manifests project to
///   the `manifests` table; the head plane reconciles `content`), so there is no
///   honest way for this classifier to emit it. The pricer still refuses to
///   cheapen it — proven over the full product space by
///   `trust::pricer`'s keystone property test — so a future arm that DOES
///   supply it is protected without changing anything here.
/// - **CounterEvidence** — likewise not a head-adoption class (it is a
///   feedback-signal class); same protection story.
pub fn content_head_floor_class(reach: &str) -> Option<crate::trust::pricer::FloorClass> {
    use crate::trust::pricer::FloorClass;
    let parsed = crate::services::epr_kind::parse_reach_key(reach)?;
    Some(if is_local_relationship_reach(&parsed) {
        FloorClass::LocalRelationship
    } else {
        FloorClass::None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::{EprKind, Reach};

    #[test]
    fn manifest_is_constitutional() {
        assert!(is_constitutional_kind(EprKind::Manifest));
    }

    #[test]
    fn content_is_not_constitutional() {
        assert!(!is_constitutional_kind(EprKind::Content));
    }

    #[test]
    fn manifest_is_protocol_load_bearing_schemaref() {
        assert!(is_protocol_load_bearing_schemaref(EprKind::Manifest));
    }

    #[test]
    fn agent_is_not_protocol_load_bearing_schemaref() {
        assert!(!is_protocol_load_bearing_schemaref(EprKind::Agent));
    }

    #[test]
    fn private_reach_is_local_relationship() {
        assert!(is_local_relationship_reach(&Reach::Private));
    }

    #[test]
    fn commons_reach_is_not_local_relationship() {
        assert!(!is_local_relationship_reach(&Reach::Commons));
    }
}
