//! Tally projection CRUD + compute for the governance_action_tally table.
//! Source of truth: local (operational) — computed from governance_actions JOIN attestations.
//! Rebuildable at any time via `compute_tally` + `upsert`.

use diesel::prelude::*;

use crate::db::diesel_schema::{attestations, governance_action_tally, governance_actions};
use crate::db::models::{AttestationRow, GovernanceActionRow, GovernanceActionTallyRow};
use crate::error::StorageError;

/// Upsert a tally row.  Called by `compute_tally` after calculating counts.
pub fn upsert(
    conn: &mut SqliteConnection,
    row: &GovernanceActionTallyRow,
) -> Result<(), StorageError> {
    diesel::insert_into(governance_action_tally::table)
        .values(row)
        .on_conflict(governance_action_tally::parent_cid)
        .do_update()
        .set(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Internal(format!("Failed to upsert governance_action_tally: {e}")))
}

/// Fetch the tally for a governance action by its CID.
pub fn get_by_parent_cid(
    conn: &mut SqliteConnection,
    parent_cid: &str,
) -> Result<Option<GovernanceActionTallyRow>, StorageError> {
    governance_action_tally::table
        .filter(governance_action_tally::parent_cid.eq(parent_cid))
        .first::<GovernanceActionTallyRow>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to fetch governance_action_tally for {parent_cid}: {e}"
            ))
        })
}

/// List tally rows filtered by computed_status.
pub fn list_by_status(
    conn: &mut SqliteConnection,
    status: &str,
) -> Result<Vec<GovernanceActionTallyRow>, StorageError> {
    governance_action_tally::table
        .filter(governance_action_tally::computed_status.eq(status))
        .load::<GovernanceActionTallyRow>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list governance_action_tally by status {status}: {e}"
            ))
        })
}

