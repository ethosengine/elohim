//! Anomaly detection from observation patterns.
//!
//! Computes an anomaly score (0.0-1.0) by comparing recent observation patterns
//! against the historical baseline. Feeds into TrustContext as intent_divergence.
//!
//! Scoped to observation-derived signals only (no raw interaction telemetry).

use crate::db::models::ImagodeiObservation;

/// Compute anomaly score from observation history.
///
/// Compares the last 24 hours of observations against the 30-day baseline.
/// Returns 0.0 (consistent) to 1.0 (fully divergent).
///
/// Signals analyzed:
/// - Mutation rate: sudden increase in observation frequency
/// - Trust delta trend: rapid negative shift vs baseline
///
/// With fewer than 5 observations, returns 0.0 (insufficient data).
pub fn compute_anomaly_score(observations: &[ImagodeiObservation]) -> f64 {
    if observations.len() < 5 {
        return 0.0;
    }

    let now = chrono::Utc::now().naive_utc();

    // Partition into recent (24h) and baseline (rest)
    let (recent, baseline): (Vec<&ImagodeiObservation>, Vec<&ImagodeiObservation>) = observations
        .iter()
        .partition(|obs| parse_age_hours(&obs.observed_at, &now) < 24.0);

    if recent.is_empty() || baseline.is_empty() {
        return 0.0;
    }

    // Signal 1: Mutation rate divergence
    // Compare observations/hour in recent vs baseline
    let recent_rate = recent.len() as f64 / 24.0; // obs per hour in last 24h
    let baseline_days = baseline
        .iter()
        .map(|obs| parse_age_hours(&obs.observed_at, &now) / 24.0)
        .fold(0.0_f64, f64::max)
        .max(1.0); // at least 1 day
    let baseline_rate = baseline.len() as f64 / (baseline_days * 24.0); // obs per hour in baseline

    let rate_divergence = if baseline_rate > 0.0 {
        ((recent_rate / baseline_rate) - 1.0).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Signal 2: Trust delta trend divergence
    // Compare average trust_delta in recent vs baseline
    let recent_avg_delta =
        recent.iter().map(|o| o.trust_delta as f64).sum::<f64>() / recent.len() as f64;
    let baseline_avg_delta =
        baseline.iter().map(|o| o.trust_delta as f64).sum::<f64>() / baseline.len() as f64;

    // Negative shift from baseline is anomalous
    let delta_divergence = if recent_avg_delta < baseline_avg_delta {
        (baseline_avg_delta - recent_avg_delta).min(1.0)
    } else {
        0.0
    };

    // Weighted combination
    let score = rate_divergence * 0.5 + delta_divergence * 0.5;
    score.clamp(0.0, 1.0)
}

fn parse_age_hours(observed_at: &str, now: &chrono::NaiveDateTime) -> f64 {
    chrono::NaiveDateTime::parse_from_str(observed_at, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(observed_at, "%Y-%m-%dT%H:%M:%S"))
        .map(|dt| (*now - dt).num_seconds() as f64 / 3600.0)
        .unwrap_or(f64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_observation(trust_delta: f32, hours_ago: i64) -> ImagodeiObservation {
        let observed_at = (chrono::Utc::now().naive_utc() - chrono::TimeDelta::hours(hours_ago))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        ImagodeiObservation {
            id: uuid::Uuid::new_v4().to_string(),
            h_app_id: "lamad".to_string(),
            human_id: "human-1".to_string(),
            observed_at,
            observation_type: "growth_signal".to_string(),
            content: "test".to_string(),
            structured_signals_json: None,
            trust_delta,
            visibility_layer: "individual".to_string(),
            originating_elohim: "test".to_string(),
            relevance_decay: 1.0,
            superseded_by: None,
            created_at: "2026-03-14 00:00:00".to_string(),
            dht_anchor_hash: None,
        }
    }

    #[test]
    fn insufficient_data_returns_zero() {
        let obs = vec![make_observation(0.1, 1)];
        assert_eq!(compute_anomaly_score(&obs), 0.0);
    }

    #[test]
    fn consistent_patterns_return_low_score() {
        // 1 recent + 4 baseline at similar rate, all positive deltas
        let obs: Vec<_> = (0..1)
            .map(|_| make_observation(0.01, 12)) // 1 in last 24h
            .chain((0..4).map(|i| make_observation(0.01, 24 + (i + 1) * 24))) // 4 across days 2-5
            .collect();
        let score = compute_anomaly_score(&obs);
        assert!(
            score < 0.3,
            "Expected low score for consistent patterns, got {}",
            score
        );
    }

    #[test]
    fn negative_delta_shift_increases_score() {
        // Recent observations have negative delta, baseline is positive
        let obs: Vec<_> = (0..3)
            .map(|_| make_observation(-0.1, 1)) // recent: negative
            .chain((0..5).map(|_| make_observation(0.05, 72))) // baseline: positive
            .collect();
        let score = compute_anomaly_score(&obs);
        assert!(
            score > 0.0,
            "Expected positive score for negative delta shift, got {}",
            score
        );
    }

    #[test]
    fn score_clamped_to_unit_interval() {
        // Extreme divergence
        let obs: Vec<_> = (0..20)
            .map(|_| make_observation(-0.5, 1)) // many negative recent
            .chain((0..3).map(|_| make_observation(0.5, 720))) // few positive baseline
            .collect();
        let score = compute_anomaly_score(&obs);
        assert!(
            score >= 0.0 && score <= 1.0,
            "Score {} not in [0, 1]",
            score
        );
    }
}
