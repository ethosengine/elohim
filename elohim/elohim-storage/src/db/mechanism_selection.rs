//! MechanismSelection projection — M-POLICY-2.
//!
//! Source of truth: derived projection of GovernanceState × Content.contentType
//! × Proposal? × Manifest(kind: pillar-projection, pillar: qahal).
//! Category C operational — reconstructable at any time.
//!
//! The projection is recomputed on four triggers:
//! 1. `Signal::GovernanceStateUpdated` — governance state changed for entity
//! 2. `Signal::ContentCreated/Updated` — content type changed for entity
//! 3. `Signal::ProposalCreated/Closed` — active proposal appeared or closed
//! 4. `Signal::ManifestUpdated{kind: "pillar-projection", pillar: "qahal"}` —
//!    mechanism ladder policy changed; all rows recomputed against new ladder.
//!
//! Protocol-default mechanism ladder (mirrors the client-side constants from
//! mechanism-selection.service.ts that are being retired by this ticket):
//! - settled_states: ["constitutional", "settled"]
//! - mechanism_level_map: {approval:3, dot-vote:3, ranked-choice:4,
//!   score-vote:5, conviction:5, consent:6}
//! - feedback_inviting_content_types: ["discussion", "proposal-draft",
//!   "request-for-comment", "reflection"]
//! - default_level_no_proposal: 1 (reactions only)

use diesel::prelude::*;
use serde_json::Value;

use crate::db::diesel_schema::governance_states;
use crate::db::diesel_schema::manifests;
use crate::db::diesel_schema::mechanism_selection;
use crate::db::diesel_schema::proposals;
use crate::error::StorageError;
use elohim_views::MechanismSelectionView;

// ---------------------------------------------------------------------------
// Protocol-default mechanism ladder constants
// (mirrors the retired client-side constants)
// ---------------------------------------------------------------------------

const DEFAULT_SETTLED_STATES: &[&str] = &["constitutional", "settled"];
const DEFAULT_FEEDBACK_INVITING_TYPES: &[&str] = &[
    "discussion",
    "proposal-draft",
    "request-for-comment",
    "reflection",
];
const DEFAULT_LEVEL_NO_PROPOSAL: i32 = 1; // reactions only

/// Default mechanism level for a given votingMechanism string.
/// Returns 3 (approval-level) for unknown mechanisms.
fn default_mechanism_level(voting_mechanism: &str) -> i32 {
    match voting_mechanism {
        "approval" | "dot-vote" => 3,
        "ranked-choice" => 4,
        "score-vote" | "conviction" => 5,
        "consent" => 6,
        _ => 3,
    }
}

// ---------------------------------------------------------------------------
// Policy parsing from pillar-projection Manifest
// ---------------------------------------------------------------------------

struct MechanismLadder {
    settled_states: Vec<String>,
    mechanism_level_map: std::collections::HashMap<String, i32>,
    feedback_inviting_types: Vec<String>,
    default_level_no_proposal: i32,
}

impl Default for MechanismLadder {
    fn default() -> Self {
        Self {
            settled_states: DEFAULT_SETTLED_STATES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            mechanism_level_map: {
                let mut m = std::collections::HashMap::new();
                m.insert("approval".to_string(), 3);
                m.insert("dot-vote".to_string(), 3);
                m.insert("ranked-choice".to_string(), 4);
                m.insert("score-vote".to_string(), 5);
                m.insert("conviction".to_string(), 5);
                m.insert("consent".to_string(), 6);
                m
            },
            feedback_inviting_types: DEFAULT_FEEDBACK_INVITING_TYPES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            default_level_no_proposal: DEFAULT_LEVEL_NO_PROPOSAL,
        }
    }
}

