//! Approval voting tally — count approvals per option.

use std::collections::HashSet;

use crate::db::models::{ProposalOption, RankedVote};

use super::{check_quorum, BallotError, OptionResult, TallyResult, TallyStrategy, VotingConfig};

pub struct ApprovalTally;

impl TallyStrategy for ApprovalTally {
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

        // Count approvals per option
        let mut approval_counts: std::collections::HashMap<String, usize> =
            options.iter().map(|o| (o.id.clone(), 0)).collect();

        for v in votes {
            if v.approved == Some(1) {
                if let Some(count) = approval_counts.get_mut(&v.option_id) {
                    *count += 1;
                }
            }
        }

        let mut option_results: Vec<OptionResult> = options
            .iter()
            .map(|opt| {
                let count = approval_counts.get(&opt.id).copied().unwrap_or(0);
                let pct = if total_voters > 0 {
                    (count as f64 / total_voters as f64) * 100.0
                } else {
                    0.0
                };
                OptionResult {
                    option_id: opt.id.clone(),
                    label: opt.label.clone(),
                    votes: count as f64,
                    percentage: pct,
                    rank: None,
                    eliminated: false,
                }
            })
            .collect();

        // Sort by count descending
        option_results.sort_by(|a, b| {
            b.votes
                .partial_cmp(&a.votes)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign ranks
        for (i, result) in option_results.iter_mut().enumerate() {
            result.rank = Some(i as i32 + 1);
        }

        let passage_threshold = config.passage_threshold.unwrap_or(0.5);
        let recommendation = if let Some(top) = option_results.first() {
            if top.percentage / 100.0 >= passage_threshold && quorum_met {
                "pass".to_string()
            } else {
                "fail".to_string()
            }
        } else {
            "no-options".to_string()
        };

        TallyResult {
            mechanism: "approval".to_string(),
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