/// Compute the tally for a single governance action by joining parent + children.
///
/// Applies latest-vote-per-issuer semantics per spec §4.5: children are
/// ordered by `created_at DESC`, and only the first occurrence of each
/// `issuer_cid` is counted.
pub fn compute_tally(
    conn: &mut SqliteConnection,
    parent_cid: &str,
) -> Result<GovernanceActionTallyRow, StorageError> {
    let parent: GovernanceActionRow = governance_actions::table
        .filter(governance_actions::id.eq(parent_cid))
        .first(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "governance_action not found for tally computation {parent_cid}: {e}"
            ))
        })?;

    // Load children newest-first so latest-per-issuer semantics work with a seen-set.
    let children: Vec<AttestationRow> = attestations::table
        .filter(attestations::parent_governance_action_cid.eq(parent_cid))
        .order_by(attestations::created_at.desc())
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to load children for tally {parent_cid}: {e}"
            ))
        })?;

    let threshold: serde_json::Value =
        serde_json::from_str(&parent.threshold_json).unwrap_or_default();
    let threshold_m = threshold["m"].as_i64().unwrap_or(0) as i32;
    let threshold_n = threshold["n"].as_i64().map(|n| n as i32);
    let threshold_percentage = threshold["percentage"].as_f64();

    // Latest-per-issuer: track seen issuer CIDs; skip duplicates.
    let mut seen_issuers = std::collections::HashSet::new();
    let mut approve = 0i32;
    let mut reject = 0i32;
    let mut abstain = 0i32;
    let mut last_child_at: Option<String> = None;

    for child in &children {
        // Track latest child timestamp (children already sorted DESC so first = latest)
        if last_child_at.is_none() {
            last_child_at = Some(child.created_at.clone());
        }
        if !seen_issuers.insert(child.issuer_cid.clone()) {
            continue; // later (older) vote from same issuer — skip
        }
        match child.vote_value.as_deref() {
            Some("approve") => approve += 1,
            Some("reject") => reject += 1,
            Some("abstain") => abstain += 1,
            _ => {}
        }
    }

    let computed_status = derive_status(
        threshold_m,
        threshold_n,
        threshold_percentage,
        approve,
        reject,
        abstain,
        &parent.closes_at,
    );

    Ok(GovernanceActionTallyRow {
        parent_cid: parent_cid.to_string(),
        governance_kind: parent.governance_kind,
        subject_cid: parent.subject_cid,
        threshold_m,
        threshold_n,
        threshold_percentage,
        closes_at: parent.closes_at,
        current_approve_count: approve,
        current_reject_count: reject,
        current_abstain_count: abstain,
        computed_status,
        last_child_at,
        rebuilt_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Derive the computed status string from counts + threshold + close time.
///
/// Status values:
/// - `"reached-quorum"` — approve count >= threshold_m
/// - `"failed-quorum"` — closed and did not reach quorum
/// - `"closed-no-decision"` — closed with no definitive outcome
/// - `"pending"` — still open, quorum not yet reached
fn derive_status(
    m: i32,
    n: Option<i32>,
    _percentage: Option<f64>,
    approve: i32,
    _reject: i32,
    _abstain: i32,
    closes_at: &str,
) -> String {
    if approve >= m {
        return "reached-quorum".to_string();
    }

    let now = chrono::Utc::now();
    let closed = chrono::DateTime::parse_from_rfc3339(closes_at)
        .map(|d| d.with_timezone(&chrono::Utc) < now)
        .unwrap_or(false);

    if closed {
        // Check if quorum was mathematically impossible
        if let Some(n_val) = n {
            let remaining = n_val - approve; // pessimistic: count only approves
            if approve + remaining < m {
                return "failed-quorum".to_string();
            }
        }
        return "closed-no-decision".to_string();
    }

    "pending".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    use crate::db::{attestations as attest_db, governance_actions as gov_db};
    use crate::db::models::{AttestationRow, GovernanceActionRow};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn make_gov(id: &str, m: i32, closes_at: &str) -> GovernanceActionRow {
        GovernanceActionRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000000".to_vec(),
            governance_kind: "governance-action:recovery".to_string(),
            subject_cid: "subject-001".to_string(),
            proposer_cid: "proposer-001".to_string(),
            threshold_json: format!(r#"{{"m":{m}}}"#),
            eligibility_predicate_json: None,
            ballot_format: "approve-reject".to_string(),
            closes_at: closes_at.to_string(),
            parameters_json: None,
            title: "Test governance action".to_string(),
            description: None,
            created_at: "2026-05-12T10:00:00Z".to_string(),
        }
    }

    fn make_vote(
        id: &str,
        parent_cid: &str,
        issuer: &str,
        vote: &str,
        created_at: &str,
    ) -> AttestationRow {
        AttestationRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000000".to_vec(),
            attestation_kind: "attestation:governance-vote".to_string(),
            subject_cid: "subject-001".to_string(),
            subject_kind: "governance-action".to_string(),
            issuer_cid: issuer.to_string(),
            parent_governance_action_cid: Some(parent_cid.to_string()),
            vote_value: Some(vote.to_string()),
            vote_weight: None,
            proof_class: "witness".to_string(),
            proof_evidence_json: "{}".to_string(),
            evidence_json: "{}".to_string(),
            expires_at: None,
            supersedes_cid: None,
            revocation_reason: None,
            revoked_at: None,
            created_at: created_at.to_string(),
            manifest_ref: "mishpat".to_string(),
            title: "Vote".to_string(),
            description: None,
        }
    }

    #[test]
    fn tally_empty_is_pending() {
        let mut conn = setup();
        gov_db::upsert(&mut conn, &make_gov("gov-001", 3, "2099-01-01T00:00:00Z")).unwrap();
        let tally = compute_tally(&mut conn, "gov-001").unwrap();
        assert_eq!(tally.computed_status, "pending");
        assert_eq!(tally.current_approve_count, 0);
    }

    #[test]
    fn tally_reaches_quorum() {
        let mut conn = setup();
        gov_db::upsert(&mut conn, &make_gov("gov-002", 3, "2099-01-01T00:00:00Z")).unwrap();
        attest_db::upsert(&mut conn, &make_vote("v1", "gov-002", "issuer-1", "approve", "2026-05-12T10:01:00Z")).unwrap();
        attest_db::upsert(&mut conn, &make_vote("v2", "gov-002", "issuer-2", "approve", "2026-05-12T10:02:00Z")).unwrap();
        attest_db::upsert(&mut conn, &make_vote("v3", "gov-002", "issuer-3", "approve", "2026-05-12T10:03:00Z")).unwrap();
        let tally = compute_tally(&mut conn, "gov-002").unwrap();
        assert_eq!(tally.computed_status, "reached-quorum");
        assert_eq!(tally.current_approve_count, 3);
    }

    #[test]
    fn tally_latest_per_issuer_semantics() {
        // Issuer 1 votes twice — only latest (approve) should count.
        let mut conn = setup();
        gov_db::upsert(&mut conn, &make_gov("gov-003", 2, "2099-01-01T00:00:00Z")).unwrap();
        // Older vote — reject
        attest_db::upsert(&mut conn, &make_vote("v4", "gov-003", "issuer-1", "reject", "2026-05-12T10:00:00Z")).unwrap();
        // Newer vote — approve (latest wins)
        attest_db::upsert(&mut conn, &make_vote("v5", "gov-003", "issuer-1", "approve", "2026-05-12T10:05:00Z")).unwrap();
        attest_db::upsert(&mut conn, &make_vote("v6", "gov-003", "issuer-2", "approve", "2026-05-12T10:06:00Z")).unwrap();
        let tally = compute_tally(&mut conn, "gov-003").unwrap();
        // Both should count as approve with latest-per-issuer
        assert_eq!(tally.current_approve_count, 2);
        assert_eq!(tally.computed_status, "reached-quorum");
    }

    #[test]
    fn tally_closed_no_decision() {
        let mut conn = setup();
        // Closes in the past
        gov_db::upsert(&mut conn, &make_gov("gov-004", 3, "2020-01-01T00:00:00Z")).unwrap();
        attest_db::upsert(&mut conn, &make_vote("v7", "gov-004", "issuer-1", "approve", "2019-12-31T23:00:00Z")).unwrap();
        let tally = compute_tally(&mut conn, "gov-004").unwrap();
        assert_eq!(tally.computed_status, "closed-no-decision");
        assert_eq!(tally.current_approve_count, 1);
    }

    #[test]
    fn upsert_and_get_by_parent_cid() {
        let mut conn = setup();
        gov_db::upsert(&mut conn, &make_gov("gov-005", 2, "2099-01-01T00:00:00Z")).unwrap();
        let tally = compute_tally(&mut conn, "gov-005").unwrap();
        upsert(&mut conn, &tally).unwrap();
        let fetched = get_by_parent_cid(&mut conn, "gov-005").unwrap().unwrap();
        assert_eq!(fetched.parent_cid, "gov-005");
        assert_eq!(fetched.computed_status, "pending");
    }
}
