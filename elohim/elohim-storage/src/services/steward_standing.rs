//! Steward standing computation from stewardship allocation records.
//!
//! Aggregates StewardshipAllocation records into a single steward_standing
//! signal (0.0-1.0) reflecting portfolio health: active ratio, recognition
//! accumulation, and dispute penalties.

use crate::db::models::StewardshipAllocation;

/// Compute steward standing from allocation records.
///
/// Algorithm:
/// - Empty allocations -> 0.5 (neutral — unknown steward)
/// - Base (60% weight): ratio of active allocations to total
/// - Recognition boost (30% weight): log-scaled total recognition across active allocations
/// - Dispute penalty (10% weight): count of disputed allocations x 0.1, capped at 1.0
/// - Final: (active_ratio x 0.6) + (recognition_boost x 0.3) - (dispute_penalty x 0.1)
/// - Clamp to [0.0, 1.0]
pub fn compute(allocations: &[StewardshipAllocation]) -> f64 {
    if allocations.is_empty() {
        return 0.5;
    }

    let total = allocations.len() as f64;

    let active_count = allocations
        .iter()
        .filter(|a| a.governance_state == "active")
        .count() as f64;
    let active_ratio = active_count / total;

    let total_recognition: f64 = allocations
        .iter()
        .filter(|a| a.governance_state == "active")
        .map(|a| a.recognition_accumulated as f64)
        .sum();
    let recognition_boost = ((1.0 + total_recognition).ln() / (1.0 + 1000.0_f64).ln()).min(1.0);

    let disputed_count = allocations
        .iter()
        .filter(|a| a.governance_state == "disputed")
        .count() as f64;
    let dispute_penalty = (disputed_count * 0.1).min(1.0);

    let score = (active_ratio * 0.6) + (recognition_boost * 0.3) - (dispute_penalty * 0.1);
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_allocation(governance_state: &str, recognition: f32) -> StewardshipAllocation {
        StewardshipAllocation {
            id: "test-id".to_string(),
            h_app_id: "lamad".to_string(),
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
            recognition_accumulated: recognition,
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
            make_allocation("active", 50.0),
            make_allocation("active", 50.0),
        ];
        let result = compute(&allocations);
        assert!(
            result > 0.7,
            "expected > 0.7 for all-active portfolio, got {result}"
        );
    }

    #[test]
    fn disputed_allocations_reduce_standing() {
        let clean = vec![
            make_allocation("active", 10.0),
            make_allocation("active", 10.0),
        ];
        let disputed = vec![
            make_allocation("active", 10.0),
            make_allocation("disputed", 10.0),
        ];
        assert!(
            compute(&clean) > compute(&disputed),
            "clean portfolio should have higher standing than disputed"
        );
    }

    #[test]
    fn recognition_boosts_standing() {
        let high_recognition = vec![make_allocation("active", 100.0)];
        let low_recognition = vec![make_allocation("active", 0.0)];
        assert!(
            compute(&high_recognition) > compute(&low_recognition),
            "higher recognition should yield higher standing"
        );
    }

    #[test]
    fn result_clamped_to_unit_interval() {
        let many_active: Vec<_> = (0..50).map(|_| make_allocation("active", 100.0)).collect();
        let result_high = compute(&many_active);
        assert!(
            result_high <= 1.0,
            "result should be <= 1.0, got {result_high}"
        );

        let many_disputed: Vec<_> = (0..50).map(|_| make_allocation("disputed", 0.0)).collect();
        let result_low = compute(&many_disputed);
        assert!(
            result_low >= 0.0,
            "result should be >= 0.0, got {result_low}"
        );
    }
}
