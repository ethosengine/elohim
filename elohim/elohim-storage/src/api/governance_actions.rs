//! Governance-action API controller
//!
//! Routes:
//! - `GET  /api/v1/governance-actions`               — list by subjectCid or open
//! - `GET  /api/v1/governance-actions/{id}`          — fetch parent + children + tally
//! - `GET  /api/v1/governance-actions/{id}/tally`    — fetch tally only
//! - `POST /api/v1/governance-actions/{id}/vote`     — record a vote (upsert attestation + recompute tally)
//!
//! Source of truth: Holochain DHT (Category A for governance-actions and attestation votes).
//! The tally is Category C (operational local projection) — recomputable from parent + children.
//!
//! ## Design note
//! These handlers operate on the projection layer only. They do NOT call the Holochain
//! conductor — that path goes through the Holochain signal pipeline:
//!   zome fn → post-commit signal → AttestationProjector.handle_signal → db upsert
//! The HTTP handlers here serve reads and provide an admin path for direct projection
//! upserts (e.g. during signal replay or recovery).

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::{Deserialize, Serialize};

use crate::db::{attestations as attestation_db, governance_action_tally, governance_actions, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::{response, tally_projector};
use crate::views::{AttestationView, GovernanceActionTallyView, GovernanceActionView};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Query param types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GovernanceActionQuery {
    /// Filter by subject CID.
    pub subject_cid: Option<String>,
    /// If "true", return only open actions (closes_at > now).
    pub open_only: Option<String>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn extract_id(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let id = trimmed.split('/').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

// ---------------------------------------------------------------------------
// Aggregated response type
// ---------------------------------------------------------------------------

/// Full detail response for a governance action — parent + child votes + tally.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GovernanceActionDetailView {
    pub action: GovernanceActionView,
    pub votes: Vec<AttestationView>,
    pub tally: Option<GovernanceActionTallyView>,
}

// ---------------------------------------------------------------------------
// Input for casting a vote
// ---------------------------------------------------------------------------

/// Input for `POST /api/v1/governance-actions/{id}/vote`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VoteInput {
    /// CID of the vote attestation (content-addressed, must be globally unique).
    pub id: String,
    /// ActionHash (hex) of the DHT entry for this vote attestation.
    pub dht_anchor_hash: String,
    /// Issuing agent CID.
    pub issuer_cid: String,
    /// "approve" | "reject" | "abstain".
    pub vote_value: String,
    /// Optional vote weight as decimal string.
    pub vote_weight: Option<String>,
    /// Proof class ("witness" | "self-attest" | ...).
    pub proof_class: String,
    /// Serialised proof evidence JSON.
    pub proof_evidence_json: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// Manifest reference (e.g. "mishpat").
    pub manifest_ref: String,
    /// Human-readable vote title.
    pub title: String,
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let (path_only, _) = resource_path.split_once('?').unwrap_or((resource_path, ""));
    let query_str = req.uri().query().unwrap_or("");

    match (&method, path_only) {
        // GET /api/v1/governance-actions?subjectCid=X or ?openOnly=true
        (&Method::GET, "/" | "") => {
            let params: GovernanceActionQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let mut conn = get_conn(pool)?;

            if let Some(subject_cid) = &params.subject_cid {
                let rows = governance_actions::list_by_subject(&mut conn, subject_cid)?;
                let views: Vec<GovernanceActionView> =
                    rows.into_iter().map(GovernanceActionView::from).collect();
                Ok(response::ok(&views))
            } else if params.open_only.as_deref() == Some("true") {
                let now = chrono::Utc::now().to_rfc3339();
                let rows = governance_actions::list_open(&mut conn, &now)?;
                let views: Vec<GovernanceActionView> =
                    rows.into_iter().map(GovernanceActionView::from).collect();
                Ok(response::ok(&views))
            } else {
                Ok(response::bad_request(
                    "subjectCid or openOnly=true query param is required",
                ))
            }
        }

        // GET /api/v1/governance-actions/{id}/tally
        (&Method::GET, p) if p.trim_start_matches('/').ends_with("/tally") => {
            let without_tally = p.trim_start_matches('/').trim_end_matches("/tally");
            let id = extract_id(without_tally).ok_or_else(|| {
                StorageError::InvalidInput("Governance action CID required".to_string())
            })?;
            let mut conn = get_conn(pool)?;
            let tally_row =
                governance_action_tally::get_by_parent_cid(&mut conn, id)?;
            Ok(response::from_option(
                Ok(tally_row.map(GovernanceActionTallyView::from)),
                &format!("Tally for {} not found", id),
            ))
        }

        // GET /api/v1/governance-actions/{id} — full detail (parent + votes + tally)
        (&Method::GET, p) => {
            let id = extract_id(p).ok_or_else(|| {
                StorageError::InvalidInput("Governance action CID required".to_string())
            })?;
            let mut conn = get_conn(pool)?;

            let action_row = governance_actions::get_by_id(&mut conn, id)?;
            let Some(action_row) = action_row else {
                return Ok(response::not_found(&format!(
                    "Governance action {} not found",
                    id
                )));
            };

            let vote_rows =
                attestation_db::list_by_parent_governance_action(&mut conn, id)?;
            let tally_row = governance_action_tally::get_by_parent_cid(&mut conn, id)?;

            let detail = GovernanceActionDetailView {
                action: GovernanceActionView::from(action_row),
                votes: vote_rows.into_iter().map(AttestationView::from).collect(),
                tally: tally_row.map(GovernanceActionTallyView::from),
            };
            Ok(response::ok(&detail))
        }

        // POST /api/v1/governance-actions/{id}/vote
        (&Method::POST, p) if p.trim_start_matches('/').ends_with("/vote") => {
            let without_vote = p.trim_start_matches('/').trim_end_matches("/vote");
            let parent_id = extract_id(without_vote).ok_or_else(|| {
                StorageError::InvalidInput("Governance action CID required".to_string())
            })?;
            let input: VoteInput = parse_body(req).await?;
            let mut conn = get_conn(pool)?;

            // Verify parent governance action exists.
            governance_actions::get_by_id(&mut conn, parent_id)?
                .ok_or_else(|| {
                    StorageError::NotFound(format!(
                        "Governance action {} not found",
                        parent_id
                    ))
                })?;

            // Decode hex dht_anchor_hash back to Vec<u8>.
            let dht_anchor_hash = hex::decode(&input.dht_anchor_hash).map_err(|e| {
                StorageError::InvalidInput(format!("Invalid dhtAnchorHash hex: {e}"))
            })?;

            // Upsert the vote attestation into the projection.
            let vote_row = crate::db::models::AttestationRow {
                id: input.id.clone(),
                dht_anchor_hash,
                attestation_kind: "attestation:governance-vote".to_string(),
                subject_cid: parent_id.to_string(),
                subject_kind: "governance-action".to_string(),
                issuer_cid: input.issuer_cid,
                parent_governance_action_cid: Some(parent_id.to_string()),
                vote_value: Some(input.vote_value),
                vote_weight: input.vote_weight,
                proof_class: input.proof_class,
                proof_evidence_json: input.proof_evidence_json,
                evidence_json: "{}".to_string(),
                expires_at: None,
                supersedes_cid: None,
                revocation_reason: None,
                revoked_at: None,
                created_at: input.created_at,
                manifest_ref: input.manifest_ref,
                title: input.title,
                description: None,
            };
            attestation_db::upsert(&mut conn, &vote_row)?;

            // Recompute tally after vote lands.
            let tally = tally_projector::recompute(&mut conn, parent_id)?;
            Ok(response::ok(&GovernanceActionTallyView::from(tally)))
        }

        _ => Ok(response::not_found(&format!(
            "Unknown governance-actions route: {} {}",
            method, resource_path
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{AttestationRow, GovernanceActionRow};
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn make_gov_row(id: &str, subject: &str) -> GovernanceActionRow {
        GovernanceActionRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000000".to_vec(),
            governance_kind: "governance-action:recovery".to_string(),
            subject_cid: subject.to_string(),
            proposer_cid: "proposer-001".to_string(),
            threshold_json: r#"{"m":2}"#.to_string(),
            eligibility_predicate_json: None,
            ballot_format: "approve-reject".to_string(),
            closes_at: "2099-01-01T00:00:00Z".to_string(),
            parameters_json: None,
            title: "Recovery request".to_string(),
            description: None,
            created_at: "2026-05-12T10:00:00Z".to_string(),
        }
    }

    fn make_vote_row(id: &str, parent: &str, issuer: &str, value: &str) -> AttestationRow {
        AttestationRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000001".to_vec(),
            attestation_kind: "attestation:governance-vote".to_string(),
            subject_cid: parent.to_string(),
            subject_kind: "governance-action".to_string(),
            issuer_cid: issuer.to_string(),
            parent_governance_action_cid: Some(parent.to_string()),
            vote_value: Some(value.to_string()),
            vote_weight: None,
            proof_class: "self-attest".to_string(),
            proof_evidence_json: "{}".to_string(),
            evidence_json: "{}".to_string(),
            expires_at: None,
            supersedes_cid: None,
            revocation_reason: None,
            revoked_at: None,
            created_at: "2026-05-12T10:01:00Z".to_string(),
            manifest_ref: "mishpat".to_string(),
            title: "Vote".to_string(),
            description: None,
        }
    }

    #[test]
    fn governance_action_view_from_row() {
        let row = make_gov_row("gov-001", "subject-001");
        let view = GovernanceActionView::from(row.clone());
        assert_eq!(view.id, "gov-001");
        assert_eq!(view.governance_kind, "governance-action:recovery");
        assert_eq!(view.dht_anchor_hash, hex::encode(&row.dht_anchor_hash));
    }

    #[test]
    fn detail_view_includes_votes_and_tally() {
        let mut conn = setup();
        let gov = make_gov_row("gov-002", "subject-002");
        governance_actions::upsert(&mut conn, &gov).unwrap();

        let v1 = make_vote_row("vote-001", "gov-002", "issuer-a", "approve");
        let v2 = make_vote_row("vote-002", "gov-002", "issuer-b", "approve");
        attestation_db::upsert(&mut conn, &v1).unwrap();
        attestation_db::upsert(&mut conn, &v2).unwrap();

        // Recompute tally as the handler would.
        let tally = tally_projector::recompute(&mut conn, "gov-002").unwrap();
        assert_eq!(tally.computed_status, "reached-quorum");

        // Verify list of children.
        let votes =
            attestation_db::list_by_parent_governance_action(&mut conn, "gov-002").unwrap();
        assert_eq!(votes.len(), 2);

        // Verify tally view.
        let tally_view = GovernanceActionTallyView::from(tally);
        assert_eq!(tally_view.current_approve_count, 2);
        assert_eq!(tally_view.computed_status, "reached-quorum");
    }

    #[test]
    fn tally_view_fields_camel_case() {
        let row = crate::db::models::GovernanceActionTallyRow {
            parent_cid: "gov-003".to_string(),
            governance_kind: "governance-action:recovery".to_string(),
            subject_cid: "subject-003".to_string(),
            threshold_m: 2,
            threshold_n: Some(3),
            threshold_percentage: None,
            closes_at: "2099-01-01T00:00:00Z".to_string(),
            current_approve_count: 1,
            current_reject_count: 0,
            current_abstain_count: 0,
            computed_status: "pending".to_string(),
            last_child_at: None,
            rebuilt_at: "2026-05-12T10:00:00Z".to_string(),
        };
        let view = GovernanceActionTallyView::from(row);
        let json = serde_json::to_value(&view).unwrap();
        assert!(json.get("parentCid").is_some(), "parentCid must be camelCase");
        assert!(json.get("currentApproveCount").is_some(), "currentApproveCount must be camelCase");
        assert!(json.get("thresholdN").is_some(), "thresholdN must be camelCase");
    }
}
