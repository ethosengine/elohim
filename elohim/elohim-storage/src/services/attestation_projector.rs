//! Post-commit signal handler — projects attestation + governance-action Content
//! entries from the Holochain DHT into the storage projection tables.
//!
//! Source of truth: Holochain DHT. This module writes the projection on signal
//! arrival; reads from these tables MUST treat them as caches, not as
//! authoritative. The DHT anchor hash in every projection row provides
//! provenance back to the DHT.
//!
//! Entry point: call `handle_content_signal` for each `ElohimContentSignal`
//! whose `content_type` starts with `attestation:` or `governance-action:`.

use diesel::sqlite::SqliteConnection;
use tracing::debug;

use crate::db::{attestations, governance_actions};
use crate::db::models::{AttestationRow, GovernanceActionRow};
use crate::error::StorageError;
use crate::services::tally_projector;
use crate::signals::ElohimContentSignal;

/// Handle an `ElohimContentSignal` by projecting it into the relevant table.
///
/// Routing:
/// - `content_type` starts with `attestation:` → `attestations` table,
///   then recompute parent tally if `parent_governance_action_cid` is present.
/// - `content_type` starts with `governance-action:` → `governance_actions`
///   table, then initialise a zero-count tally row.
/// - All other content_types are silently ignored.
pub fn handle_content_signal(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    if signal.content_type.starts_with("attestation:") {
        let row = build_attestation_row(signal)?;
        attestations::upsert(conn, &row)?;
        debug!(
            id = %signal.id,
            kind = %signal.content_type,
            "attestation projection upserted"
        );

        // If this is a vote-child, recompute the parent tally.
        if let Some(ref parent_cid) = row.parent_governance_action_cid {
            let tally = tally_projector::recompute(conn, parent_cid)?;
            debug!(
                parent_cid = %parent_cid,
                status = %tally.computed_status,
                approve = tally.current_approve_count,
                reject = tally.current_reject_count,
                "governance tally recomputed after vote"
            );
        }
    } else if signal.content_type.starts_with("governance-action:") {
        let row = build_governance_action_row(signal)?;
        governance_actions::upsert(conn, &row)?;
        debug!(
            id = %signal.id,
            kind = %signal.content_type,
            "governance_action projection upserted"
        );

        // Initialise a zero-count tally row so queries never 404 on a new action.
        let initial_tally = tally_projector::recompute(conn, &row.id)?;
        debug!(
            id = %row.id,
            status = %initial_tally.computed_status,
            "governance_action_tally initialised"
        );
    }
    // Unknown content_type — not an attestation or governance-action; ignore.
    Ok(())
}

// ─── Private helpers ─────────────────────────────────────────────────────────

