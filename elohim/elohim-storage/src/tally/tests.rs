use super::*;
use crate::db::models::{ProposalOption, RankedVote};

fn make_option(id: &str, label: &str, pos: i32) -> ProposalOption {
    ProposalOption {
        id: id.to_string(),
        proposal_id: "prop-1".to_string(),
        label: label.to_string(),
        description: String::new(),
        position: pos,
        source: None,
        source_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn make_ranked_vote(human: &str, option: &str, rank: i32) -> RankedVote {
    RankedVote {
        id: format!("rv-{}-{}", human, option),
        proposal_id: "prop-1".to_string(),
        human_id: human.to_string(),
        option_id: option.to_string(),
        rank: Some(rank),
        score: None,
        dots: None,
        approved: None,
        reasoning: None,
        proxy_elohim_id: None,
        proxy_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn make_approval_vote(human: &str, option: &str, approved: bool) -> RankedVote {
    RankedVote {
        id: format!("av-{}-{}", human, option),
        proposal_id: "prop-1".to_string(),
        human_id: human.to_string(),
        option_id: option.to_string(),
        rank: None,
        score: None,
        dots: None,
        approved: Some(if approved { 1 } else { 0 }),
        reasoning: None,
        proxy_elohim_id: None,
        proxy_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn make_score_vote(human: &str, option: &str, score: i32) -> RankedVote {
    RankedVote {
        id: format!("sv-{}-{}", human, option),
        proposal_id: "prop-1".to_string(),
        human_id: human.to_string(),
        option_id: option.to_string(),
        rank: None,
        score: Some(score),
        dots: None,
        approved: None,
        reasoning: None,
        proxy_elohim_id: None,
        proxy_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn make_dot_vote(human: &str, option: &str, dots: i32) -> RankedVote {
    RankedVote {
        id: format!("dv-{}-{}", human, option),
        proposal_id: "prop-1".to_string(),
        human_id: human.to_string(),
        option_id: option.to_string(),
        rank: None,
        score: None,
        dots: Some(dots),
        approved: None,
        reasoning: None,
        proxy_elohim_id: None,
        proxy_justification: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn default_config() -> VotingConfig {
    VotingConfig {
        score_min: None,
        score_max: None,
        dots_per_voter: None,
        quorum_percentage: None,
        passage_threshold: None,
    }
}

// ============================================================================
// Ranked-Choice (IRV)
// ============================================================================

#[test]
fn ranked_choice_clear_majority_first_round() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
        make_option("c", "Option C", 3),
    ];
    // 3 out of 5 voters rank A first
    let votes = vec![
        make_ranked_vote("h1", "a", 1),
        make_ranked_vote("h2", "a", 1),
        make_ranked_vote("h3", "a", 1),
        make_ranked_vote("h4", "b", 1),
        make_ranked_vote("h5", "c", 1),
    ];
    let strategy = ranked_choice::RankedChoiceTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.mechanism, "ranked-choice");
    assert_eq!(result.total_voters, 5);
    assert!(result.quorum_met);
    assert_eq!(result.recommendation, "pass");

    // A should be ranked #1 with 3 votes (60%)
    let top = &result.option_results[0];
    assert_eq!(top.option_id, "a");
    assert_eq!(top.votes, 3.0);
    assert!((top.percentage - 60.0).abs() < 0.01);

    // Should resolve in round 1 (majority found immediately)
    let rounds = result.rounds.as_ref().unwrap();
    assert!(!rounds.is_empty());
}

#[test]
fn ranked_choice_elimination_redistributes() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
        make_option("c", "Option C", 3),
    ];
    // A=2, B=2, C=1 first-choice votes
    // C's voter has B as second choice -> after C eliminated, B gets 3 votes
    let votes = vec![
        make_ranked_vote("h1", "a", 1),
        make_ranked_vote("h2", "a", 1),
        make_ranked_vote("h3", "b", 1),
        make_ranked_vote("h4", "b", 1),
        make_ranked_vote("h5", "c", 1),
        make_ranked_vote("h5", "b", 2), // C-voter's second choice is B
    ];
    let strategy = ranked_choice::RankedChoiceTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.total_voters, 5);
    assert_eq!(result.recommendation, "pass");

    // B should win after redistribution
    let top = &result.option_results[0];
    assert_eq!(top.option_id, "b");
    assert_eq!(top.votes, 3.0);

    // Should have multiple rounds (C eliminated in round 1)
    let rounds = result.rounds.as_ref().unwrap();
    assert!(rounds.len() >= 2);
    assert_eq!(rounds[0].eliminated_option_id.as_deref(), Some("c"));
}

#[test]
fn ranked_choice_empty_votes() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
    ];
    let votes: Vec<RankedVote> = vec![];
    let strategy = ranked_choice::RankedChoiceTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.total_voters, 0);
}

// ============================================================================
// Approval
// ============================================================================

#[test]
fn approval_highest_approved_wins() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
        make_option("c", "Option C", 3),
    ];
    // 3 voters: all approve B, only 1 approves A, 2 approve C
    let votes = vec![
        make_approval_vote("h1", "a", true),
        make_approval_vote("h1", "b", true),
        make_approval_vote("h2", "b", true),
        make_approval_vote("h2", "c", true),
        make_approval_vote("h3", "b", true),
        make_approval_vote("h3", "c", true),
    ];
    let strategy = approval::ApprovalTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.mechanism, "approval");
    assert_eq!(result.total_voters, 3);
    assert!(result.quorum_met);
    assert_eq!(result.recommendation, "pass");

    // B should be ranked #1 with 3 approvals (100%)
    let top = &result.option_results[0];
    assert_eq!(top.option_id, "b");
    assert_eq!(top.votes, 3.0);
    assert!((top.percentage - 100.0).abs() < 0.01);
}

