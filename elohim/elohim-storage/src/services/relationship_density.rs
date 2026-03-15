//! Relationship density computation from human relationship records.
//!
//! Aggregates HumanRelationship records into a single relationship_density
//! signal (0.0-1.0) using log-scale normalization with diminishing returns.

use crate::db::models::HumanRelationship;

/// Compute relationship density from human relationship records.
///
/// Algorithm:
/// - Empty relationships → 0.5 (neutral baseline)
/// - Score each relationship: base 1.0 + verified bonus 0.5 + bidirectional bonus 0.3 + mutual consent bonus 0.2
/// - Sum all weighted scores
/// - Log-scale normalize: ln(1 + weighted_count) / ln(1 + 40.0) (threshold ~20 strong relationships)
/// - Clamp to [0.0, 1.0]
pub fn compute(relationships: &[HumanRelationship]) -> f64 {
    if relationships.is_empty() {
        return 0.5;
    }

    let mut weighted_sum = 0.0_f64;

    for rel in relationships {
        let mut score = 1.0_f64;

        // Verified bonus: verified_at is Some
        if rel.verified_at.is_some() {
            score += 0.5;
        }

        // Bidirectional bonus
        if rel.is_bidirectional == 1 {
            score += 0.3;
        }

        // Mutual consent bonus: both parties gave consent
        if rel.consent_given_by_a == 1 && rel.consent_given_by_b == 1 {
            score += 0.2;
        }

        weighted_sum += score;
    }

    let density = (1.0 + weighted_sum).ln() / (1.0 + 40.0_f64).ln();
    density.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_relationship(
        verified: bool,
        bidirectional: bool,
        mutual_consent: bool,
    ) -> HumanRelationship {
        HumanRelationship {
            id: "test-id".to_string(),
            app_id: "lamad".to_string(),
            party_a_id: "human-a".to_string(),
            party_b_id: "human-b".to_string(),
            relationship_type: "peer".to_string(),
            intimacy_level: "acquaintance".to_string(),
            is_bidirectional: if bidirectional { 1 } else { 0 },
            consent_given_by_a: if mutual_consent { 1 } else { 0 },
            consent_given_by_b: if mutual_consent { 1 } else { 0 },
            custody_enabled_by_a: 0,
            custody_enabled_by_b: 0,
            auto_custody_enabled: 0,
            emergency_access_enabled: 0,
            initiated_by: "human-a".to_string(),
            verified_at: if verified {
                Some("2026-03-14 00:00:00".to_string())
            } else {
                None
            },
            governance_layer: None,
            reach: "local".to_string(),
            context_json: None,
            created_at: "2026-03-14 00:00:00".to_string(),
            updated_at: "2026-03-14 00:00:00".to_string(),
            expires_at: None,
        }
    }

    #[test]
    fn empty_relationships_returns_neutral() {
        assert_eq!(compute(&[]), 0.5);
    }

    #[test]
    fn verified_bidirectional_scores_higher() {
        let strong = vec![make_relationship(true, true, true)];
        let weak = vec![make_relationship(false, false, false)];
        assert!(
            compute(&strong) > compute(&weak),
            "strong ({}) should exceed weak ({})",
            compute(&strong),
            compute(&weak)
        );
    }

    #[test]
    fn more_relationships_increases_density() {
        let one: Vec<_> = (0..1)
            .map(|_| make_relationship(true, true, true))
            .collect();
        let ten: Vec<_> = (0..10)
            .map(|_| make_relationship(true, true, true))
            .collect();
        assert!(
            compute(&ten) > compute(&one),
            "ten ({}) should exceed one ({})",
            compute(&ten),
            compute(&one)
        );
    }

    #[test]
    fn diminishing_returns_after_threshold() {
        let twenty: Vec<_> = (0..20)
            .map(|_| make_relationship(true, true, true))
            .collect();
        let hundred: Vec<_> = (0..100)
            .map(|_| make_relationship(true, true, true))
            .collect();
        let diff = compute(&hundred) - compute(&twenty);
        assert!(
            diff < 0.2,
            "difference between 100 and 20 relationships ({diff}) should be < 0.2"
        );
    }

    #[test]
    fn result_clamped_to_unit_interval() {
        let massive: Vec<_> = (0..1000)
            .map(|_| make_relationship(true, true, true))
            .collect();
        let result = compute(&massive);
        assert!(
            result <= 1.0,
            "result ({result}) should be clamped to <= 1.0"
        );
    }
}
