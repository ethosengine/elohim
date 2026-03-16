//! Governance health computation from stewardship allocation records.
//!
//! Derives a governance health signal (0.0-1.0) from the governance states
//! across a steward's allocation portfolio.

use crate::db::models::StewardshipAllocation;

/// Compute governance health from allocation governance states.
///
/// Algorithm:
/// - Empty allocations -> 0.5 (neutral — unknown steward)
/// - Score each allocation by governance_state:
///   active=1.0, superseded=0.8, pending_review=0.4, disputed=0.0, unknown=0.5
/// - Average across all allocations
/// - Clamp to [0.0, 1.0]
pub fn compute(allocations: &[StewardshipAllocation]) -> f64 {
    if allocations.is_empty() {
        return 0.5;
    }

    let total = allocations.len() as f64;

    let sum: f64 = allocations
        .iter()
        .map(|a| match a.governance_state.as_str() {
            "active" => 1.0,
            "superseded" => 0.8,
            "pending_review" => 0.4,
            "disputed" => 0.0,
            _ => 0.5,
        })
        .sum();

    (sum / total).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_allocation(governance_state: &str) -> StewardshipAllocation {
        StewardshipAllocation {
            id: "test-id".to_string(),
            app_id: "lamad".to_string(),
            content_id: "content-1".to_string(),
            steward_presence_id: "presence-1".to_string(),
            allocation_ratio: 1.0,
            allocation_method: "automatic".to_string(),
            contribution_type: "author".to_string(),
            contribution_evidence_json: None,
            governance_state: governance_state.to_string(),
            dispute_id: None,
            dispute_reason: None,
            disputed_at: None,
            disputed_by: None,
            negotiation_session_id: None,
            elohim_ratified_at: None,
            elohim_ratifier_id: None,
            effective_from: "2026-03-14 00:00:00".to_string(),
            effective_until: None,
            superseded_by: None,
            recognition_accumulated: 0.0,
            last_recognition_at: None,
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
            created_at: "2026-03-14 00:00:00".to_string(),
            updated_at: "2026-03-14 00:00:00".to_string(),
        }
    }

    #[test]
    fn empty_allocations_returns_neutral() {
        assert_eq!(compute(&[]), 0.5);
    }

    #[test]
    fn all_active_returns_high() {
        let allocations = vec![
            make_allocation("active"),
            make_allocation("active"),
            make_allocation("active"),
        ];
        let result = compute(&allocations);
        assert!(
            result > 0.8,
            "expected > 0.8 for all-active portfolio, got {result}"
        );
    }

    #[test]
    fn disputed_reduces_health() {
        let clean = vec![make_allocation("active")];
        let mixed = vec![make_allocation("active"), make_allocation("disputed")];
        assert!(
            compute(&clean) > compute(&mixed),
            "active-only should have higher health than active+disputed"
        );
    }

    #[test]
    fn all_disputed_returns_low() {
        let allocations = vec![make_allocation("disputed"), make_allocation("disputed")];
        let result = compute(&allocations);
        assert!(
            result < 0.2,
            "expected < 0.2 for all-disputed portfolio, got {result}"
        );
    }

    #[test]
    fn pending_review_is_mildly_unhealthy() {
        let active = compute(&[make_allocation("active")]);
        let pending = compute(&[make_allocation("pending_review")]);
        let disputed = compute(&[make_allocation("disputed")]);
        assert!(
            active > pending,
            "active ({active}) should be higher than pending_review ({pending})"
        );
        assert!(
            pending > disputed,
            "pending_review ({pending}) should be higher than disputed ({disputed})"
        );
    }
}
