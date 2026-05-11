//! Tally projector — computes and persists governance_action_tally on demand.
//!
//! Source of truth: local (operational) — rebuildable from parent + children any time.
//! This module is the canonical entry point for tally recomputation. The
//! `AttestationProjector` calls `recompute` after each vote child arrives;
//! the admin API can call `rebuild_all` to repair the tally table from scratch.

use diesel::prelude::*;

use crate::db::diesel_schema::governance_actions;
use crate::db::governance_action_tally;
use crate::db::models::GovernanceActionTallyRow;
use crate::error::StorageError;

/// Recompute and persist the tally for a single governance action.
///
/// Returns the newly-computed tally row after upserting it.
pub fn recompute(
    conn: &mut SqliteConnection,
    parent_cid: &str,
) -> Result<GovernanceActionTallyRow, StorageError> {
    let tally = governance_action_tally::compute_tally(conn, parent_cid)?;
    governance_action_tally::upsert(conn, &tally)?;
    Ok(tally)
}

/// Rebuild every tally row from scratch by iterating all governance actions.
///
/// Safe to call at startup or after signal replay. Returns the count of
/// governance actions processed.
pub fn rebuild_all(conn: &mut SqliteConnection) -> Result<usize, StorageError> {
    let parent_cids: Vec<String> = governance_actions::table
        .select(governance_actions::id)
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to list governance_actions for rebuild: {e}"
            ))
        })?;

    let mut count = 0usize;
    for parent_cid in &parent_cids {
        recompute(conn, parent_cid)?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    use crate::db::attestations as attest_db;
    use crate::db::governance_actions as gov_db;
    use crate::db::models::{AttestationRow, GovernanceActionRow};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn make_gov(id: &str, m: i32) -> GovernanceActionRow {
        GovernanceActionRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000000".to_vec(),
            governance_kind: "governance-action:recovery".to_string(),
            subject_cid: "subject-tally".to_string(),
            proposer_cid: "proposer-001".to_string(),
            threshold_json: format!(r#"{{"m":{m}}}"#),
            eligibility_predicate_json: None,
            ballot_format: "approve-reject".to_string(),
            closes_at: "2099-01-01T00:00:00Z".to_string(),
            parameters_json: None,
            title: "Tally test gov".to_string(),
            description: None,
            created_at: "2026-05-12T10:00:00Z".to_string(),
        }
    }

    fn make_vote(id: &str, parent: &str, issuer: &str, vote: &str) -> AttestationRow {
        AttestationRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000000".to_vec(),
            attestation_kind: "attestation:governance-vote".to_string(),
            subject_cid: "subject-tally".to_string(),
            subject_kind: "governance-action".to_string(),
            issuer_cid: issuer.to_string(),
            parent_governance_action_cid: Some(parent.to_string()),
            vote_value: Some(vote.to_string()),
            vote_weight: None,
            proof_class: "witness".to_string(),
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
    fn recompute_persists_tally() {
        let mut conn = setup();
        gov_db::upsert(&mut conn, &make_gov("tgov-001", 2)).unwrap();
        attest_db::upsert(&mut conn, &make_vote("tv1", "tgov-001", "i1", "approve")).unwrap();
        attest_db::upsert(&mut conn, &make_vote("tv2", "tgov-001", "i2", "approve")).unwrap();

        let tally = recompute(&mut conn, "tgov-001").unwrap();
        assert_eq!(tally.computed_status, "reached-quorum");
        assert_eq!(tally.current_approve_count, 2);

        // Confirm persisted
        let fetched = governance_action_tally::get_by_parent_cid(&mut conn, "tgov-001")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.computed_status, "reached-quorum");
    }

    #[test]
    fn rebuild_all_processes_all_parents() {
        let mut conn = setup();
        gov_db::upsert(&mut conn, &make_gov("tgov-002", 1)).unwrap();
        gov_db::upsert(&mut conn, &make_gov("tgov-003", 1)).unwrap();
        attest_db::upsert(&mut conn, &make_vote("tv3", "tgov-002", "i1", "approve")).unwrap();

        let count = rebuild_all(&mut conn).unwrap();
        assert_eq!(count, 2);

        let t2 = governance_action_tally::get_by_parent_cid(&mut conn, "tgov-002")
            .unwrap()
            .unwrap();
        assert_eq!(t2.computed_status, "reached-quorum");

        let t3 = governance_action_tally::get_by_parent_cid(&mut conn, "tgov-003")
            .unwrap()
            .unwrap();
        assert_eq!(t3.computed_status, "pending");
    }

    #[test]
    fn recompute_updates_tally_on_second_vote() {
        let mut conn = setup();
        gov_db::upsert(&mut conn, &make_gov("tgov-004", 2)).unwrap();
        attest_db::upsert(&mut conn, &make_vote("tv4", "tgov-004", "i1", "approve")).unwrap();

        let t1 = recompute(&mut conn, "tgov-004").unwrap();
        assert_eq!(t1.computed_status, "pending");

        attest_db::upsert(&mut conn, &make_vote("tv5", "tgov-004", "i2", "approve")).unwrap();
        let t2 = recompute(&mut conn, "tgov-004").unwrap();
        assert_eq!(t2.computed_status, "reached-quorum");
    }
}
