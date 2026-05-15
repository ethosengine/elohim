//! Post-commit signal handler — projects Recovery Protocol Phase 2 (M4)
//! governance-action + attestation Content entries from the Holochain DHT
//! into the `recovery_flows` and `key_revocations` projection tables.
//!
//! Source of truth: Holochain DHT. This module writes the projection on signal
//! arrival; reads from these tables MUST treat them as caches, not as
//! authoritative. The DHT anchor hash in every projection row provides
//! provenance back to the DHT.
//!
//! # State-machine vs accumulator
//!
//! The sibling [`crate::services::attestation_projector`] is an **accumulator**:
//! each signal is independently projected as a row; vote children trigger a
//! tally *recompute* over the full child set. The resulting `tally` row is a
//! pure function of the children — replaying the same set in any order yields
//! the same answer.
//!
//! This projector is a **state machine**. A recovery flow advances through
//! discrete states (`Open` → `Quorum` → `Effective` → `Closed`) driven by the
//! arrival of vote children and policy events (effective rotation, expiry).
//! Each transition mutates the parent row in-place; the row itself encodes the
//! current state, not just a count. Replay is still idempotent because each
//! transition is keyed on the parent flow id and target state, but care must
//! be taken not to regress a flow (e.g. don't move `Effective` back to `Open`).
//!
//! Task 8 (this commit) only implements the **Open-state** branch — the entry
//! point that creates the flow row on the first `governance-action:*` signal.
//! Tasks 9+ fill the vote-driven `Quorum`/`Effective` transitions, the
//! `key_revocations` projection, and the `identity-freeze` branch.
//!
//! Entry point: call [`handle_content_signal`] for each [`ElohimContentSignal`]
//! whose `content_type` matches one of the recovery/revocation kinds below.

use diesel::sqlite::SqliteConnection;
use tracing::debug;

use crate::db::models::RecoveryFlowRow;
use crate::db::recovery_flows;
use crate::error::StorageError;
use crate::signals::ElohimContentSignal;

/// State machine for a recovery flow.
///
/// Transitions: Open → Quorum (threshold reached) → Effective (key rotated / freeze applied)
///            → Closed (terminal — withdrawn or expired)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryFlowState {
    Open,
    Quorum,
    Effective,
    Closed,
}

impl RecoveryFlowState {
    /// String form persisted in the `recovery_flows.state` column.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Quorum => "Quorum",
            Self::Effective => "Effective",
            Self::Closed => "Closed",
        }
    }
}

/// Handle an `ElohimContentSignal` by projecting it into the recovery / revocation
/// projection tables.
///
/// Routing:
/// - `content_type == "governance-action:recovery-request"` → open a recovery_flows row.
/// - `content_type == "governance-action:identity-freeze"` → project as identity-freeze flow (T9).
/// - `content_type == "governance-action:key-revocation"` → project as key_revocations row (T9).
/// - `content_type.starts_with("attestation:recovery-approval")` → recovery vote (T9).
/// - `content_type.starts_with("attestation:revocation-vote")` → revocation vote (T9).
/// - All other content_types are silently ignored (this projector is not their owner).
pub fn handle_content_signal(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    let kind = signal.content_type.as_str();
    match kind {
        "governance-action:recovery-request" => project_recovery_request_open(conn, signal),
        "governance-action:identity-freeze" => project_identity_freeze(conn, signal),
        "governance-action:key-revocation" => project_key_revocation_open(conn, signal),
        _ if kind.starts_with("attestation:recovery-approval") => {
            project_recovery_vote(conn, signal)
        }
        _ if kind.starts_with("attestation:revocation-vote") => {
            project_revocation_vote(conn, signal)
        }
        _ => {
            debug!(
                id = %signal.id,
                kind = %signal.content_type,
                "recovery_flow_projector: content_type not owned by this projector; ignoring"
            );
            Ok(())
        }
    }
}

// ─── Open-state branch (Task 8) ──────────────────────────────────────────────

