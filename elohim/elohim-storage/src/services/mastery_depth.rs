//! Mastery depth computation from content mastery records.
//!
//! Aggregates ContentMastery records into a single mastery_depth
//! signal (0.0-1.0) using Bloom's taxonomy levels weighted by freshness.

use crate::db::models::ContentMastery;

/// Compute mastery depth from content mastery records.
///
/// Algorithm:
/// - Empty records → 0.5 (neutral baseline)
/// - Each mastery level maps to a Bloom's taxonomy depth (0.0-1.0)
/// - Weight each record's depth by its freshness_score
/// - Compute weighted average: sum(depth × freshness) / sum(freshness)
/// - If total weight is near zero, return 0.5
/// - Clamp to [0.0, 1.0]
pub fn compute(records: &[ContentMastery]) -> f64 {
    if records.is_empty() {
        return 0.5;
    }

    let mut weighted_sum = 0.0_f64;
    let mut total_weight = 0.0_f64;

    for record in records {
        let depth = level_to_depth(&record.mastery_level);
        let freshness = record.freshness_score as f64;
        weighted_sum += depth * freshness;
        total_weight += freshness;
    }

    if total_weight < 1e-9 {
        return 0.5;
    }

    (weighted_sum / total_weight).clamp(0.0, 1.0)
}

/// Map Bloom's taxonomy mastery levels to depth values.
fn level_to_depth(level: &str) -> f64 {
    match level {
        "not_started" => 0.0,
        "aware" => 0.14,
        "remember" => 0.29,
        "understand" => 0.43,
        "apply" => 0.57,
        "analyze" => 0.71,
        "evaluate" => 0.86,
        "create" => 1.0,
        _ => 0.0, // unknown levels treated as not_started
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mastery(level: &str, freshness: f32) -> ContentMastery {
        ContentMastery {
            id: "test-id".to_string(),
            h_app_id: "lamad".to_string(),
            human_id: "human-1".to_string(),
            content_id: "content-1".to_string(),
            mastery_level: level.to_string(),
            mastery_level_index: 0,
            freshness_score: freshness,
            needs_refresh: 0,
            engagement_count: 1,
            last_engagement_type: None,
            last_engagement_at: None,
            level_achieved_at: None,
            content_version_at_mastery: None,
            assessment_evidence_json: None,
            privileges_json: None,
            created_at: "2026-03-14 00:00:00".to_string(),
            updated_at: "2026-03-14 00:00:00".to_string(),
            dht_anchor_hash: None,
        }
    }

    #[test]
    fn empty_records_returns_neutral() {
        assert_eq!(compute(&[]), 0.5);
    }

    #[test]
    fn single_create_level_full_freshness() {
        let records = vec![make_mastery("create", 1.0)];
        let result = compute(&records);
        assert!((result - 1.0).abs() < 1e-9, "expected ~1.0, got {result}");
    }

    #[test]
    fn single_not_started_returns_zero() {
        let records = vec![make_mastery("not_started", 1.0)];
        let result = compute(&records);
        assert!((result - 0.0).abs() < 1e-9, "expected ~0.0, got {result}");
    }

    #[test]
    fn freshness_decay_reduces_depth() {
        let fresh = vec![make_mastery("create", 1.0)];
        let stale = vec![make_mastery("create", 0.3)];
        // Both should return 1.0 (single record weighted average is just depth),
        // but let's add a second record to see the effect
        let fresh_mixed = vec![
            make_mastery("create", 1.0),
            make_mastery("not_started", 1.0),
        ];
        let stale_mixed = vec![
            make_mastery("create", 0.3),
            make_mastery("not_started", 1.0),
        ];
        // fresh_mixed: (1.0*1.0 + 0.0*1.0) / (1.0+1.0) = 0.5
        // stale_mixed: (1.0*0.3 + 0.0*1.0) / (0.3+1.0) = 0.3/1.3 ≈ 0.23
        assert!(
            compute(&fresh_mixed) > compute(&stale_mixed),
            "fresher create should yield higher depth"
        );
        // Also verify single records: both are 1.0 since weighted avg of single = depth
        assert!((compute(&fresh) - 1.0).abs() < 1e-9);
        assert!((compute(&stale) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn mixed_levels_averaged() {
        let records = vec![
            make_mastery("understand", 1.0), // 0.43
            make_mastery("evaluate", 1.0),   // 0.86
        ];
        let result = compute(&records);
        let expected = (0.43 + 0.86) / 2.0; // 0.645
        assert!(
            (result - expected).abs() < 1e-9,
            "expected ~{expected}, got {result}"
        );
    }
}