/// Parse the mechanism_ladder from a pillar-projection Manifest payload_json.
/// Returns protocol defaults when the payload is missing or malformed.
fn parse_mechanism_ladder(payload_json: &str) -> MechanismLadder {
    let defaults = MechanismLadder::default();
    let Ok(payload) = serde_json::from_str::<Value>(payload_json) else {
        return defaults;
    };
    let Some(ladder) = payload.get("mechanism_ladder") else {
        return defaults;
    };

    let settled_states: Vec<String> = ladder
        .get("settled_states")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| defaults.settled_states.clone());

    let mechanism_level_map: std::collections::HashMap<String, i32> = ladder
        .get("mechanism_level_map")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_i64().map(|n| (k.clone(), n as i32)))
                .collect()
        })
        .filter(|m: &std::collections::HashMap<String, i32>| !m.is_empty())
        .unwrap_or_else(|| defaults.mechanism_level_map.clone());

    let feedback_inviting_types: Vec<String> = ladder
        .get("feedback_inviting_content_types")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| defaults.feedback_inviting_types.clone());

    let default_level_no_proposal: i32 = ladder
        .get("default_level_no_proposal")
        .and_then(|v| v.as_i64())
        .map(|n| n as i32)
        .unwrap_or(defaults.default_level_no_proposal);

    MechanismLadder {
        settled_states,
        mechanism_level_map,
        feedback_inviting_types,
        default_level_no_proposal,
    }
}

// ---------------------------------------------------------------------------
// Diesel row types
// ---------------------------------------------------------------------------

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = mechanism_selection)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct MechanismSelectionRow {
    pub entity_type: String,
    pub entity_id: String,
    pub level: i32,
    pub mechanism: String,
    pub render_target: String,
    pub context_menu_only: i32,
    pub allow_reactions: i32,
    pub allow_graduated_feedback: i32,
    pub active_proposal_id: Option<String>,
    pub active_proposal_mechanism: Option<String>,
    pub policy_manifest_cid: Option<String>,
    pub computed_at: String,
}

#[derive(Insertable, AsChangeset, Debug, Clone)]
#[diesel(table_name = mechanism_selection)]
pub struct UpsertMechanismSelection<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub level: i32,
    pub mechanism: &'a str,
    pub render_target: &'a str,
    pub context_menu_only: i32,
    pub allow_reactions: i32,
    pub allow_graduated_feedback: i32,
    pub active_proposal_id: Option<&'a str>,
    pub active_proposal_mechanism: Option<&'a str>,
    pub policy_manifest_cid: Option<&'a str>,
    pub computed_at: &'a str,
}

impl From<MechanismSelectionRow> for MechanismSelectionView {
    fn from(r: MechanismSelectionRow) -> Self {
        Self {
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            level: r.level,
            mechanism: r.mechanism,
            render_target: r.render_target,
            context_menu_only: r.context_menu_only == 1,
            allow_reactions: r.allow_reactions == 1,
            allow_graduated_feedback: r.allow_graduated_feedback == 1,
            active_proposal_id: r.active_proposal_id,
            active_proposal_mechanism: r.active_proposal_mechanism,
            policy_manifest_cid: r.policy_manifest_cid,
            computed_at: r.computed_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Core projection logic
// ---------------------------------------------------------------------------

/// Governance state row shape we need from the DB.
#[derive(Queryable, Debug)]
struct GovernanceStateSlim {
    voting_state: String,
}

/// Proposal row shape — minimal fields for mechanism derivation.
#[derive(Queryable, Debug)]
struct ProposalSlim {
    id: String,
    voting_mechanism: String,
    // Queried for symmetry with the proposals table shape but not yet consumed
    // by the mechanism-derivation logic. Retain the field so the row decodes
    // against the live schema; mark unread to satisfy clippy.
    #[allow(dead_code)]
    status: String,
}

/// Compute the MechanismSelectionView for (entity_type, entity_id).
///
/// Rules (mirroring the retired client-side MechanismSelectionService):
/// 1. If GovernanceState.votingState is in settled_states → level 0 (context menu only)
/// 2. If there is an active Proposal → level from mechanism_level_map (3–7)
/// 3. If contentType is in feedback_inviting_content_types → level 2 (graduated feedback)
/// 4. Otherwise → default_level_no_proposal (default: 1, reactions only)
///
/// For levels 0–2: renderTarget = "angular".
/// For levels 3–7: renderTarget = "psephos".
pub fn project_mechanism_selection(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<MechanismSelectionView, StorageError> {
    let now = chrono::Utc::now().to_rfc3339();

    // 1. Load the most recent pillar-projection Manifest for qahal (if any).
    let policy_row = manifests::table
        .filter(manifests::manifest_kind.eq("pillar-projection"))
        .filter(manifests::pillar.eq("qahal"))
        .order(manifests::revision.desc())
        .first::<crate::db::manifests::ManifestRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("mechanism_selection manifest load: {e}")))?;

    let (ladder, policy_cid) = match &policy_row {
        Some(row) => (
            parse_mechanism_ladder(&row.payload_json),
            Some(row.cid.as_str()),
        ),
        None => (MechanismLadder::default(), None),
    };

    // 2. Load the GovernanceState for this entity (if any).
    let gov_state: Option<GovernanceStateSlim> = governance_states::table
        .filter(governance_states::entity_type.eq(entity_type))
        .filter(governance_states::entity_id.eq(entity_id))
        .select(governance_states::voting_state)
        .first::<String>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Database(format!("mechanism_selection governance_state load: {e}"))
        })?
        .map(|voting_state| GovernanceStateSlim { voting_state });

