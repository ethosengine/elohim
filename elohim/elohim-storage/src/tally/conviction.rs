//! Conviction voting tally — time-weighted vote accumulation.
//!
//! Votes build conviction over time using exponential growth with a
//! half-life decay. Weight = 1.0 - 0.5^(days_held / HALF_LIFE_DAYS).

use std::collections::HashSet;

use chrono::{NaiveDateTime, Utc};

use crate::db::models::{ProposalOption, RankedVote};

use super::{check_quorum, BallotError, OptionResult, TallyResult, TallyStrategy, VotingConfig};

/// Half-life in days for conviction decay
const HALF_LIFE_DAYS: f64 = 7.0;

pub struct ConvictionTally;

impl ConvictionTally {
    /// Calculate conviction weight for a vote based on how long it's been held.
    fn conviction_weight(created_at: &str) -> f64 {
        let now = Utc::now().naive_utc();

        // Parse ISO 8601 timestamp (e.g. "2026-03-10T12:00:00Z")
        let created = created_at
            .trim_end_matches('Z')
            .parse::<NaiveDateTime>()
            .or_else(|_| {
                NaiveDateTime::parse_from_str(created_at, "%Y-%m-%dT%H:%M:%S%.fZ")
            })
            .or_else(|_| {
                NaiveDateTime::parse_from_str(created_at, "%Y-%m-%dT%H:%M:%S")
            })
            .unwrap_or(now);

        let days_held = (now - created).num_seconds() as f64 / 86400.0;
        let days_held = days_held.max(0.0);

        1.0 - 0.5_f64.powf(days_held / HALF_LIFE_DAYS)
    }
}

impl TallyStrategy for ConvictionTally {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult {
        let total_voters: usize = votes
            .iter()
            .map(|v| v.human_id.as_str())
            .collect::<HashSet<_>>()
            .len();
        let quorum_met = check_quorum(total_voters, config);

        // Sum conviction-weighted values per option (only approved votes count)
        let mut weighted_sums: std::collections::HashMap<String, f64> =
            options.iter().map(|o| (o.id.clone(), 0.0)).collect();

        for v in votes {
            if v.approved == Some(1) {
                let weight = Self::conviction_weight(&v.created_at);
                if let Some(sum) = weighted_sums.get_mut(&v.option_id) {
                    *sum += weight;
                }
            }
        }

        let total_weight: f64 = weighted_sums.values().sum();

        let mut option_results: Vec<OptionResult> = options
            .iter()
            .map(|opt| {
                let weight = weighted_sums.get(&opt.id).copied().unwrap_or(0.0);
                let pct = if total_weight > 0.0 {
                    (weight / total_weight) * 100.0
                } else {
                    0.0
                };
                OptionResult {
                    option_id: opt.id.clone(),
                    label: opt.label.clone(),
                    votes: weight,
                    percentage: pct,
                    rank: None,
                    eliminated: false,
                }
            })
            .collect();

        // Sort by weighted total descending
        option_results.sort_by(|a, b| {
            b.votes
                .partial_cmp(&a.votes)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign ranks
        for (i, result) in option_results.iter_mut().enumerate() {
            result.rank = Some(i as i32 + 1);
        }

        let recommendation = if quorum_met && option_results.first().map_or(false, |r| r.votes > 0.0)
        {
            "pass".to_string()
        } else {
            "insufficient-conviction".to_string()
        };

        TallyResult {
            mechanism: "conviction".to_string(),
            total_voters,
            quorum_met,
            option_results,
            recommendation,
            rounds: None,
        }
    }

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        _config: &VotingConfig,
    ) -> Result<(), BallotError> {
        let valid_ids: HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();

        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
        }

        Ok(())
    }
}
