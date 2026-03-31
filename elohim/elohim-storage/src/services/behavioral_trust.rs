//! Behavioral trust computation from observation history.
//!
//! Aggregates imagodei observations into a single behavioral_trust
//! signal (0.0-1.0) with time-based relevance decay.

use crate::db::models::ImagodeiObservation;

/// Compute behavioral trust from observation history.
///
/// Algorithm:
/// - Start at 0.5 (neutral baseline)
/// - Each observation adjusts by: trust_delta x decay_factor
/// - decay_factor = relevance_decay x (1.0 - age_days / 365.0).max(0.0)
/// - Clamp result to [0.0, 1.0]
pub fn compute(observations: &[ImagodeiObservation]) -> f64 {
    if observations.is_empty() {
        return 0.5;
    }

    let now = chrono::Utc::now().naive_utc();
    let mut trust = 0.5_f64;

    for obs in observations {
        let age_days = parse_age_days(&obs.observed_at, &now);
        let decay_factor = (obs.relevance_decay as f64) * (1.0 - age_days / 365.0).max(0.0);
        trust += (obs.trust_delta as f64) * decay_factor;
    }

    trust.clamp(0.0, 1.0)
}

fn parse_age_days(observed_at: &str, now: &chrono::NaiveDateTime) -> f64 {
    // Try both formats: "YYYY-MM-DD HH:MM:SS" (SQLite default) and ISO 8601
    chrono::NaiveDateTime::parse_from_str(observed_at, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(observed_at, "%Y-%m-%dT%H:%M:%S"))
        .map(|dt| (*now - dt).num_seconds() as f64 / 86400.0)
        .unwrap_or(365.0) // unparseable dates treated as maximally old
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_observation(
        trust_delta: f32,
        age_days: i64,
        relevance_decay: f32,
    ) -> ImagodeiObservation {
        let observed_at = (chrono::Utc::now().naive_utc() - chrono::Duration::days(age_days))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        ImagodeiObservation {
            id: "test-id".to_string(),
            h_app_id: "lamad".to_string(),
            human_id: "human-1".to_string(),
            observed_at,
            observation_type: "growth_signal".to_string(),
            content: "test".to_string(),
            structured_signals_json: None,
            trust_delta,
            visibility_layer: "individual".to_string(),
            originating_elohim: "test".to_string(),
            relevance_decay,
            superseded_by: None,
            created_at: "2026-03-14 00:00:00".to_string(),
            dht_anchor_hash: None,
        }
    }

    #[test]
    fn empty_history_returns_neutral() {
        assert_eq!(compute(&[]), 0.5);
    }

    #[test]
    fn positive_observations_increase_trust() {
        let obs = vec![make_observation(0.1, 0, 1.0)];
        assert!(compute(&obs) > 0.5);
    }

    #[test]
    fn negative_observations_decrease_trust() {
        let obs = vec![make_observation(-0.1, 0, 1.0)];
        assert!(compute(&obs) < 0.5);
    }

    #[test]
    fn old_observations_decay_toward_irrelevance() {
        let recent = vec![make_observation(0.1, 1, 1.0)];
        let old = vec![make_observation(0.1, 300, 1.0)];
        assert!(compute(&recent) > compute(&old));
    }

    #[test]
    fn result_clamped_to_unit_interval() {
        let obs: Vec<_> = (0..100).map(|_| make_observation(0.5, 0, 1.0)).collect();
        assert_eq!(compute(&obs), 1.0);
        let neg: Vec<_> = (0..100).map(|_| make_observation(-0.5, 0, 1.0)).collect();
        assert_eq!(compute(&neg), 0.0);
    }
}
