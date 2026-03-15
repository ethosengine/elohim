//! Instant-runoff (ranked-choice) voting tally.

use std::collections::{HashMap, HashSet};

use crate::db::models::{ProposalOption, RankedVote};

use super::{check_quorum, BallotError, OptionResult, TallyResult, TallyRound, TallyStrategy, VotingConfig};

pub struct RankedChoiceTally;

impl TallyStrategy for RankedChoiceTally {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult {
        let option_ids: HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();
        let option_labels: HashMap<&str, &str> =
            options.iter().map(|o| (o.id.as_str(), o.label.as_str())).collect();

        // Group votes by voter, sorted by rank
        let mut ballots: HashMap<&str, Vec<&RankedVote>> = HashMap::new();
        for v in votes {
            if option_ids.contains(v.option_id.as_str()) {
                ballots.entry(v.human_id.as_str()).or_default().push(v);
            }
        }
        for ballot in ballots.values_mut() {
            ballot.sort_by_key(|v| v.rank.unwrap_or(i32::MAX));
        }

        let total_voters = ballots.len();
        let quorum_met = check_quorum(total_voters, config);

        let mut eliminated: HashSet<String> = HashSet::new();
        let mut rounds: Vec<TallyRound> = Vec::new();
        let mut final_counts: HashMap<String, f64> = HashMap::new();

        loop {
            // Count first-valid-preference for each active option
            let mut counts: HashMap<String, usize> = HashMap::new();
            for opt in options {
                if !eliminated.contains(&opt.id) {
                    counts.insert(opt.id.clone(), 0);
                }
            }

            for ballot in ballots.values() {
                for v in ballot {
                    if !eliminated.contains(&v.option_id) {
                        *counts.entry(v.option_id.clone()).or_insert(0) += 1;
                        break;
                    }
                }
            }

            let majority_threshold = if total_voters > 0 {
                total_voters as f64 / 2.0
            } else {
                0.0
            };

            // Check for majority winner
            let mut winner: Option<String> = None;
            for (opt_id, count) in &counts {
                if (*count as f64) > majority_threshold {
                    winner = Some(opt_id.clone());
                }
            }

            // Build standings for this round
            let mut standings: Vec<OptionResult> = counts
                .iter()
                .map(|(opt_id, &count)| {
                    let pct = if total_voters > 0 {
                        (count as f64 / total_voters as f64) * 100.0
                    } else {
                        0.0
                    };
                    OptionResult {
                        option_id: opt_id.clone(),
                        label: option_labels.get(opt_id.as_str()).unwrap_or(&"").to_string(),
                        votes: count as f64,
                        percentage: pct,
                        rank: None,
                        eliminated: false,
                    }
                })
                .collect();
            standings.sort_by(|a, b| b.votes.partial_cmp(&a.votes).unwrap_or(std::cmp::Ordering::Equal));

            if winner.is_some() || counts.len() <= 2 {
                // Store final counts
                for (opt_id, count) in &counts {
                    final_counts.insert(opt_id.clone(), *count as f64);
                }
                rounds.push(TallyRound {
                    round_number: rounds.len() as i32 + 1,
                    eliminated_option_id: None,
                    standings,
                });
                break;
            }

            // Find the option with fewest votes to eliminate
            let min_count = *counts.values().min().unwrap_or(&0);
            let to_eliminate = counts
                .iter()
                .find(|(_, &c)| c == min_count)
                .map(|(id, _)| id.clone());

            if let Some(ref elim_id) = to_eliminate {
                eliminated.insert(elim_id.clone());
            }

            rounds.push(TallyRound {
                round_number: rounds.len() as i32 + 1,
                eliminated_option_id: to_eliminate.clone(),
                standings,
            });

            // Safety: if all eliminated, break
            if eliminated.len() >= options.len() {
                break;
            }
        }

        // Build final option results with ranks
        let mut option_results: Vec<OptionResult> = options
            .iter()
            .map(|opt| {
                let count = final_counts.get(&opt.id).copied().unwrap_or(0.0);
                let pct = if total_voters > 0 {
                    (count / total_voters as f64) * 100.0
                } else {
                    0.0
                };
                OptionResult {
                    option_id: opt.id.clone(),
                    label: opt.label.clone(),
                    votes: count,
                    percentage: pct,
                    rank: None,
                    eliminated: eliminated.contains(&opt.id),
                }
            })
            .collect();

        // Assign ranks by vote count (highest first)
        option_results.sort_by(|a, b| b.votes.partial_cmp(&a.votes).unwrap_or(std::cmp::Ordering::Equal));
        for (i, result) in option_results.iter_mut().enumerate() {
            result.rank = Some(i as i32 + 1);
        }

        let recommendation = if let Some(winner) = option_results.first() {
            if winner.percentage > 50.0 && quorum_met {
                "pass".to_string()
            } else {
                "no-majority".to_string()
            }
        } else {
            "no-options".to_string()
        };

        TallyResult {
            mechanism: "ranked-choice".to_string(),
            total_voters,
            quorum_met,
            option_results,
            recommendation,
            rounds: Some(rounds),
        }
    }

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        _config: &VotingConfig,
    ) -> Result<(), BallotError> {
        let valid_ids: HashSet<&str> = options.iter().map(|o| o.id.as_str()).collect();

        // Check for invalid option IDs
        for v in votes {
            if !valid_ids.contains(v.option_id.as_str()) {
                return Err(BallotError::InvalidOptionId(v.option_id.clone()));
            }
        }

        // Check for duplicate ranks within a single voter's ballot
        let mut ranks_seen: HashSet<i32> = HashSet::new();
        for v in votes {
            if let Some(rank) = v.rank {
                if !ranks_seen.insert(rank) {
                    return Err(BallotError::DuplicateRanks);
                }
            }
        }

        Ok(())
    }
}
