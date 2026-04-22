//! Disposition computation service
//!
//! Computes a governance disposition profile for a human based on their
//! voting history, challenges filed, and governance signals.
//!
//! Classification: B (Agent-Scoped). The disposition is a private governance
//! profile stored on the agent's source chain. NOT published to DHT.

use diesel::prelude::*;
use std::collections::HashMap;

use crate::db::governance;
use crate::db::models::{GovernanceDisposition, NewGovernanceDisposition};
use crate::error::StorageError;

/// Compute (or recompute) a governance disposition for a human.
///
/// Rule-based computation:
/// 1. Count ranked_votes for this human -> total_votes_cast
/// 2. Count challenges where challenger_id = human_id -> total_challenges_filed
/// 3. Count governance_signals where human_id matches -> total_signals_recorded
/// 4. Group votes by mechanism -> voting_pattern_summary JSON
/// 5. Risk tolerance: more challenges filed -> higher
/// 6. Change openness: more votes cast (engagement) -> higher
/// 7. Consensus preference: proportion of consent mechanism votes vs competitive votes
/// 8. Priority values: extract from challenge grounds_primary (most frequent grounds)
pub fn compute_disposition(
    conn: &mut SqliteConnection,
    human_id: &str,
) -> Result<GovernanceDisposition, StorageError> {
    // 1. Count total votes
    let total_votes_cast = governance::count_ranked_votes_for_human(conn, human_id)?;

    // 2. Count challenges filed
    let total_challenges_filed = governance::count_challenges_for_human(conn, human_id)?;

    // 3. Count governance signals
    let total_signals_recorded = governance::count_signals_for_human(conn, human_id)?;

    // 4. Group votes by mechanism (via proposal lookup)
    let all_votes = governance::get_all_ranked_votes_for_human(conn, human_id)?;
    let mut mechanism_counts: HashMap<String, i64> = HashMap::new();
    let abstention_count: i64 = 0;
    let mut consent_mechanism_votes: i64 = 0;
    let mut competitive_mechanism_votes: i64 = 0;

    // Group by proposal_id to get unique proposals voted on
    let mut proposal_ids: Vec<String> = all_votes.iter().map(|v| v.proposal_id.clone()).collect();
    proposal_ids.sort();
    proposal_ids.dedup();

    for pid in &proposal_ids {
        if let Ok(Some(proposal)) = governance::get_proposal(conn, pid) {
            *mechanism_counts
                .entry(proposal.voting_mechanism.clone())
                .or_insert(0) += 1;

            // Categorize: consent-based vs competitive mechanisms
            match proposal.voting_mechanism.as_str() {
                "consent" | "approval" => consent_mechanism_votes += 1,
                "ranked-choice" | "score" | "dot" | "plurality" => competitive_mechanism_votes += 1,
                _ => {}
            }
        }
    }

    let voting_pattern_summary = serde_json::to_string(&mechanism_counts).unwrap_or_default();

    // 5. Risk tolerance: more challenges filed -> higher risk tolerance
    // Normalize: challenges / (challenges + votes + 1)
    let risk_tolerance = total_challenges_filed as f64
        / (total_challenges_filed as f64 + total_votes_cast as f64 + 1.0);

    // 6. Change openness: more votes cast (engagement) -> higher
    // Normalize: votes / (votes + abstentions + 1)
    // We approximate abstentions as proposals where the human had signals but didn't vote
    let _ = abstention_count; // TODO: proper abstention tracking
    let change_openness =
        total_votes_cast as f64 / (total_votes_cast as f64 + abstention_count as f64 + 1.0);

    // 7. Consensus preference: proportion of consent mechanism votes vs competitive
    let total_mechanism_votes = consent_mechanism_votes + competitive_mechanism_votes;
    let consensus_preference = if total_mechanism_votes > 0 {
        consent_mechanism_votes as f64 / total_mechanism_votes as f64
    } else {
        0.5 // Default neutral when no data
    };

    // 8. Priority values: extract from challenge grounds_primary (most frequent grounds)
    let challenges = governance::get_challenges_by_human(conn, human_id)?;
    let mut grounds_counts: HashMap<String, i64> = HashMap::new();
    for challenge in &challenges {
        *grounds_counts
            .entry(challenge.grounds_primary.clone())
            .or_insert(0) += 1;
    }
    // Sort by frequency descending, take top 5
    let mut grounds_vec: Vec<(String, i64)> = grounds_counts.into_iter().collect();
    grounds_vec.sort_by_key(|x| std::cmp::Reverse(x.1));
    let priority_values: Vec<String> = grounds_vec
        .into_iter()
        .take(5)
        .map(|(ground, _)| ground)
        .collect();
    let priority_values_json = serde_json::to_string(&priority_values).unwrap_or_default();

    // Upsert the computed disposition
    let now = crate::db::models::current_timestamp();
    let disposition_id = format!("disp-{}", human_id);

    let new = NewGovernanceDisposition {
        id: &disposition_id,
        human_id,
        risk_tolerance: risk_tolerance as f32,
        change_openness: change_openness as f32,
        consensus_preference: consensus_preference as f32,
        priority_values: &priority_values_json,
        voting_pattern_summary: &voting_pattern_summary,
        total_votes_cast: total_votes_cast as i32,
        total_challenges_filed: total_challenges_filed as i32,
        total_signals_recorded: total_signals_recorded as i32,
        last_computed_at: &now,
        created_at: &now,
        updated_at: &now,
    };

    governance::upsert_disposition(conn, &new)
}