    // Rule 1: No governance state, or votingState in settled_states → level 0
    if let Some(ref state) = gov_state {
        if ladder
            .settled_states
            .iter()
            .any(|s| s == &state.voting_state)
        {
            return upsert_and_return(
                conn,
                entity_type,
                entity_id,
                0,
                "none",
                "angular",
                true,
                false,
                false,
                None,
                None,
                policy_cid,
                &now,
            );
        }
    } else {
        // No governance state at all → level 0 (untracked content)
        return upsert_and_return(
            conn,
            entity_type,
            entity_id,
            0,
            "none",
            "angular",
            true,
            false,
            false,
            None,
            None,
            policy_cid,
            &now,
        );
    }

    // 3. Check for an active Proposal on this entity (content_id = entity_id).
    //    "Active" = status NOT IN ('rejected', 'withdrawn', 'superseded', 'closed').
    let active_proposal: Option<ProposalSlim> = proposals::table
        .filter(proposals::content_id.eq(entity_id))
        .filter(proposals::status.ne("rejected"))
        .filter(proposals::status.ne("withdrawn"))
        .filter(proposals::status.ne("superseded"))
        .filter(proposals::status.ne("closed"))
        .select((
            proposals::id,
            proposals::voting_mechanism,
            proposals::status,
        ))
        .first::<(String, String, String)>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("mechanism_selection proposal load: {e}")))?
        .map(|(id, voting_mechanism, status)| ProposalSlim {
            id,
            voting_mechanism,
            status,
        });

    // Rule 2: Active proposal → level from mechanism_level_map (3–7), renderTarget = "psephos"
    if let Some(ref proposal) = active_proposal {
        let level = ladder
            .mechanism_level_map
            .get(&proposal.voting_mechanism)
            .copied()
            .unwrap_or_else(|| default_mechanism_level(&proposal.voting_mechanism));
        return upsert_and_return(
            conn,
            entity_type,
            entity_id,
            level,
            &proposal.voting_mechanism.clone(),
            "psephos",
            false,
            true,
            true,
            Some(proposal.id.as_str()),
            Some(proposal.voting_mechanism.as_str()),
            policy_cid,
            &now,
        );
    }

    // 4. Content type check. The entity_id is used as content_id in the content table.
    //    Load contentType from the content table if available.
    let content_type: Option<String> = {
        use crate::db::diesel_schema::content;
        content::table
            .filter(content::id.eq(entity_id))
            .select(content::content_type)
            .first::<String>(conn)
            .optional()
            .map_err(|e| {
                StorageError::Database(format!("mechanism_selection content_type load: {e}"))
            })?
    };

    // Rule 3: Content type in feedback_inviting_content_types → level 2 (graduated feedback)
    if let Some(ref ct) = content_type {
        if ladder.feedback_inviting_types.iter().any(|t| t == ct) {
            return upsert_and_return(
                conn,
                entity_type,
                entity_id,
                2,
                "graduated-feedback",
                "angular",
                false,
                true,
                true,
                None,
                None,
                policy_cid,
                &now,
            );
        }
    }

    // Rule 4: Default level (usually 1 = reactions only)
    let level = ladder.default_level_no_proposal;
    let mechanism = if level <= 1 {
        "reactions"
    } else {
        "graduated-feedback"
    };
    upsert_and_return(
        conn,
        entity_type,
        entity_id,
        level,
        mechanism,
        "angular",
        false,
        level >= 1,
        level >= 2,
        None,
        None,
        policy_cid,
        &now,
    )
}

