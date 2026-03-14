//! Steward affinity service — mastery gate and curation mutation logic
//!
//! The mastery gate ensures only learners who have demonstrated mastery
//! can build stewardship standing through curation acts.

use diesel::SqliteConnection;

use crate::db::content_mastery::{self, MasteryQuery};
use crate::db::steward_affinity::{self, CreateAffinityInput};
use crate::db::AppContext;
use crate::error::StorageError;

/// Mastery level index threshold for stewardship eligibility.
/// APPLY (index 4) = you can apply the knowledge = ready to steward.
const MASTERY_GATE_THRESHOLD: i32 = 4;

/// Check if a human has reached mastery on content (or any child content).
/// Returns true if any mastery record for this human+content is at or above APPLY level.
pub fn check_mastery_gate(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    content_id: &str,
) -> Result<bool, StorageError> {
    let mastery_records = content_mastery::list_mastery(
        conn,
        ctx,
        &MasteryQuery {
            human_id: Some(human_id.to_string()),
            content_id: Some(content_id.to_string()),
            ..Default::default()
        },
    )?;

    Ok(mastery_records
        .iter()
        .any(|m| m.mastery_level_index >= MASTERY_GATE_THRESHOLD))
}

/// Curation activity types and their affinity deltas
pub fn curation_delta(activity_type: &str) -> Option<f32> {
    match activity_type {
        "edit" => Some(0.10),
        "review" => Some(0.05),
        "dispute_resolution" => Some(0.15),
        _ => None,
    }
}

/// Record a curation activity and update steward affinity.
/// Returns error if mastery gate is not met.
pub fn record_curation_activity(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    steward_id: &str,
    content_id: &str,
    activity_type: &str,
) -> Result<crate::db::models::StewardAffinity, StorageError> {
    // Validate activity type
    let delta = curation_delta(activity_type).ok_or_else(|| {
        StorageError::InvalidInput(format!("Unknown curation activity type: {}", activity_type))
    })?;

    // Check mastery gate
    if !check_mastery_gate(conn, ctx, steward_id, content_id)? {
        return Err(StorageError::Forbidden(format!(
            "Mastery gate not met: {} has not reached mastery on content {}",
            steward_id, content_id
        )));
    }

    // Map activity to affinity source
    let source = match activity_type {
        "edit" => "curation_edit",
        "review" => "curation_review",
        "dispute_resolution" => "dispute_resolution",
        _ => "curation_edit",
    };

    // Check if affinity record exists; create if first curation act
    let existing =
        steward_affinity::get_affinity_for_steward_content(conn, ctx, steward_id, content_id)?;

    if existing.is_some() {
        steward_affinity::update_affinity_score(conn, ctx, steward_id, content_id, delta, source)
    } else {
        // First curation act after mastery — create initial affinity
        let input = CreateAffinityInput {
            steward_id: steward_id.to_string(),
            content_id: content_id.to_string(),
            affinity_score: delta,
            source: source.to_string(),
        };
        steward_affinity::create_affinity(conn, ctx, &input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curation_delta_edit() {
        assert!((curation_delta("edit").unwrap() - 0.10).abs() < f32::EPSILON);
    }

    #[test]
    fn curation_delta_review() {
        assert!((curation_delta("review").unwrap() - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn curation_delta_dispute() {
        assert!((curation_delta("dispute_resolution").unwrap() - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn curation_delta_unknown_returns_none() {
        assert!(curation_delta("unknown").is_none());
    }
}