/// Project a `governance-action:recovery-request` Content signal as a new
/// `recovery_flows` row in the `Open` state.
///
/// Idempotent: `upsert` keys on `id`, so replaying the same signal lands the
/// same row. Transitions out of `Open` are owned by Task 9's vote handlers and
/// transition policies; this function MUST NOT regress a row from a later
/// state back to `Open` (callers route vote-bearing signals to the vote
/// branches above instead).
fn project_recovery_request_open(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    // Defensive parse: empty / malformed metadata becomes an empty object so
    // we still land an Open row. The DHT entry is the source of truth — a
    // malformed projection is a query-side bug, not a write-side panic.
    let metadata: serde_json::Value =
        serde_json::from_str(&signal.metadata_json).unwrap_or_default();

    let required_votes = metadata["threshold"]["m"].as_i64().unwrap_or(0) as i32;

    let row = RecoveryFlowRow {
        id: signal.id.clone(),
        // Until T13/T14 wires real ActionHash bytes through the signal payload,
        // we project the on-wire entry_hash string as bytes — same pattern as
        // signals.rs / attestation_projector.rs.
        dht_anchor_hash: signal.entry_hash.as_bytes().to_vec(),
        flow_kind: "recovery-request".to_string(),
        subject_human_id: metadata["human_id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        initiated_by_cid: signal.author_id.clone().unwrap_or_default(),
        state: RecoveryFlowState::Open.as_str().to_string(),
        required_votes,
        current_votes: 0,
        threshold_reached: 0,
        effective_at: None,
        closes_at: metadata["closes_at"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        metadata_json: signal.metadata_json.clone(),
        created_at: signal.created_at.clone(),
        updated_at: signal.created_at.clone(),
    };

    recovery_flows::upsert(conn, &row)?;
    debug!(
        id = %signal.id,
        flow_kind = %row.flow_kind,
        subject_human_id = %row.subject_human_id,
        required_votes = row.required_votes,
        "recovery_flow opened"
    );
    Ok(())
}

// ─── State-machine branches stubbed for Task 9 ───────────────────────────────

/// Project a `governance-action:identity-freeze` Content signal.
///
/// Implementation deferred to Task 9.
fn project_identity_freeze(
    _conn: &mut SqliteConnection,
    _signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    unimplemented!("project_identity_freeze lands in Task 9 (recovery state-machine branches)")
}

/// Project a `governance-action:key-revocation` Content signal as a new
/// `key_revocations` row in the `Open` state.
///
/// Implementation deferred to Task 9.
fn project_key_revocation_open(
    _conn: &mut SqliteConnection,
    _signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    unimplemented!("project_key_revocation_open lands in Task 9 (recovery state-machine branches)")
}

/// Project an `attestation:recovery-approval-*` Content signal as a vote
/// against its parent recovery_flow, advancing the flow's state machine.
///
/// Implementation deferred to Task 9.
fn project_recovery_vote(
    _conn: &mut SqliteConnection,
    _signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    unimplemented!("project_recovery_vote lands in Task 9 (recovery state-machine branches)")
}

/// Project an `attestation:revocation-vote-*` Content signal as a vote
/// against its parent key_revocation, advancing that flow's state machine.
///
/// Implementation deferred to Task 9.
fn project_revocation_vote(
    _conn: &mut SqliteConnection,
    _signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    unimplemented!("project_revocation_vote lands in Task 9 (recovery state-machine branches)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

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
            author_id: Some("initiator-cid-001".to_string()),
            title: "Recovery request".to_string(),
            description: String::new(),
            created_at: "2026-05-15T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn recovery_request_opens_a_flow() {
        let mut conn = setup();
        let signal = make_signal(
            "recovery-flow-001",
            "governance-action:recovery-request",
            r#"{
                "human_id": "human-X",
                "threshold": {"m": 3, "n": 5},
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );
        handle_content_signal(&mut conn, &signal).unwrap();

        let row = recovery_flows::get_by_id(&mut conn, "recovery-flow-001")
            .unwrap()
            .expect("row exists");
        assert_eq!(row.state, "Open");
        assert_eq!(row.required_votes, 3);
        assert_eq!(row.subject_human_id, "human-X");
        assert_eq!(row.initiated_by_cid, "initiator-cid-001");
        assert_eq!(row.flow_kind, "recovery-request");
        assert_eq!(row.current_votes, 0);
        assert_eq!(row.threshold_reached, 0);
        assert!(row.effective_at.is_none());
        assert_eq!(row.closes_at, "2099-01-01T00:00:00Z");
        assert_eq!(row.created_at, row.updated_at);
    }

    #[test]
    fn recovery_request_with_malformed_metadata_still_opens_flow() {
        let mut conn = setup();
        let signal = make_signal(
            "recovery-flow-002",
            "governance-action:recovery-request",
            "this is not json",
        );
        // Malformed metadata defaults to {} — flow still opens with zero defaults.
        handle_content_signal(&mut conn, &signal).unwrap();
        let row = recovery_flows::get_by_id(&mut conn, "recovery-flow-002")
            .unwrap()
            .expect("row exists");
        assert_eq!(row.state, "Open");
        assert_eq!(row.required_votes, 0);
        assert_eq!(row.subject_human_id, "");
    }

    #[test]
    fn unrelated_content_type_is_silently_ignored() {
        let mut conn = setup();
        let signal = make_signal("noise-001", "attestation:humanness", r#"{}"#);
        // Must not error; must not project a recovery_flows row.
        handle_content_signal(&mut conn, &signal).unwrap();
        assert!(recovery_flows::get_by_id(&mut conn, "noise-001")
            .unwrap()
            .is_none());
    }

    #[test]
    fn recovery_flow_state_strings_match_db_contract() {
        // Compile-time guarantee: enum → string round-trip matches the DB
        // CHECK contract defined by the recovery_flows migration.
        assert_eq!(RecoveryFlowState::Open.as_str(), "Open");
        assert_eq!(RecoveryFlowState::Quorum.as_str(), "Quorum");
        assert_eq!(RecoveryFlowState::Effective.as_str(), "Effective");
        assert_eq!(RecoveryFlowState::Closed.as_str(), "Closed");
    }
}