/// Upsert a mechanism_selection row and return the view.
#[allow(clippy::too_many_arguments)]
fn upsert_and_return(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
    level: i32,
    mechanism: &str,
    render_target: &str,
    context_menu_only: bool,
    allow_reactions: bool,
    allow_graduated_feedback: bool,
    active_proposal_id: Option<&str>,
    active_proposal_mechanism: Option<&str>,
    policy_manifest_cid: Option<&str>,
    computed_at: &str,
) -> Result<MechanismSelectionView, StorageError> {
    let row = UpsertMechanismSelection {
        entity_type,
        entity_id,
        level,
        mechanism,
        render_target,
        context_menu_only: context_menu_only as i32,
        allow_reactions: allow_reactions as i32,
        allow_graduated_feedback: allow_graduated_feedback as i32,
        active_proposal_id,
        active_proposal_mechanism,
        policy_manifest_cid,
        computed_at,
    };

    diesel::insert_into(mechanism_selection::table)
        .values(&row)
        .on_conflict((
            mechanism_selection::entity_type,
            mechanism_selection::entity_id,
        ))
        .do_update()
        .set(&row)
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("mechanism_selection upsert: {e}")))?;

    Ok(MechanismSelectionView {
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        level,
        mechanism: mechanism.to_string(),
        render_target: render_target.to_string(),
        context_menu_only,
        allow_reactions,
        allow_graduated_feedback,
        active_proposal_id: active_proposal_id.map(String::from),
        active_proposal_mechanism: active_proposal_mechanism.map(String::from),
        policy_manifest_cid: policy_manifest_cid.map(String::from),
        computed_at: computed_at.to_string(),
    })
}