// ============================================================================
// Score
// ============================================================================

#[test]
fn score_highest_total_wins() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
    ];
    // A total = 4+7 = 11, B total = 8+7 = 15
    let votes = vec![
        make_score_vote("h1", "a", 4),
        make_score_vote("h1", "b", 8),
        make_score_vote("h2", "a", 7),
        make_score_vote("h2", "b", 7),
    ];
    let strategy = score::ScoreTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.mechanism, "score-vote");
    assert_eq!(result.total_voters, 2);
    assert_eq!(result.recommendation, "pass");

    // B should win with 15 total score
    let top = &result.option_results[0];
    assert_eq!(top.option_id, "b");
    assert_eq!(top.votes, 15.0);

    // A should be second with 11
    let second = &result.option_results[1];
    assert_eq!(second.option_id, "a");
    assert_eq!(second.votes, 11.0);
}

#[test]
fn score_validates_range() {
    let options = vec![make_option("a", "Option A", 1)];
    let votes = vec![make_score_vote("h1", "a", 7)];
    let config = VotingConfig {
        score_min: Some(0),
        score_max: Some(5),
        dots_per_voter: None,
        quorum_percentage: None,
        passage_threshold: None,
    };
    let strategy = score::ScoreTally;
    let err = strategy.validate_ballot(&votes, &options, &config);

    assert!(err.is_err());
    match err.unwrap_err() {
        BallotError::ScoreOutOfRange { option_id, score } => {
            assert_eq!(option_id, "a");
            assert_eq!(score, 7);
        }
        other => panic!("Expected ScoreOutOfRange, got: {:?}", other),
    }
}

// ============================================================================
// Dot
// ============================================================================

#[test]
fn dot_distributes_budget() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
    ];
    // Voter 1: A=3, B=4; Voter 2: A=6, B=7  -> A=9, B=11
    let votes = vec![
        make_dot_vote("h1", "a", 3),
        make_dot_vote("h1", "b", 4),
        make_dot_vote("h2", "a", 6),
        make_dot_vote("h2", "b", 7),
    ];
    let strategy = dot::DotTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.mechanism, "dot-vote");
    assert_eq!(result.total_voters, 2);
    assert_eq!(result.recommendation, "pass");

    // B wins with 11 dots
    let top = &result.option_results[0];
    assert_eq!(top.option_id, "b");
    assert_eq!(top.votes, 11.0);

    // A second with 9 dots
    let second = &result.option_results[1];
    assert_eq!(second.option_id, "a");
    assert_eq!(second.votes, 9.0);
}

#[test]
fn dot_validates_budget() {
    let options = vec![
        make_option("a", "Option A", 1),
        make_option("b", "Option B", 2),
    ];
    // Voter uses 7 dots total but budget is 5
    let votes = vec![
        make_dot_vote("h1", "a", 3),
        make_dot_vote("h1", "b", 4),
    ];
    let config = VotingConfig {
        score_min: None,
        score_max: None,
        dots_per_voter: Some(5),
        quorum_percentage: None,
        passage_threshold: None,
    };
    let strategy = dot::DotTally;
    let err = strategy.validate_ballot(&votes, &options, &config);

    assert!(err.is_err());
    match err.unwrap_err() {
        BallotError::DotsExceedBudget { used, budget } => {
            assert_eq!(used, 7);
            assert_eq!(budget, 5);
        }
        other => panic!("Expected DotsExceedBudget, got: {:?}", other),
    }
}

// ============================================================================
// Consent
// ============================================================================

#[test]
fn consent_passes_without_blocks() {
    let options = vec![make_option("a", "Proposal A", 1)];
    // All 3 voters consent
    let votes = vec![
        make_approval_vote("h1", "a", true),
        make_approval_vote("h2", "a", true),
        make_approval_vote("h3", "a", true),
    ];
    let strategy = consent::ConsentTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.mechanism, "consent");
    assert_eq!(result.total_voters, 3);
    assert!(result.quorum_met);
    assert_eq!(result.recommendation, "pass");
    assert!(!result.option_results[0].eliminated);
}

#[test]
fn consent_blocked_triggers_escalation() {
    let options = vec![make_option("a", "Proposal A", 1)];
    // 2 consent, 1 blocks
    let votes = vec![
        make_approval_vote("h1", "a", true),
        make_approval_vote("h2", "a", true),
        make_approval_vote("h3", "a", false), // block
    ];
    let strategy = consent::ConsentTally;
    let result = strategy.tally(&votes, &options, &default_config());

    assert_eq!(result.mechanism, "consent");
    assert_eq!(result.total_voters, 3);
    assert_eq!(result.recommendation, "blocked");
    assert!(result.option_results[0].eliminated); // eliminated = has blocks
}

// ============================================================================
// Registry
// ============================================================================

#[test]
fn get_strategy_returns_all_known_mechanisms() {
    let known = [
        "ranked-choice",
        "approval",
        "score-vote",
        "dot-vote",
        "consent",
        "conviction",
    ];
    for mechanism in &known {
        assert!(
            get_strategy(mechanism).is_some(),
            "get_strategy({}) should return Some",
            mechanism
        );
    }
    assert!(get_strategy("unknown").is_none());
}
