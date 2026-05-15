//! Central dispatcher for ElohimContentSignal — fans to AttestationProjector
//! (accumulator + tally) AND RecoveryFlowProjector (state machine).
//!
//! See D1 brainstorm at genesis/docs/plans/2026-05-15-recovery-m4-brainstorm.md
//! for the architectural rationale. Both projectors are independent owners;
//! routing here is by content_type prefix, not exclusive.

use diesel::sqlite::SqliteConnection;
use tracing::warn;

use crate::error::StorageError;
use crate::services::{attestation_projector, recovery_flow_projector};
use crate::signals::ElohimContentSignal;

/// Dispatch an `ElohimContentSignal` to all relevant projectors.
///
/// **Routing:**
/// - All `attestation:*` and `governance-action:*` content_types are dispatched
///   to `AttestationProjector` (the generic accumulator + tally writer).
/// - The recovery family — `governance-action:recovery-request`,
///   `governance-action:identity-freeze`, `governance-action:key-revocation`,
///   and attestation vote-kinds `attestation:recovery-approval*`,
///   `attestation:revocation-vote*` — is ADDITIONALLY dispatched to
///   `RecoveryFlowProjector` (the state-machine controller).
///
/// **Idempotency:** both projectors are idempotent under signal redelivery
/// (AttestationProjector's `upsert` paths are keyed on signal.id; RecoveryFlowProjector
/// uses count-from-attestations for vote tallies). Calling dispatch twice with
/// the same signal is safe.
pub fn dispatch(
    conn: &mut SqliteConnection,
    signal: &ElohimContentSignal,
) -> Result<(), StorageError> {
    // 1) Always run AttestationProjector first for attestation:* + governance-action:*
    if signal.content_type.starts_with("attestation:")
        || signal.content_type.starts_with("governance-action:")
    {
        attestation_projector::handle_content_signal(conn, signal)?;
    }

    // 2) Additionally route to RecoveryFlowProjector for recovery-family signals.
    let is_recovery_family = matches!(
        signal.content_type.as_str(),
        "governance-action:recovery-request"
            | "governance-action:identity-freeze"
            | "governance-action:key-revocation"
    ) || signal
        .content_type
        .starts_with("attestation:recovery-approval")
        || signal
            .content_type
            .starts_with("attestation:revocation-vote");

    if is_recovery_family {
        if let Err(e) = recovery_flow_projector::handle_content_signal(conn, signal) {
            warn!(
                kind = %signal.content_type,
                error = %e,
                "recovery_flow_projector dispatch failed"
            );
            return Err(e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    use crate::db::{attestations, governance_actions, recovery_flows};

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
            created_at: "2026-05-15T10:00:00Z".to_string(),
        }
    }

    /// Canonical fan-out test: a recovery-approval vote should land in BOTH
    /// projectors — the AttestationProjector writes the vote row, and the
    /// RecoveryFlowProjector increments the parent flow's vote count.
    #[test]
    fn recovery_approval_lands_in_both_projectors() {
        let mut conn = setup();

        // Seed: an Open recovery_flows row with required_votes=2.
        let flow_signal = make_signal(
            "flow-001",
            "governance-action:recovery-request",
            r#"{
                "human_id": "human-X",
                "threshold": {"m": 2, "n": 3},
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );
        dispatch(&mut conn, &flow_signal).expect("seed flow");

        // Dispatch one recovery-approval vote — below threshold (1/2).
        let vote_signal = make_signal(
            "vote-001",
            "attestation:recovery-approval",
            r#"{
                "subject_cid": "flow-001",
                "subject_kind": "governance-action",
                "parent_governance_action_cid": "flow-001",
                "vote_value": "approve"
            }"#,
        );
        dispatch(&mut conn, &vote_signal).expect("dispatch vote");

        // AttestationProjector wrote the vote row.
        let vote_row = attestations::get_by_id(&mut conn, "vote-001")
            .expect("query attestation")
            .expect("vote row present");
        assert_eq!(vote_row.attestation_kind, "attestation:recovery-approval");
        assert_eq!(
            vote_row.parent_governance_action_cid.as_deref(),
            Some("flow-001")
        );

        // RecoveryFlowProjector incremented the parent flow vote count.
        let flow_row = recovery_flows::get_by_id(&mut conn, "flow-001")
            .expect("query flow")
            .expect("flow row present");
        assert_eq!(flow_row.current_votes, 1);
        assert_eq!(flow_row.state, "Open"); // below threshold of 2
        assert_eq!(flow_row.required_votes, 2);
    }

    /// A `governance-action:recovery-request` signal must land in BOTH
    /// projectors — the AttestationProjector writes the governance_actions row
    /// and initialises tally; the RecoveryFlowProjector opens a recovery_flows
    /// row in the `Open` state.
    #[test]
    fn governance_action_recovery_request_lands_in_both() {
        let mut conn = setup();
        let signal = make_signal(
            "flow-002",
            "governance-action:recovery-request",
            r#"{
                "human_id": "human-Y",
                "threshold": {"m": 3, "n": 5},
                "ballot_format": "approve-reject",
                "closes_at": "2099-01-01T00:00:00Z"
            }"#,
        );

        dispatch(&mut conn, &signal).expect("dispatch");

        // AttestationProjector landed the governance_actions row.
        let gov_row = governance_actions::get_by_id(&mut conn, "flow-002")
            .expect("query governance_action")
            .expect("governance_action present");
        assert_eq!(gov_row.id, "flow-002");

        // RecoveryFlowProjector opened the flow row.
        let flow_row = recovery_flows::get_by_id(&mut conn, "flow-002")
            .expect("query flow")
            .expect("flow row present");
        assert_eq!(flow_row.state, "Open");
        assert_eq!(flow_row.flow_kind, "recovery-request");
        assert_eq!(flow_row.required_votes, 3);
    }

    /// A generic (non-recovery) attestation should land ONLY in the
    /// AttestationProjector — the RecoveryFlowProjector must not fan out.
    #[test]
    fn generic_attestation_only_lands_in_attestation_projector() {
        let mut conn = setup();
        let signal = make_signal(
            "humanness-001",
            "attestation:humanness",
            r#"{"subject_cid":"subj-001","subject_kind":"human"}"#,
        );

        dispatch(&mut conn, &signal).expect("dispatch");

        // AttestationProjector wrote the row.
        let row = attestations::get_by_id(&mut conn, "humanness-001")
            .expect("query attestation")
            .expect("row present");
        assert_eq!(row.attestation_kind, "attestation:humanness");

        // RecoveryFlowProjector did not fan out — no row in recovery_flows.
        assert!(recovery_flows::get_by_id(&mut conn, "humanness-001")
            .expect("query recovery_flows")
            .is_none());
    }

    /// An unrelated content_type (e.g. plain `concept`) is a complete no-op:
    /// neither projector should write anything.
    #[test]
    fn unrelated_content_type_is_a_noop() {
        let mut conn = setup();
        let signal = make_signal("concept-001", "concept", r#"{}"#);

        dispatch(&mut conn, &signal).expect("dispatch (noop)");

        assert!(attestations::get_by_id(&mut conn, "concept-001")
            .expect("query attestation")
            .is_none());
        assert!(recovery_flows::get_by_id(&mut conn, "concept-001")
            .expect("query recovery_flows")
            .is_none());
    }
}
