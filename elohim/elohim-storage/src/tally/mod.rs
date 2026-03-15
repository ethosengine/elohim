//! Tally strategies for multi-mechanism voting.
//!
//! Each voting mechanism implements TallyStrategy. New mechanisms are
//! added by creating a struct, implementing the trait, and registering
//! it in `get_strategy()`.

pub mod approval;
pub mod consent;
pub mod conviction;
pub mod dot;
pub mod ranked_choice;
pub mod score;

use crate::db::models::{ProposalOption, RankedVote};
use serde::Serialize;
use ts_rs::TS;

/// Configuration passed to tally strategies
#[derive(Debug, Clone)]
pub struct VotingConfig {
    pub score_min: Option<i32>,
    pub score_max: Option<i32>,
    pub dots_per_voter: Option<i32>,
    pub quorum_percentage: Option<f64>,
    pub passage_threshold: Option<f64>,
}

/// Result of tallying votes
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TallyResult {
    pub mechanism: String,
    pub total_voters: usize,
    pub quorum_met: bool,
    pub option_results: Vec<OptionResult>,
    pub recommendation: String,
    pub rounds: Option<Vec<TallyRound>>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OptionResult {
    pub option_id: String,
    pub label: String,
    pub votes: f64,
    pub percentage: f64,
    pub rank: Option<i32>,
    pub eliminated: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TallyRound {
    pub round_number: i32,
    pub eliminated_option_id: Option<String>,
    pub standings: Vec<OptionResult>,
}

#[derive(Debug, Clone)]
pub enum BallotError {
    MissingOptions(Vec<String>),
    DuplicateRanks,
    ScoreOutOfRange { option_id: String, score: i32 },
    DotsExceedBudget { used: i32, budget: i32 },
    InvalidOptionId(String),
}

impl std::fmt::Display for BallotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOptions(ids) => write!(f, "Missing options: {}", ids.join(", ")),
            Self::DuplicateRanks => write!(f, "Duplicate ranks in ballot"),
            Self::ScoreOutOfRange { option_id, score } => {
                write!(f, "Score {} out of range for option {}", score, option_id)
            }
            Self::DotsExceedBudget { used, budget } => {
                write!(f, "Dots used ({}) exceeds budget ({})", used, budget)
            }
            Self::InvalidOptionId(id) => write!(f, "Invalid option ID: {}", id),
        }
    }
}

/// The core trait. Each voting mechanism implements this.
pub trait TallyStrategy: Send + Sync {
    fn tally(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> TallyResult;

    fn validate_ballot(
        &self,
        votes: &[RankedVote],
        options: &[ProposalOption],
        config: &VotingConfig,
    ) -> Result<(), BallotError>;
}

/// Look up a tally strategy by mechanism name.
pub fn get_strategy(mechanism: &str) -> Option<Box<dyn TallyStrategy>> {
    match mechanism {
        "ranked-choice" => Some(Box::new(ranked_choice::RankedChoiceTally)),
        "approval" => Some(Box::new(approval::ApprovalTally)),
        "score-vote" => Some(Box::new(score::ScoreTally)),
        "dot-vote" => Some(Box::new(dot::DotTally)),
        "consent" => Some(Box::new(consent::ConsentTally)),
        "conviction" => Some(Box::new(conviction::ConvictionTally)),
        _ => None,
    }
}

/// Shared quorum check — returns true if quorum is met.
/// If no quorum_percentage is configured, quorum is always met.
pub fn check_quorum(total_voters: usize, config: &VotingConfig) -> bool {
    config
        .quorum_percentage
        .map_or(true, |q| q <= 0.0 || total_voters > 0)
}
