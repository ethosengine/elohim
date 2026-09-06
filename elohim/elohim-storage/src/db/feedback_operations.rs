//! `feedback_operations` — the storage-side submission outbox.
//!
//! **Category B**: local durable INTENT, not rebuildable from the DHT. An
//! operation records what this cell meant to do before the act exists on chain,
//! and intent precedes the act — which is exactly why it cannot be a projection.
//!
//! Accountable-correction contract §8. The guarantee is AT MOST ONE
//! CONTRIBUTION PER OPERATION — not at most one action, and explicitly not an
//! exactly-once remote commit protocol.

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::feedback_operations;

/// Phase 1: author the Correction EPR (the evidence).
pub const PHASE_EVIDENCE: &str = "evidence";
/// Phase 2: file the FeedbackSignal against the phase-1 action.
pub const PHASE_FEEDBACK: &str = "feedback";
pub const PHASE_DONE: &str = "done";

pub const STATUS_PENDING: &str = "pending";
/// An uncertain call whose recovery enumeration found ZERO matches. NEVER
/// auto-resubmitted: absence from an eventually-consistent index does not
/// authorise a second create. Explicit user resubmission reuses the operation
/// id, hence the same evidence action and group.
pub const STATUS_UNRESOLVED: &str = "unresolved";
pub const STATUS_RESOLVED: &str = "resolved";
pub const STATUS_REFUSED: &str = "refused";

#[derive(Debug, Clone, PartialEq, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = feedback_operations)]
pub struct OperationRow {
    pub operation_id: String,
    pub origin_dna_hash: String,
    pub submitting_cell_agent: String,
    pub request_bytes_cid: String,
    pub request_bytes: String,
    pub target_action_hash: String,
    pub signal_kind: String,
    pub standing_impact: String,
    pub vouch_kind: Option<String>,
    pub phase: String,
    pub evidence_action_hash: Option<String>,
    pub feedback_action_hash: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn fetch(
    conn: &mut SqliteConnection,
    operation_id: &str,
) -> Result<Option<OperationRow>, DieselError> {
    use crate::db::diesel_schema::feedback_operations::dsl as t;
    t::feedback_operations
        .filter(t::operation_id.eq(operation_id))
        .first::<OperationRow>(conn)
        .optional()
}

/// Insert a NEW operation. Returns `Ok(false)` when the id already exists —
/// the caller then compares `request_bytes_cid` and refuses a reuse with
/// different bytes (the request is immutable, §8).
pub fn insert_new(conn: &mut SqliteConnection, row: &OperationRow) -> Result<bool, DieselError> {
    use crate::db::diesel_schema::feedback_operations::dsl as t;
    let inserted = diesel::insert_into(t::feedback_operations)
        .values(row)
        .on_conflict(t::operation_id)
        .do_nothing()
        .execute(conn)?;
    Ok(inserted == 1)
}

pub fn update(conn: &mut SqliteConnection, row: &OperationRow) -> Result<(), DieselError> {
    use crate::db::diesel_schema::feedback_operations::dsl as t;
    diesel::update(t::feedback_operations.filter(t::operation_id.eq(&row.operation_id)))
        .set(row)
        .execute(conn)?;
    Ok(())
}

/// Claim an operation for execution — the SINGLE-FLIGHT gate.
///
/// Moves `pending` → the given in-flight phase in ONE conditional UPDATE, so
/// two concurrent requests for the same operation cannot both proceed: the
/// loser sees 0 rows changed and returns the operation's current state instead
/// of issuing a second zome call.
pub fn claim_for_phase(
    conn: &mut SqliteConnection,
    operation_id: &str,
    expect_status: &str,
    expect_phase: &str,
    new_status: &str,
    now: &str,
) -> Result<bool, DieselError> {
    use crate::db::diesel_schema::feedback_operations::dsl as t;
    let changed = diesel::update(
        t::feedback_operations
            .filter(t::operation_id.eq(operation_id))
            .filter(t::status.eq(expect_status))
            .filter(t::phase.eq(expect_phase)),
    )
    .set((t::status.eq(new_status), t::updated_at.eq(now)))
    .execute(conn)?;
    Ok(changed == 1)
}