/// Fetch the pre-computed mechanism_selection row for (entity_type, entity_id).
///
/// Returns None if no row has been projected yet. The HTTP route calls
/// `project_mechanism_selection` directly for fresh results; this is available
/// for callers that want to avoid re-computation when the row is recent.
pub fn fetch_mechanism_selection(
    conn: &mut SqliteConnection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Option<MechanismSelectionView>, StorageError> {
    mechanism_selection::table
        .filter(mechanism_selection::entity_type.eq(entity_type))
        .filter(mechanism_selection::entity_id.eq(entity_id))
        .first::<MechanismSelectionRow>(conn)
        .optional()
        .map(|opt| opt.map(MechanismSelectionView::from))
        .map_err(|e| StorageError::Database(format!("mechanism_selection fetch: {e}")))
}

/// Recompute mechanism_selection for ALL (entity_type, entity_id) pairs in the table.
///
/// Triggered on `Signal::ManifestUpdated{kind: "pillar-projection", pillar: "qahal"}`
/// so that every projected row is updated when the mechanism ladder policy changes.
pub fn reproject_all_mechanism_selections(
    conn: &mut SqliteConnection,
) -> Result<usize, StorageError> {
    // Load all existing entity pairs to recompute.
    let pairs: Vec<(String, String)> = mechanism_selection::table
        .select((
            mechanism_selection::entity_type,
            mechanism_selection::entity_id,
        ))
        .load::<(String, String)>(conn)
        .map_err(|e| {
            StorageError::Database(format!("mechanism_selection list for reproject: {e}"))
        })?;

    let count = pairs.len();
    for (entity_type, entity_id) in pairs {
        project_mechanism_selection(conn, &entity_type, &entity_id)?;
    }
    Ok(count)
}

// ---------------------------------------------------------------------------
// Protocol-default logic unit tests (no conductor required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ladder_has_two_settled_states() {
        let ladder = MechanismLadder::default();
        assert!(
            ladder
                .settled_states
                .contains(&"constitutional".to_string()),
            "constitutional must be a settled state"
        );
        assert!(
            ladder.settled_states.contains(&"settled".to_string()),
            "settled must be a settled state"
        );
        assert_eq!(
            ladder.settled_states.len(),
            2,
            "protocol default has exactly 2 settled states"
        );
    }

    #[test]
    fn default_mechanism_level_map_matches_retired_client_constants() {
        // Mirrors MECHANISM_LEVEL_MAP from mechanism-selection.service.ts:
        //   approval: 3, dot-vote: 3, ranked-choice: 4,
        //   score-vote: 5, conviction: 5, consent: 6
        let ladder = MechanismLadder::default();
        assert_eq!(ladder.mechanism_level_map["approval"], 3);
        assert_eq!(ladder.mechanism_level_map["dot-vote"], 3);
        assert_eq!(ladder.mechanism_level_map["ranked-choice"], 4);
        assert_eq!(ladder.mechanism_level_map["score-vote"], 5);
        assert_eq!(ladder.mechanism_level_map["conviction"], 5);
        assert_eq!(ladder.mechanism_level_map["consent"], 6);
    }

    #[test]
    fn default_feedback_inviting_types_match_retired_client_constants() {
        // Mirrors FEEDBACK_CONTENT_TYPES from mechanism-selection.service.ts:
        //   discussion, proposal-draft, request-for-comment, reflection
        let ladder = MechanismLadder::default();
        assert!(ladder
            .feedback_inviting_types
            .contains(&"discussion".to_string()));
        assert!(ladder
            .feedback_inviting_types
            .contains(&"proposal-draft".to_string()));
        assert!(ladder
            .feedback_inviting_types
            .contains(&"request-for-comment".to_string()));
        assert!(ladder
            .feedback_inviting_types
            .contains(&"reflection".to_string()));
        assert_eq!(
            ladder.feedback_inviting_types.len(),
            4,
            "protocol default has exactly 4 feedback-inviting content types"
        );
    }

    #[test]
    fn parse_mechanism_ladder_uses_defaults_on_empty_payload() {
        let ladder = parse_mechanism_ladder(r#"{"some_other_field": true}"#);
        // Should fall back to defaults when mechanism_ladder key is absent
        assert_eq!(ladder.settled_states.len(), 2);
        assert_eq!(ladder.mechanism_level_map["approval"], 3);
    }

    #[test]
    fn parse_mechanism_ladder_reads_custom_payload() {
        let payload = serde_json::json!({
            "mechanism_ladder": {
                "settled_states": ["constitutional", "settled", "archived"],
                "mechanism_level_map": {
                    "approval": 3,
                    "custom-mechanism": 7
                },
                "feedback_inviting_content_types": ["discussion", "custom-type"],
                "default_level_no_proposal": 2
            }
        })
        .to_string();

        let ladder = parse_mechanism_ladder(&payload);
        assert_eq!(ladder.settled_states.len(), 3);
        assert!(ladder.settled_states.contains(&"archived".to_string()));
        assert_eq!(ladder.mechanism_level_map["custom-mechanism"], 7);
        assert!(ladder
            .feedback_inviting_types
            .contains(&"custom-type".to_string()));
        assert_eq!(ladder.default_level_no_proposal, 2);
    }

    #[test]
    fn default_mechanism_level_fallback_returns_3() {
        // Unknown mechanism defaults to level 3 (approval-equivalent)
        assert_eq!(default_mechanism_level("unknown-mechanism"), 3);
        assert_eq!(default_mechanism_level(""), 3);
    }
}
