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

use crate::db::models::{AttestationRow, KeyRevocationRow, RecoveryFlowRow};
use crate::db::{attestations, key_revocations, recovery_flows};
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

// ─── State-machine branches (Task 9) ─────────────────────────────────────────

/// Project a `governance-action:identity-freeze` Content signal.
///
/// Identity-freeze is a unilateral governance action — it lands in the
/// `Effective` state immediately with `effective_at = signal.created_at`. The
/// threshold is trivially met (no vote children participate); `current_votes`
/// and `required_votes` mirror `metadata.threshold.m` for consistency with the
/// other flow_kinds, but the state machine does not gate on them here.
///
/// Idempotent under signal redelivery: upsert on `id` replaces the same row.
fn project_identity_freeze(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    let metadata: serde_json::Value =
        serde_json::from_str(&signal.metadata_json).unwrap_or_default();

    let required_votes = metadata["threshold"]["m"].as_i64().unwrap_or(0) as i32;

    let row = RecoveryFlowRow {
        id: signal.id.clone(),
        dht_anchor_hash: signal.entry_hash.as_bytes().to_vec(),
        flow_kind: "identity-freeze".to_string(),
        subject_human_id: metadata["human_id"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        initiated_by_cid: signal.author_id.clone().unwrap_or_default(),
        state: RecoveryFlowState::Effective.as_str().to_string(),
        required_votes,
        current_votes: required_votes,
        threshold_reached: 1,
        effective_at: Some(signal.created_at.clone()),
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
        effective_at = ?row.effective_at,
        "identity-freeze projected as Effective"
    );
    Ok(())
}

/// Project a `governance-action:key-revocation` Content signal.
///
/// Writes a new `key_revocations` row (per Task 7's schema) in the pending
/// state (`threshold_reached = 0`, `effective_at = None`). Also mirrors the
/// signal into `recovery_flows` so dashboards and `list_by_state`-style queries
/// observe revocations alongside recovery-requests and freezes:
/// `flow_kind = "key-revocation"`, initial `state = "Open"`.
///
/// `derived_compromise_at` defaults to `None` here — EPR W2D writes it when
/// cross-stack compromise inference fires.
///
/// Idempotent under signal redelivery: both upserts key on `id`.
fn project_key_revocation_open(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    let metadata: serde_json::Value =
        serde_json::from_str(&signal.metadata_json).unwrap_or_default();

    let required_votes = metadata["threshold"]["m"].as_i64().unwrap_or(0) as i32;
    let subject_human_id = metadata["human_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let initiated_by_cid = signal.author_id.clone().unwrap_or_default();

    let revocation_row = KeyRevocationRow {
        id: signal.id.clone(),
        dht_anchor_hash: signal.entry_hash.as_bytes().to_vec(),
        subject_human_id: subject_human_id.clone(),
        revoked_key: metadata["revoked_key"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        trigger_type: metadata["trigger_type"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        reason: metadata["reason"].as_str().unwrap_or_default().to_string(),
        initiated_by_cid: initiated_by_cid.clone(),
        required_votes,
        current_votes: 0,
        threshold_reached: 0,
        effective_at: None,
        derived_compromise_at: None,
        created_at: signal.created_at.clone(),
        updated_at: signal.created_at.clone(),
    };
    key_revocations::upsert(conn, &revocation_row)?;

    // Mirror into recovery_flows so the unified state-machine surface sees
    // revocations. The mirror row's primary key matches the revocation id, so
    // vote-children can advance both rows by lookup.
    let mirror_row = RecoveryFlowRow {
        id: signal.id.clone(),
        dht_anchor_hash: signal.entry_hash.as_bytes().to_vec(),
        flow_kind: "key-revocation".to_string(),
        subject_human_id,
        initiated_by_cid,
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
    recovery_flows::upsert(conn, &mirror_row)?;

    debug!(
        id = %signal.id,
        subject_human_id = %revocation_row.subject_human_id,
        required_votes = revocation_row.required_votes,
        "key-revocation opened (revocations row + recovery_flows mirror)"
    );
    Ok(())
}

/// Project an `attestation:recovery-approval` Content signal as a vote against
/// its parent recovery_flow.
///
/// **Idempotency strategy** (count-from-attestations): the vote's
/// `AttestationRow` is upserted first — keyed on `signal.id` (the vote's
/// content CID), so redelivering the same signal lands the same row. The
/// flow's `current_votes` is then *recomputed* from
/// `attestations::count_by_parent_governance_action_cid`, not incremented.
/// Replay of the same signal yields the same count.
///
/// Transition rule: `Open → Quorum` only when `current_votes >= required_votes`.
/// `recovery-request` flow_kind stops at Quorum here — `effective_at` is set
/// later by `commit_key_rotation`. For `identity-freeze` / `key-revocation`
/// mirrors, the threshold transition also stops at Quorum; the side-effect
/// table (`key_revocations.effective_at`) is updated by
/// [`project_revocation_vote`].
fn project_recovery_vote(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    let metadata: serde_json::Value =
        serde_json::from_str(&signal.metadata_json).unwrap_or_default();

    let Some(parent_cid) = metadata["parent_governance_action_cid"].as_str() else {
        debug!(
            id = %signal.id,
            "project_recovery_vote: vote missing parent_governance_action_cid; ignoring"
        );
        return Ok(());
    };

    // Dedupe-by-upsert: replay of the same signal id lands the same row.
    let vote_row = build_attestation_row(signal, &metadata)?;
    attestations::upsert(conn, &vote_row)?;

    // Recompute the count from the attestations table — never increment.
    let new_count = attestations::count_by_parent_governance_action_cid(conn, parent_cid)? as i32;

    let Some(parent) = recovery_flows::get_by_id(conn, parent_cid)? else {
        debug!(
            id = %signal.id,
            parent_cid,
            "project_recovery_vote: parent recovery_flows row not found; vote stored, flow not advanced"
        );
        return Ok(());
    };

    recovery_flows::update_current_votes(conn, parent_cid, new_count, &signal.created_at)?;

    // Open → Quorum on threshold. Do not regress non-Open states.
    if parent.state == RecoveryFlowState::Open.as_str()
        && new_count >= parent.required_votes
        && parent.required_votes > 0
    {
        recovery_flows::transition_state(
            conn,
            parent_cid,
            RecoveryFlowState::Quorum.as_str(),
            1,
            // recovery-request: effective_at remains None — key rotation lands
            // via commit_key_rotation. key-revocation mirror also stops here;
            // the key_revocations.effective_at is set by project_revocation_vote.
            None,
            &signal.created_at,
        )?;
        debug!(
            id = %signal.id,
            parent_cid,
            new_count,
            required = parent.required_votes,
            flow_kind = %parent.flow_kind,
            "recovery_flow transitioned Open → Quorum"
        );
    }

    Ok(())
}

/// Project an `attestation:revocation-vote` Content signal as a vote against
/// its parent `key_revocations` row.
///
/// Idempotency strategy identical to [`project_recovery_vote`]: upsert the
/// vote's attestation row, then recompute the count.
///
/// When threshold met, sets `effective_at` and `threshold_reached = 1` on the
/// `key_revocations` row. Also advances the `recovery_flows` mirror row to
/// `Quorum` so the unified state-machine surface reflects the transition.
fn project_revocation_vote(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    let metadata: serde_json::Value =
        serde_json::from_str(&signal.metadata_json).unwrap_or_default();

    let Some(parent_cid) = metadata["parent_governance_action_cid"].as_str() else {
        debug!(
            id = %signal.id,
            "project_revocation_vote: vote missing parent_governance_action_cid; ignoring"
        );
        return Ok(());
    };

    // Dedupe-by-upsert.
    let vote_row = build_attestation_row(signal, &metadata)?;
    attestations::upsert(conn, &vote_row)?;

    let new_count = attestations::count_by_parent_governance_action_cid(conn, parent_cid)? as i32;

    let Some(parent) = key_revocations::get_by_id(conn, parent_cid)? else {
        debug!(
            id = %signal.id,
            parent_cid,
            "project_revocation_vote: parent key_revocations row not found; vote stored, revocation not advanced"
        );
        return Ok(());
    };

    key_revocations::update_current_votes(conn, parent_cid, new_count, &signal.created_at)?;
    // Keep the recovery_flows mirror's current_votes in sync. The mirror may
    // not exist (e.g. tests/replays that only seed the revocation row); that's
    // benign — transition_state warns instead of erroring.
    recovery_flows::update_current_votes(conn, parent_cid, new_count, &signal.created_at)?;

    if parent.threshold_reached == 0
        && new_count >= parent.required_votes
        && parent.required_votes > 0
    {
        // key_revocations: set effective_at + threshold_reached.
        key_revocations::set_effective(conn, parent_cid, &signal.created_at, &signal.created_at)?;
        // recovery_flows mirror: advance Open → Quorum.
        recovery_flows::transition_state(
            conn,
            parent_cid,
            RecoveryFlowState::Quorum.as_str(),
            1,
            None,
            &signal.created_at,
        )?;
        debug!(
            id = %signal.id,
            parent_cid,
            new_count,
            required = parent.required_votes,
            "key_revocation reached threshold — effective_at set, mirror to Quorum"
        );
    }

    Ok(())
}

/// Build an `AttestationRow` for a vote signal. Kept local to the recovery
/// projector because Task 9 is the first consumer that needs to upsert vote
/// children for dedupe — once the central dispatcher (Task 10) routes vote
/// signals to *both* `attestation_projector` and this projector, this helper
/// can be deleted (the attestation upsert will happen in the sibling
/// projector and we'll only count here).
fn build_attestation_row(
    signal: &ElohimContentSignal,
    metadata: &serde_json::Value,
) -> Result<AttestationRow, StorageError> {
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
        manifest_ref: "imagodei".to_string(),
        title: signal.title.clone(),
        description: if signal.description.is_empty() {
            None
        } else {
            Some(signal.description.clone())
        },
    })
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

    // ─── Task 9: state-machine branch tests ──────────────────────────────────

    /// `governance-action:identity-freeze` lands in the `Effective` state
    /// immediately with `effective_at` set to the signal's `created_at`.
    #[test]
    fn identity_freeze_lands_effective() {
        let mut conn = setup();
        let signal = make_signal(
            "freeze-flow-001",
            "governance-action:identity-freeze",
            r#"{
                "human_id": "human-Y",
                "threshold": {"m": 1, "n": 1},
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );
        handle_content_signal(&mut conn, &signal).unwrap();

        let row = recovery_flows::get_by_id(&mut conn, "freeze-flow-001")
            .unwrap()
            .expect("row exists");
        assert_eq!(row.flow_kind, "identity-freeze");
        assert_eq!(row.state, "Effective");
        assert_eq!(row.threshold_reached, 1);
        assert_eq!(row.effective_at.as_deref(), Some("2026-05-15T10:00:00Z"));
        assert_eq!(row.subject_human_id, "human-Y");
    }

    /// `governance-action:key-revocation` seeds both a `key_revocations` row
    /// (Open / threshold_reached=0 / effective_at=None) and a mirror row in
    /// `recovery_flows` with flow_kind = "key-revocation", state = "Open".
    #[test]
    fn key_revocation_open_seeds_revocations_table() {
        let mut conn = setup();
        let signal = make_signal(
            "rev-flow-001",
            "governance-action:key-revocation",
            r#"{
                "human_id": "human-Z",
                "revoked_key": "uhCAkTESTKEY",
                "trigger_type": "voluntary",
                "reason": "compromised",
                "threshold": {"m": 2, "n": 3},
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );
        handle_content_signal(&mut conn, &signal).unwrap();

        // key_revocations row exists in pending state.
        let rev = key_revocations::get_by_id(&mut conn, "rev-flow-001")
            .unwrap()
            .expect("key_revocations row exists");
        assert_eq!(rev.subject_human_id, "human-Z");
        assert_eq!(rev.revoked_key, "uhCAkTESTKEY");
        assert_eq!(rev.trigger_type, "voluntary");
        assert_eq!(rev.reason, "compromised");
        assert_eq!(rev.initiated_by_cid, "initiator-cid-001");
        assert_eq!(rev.required_votes, 2);
        assert_eq!(rev.current_votes, 0);
        assert_eq!(rev.threshold_reached, 0);
        assert!(rev.effective_at.is_none());
        assert!(rev.derived_compromise_at.is_none());

        // recovery_flows mirror row exists, state Open, flow_kind key-revocation.
        let flow = recovery_flows::get_by_id(&mut conn, "rev-flow-001")
            .unwrap()
            .expect("recovery_flows mirror row exists");
        assert_eq!(flow.flow_kind, "key-revocation");
        assert_eq!(flow.state, "Open");
        assert_eq!(flow.required_votes, 2);
        assert_eq!(flow.current_votes, 0);
        assert_eq!(flow.threshold_reached, 0);
        assert!(flow.effective_at.is_none());
        assert_eq!(flow.subject_human_id, "human-Z");
    }

    /// `attestation:recovery-approval` with `parent_governance_action_cid`
    /// increments the parent flow's `current_votes`; transitions Open → Quorum
    /// when threshold is reached.
    #[test]
    fn recovery_vote_advances_parent_flow() {
        let mut conn = setup();
        // Seed an Open recovery-request flow with required_votes = 2.
        let parent = make_signal(
            "parent-flow-100",
            "governance-action:recovery-request",
            r#"{
                "human_id": "human-A",
                "threshold": {"m": 2, "n": 3},
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );
        handle_content_signal(&mut conn, &parent).unwrap();

        // First vote: count → 1, state stays Open.
        let vote1 = make_signal(
            "vote-100",
            "attestation:recovery-approval",
            r#"{
                "subject_cid": "human-A",
                "subject_kind": "human",
                "parent_governance_action_cid": "parent-flow-100",
                "vote_value": "approve"
            }"#,
        );
        handle_content_signal(&mut conn, &vote1).unwrap();

        let after_one = recovery_flows::get_by_id(&mut conn, "parent-flow-100")
            .unwrap()
            .expect("parent row exists after first vote");
        assert_eq!(after_one.current_votes, 1);
        assert_eq!(after_one.state, "Open");
        assert_eq!(after_one.threshold_reached, 0);
        assert!(after_one.effective_at.is_none());

        // Second vote: count → 2, transitions to Quorum, threshold_reached=1.
        // recovery-request stays at Quorum (no effective_at — that lands via
        // commit_key_rotation later).
        let vote2 = make_signal(
            "vote-101",
            "attestation:recovery-approval",
            r#"{
                "subject_cid": "human-A",
                "subject_kind": "human",
                "parent_governance_action_cid": "parent-flow-100",
                "vote_value": "approve"
            }"#,
        );
        handle_content_signal(&mut conn, &vote2).unwrap();

        let after_two = recovery_flows::get_by_id(&mut conn, "parent-flow-100")
            .unwrap()
            .expect("parent row exists after second vote");
        assert_eq!(after_two.current_votes, 2);
        assert_eq!(after_two.state, "Quorum");
        assert_eq!(after_two.threshold_reached, 1);
        // recovery-request: effective_at remains None — key rotation lands later.
        assert!(after_two.effective_at.is_none());
    }

    /// `attestation:revocation-vote` with `parent_governance_action_cid`
    /// increments the parent key_revocation's `current_votes`; when threshold
    /// reached sets `effective_at` + `threshold_reached = 1`.
    #[test]
    fn revocation_vote_advances_key_revocation() {
        let mut conn = setup();
        // Seed an Open key-revocation with required_votes = 2.
        let parent = make_signal(
            "rev-parent-200",
            "governance-action:key-revocation",
            r#"{
                "human_id": "human-B",
                "revoked_key": "uhCAkOLD",
                "trigger_type": "specialist",
                "reason": "device-loss",
                "threshold": {"m": 2, "n": 3},
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );
        handle_content_signal(&mut conn, &parent).unwrap();

        // First vote: count → 1, not yet effective.
        let vote1 = make_signal(
            "rvote-200",
            "attestation:revocation-vote",
            r#"{
                "subject_cid": "human-B",
                "subject_kind": "human",
                "parent_governance_action_cid": "rev-parent-200",
                "vote_value": "approve"
            }"#,
        );
        handle_content_signal(&mut conn, &vote1).unwrap();

        let after_one = key_revocations::get_by_id(&mut conn, "rev-parent-200")
            .unwrap()
            .expect("revocation row after first vote");
        assert_eq!(after_one.current_votes, 1);
        assert_eq!(after_one.threshold_reached, 0);
        assert!(after_one.effective_at.is_none());

        // Second vote: count → 2, threshold met, effective_at set.
        let vote2 = make_signal(
            "rvote-201",
            "attestation:revocation-vote",
            r#"{
                "subject_cid": "human-B",
                "subject_kind": "human",
                "parent_governance_action_cid": "rev-parent-200",
                "vote_value": "approve"
            }"#,
        );
        handle_content_signal(&mut conn, &vote2).unwrap();

        let after_two = key_revocations::get_by_id(&mut conn, "rev-parent-200")
            .unwrap()
            .expect("revocation row after second vote");
        assert_eq!(after_two.current_votes, 2);
        assert_eq!(after_two.threshold_reached, 1);
        assert!(after_two.effective_at.is_some());

        // The recovery_flows mirror row should also advance to Quorum + Effective.
        let mirror = recovery_flows::get_by_id(&mut conn, "rev-parent-200")
            .unwrap()
            .expect("recovery_flows mirror row after threshold");
        assert_eq!(mirror.state, "Quorum");
        assert_eq!(mirror.current_votes, 2);
        assert_eq!(mirror.threshold_reached, 1);
    }

    /// Replaying the same vote signal twice must NOT double-increment.
    /// Dedupe is achieved by upserting the vote's `AttestationRow` first,
    /// then recomputing the count from the `attestations` table by parent.
    #[test]
    fn idempotency_under_signal_redelivery() {
        let mut conn = setup();
        // Seed an Open recovery-request flow with required_votes = 3.
        let parent = make_signal(
            "idem-parent-001",
            "governance-action:recovery-request",
            r#"{
                "human_id": "human-C",
                "threshold": {"m": 3, "n": 5},
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );
        handle_content_signal(&mut conn, &parent).unwrap();

        let vote = make_signal(
            "idem-vote-001",
            "attestation:recovery-approval",
            r#"{
                "subject_cid": "human-C",
                "subject_kind": "human",
                "parent_governance_action_cid": "idem-parent-001",
                "vote_value": "approve"
            }"#,
        );

        // Replay the SAME vote signal three times.
        handle_content_signal(&mut conn, &vote).unwrap();
        handle_content_signal(&mut conn, &vote).unwrap();
        handle_content_signal(&mut conn, &vote).unwrap();

        let row = recovery_flows::get_by_id(&mut conn, "idem-parent-001")
            .unwrap()
            .expect("parent row exists");
        // Count must still be 1, not 3.
        assert_eq!(row.current_votes, 1);
        assert_eq!(row.state, "Open");
        assert_eq!(row.threshold_reached, 0);

        // Same invariant for revocation votes.
        let rev_parent = make_signal(
            "idem-rev-001",
            "governance-action:key-revocation",
            r#"{
                "human_id": "human-D",
                "revoked_key": "uhCAkX",
                "trigger_type": "voluntary",
                "reason": "rotation",
                "threshold": {"m": 3, "n": 5},
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );
        handle_content_signal(&mut conn, &rev_parent).unwrap();

        let rvote = make_signal(
            "idem-rvote-001",
            "attestation:revocation-vote",
            r#"{
                "subject_cid": "human-D",
                "subject_kind": "human",
                "parent_governance_action_cid": "idem-rev-001",
                "vote_value": "approve"
            }"#,
        );
        handle_content_signal(&mut conn, &rvote).unwrap();
        handle_content_signal(&mut conn, &rvote).unwrap();
        handle_content_signal(&mut conn, &rvote).unwrap();

        let rev = key_revocations::get_by_id(&mut conn, "idem-rev-001")
            .unwrap()
            .expect("revocation row exists");
        assert_eq!(rev.current_votes, 1);
        assert_eq!(rev.threshold_reached, 0);
        assert!(rev.effective_at.is_none());
    }
}