fn build_attestation_row(signal: &ElohimContentSignal) -> Result<AttestationRow, StorageError> {
    let metadata: serde_json::Value =
        serde_json::from_str(&signal.metadata_json).unwrap_or_default();

    Ok(AttestationRow {
        id: signal.id.clone(),
        dht_anchor_hash: signal.entry_hash.as_bytes().to_vec(),
        attestation_kind: signal.content_type.clone(),
        subject_cid: metadata["subject_cid"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        subject_kind: metadata["subject_kind"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        issuer_cid: signal.author_id.clone().unwrap_or_default(),
        parent_governance_action_cid: metadata["parent_governance_action_cid"]
            .as_str()
            .map(String::from),
        vote_value: metadata["vote_value"].as_str().map(String::from),
        vote_weight: metadata["vote_weight"].as_str().map(String::from),
        proof_class: metadata["proof_evidence"]["class"]
            .as_str()
            .unwrap_or("witness")
            .to_string(),
        proof_evidence_json: serde_json::to_string(
            metadata
                .get("proof_evidence")
                .unwrap_or(&serde_json::Value::Object(Default::default())),
        )?,
        evidence_json: serde_json::to_string(
            metadata
                .get("evidence")
                .unwrap_or(&serde_json::Value::Object(Default::default())),
        )?,
        expires_at: metadata["expires_at"].as_str().map(String::from),
        supersedes_cid: metadata["revocation"]["supersedes_cid"]
            .as_str()
            .map(String::from),
        revocation_reason: metadata["revocation"]["reason"].as_str().map(String::from),
        revoked_at: metadata["revocation"]["revoked_at"]
            .as_str()
            .map(String::from),
        created_at: signal.created_at.clone(),
        manifest_ref: resolve_manifest_ref(&signal.content_type),
        title: signal.title.clone(),
        description: if signal.description.is_empty() {
            None
        } else {
            Some(signal.description.clone())
        },
    })
}

fn build_governance_action_row(
    signal: &ElohimContentSignal,
) -> Result<GovernanceActionRow, StorageError> {
    let metadata: serde_json::Value =
        serde_json::from_str(&signal.metadata_json).unwrap_or_default();

    Ok(GovernanceActionRow {
        id: signal.id.clone(),
        dht_anchor_hash: signal.entry_hash.as_bytes().to_vec(),
        governance_kind: signal.content_type.clone(),
        subject_cid: metadata["subject_cid"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        proposer_cid: signal.author_id.clone().unwrap_or_default(),
        threshold_json: serde_json::to_string(
            metadata
                .get("threshold")
                .unwrap_or(&serde_json::json!({"m": 0})),
        )?,
        eligibility_predicate_json: metadata
            .get("eligibility_predicate")
            .and_then(|v| if v.is_null() { None } else { Some(v) })
            .map(serde_json::to_string)
            .transpose()?,
        ballot_format: metadata["ballot_format"]
            .as_str()
            .unwrap_or("approve-reject")
            .to_string(),
        closes_at: metadata["closes_at"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        parameters_json: metadata
            .get("parameters")
            .and_then(|v| if v.is_null() { None } else { Some(v) })
            .map(serde_json::to_string)
            .transpose()?,
        title: signal.title.clone(),
        description: if signal.description.is_empty() {
            None
        } else {
            Some(signal.description.clone())
        },
        created_at: signal.created_at.clone(),
    })
}

/// Map a content_type prefix to the pillar manifest name.
///
/// Used to populate `manifest_ref` on attestation rows. Unknown prefixes
/// fall back to `"unknown"` — not an error, just an unclassified kind.
fn resolve_manifest_ref(content_type: &str) -> String {
    if content_type.starts_with("attestation:humanness")
        || content_type.starts_with("attestation:identity-")
        || content_type.starts_with("attestation:key-")
        || content_type.starts_with("attestation:stewardship-")
        || content_type.starts_with("attestation:policy-")
        || content_type.starts_with("attestation:renewal-")
        || content_type.starts_with("attestation:recovery-")
        || content_type.starts_with("attestation:revocation-")
        || content_type.starts_with("attestation:challenge-")
    {
        "imagodei".to_string()
    } else if content_type.starts_with("attestation:mastery")
        || content_type.starts_with("attestation:content-")
        || content_type.starts_with("attestation:custodian-")
    {
        "lamad".to_string()
    } else if content_type.starts_with("attestation:device-")
        || content_type.starts_with("attestation:doorway-")
    {
        "infrastructure".to_string()
    } else if content_type.starts_with("attestation:governance-")
        || content_type.starts_with("attestation:gate-")
        || content_type.starts_with("attestation:proposal-")
        || content_type.starts_with("attestation:statement-")
    {
        "mishpat".to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    use crate::db::governance_actions as _gov_db;

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn make_signal(id: &str, content_type: &str, metadata_json: &str) -> ElohimContentSignal {
        ElohimContentSignal {
            id: id.to_string(),
            content_type: content_type.to_string(),
            entry_hash: format!("entry_hash_for_{id}"),
            metadata_json: metadata_json.to_string(),
            author_id: Some("author-001".to_string()),
            title: "Test signal".to_string(),
            description: String::new(),
            created_at: "2026-05-12T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn attestation_signal_projects_to_attestations_table() {
        let mut conn = setup();
        let signal = make_signal(
            "attest-001",
            "attestation:humanness",
            r#"{"subject_cid":"subj-001","subject_kind":"human"}"#,
        );
        handle_content_signal(&mut conn, &signal).unwrap();
        let row = attestations::get_by_id(&mut conn, "attest-001").unwrap().unwrap();
        assert_eq!(row.attestation_kind, "attestation:humanness");
        assert_eq!(row.subject_cid, "subj-001");
        assert_eq!(row.manifest_ref, "imagodei");
    }

    #[test]
    fn governance_action_signal_initialises_tally() {
        let mut conn = setup();
        let signal = make_signal(
            "gov-sig-001",
            "governance-action:recovery",
            r#"{"subject_cid":"subj-002","threshold":{"m":3},"ballot_format":"approve-reject","closes_at":"2099-01-01T00:00:00Z"}"#,
        );
        handle_content_signal(&mut conn, &signal).unwrap();
        let tally = governance_action_tally::get_by_parent_cid(&mut conn, "gov-sig-001")
            .unwrap()
            .unwrap();
        assert_eq!(tally.computed_status, "pending");
        assert_eq!(tally.current_approve_count, 0);
    }

    #[test]
    fn vote_signal_triggers_tally_recompute() {
        let mut conn = setup();
        // First create the governance action
        let gov_signal = make_signal(
            "gov-sig-002",
            "governance-action:recovery",
            r#"{"subject_cid":"subj-003","threshold":{"m":1},"ballot_format":"approve-reject","closes_at":"2099-01-01T00:00:00Z"}"#,
        );
        handle_content_signal(&mut conn, &gov_signal).unwrap();

        // Then cast a vote
        let vote_signal = make_signal(
            "vote-001",
            "attestation:governance-vote",
            r#"{"subject_cid":"subj-003","subject_kind":"governance-action","parent_governance_action_cid":"gov-sig-002","vote_value":"approve"}"#,
        );
        handle_content_signal(&mut conn, &vote_signal).unwrap();

        let tally = governance_action_tally::get_by_parent_cid(&mut conn, "gov-sig-002")
            .unwrap()
            .unwrap();
        assert_eq!(tally.computed_status, "reached-quorum");
        assert_eq!(tally.current_approve_count, 1);
    }

    #[test]
    fn unknown_content_type_is_silently_ignored() {
        let mut conn = setup();
        let signal = make_signal("content-001", "concept", r#"{}"#);
        // Must not error
        handle_content_signal(&mut conn, &signal).unwrap();
        // Must not project anything
        assert!(attestations::get_by_id(&mut conn, "content-001").unwrap().is_none());
    }

    #[test]
    fn resolve_manifest_ref_covers_all_pillars() {
        assert_eq!(resolve_manifest_ref("attestation:humanness"), "imagodei");
        assert_eq!(resolve_manifest_ref("attestation:mastery"), "lamad");
        assert_eq!(resolve_manifest_ref("attestation:content-quality"), "lamad");
        assert_eq!(resolve_manifest_ref("attestation:device-binding"), "infrastructure");
        assert_eq!(resolve_manifest_ref("attestation:governance-vote"), "mishpat");
        assert_eq!(resolve_manifest_ref("attestation:custom"), "unknown");
    }
}
