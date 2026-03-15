//! Dot voting tally — sum dots per option within a per-voter budget.

use std::collections::{HashMap, HashSet};

use crate::db::models::{ProposalOption, RankedVote};

use super::{check_quorum, BallotError, OptionResult, TallyResult, TallyStrategy, VotingConfig};

pub struct DotTally;

impl TallyStrategy for DotTally {
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

        // Sum dots per option
        let mut dot_sums: HashMap<String, i64> =
            options.iter().map(|o| (o.id.clone(), 0i64)).collect();

        for v in votes {
            if let Some(d) = v.dots {
                if let Some(sum) = dot_sums.get_mut(&v.option_id) {
                    *sum += d as i64;
                }
            }
        }

        let total_dots: i64 = dot_sums.values().sum();

        let mut option_results: Vec<OptionResult> = options
            .iter()
            .map(|opt| {
                let dots = dot_sums.get(&opt.id).copied().unwrap_or(0);
                let pct = if total_dots > 0 {
                    (dots as f64 / total_dots as f64) * 100.0
                } else {
                    0.0
                };
                OptionResult {
                    option_id: opt.id.clone(),
                    label: opt.label.clone(),
                    votes: dots as f64,
                    percentage: pct,
                    rank: None,
                    eliminated: false,
                }
            })
            .collect();

        // Sort by dots descending
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
            "no-votes".to_string()
        };

        TallyResult {
            mechanism: "dot-vote".to_string(),
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
        let budget = config.dots_per_voter.unwrap_or(10);

        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
        }

        // Check total dots per voter doesn't exceed budget
        let mut voter_dots: HashMap<&str, i32> = HashMap::new();
        for v in votes {
            if let Some(d) = v.dots {
                *voter_dots.entry(v.human_id.as_str()).or_insert(0) += d;
            }
        }

        for (&_voter, &used) in &voter_dots {
            if used > budget {
                return Err(BallotError::DotsExceedBudget { used, budget });
            }
        }

        Ok(())
    }
}
