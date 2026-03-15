//! Consent-based decision tally — passes unless a block exists.
//!
//! Blocks are escalation triggers, NOT vetoes. A block signals that
//! the proposal needs further dialogue before proceeding.

use std::collections::HashSet;

use crate::db::models::{ProposalOption, RankedVote};

use super::{check_quorum, BallotError, OptionResult, TallyResult, TallyStrategy, VotingConfig};

pub struct ConsentTally;

impl TallyStrategy for ConsentTally {
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

        let mut option_results: Vec<OptionResult> = options
            .iter()
            .map(|opt| {
                let option_votes: Vec<&RankedVote> =
                    votes.iter().filter(|v| v.option_id == opt.id).collect();

                let consents = option_votes
                    .iter()
                    .filter(|v| v.approved == Some(1))
                    .count();
                let blocks = option_votes
                    .iter()
                    .filter(|v| v.approved == Some(0))
                    .count();
                let _abstains = option_votes.len() - consents - blocks;

                let pct = if total_voters > 0 {
                    (consents as f64 / total_voters as f64) * 100.0
                } else {
                    0.0
                };

                OptionResult {
                    option_id: opt.id.clone(),
                    label: opt.label.clone(),
                    votes: consents as f64,
                    percentage: pct,
                    rank: None,
                    eliminated: blocks > 0,
                }
            })
            .collect();

        // Sort: unblocked first, then by consent count descending
        option_results.sort_by(|a, b| {
            a.eliminated
                .cmp(&b.eliminated)
                .then_with(|| {
                    b.votes
                        .partial_cmp(&a.votes)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        // Assign ranks
        for (i, result) in option_results.iter_mut().enumerate() {
            result.rank = Some(i as i32 + 1);
        }

        let has_blocks = option_results.iter().any(|r| r.eliminated);
        let recommendation = if has_blocks {
            "blocked".to_string()
        } else if quorum_met {
            "pass".to_string()
        } else {
            "no-quorum".to_string()
        };

        TallyResult {
            mechanism: "consent".to_string(),
            total_voters,
            quorum_met,
            option_results,
            recommendation,
            rounds: None,
        }
    }

    fn validate_ballot(
        &self,
        _votes: &[RankedVote],
        _options: &[ProposalOption],
        _config: &VotingConfig,
    ) -> Result<(), BallotError> {
        // Consent ballots are simple — always valid
        Ok(())
    }
}
