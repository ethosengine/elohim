//! Score voting tally — sum scores per option.

use std::collections::HashSet;

use crate::db::models::{ProposalOption, RankedVote};

use super::{check_quorum, BallotError, OptionResult, TallyResult, TallyStrategy, VotingConfig};

pub struct ScoreTally;

impl TallyStrategy for ScoreTally {
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

        let score_max = config.score_max.unwrap_or(10);

        // Sum scores per option
        let mut score_sums: std::collections::HashMap<String, i64> =
            options.iter().map(|o| (o.id.clone(), 0i64)).collect();

        for v in votes {
            if let Some(s) = v.score {
                if let Some(sum) = score_sums.get_mut(&v.option_id) {
                    *sum += s as i64;
                }
            }
        }

        let max_possible = if total_voters > 0 && score_max > 0 {
            total_voters as f64 * score_max as f64
        } else {
            1.0
        };

        let mut option_results: Vec<OptionResult> = options
            .iter()
            .map(|opt| {
                let total = score_sums.get(&opt.id).copied().unwrap_or(0);
                let pct = (total as f64 / max_possible) * 100.0;
                OptionResult {
                    option_id: opt.id.clone(),
                    label: opt.label.clone(),
                    votes: total as f64,
                    percentage: pct,
                    rank: None,
                    eliminated: false,
                }
            })
            .collect();

        // Sort by total descending
        option_results.sort_by(|a, b| {
            b.votes
                .partial_cmp(&a.votes)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign ranks
        for (i, result) in option_results.iter_mut().enumerate() {
            result.rank = Some(i as i32 + 1);
        }

        let recommendation = if quorum_met && option_results.first().is_some_and(|r| r.votes > 0.0)
        {
            "pass".to_string()
        } else {
            "fail".to_string()
        };

        TallyResult {
            mechanism: "score-vote".to_string(),
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
        config: &VotingConfig,
    ) -> Result<(), BallotError> {
        let valid_ids: HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();
        let score_min = config.score_min.unwrap_or(0);
        let score_max = config.score_max.unwrap_or(10);

        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
            if let Some(s) = v.score {
                if s < score_min || s > score_max {
                    return Err(BallotError::ScoreOutOfRange {
                        option_id: v.option_id.clone(),
                        score: s,
                    });
                }
            }
        }

        Ok(())
    }
}
