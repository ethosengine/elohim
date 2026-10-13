//! DevicePin CRUD (Category B local store). Airplane-mode property: nothing
//! here touches p2p, the conductor, or the network — pure local SQLite.
//!
//! Spec: 2026-06-07-epr-acquisition-pull-queue-design.md §1.1, §3.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::acquisition_pins::dsl as pins;
use super::models::{current_timestamp, AcquisitionPin, NewAcquisitionPin};

/// Backstop re-admission for a retired pin when no inventory ever names it
/// again — the FIRST rung of the hold. The primary path is inventory-driven (a
/// peer advertising the head_ref re-activates it immediately); this only bounds
/// the worst case where the pin's content simply is not being advertised.
///
/// It is a rung, not a period: each consecutive retirement doubles the hold
/// ([`pin_hold`]) up to [`PIN_READMIT_MAX_HOLD`]. As a flat period it re-probed
/// every unfillable pin four times a day forever, so the cost of a quiet hour
/// grew with every head the node had ever been asked for and never got.
pub const PIN_READMIT_COOLDOWN: chrono::Duration = chrono::Duration::hours(6);

/// Ceiling of the re-admission hold: 6 h, 12 h, 24 h, 48 h, 96 h, then 7 days
/// for good.
///
/// Days, not hours, because the cooldown is the arm that fires when NOTHING has
/// changed: a peer that acquires the bytes names them in its inventory, and that
/// arm re-admits at once whatever the rung. What the backstop still has to cover
/// is a holder whose inventory never reaches this node, and a week bounds that
/// blind spot at one probe round per pin per week — the same order as the fleet's
/// roll cadence, when peer sets actually change. Not longer: retirement is a
/// hold a person can still be waiting on, and "re-checked within the week" is a
/// promise an operator can read off the row (`nextReadmitAt`).
pub const PIN_READMIT_MAX_HOLD: chrono::Duration = chrono::Duration::days(7);

/// Most retired pins the cooldown arm re-admits in one reconcile tick.
///
/// Pins retired in the same hour come due in the same hour; un-bounded, a
/// thousand of them moved `pull.total` by a thousand in one tick. Sized to
/// `p2p::acquisition::MAX_ACQUISITION_INFLIGHT` (asserted equal in that
/// module's tests): admitting more per tick than one dispatch round can put in
/// flight only parks the surplus in the queue while it already counts against
/// caught-up. The inventory arm is NOT bounded by this — a peer naming a head is
/// evidence, and it re-admits immediately.
pub const MAX_PIN_READMITS_PER_TICK: usize = 25;

/// `last_outcome` for a pin whose every want spent its retry budget.
pub const PIN_OUTCOME_EXHAUSTED: &str = "exhausted";
/// `last_outcome` for a pin whose wanted bytes are local.
pub const PIN_OUTCOME_FETCHED: &str = "fetched";

/// The hold a retired pin serves after `consecutive_retirements` retirements in
/// a row: [`PIN_READMIT_COOLDOWN`] doubled per retirement after the first,
/// capped at [`PIN_READMIT_MAX_HOLD`].
///
/// A retired row carrying 0 (written by a build that predates the column and
/// missed by its migration's backfill) serves the first rung — the hold it
/// would have served before, never a zero-length one.
pub fn pin_hold(consecutive_retirements: i32) -> chrono::Duration {
    let doublings = u32::try_from(consecutive_retirements.max(1) - 1).unwrap_or(0);
    // 6 h << 5 already exceeds the cap; clamping the shift keeps it in range.
    let secs = PIN_READMIT_COOLDOWN.num_seconds() << doublings.min(16);
    chrono::Duration::seconds(secs).min(PIN_READMIT_MAX_HOLD)
}

/// Closed label for the rung [`pin_hold`] returns — the `hold` label of
/// `elohim_acquisition_pin_backoff_steps_total`.
pub fn pin_hold_label(consecutive_retirements: i32) -> &'static str {
    match pin_hold(consecutive_retirements).num_hours() {
        ..=6 => "6h",
        7..=12 => "12h",
        13..=24 => "24h",
        25..=48 => "48h",
        49..=96 => "96h",
        _ => "168h",
    }
}

/// Every value [`pin_hold_label`] can return, in ladder order.
pub const PIN_HOLD_LABELS: [&str; 6] = ["6h", "12h", "24h", "48h", "96h", "168h"];

/// When a retired pin's hold ends, or `None` when `updated_at` cannot be parsed.
///
/// `updated_at` is the retirement clock — the retirement write stamps it, and no
/// write to a retired row other than re-admission touches it (held demand is
/// counted without stamping). The format is `current_timestamp()`'s RFC 3339
/// (`%Y-%m-%dT%H:%M:%SZ`).
pub fn pin_readmit_at(
    updated_at: &str,
    consecutive_retirements: i32,
) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(updated_at)
        .ok()
        .map(|t| t.with_timezone(&chrono::Utc) + pin_hold(consecutive_retirements))
}

/// Has a retired pin's hold ([`pin_hold`]) elapsed?
///
/// An UNPARSEABLE timestamp returns `true` — fail toward re-admission. A pin is
/// something a person asked for; a redundant retry costs one round of fetches,
/// whereas treating a malformed clock as "not yet cooled" would strand the pin
/// permanently, which is the exact failure this whole leg exists to remove.
pub fn pin_cooled_down(
    updated_at: &str,
    consecutive_retirements: i32,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    match pin_readmit_at(updated_at, consecutive_retirements) {
        Some(due) => now > due,
        None => true,
    }
}

/// `current_timestamp()`'s format for an explicit instant, so the ledger
/// functions below can be driven by a test clock.
fn stamp(now: chrono::DateTime<chrono::Utc>) -> String {
    now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Idempotent upsert on the composite identity (agent, head_ref, kind) — the
/// EXPLICIT pin (`POST /api/v1/pins`).
///
/// Re-pinning the same identity refreshes priority and closure_rule_json and
/// revives a 'removed' or 'retired' pin back to 'active': a person declaring
/// the want again overrides both their own earlier removal and the substrate's
/// hold. It does NOT reset `consecutive_retirements` — only bytes arriving or a
/// peer inventory naming the head is evidence the answer changed, so a re-pin
/// that exhausts again steps the ladder from where it stood. A read-miss is not
/// this path: see [`record_demand_pin`].
///
/// Timestamps are always computed in Rust via `current_timestamp()` so every
/// row in the table uses the same ISO 8601 `YYYY-MM-DDTHH:MM:SSZ` format.
/// The insert path sets both `created_at` and `updated_at` explicitly (via a
/// tuple-values insert) to avoid the SQL `datetime('now')` default that
/// produces a different format (`YYYY-MM-DD HH:MM:SS`).  On conflict only
/// `updated_at` is refreshed — `created_at` is never mutated on re-pin.
pub fn upsert_pin(
    conn: &mut SqliteConnection,
    new: NewAcquisitionPin,
) -> QueryResult<AcquisitionPin> {
    let now = current_timestamp();
    diesel::insert_into(pins::acquisition_pins)
        .values((
            pins::agent_pub_key.eq(&new.agent_pub_key),
            pins::head_ref.eq(&new.head_ref),
            pins::kind.eq(&new.kind),
            pins::closure_rule_json.eq(&new.closure_rule_json),
            pins::priority.eq(new.priority),
            pins::created_at.eq(&now),
            pins::updated_at.eq(&now),
        ))
        .on_conflict((pins::agent_pub_key, pins::head_ref, pins::kind))
        .do_update()
        .set((
            pins::closure_rule_json.eq(new.closure_rule_json.clone()),
            pins::priority.eq(new.priority),
            pins::status.eq("active"),
            pins::updated_at.eq(&now),
        ))
        .execute(conn)?;
    pins::acquisition_pins
        .filter(pins::agent_pub_key.eq(&new.agent_pub_key))
        .filter(pins::head_ref.eq(&new.head_ref))
        .filter(pins::kind.eq(&new.kind))
        .first(conn)
}

/// List pins with status='active', ordered by priority descending (highest first).
pub fn list_active_pins(conn: &mut SqliteConnection) -> QueryResult<Vec<AcquisitionPin>> {
    pins::acquisition_pins
        .filter(pins::status.eq("active"))
        .order(pins::priority.desc())
        .load(conn)
}

/// Count pins with status='active'. Cheap ceiling check for the demand
/// auto-pin DoS guard (`services/demand_autopin.rs`) — a COUNT, never a load.
pub fn count_active_pins(conn: &mut SqliteConnection) -> QueryResult<i64> {
    pins::acquisition_pins
        .filter(pins::status.eq("active"))
        .count()
        .get_result(conn)
}

/// Count pins carrying an arbitrary status — the retired gauge's read, a COUNT
/// rather than a load of every held row.
pub fn count_pins_by_status(conn: &mut SqliteConnection, status: &str) -> QueryResult<i64> {
    pins::acquisition_pins
        .filter(pins::status.eq(status))
        .count()
        .get_result(conn)
}

/// List pins carrying an arbitrary status, ordered by id ascending.
///
/// The `status` column is free-form TEXT, so this is the general form of
/// [`list_active_pins`]. It exists for the RETIRED re-admission path: a pin the
/// substrate held back (no peer could supply its bytes) is re-activated the
/// moment a peer's inventory names its `head_ref` again — which needs the
/// retired set in hand at inventory-receipt time, not a full table scan.
pub fn list_pins_by_status(
    conn: &mut SqliteConnection,
    status: &str,
) -> QueryResult<Vec<AcquisitionPin>> {
    pins::acquisition_pins
        .filter(pins::status.eq(status))
        .order(pins::id.asc())
        .load(conn)
}

/// List all pins regardless of status, ordered by id ascending.
pub fn list_all_pins(conn: &mut SqliteConnection) -> QueryResult<Vec<AcquisitionPin>> {
    pins::acquisition_pins.order(pins::id.asc()).load(conn)
}

/// Fetch a single pin by its integer id, regardless of status.
///
/// Returns `Ok(None)` when no pin carries that id. This is the indexed lookup
/// `handle_remove_pin` (T10) uses to read a pin's `commitment_cid` before
/// flipping its status — far cheaper than loading the whole table and scanning.
pub fn get_pin_by_id(
    conn: &mut SqliteConnection,
    pin_id: i32,
) -> QueryResult<Option<AcquisitionPin>> {
    pins::acquisition_pins
        .filter(pins::id.eq(pin_id))
        .first::<AcquisitionPin>(conn)
        .optional()
}

/// Update the status of a single pin by its integer id.
///
/// Returns the number of rows affected (0 if the id does not exist).
pub fn set_pin_status(
    conn: &mut SqliteConnection,
    pin_id: i32,
    new_status: &str,
) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(pins::acquisition_pins.filter(pins::id.eq(pin_id)))
        .set((pins::status.eq(new_status), pins::updated_at.eq(&now)))
        .execute(conn)
}

/// Back-fill the notarized commitment back-reference on a pin after the
/// provide reconciler authors its replicates-commons Commitment.
///
/// Returns the number of rows affected (0 if the id does not exist).
pub fn set_commitment_cid(
    conn: &mut SqliteConnection,
    pin_id: i32,
    commitment_cid: &str,
) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(pins::acquisition_pins.filter(pins::id.eq(pin_id)))
        .set((
            pins::commitment_cid.eq(commitment_cid),
            pins::updated_at.eq(&now),
        ))
        .execute(conn)
}

// ============================================================================
// Retirement ledger — the hold, its ladder, and what may end it
// ============================================================================
//
// Every status flip the SUBSTRATE makes goes through one of the functions
// below, each guarded on the status it expects to leave, so a person's
// `removed` is never overwritten by a retirement or a re-admission racing it.
// Each takes `now` so the ladder is testable against a clock.

/// Retire one exhausted pin: `active` → `retired`, one rung up the ladder.
///
/// Returns the new `consecutive_retirements`, or `None` when the pin was not
/// `active` (un-pinned between the exhaustion read and this write — its
/// `removed` stands).
pub fn retire_pin(
    conn: &mut SqliteConnection,
    pin_id: i32,
    now: chrono::DateTime<chrono::Utc>,
) -> QueryResult<Option<i32>> {
    let flipped = diesel::update(
        pins::acquisition_pins
            .filter(pins::id.eq(pin_id))
            .filter(pins::status.eq("active")),
    )
    .set((
        pins::status.eq("retired"),
        pins::consecutive_retirements.eq(pins::consecutive_retirements + 1),
        pins::last_outcome.eq(PIN_OUTCOME_EXHAUSTED),
        pins::updated_at.eq(stamp(now)),
    ))
    .execute(conn)?;
    if flipped == 0 {
        return Ok(None);
    }
    pins::acquisition_pins
        .filter(pins::id.eq(pin_id))
        .select(pins::consecutive_retirements)
        .first(conn)
        .optional()
}

/// What one cooldown re-admission pass did.
#[derive(Debug, Default)]
pub struct ReadmitPass {
    /// Pins flipped back to `active` this pass (rows as read BEFORE the flip,
    /// so `consecutive_retirements` is the rung each one just served).
    pub readmitted: Vec<AcquisitionPin>,
    /// Pins whose hold has elapsed but that the per-tick bound left for a later
    /// pass.
    pub deferred: usize,
    /// Pins still inside their hold.
    pub held: usize,
}

/// Cooldown re-admission: retired pins whose hold ([`pin_hold`]) has elapsed go
/// back to `active`, at most `limit` per call.
///
/// Among the due pins, one that somebody read while it was held goes first,
/// then the longest overdue. That ordering is all demand buys a retired pin: it
/// never shortens the hold (see [`record_demand_pin`]).
///
/// `consecutive_retirements` is deliberately left as it is — a cooldown
/// re-admission is a re-probe, not evidence. Only [`settle_fetched_pins`] and
/// [`readmit_named_pins`] reset it.
pub fn readmit_due_pins(
    conn: &mut SqliteConnection,
    now: chrono::DateTime<chrono::Utc>,
    limit: usize,
) -> QueryResult<ReadmitPass> {
    let retired = list_pins_by_status(conn, "retired")?;
    let total = retired.len();
    let mut due: Vec<(Option<chrono::DateTime<chrono::Utc>>, AcquisitionPin)> = retired
        .into_iter()
        .filter(|p| pin_cooled_down(&p.updated_at, p.consecutive_retirements, now))
        .map(|p| (pin_readmit_at(&p.updated_at, p.consecutive_retirements), p))
        .collect();
    let held = total - due.len();
    // `None` (unparseable clock) sorts before every `Some`: longest overdue.
    due.sort_by(|(a_due, a), (b_due, b)| {
        (b.held_demand_count > 0)
            .cmp(&(a.held_demand_count > 0))
            .then(a_due.cmp(b_due))
            .then(a.id.cmp(&b.id))
    });
    let deferred = due.len().saturating_sub(limit);
    let stamped = stamp(now);
    let mut readmitted = Vec::new();
    for (_, pin) in due.into_iter().take(limit) {
        let flipped = diesel::update(
            pins::acquisition_pins
                .filter(pins::id.eq(pin.id))
                .filter(pins::status.eq("retired")),
        )
        .set((
            pins::status.eq("active"),
            pins::held_demand_count.eq(0),
            pins::updated_at.eq(&stamped),
        ))
        .execute(conn)?;
        if flipped > 0 {
            readmitted.push(pin);
        }
    }
    Ok(ReadmitPass {
        readmitted,
        deferred,
        held,
    })
}

/// Inventory re-admission: every retired pin whose `head_ref` a peer just
/// advertised goes back to `active` at once — whatever its rung, and outside
/// the per-tick bound — and its ladder resets, because the advertisement is
/// fresh evidence that the answer may have changed.
///
/// Returns the rows as read before the flip.
pub fn readmit_named_pins(
    conn: &mut SqliteConnection,
    advertised: &std::collections::HashSet<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> QueryResult<Vec<AcquisitionPin>> {
    let stamped = stamp(now);
    let mut readmitted = Vec::new();
    for pin in list_pins_by_status(conn, "retired")? {
        if !advertised.contains(pin.head_ref.as_str()) {
            continue;
        }
        let flipped = diesel::update(
            pins::acquisition_pins
                .filter(pins::id.eq(pin.id))
                .filter(pins::status.eq("retired")),
        )
        .set((
            pins::status.eq("active"),
            pins::consecutive_retirements.eq(0),
            pins::held_demand_count.eq(0),
            pins::updated_at.eq(&stamped),
        ))
        .execute(conn)?;
        if flipped > 0 {
            readmitted.push(pin);
        }
    }
    Ok(readmitted)
}

/// Of the active `pins`, the ids whose wanted head is local but whose row does
/// not say so yet. Empty once every fetched pin is settled, so the reconcile
/// tick that calls [`settle_fetched_pins`] writes nothing at rest.
pub fn pins_needing_fetched_settle(
    pins: &[AcquisitionPin],
    local_has: &std::collections::HashSet<String>,
) -> Vec<i32> {
    pins.iter()
        .filter(|p| p.status == "active" && local_has.contains(&p.head_ref))
        .filter(|p| {
            p.consecutive_retirements != 0 || p.last_outcome.as_deref() != Some(PIN_OUTCOME_FETCHED)
        })
        .map(|p| p.id)
        .collect()
}

/// Record that these active pins' bytes arrived: the ladder resets and
/// `last_outcome` reads `fetched`. Returns the rows written.
pub fn settle_fetched_pins(
    conn: &mut SqliteConnection,
    pin_ids: &[i32],
    now: chrono::DateTime<chrono::Utc>,
) -> QueryResult<usize> {
    if pin_ids.is_empty() {
        return Ok(0);
    }
    diesel::update(
        pins::acquisition_pins
            .filter(pins::id.eq_any(pin_ids))
            .filter(pins::status.eq("active")),
    )
    .set((
        pins::consecutive_retirements.eq(0),
        pins::last_outcome.eq(PIN_OUTCOME_FETCHED),
        pins::updated_at.eq(stamp(now)),
    ))
    .execute(conn)
}

/// What a read-miss did to the pin ledger ([`record_demand_pin`]).
#[derive(Debug)]
pub enum DemandPin {
    /// No pin existed for the head; one was born `active`.
    Authored(AcquisitionPin),
    /// An `active` pin already wants the head. Nothing written.
    AlreadyActive(AcquisitionPin),
    /// The pin is `retired`. The demand was counted on the row; the hold stands.
    HeldRetired(AcquisitionPin),
    /// The pin is `removed` (or `paused`) — a person's decision. The demand was
    /// counted on the row; the pin was not revived.
    NotRevived(AcquisitionPin),
}

/// Record demand for a head — the read-miss writer (`services::demand_autopin`).
///
/// Unlike [`upsert_pin`] this never changes the status of a row that exists:
///
/// - `retired` stays retired. The node has just concluded that no connected peer
///   could supply these bytes, and a read does not change who holds them — a
///   peer inventory does, and that arm re-admits at once. The reader loses
///   nothing: the read path that reported the miss makes its own P2P
///   resolve-and-fetch for that request. The demand is counted on the row, and
///   a demanded pin goes first when its hold ends ([`readmit_due_pins`]).
/// - `removed` stays removed. Before this function a read-miss went through
///   [`upsert_pin`] and silently re-activated a pin its person had deleted.
/// - `active` is left alone, so a read-miss no longer overwrites an explicit
///   pin's priority and closure rule with the demand defaults.
///
/// `updated_at` is NOT stamped on a held row: it is the retirement clock.
pub fn record_demand_pin(
    conn: &mut SqliteConnection,
    new: NewAcquisitionPin,
    now: chrono::DateTime<chrono::Utc>,
) -> QueryResult<DemandPin> {
    let identity = || {
        pins::acquisition_pins
            .filter(pins::agent_pub_key.eq(new.agent_pub_key.clone()))
            .filter(pins::head_ref.eq(new.head_ref.clone()))
            .filter(pins::kind.eq(new.kind.clone()))
    };
    let Some(existing) = identity().first::<AcquisitionPin>(conn).optional()? else {
        let stamped = stamp(now);
        // A racing writer of the same identity wins; either way the row exists.
        diesel::insert_into(pins::acquisition_pins)
            .values((
                pins::agent_pub_key.eq(&new.agent_pub_key),
                pins::head_ref.eq(&new.head_ref),
                pins::kind.eq(&new.kind),
                pins::closure_rule_json.eq(&new.closure_rule_json),
                pins::priority.eq(new.priority),
                pins::created_at.eq(&stamped),
                pins::updated_at.eq(&stamped),
            ))
            .on_conflict((pins::agent_pub_key, pins::head_ref, pins::kind))
            .do_nothing()
            .execute(conn)?;
        return identity().first(conn).map(DemandPin::Authored);
    };
    if existing.status == "active" {
        return Ok(DemandPin::AlreadyActive(existing));
    }
    diesel::update(pins::acquisition_pins.filter(pins::id.eq(existing.id)))
        .set(pins::held_demand_count.eq(pins::held_demand_count + 1))
        .execute(conn)?;
    let row: AcquisitionPin = identity().first(conn)?;
    Ok(if row.status == "retired" {
        DemandPin::HeldRetired(row)
    } else {
        DemandPin::NotRevived(row)
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn sample_pin(priority: i32) -> NewAcquisitionPin {
        NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            head_ref: "epr:album-1".into(),
            kind: "item".into(),
            closure_rule_json: None,
            priority,
        }
    }

    #[test]
    fn upsert_is_idempotent_on_composite_identity() {
        let mut conn = test_conn();

        // First upsert — priority 1.
        let pin1 = upsert_pin(&mut conn, sample_pin(1)).expect("first upsert");

        // Second upsert — same composite identity, higher priority.
        let pin2 = upsert_pin(&mut conn, sample_pin(5)).expect("second upsert");

        // Same database row (same id).
        assert_eq!(pin1.id, pin2.id, "re-upsert must update the existing row");

        // Priority was refreshed.
        assert_eq!(pin2.priority, 5);

        // Exactly one active pin in the table.
        let active = list_active_pins(&mut conn).expect("list_active_pins");
        assert_eq!(active.len(), 1, "exactly one active pin after two upserts");
        assert_eq!(active[0].status, "active");
    }

    #[test]
    fn removed_pins_drop_out_of_active_and_revive_on_repin() {
        let mut conn = test_conn();

        let pin = upsert_pin(&mut conn, sample_pin(1)).expect("upsert");

        // Mark removed — must disappear from active list.
        set_pin_status(&mut conn, pin.id, "removed").expect("set removed");
        let active = list_active_pins(&mut conn).expect("list after remove");
        assert!(
            active.is_empty(),
            "removed pin must not appear in active list"
        );

        // Removed pins must still appear in list_all (soft-delete, not hard-delete).
        let all = list_all_pins(&mut conn).expect("list_all after remove");
        assert_eq!(all.len(), 1, "removed pin must remain in list_all");

        // Re-upsert same identity — must revive to active.
        upsert_pin(&mut conn, sample_pin(1)).expect("re-upsert after remove");
        let active2 = list_active_pins(&mut conn).expect("list after re-pin");
        assert_eq!(active2.len(), 1, "revived pin must appear in active list");
        assert_eq!(active2[0].status, "active");
    }

    #[test]
    fn get_pin_by_id_returns_row_or_none() {
        let mut conn = test_conn();

        // Unknown id → None (no scan, no panic).
        assert!(
            get_pin_by_id(&mut conn, 999)
                .expect("query must not error")
                .is_none(),
            "unknown id must return None"
        );

        // Insert a pin, then fetch it by its assigned id.
        let pin = upsert_pin(&mut conn, sample_pin(3)).expect("upsert");
        let fetched = get_pin_by_id(&mut conn, pin.id)
            .expect("query must not error")
            .expect("existing pin must be found by id");
        assert_eq!(fetched.id, pin.id);
        assert_eq!(fetched.head_ref, "epr:album-1");

        // The lookup is status-agnostic: a 'removed' pin is still found by id
        // (so handle_remove_pin can read its commitment_cid before/after flip).
        set_pin_status(&mut conn, pin.id, "removed").expect("set removed");
        let after_remove = get_pin_by_id(&mut conn, pin.id)
            .expect("query must not error")
            .expect("removed pin must still be found by id");
        assert_eq!(after_remove.status, "removed");
    }

    #[test]
    fn commitment_cid_defaults_null_and_set_backfills() {
        let mut conn = test_conn();

        // A fresh pin is born without a commitment_cid (NULL).
        let pin = upsert_pin(&mut conn, sample_pin(1)).expect("upsert");
        assert!(
            pin.commitment_cid.is_none(),
            "a fresh pin must have NULL commitment_cid (notarized shadow not yet authored)"
        );

        // Back-fill the commitment_cid after the reconciler authors the
        // replicates-commons Commitment.
        let affected = set_commitment_cid(&mut conn, pin.id, "uhCkk-commons-commit-1")
            .expect("set_commitment_cid");
        assert_eq!(
            affected, 1,
            "set_commitment_cid must affect exactly one row"
        );

        let reread = list_all_pins(&mut conn).expect("list_all");
        assert_eq!(reread.len(), 1);
        assert_eq!(
            reread[0].commitment_cid.as_deref(),
            Some("uhCkk-commons-commit-1"),
            "commitment_cid must reflect the back-filled value"
        );

        // Re-upsert (re-pin) must NOT clobber the commitment_cid — only
        // updated_at/priority/status are refreshed on conflict.
        upsert_pin(&mut conn, sample_pin(9)).expect("re-upsert");
        let after_repin = list_all_pins(&mut conn).expect("list_all after re-pin");
        assert_eq!(
            after_repin[0].commitment_cid.as_deref(),
            Some("uhCkk-commons-commit-1"),
            "re-pin must preserve the notarized commitment_cid back-reference"
        );
    }

    /// Regression for the inert pin-retirement path (commit 192f77cd0):
    /// the original `status` CHECK only admitted `('active', 'paused',
    /// 'removed')`, so `set_pin_status(..., "retired")` failed at the SQLite
    /// engine with a CheckViolation that `retire_exhausted_pins` swallowed at
    /// `debug!` level — the pin never actually left `active`. The
    /// 2026-07-31-200000_acquisition_pins_admit_retired migration widens the
    /// CHECK to admit `'retired'`. This test runs against the crate's real
    /// embedded migrations (`test_conn()` above), so deleting that migration
    /// directory reintroduces the old CHECK and this test fails with a
    /// CheckViolation on the `set_pin_status(..., "retired")` call below.
    #[test]
    fn retired_status_is_admitted_by_the_check_constraint_and_round_trips() {
        let mut conn = test_conn();

        let pin = upsert_pin(&mut conn, sample_pin(1)).expect("upsert");

        // The write that was inert before the migration: must now Ok, not
        // CheckViolation.
        let affected = set_pin_status(&mut conn, pin.id, "retired")
            .expect("set_pin_status(\"retired\") must succeed once the CHECK admits it");
        assert_eq!(affected, 1, "exactly one row flips to retired");

        // Must be findable via the general status query (the readmission
        // paths' read side).
        let retired = list_pins_by_status(&mut conn, "retired").expect("list_pins_by_status");
        assert_eq!(retired.len(), 1, "retired pin must be listed by status");
        assert_eq!(retired[0].id, pin.id);

        // Must not still appear as active.
        let active = list_active_pins(&mut conn).expect("list_active_pins");
        assert!(
            active.is_empty(),
            "retired pin must not appear in the active list"
        );

        // Flip back to active (the readmission path) must also Ok.
        let readmitted = set_pin_status(&mut conn, pin.id, "active")
            .expect("set_pin_status(\"active\") readmission must succeed");
        assert_eq!(readmitted, 1);
        let active_again = list_active_pins(&mut conn).expect("list_active_pins after readmit");
        assert_eq!(
            active_again.len(),
            1,
            "pin must be active again after readmission"
        );
    }

    // ── Retirement ledger ───────────────────────────────────────────────────

    use chrono::{Duration, TimeZone, Utc};
    use std::collections::HashSet;

    fn t0() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 13, 12, 0, 0).unwrap()
    }

    fn pin_for(conn: &mut SqliteConnection, head: &str) -> AcquisitionPin {
        upsert_pin(
            conn,
            NewAcquisitionPin {
                head_ref: head.into(),
                ..sample_pin(1)
            },
        )
        .expect("upsert")
    }

    fn reread(conn: &mut SqliteConnection, id: i32) -> AcquisitionPin {
        get_pin_by_id(conn, id).expect("query").expect("row")
    }

    /// Retire `pin_id` at `at` and return the rung it landed on.
    fn retire(conn: &mut SqliteConnection, pin_id: i32, at: chrono::DateTime<Utc>) -> i32 {
        retire_pin(conn, pin_id, at)
            .expect("retire")
            .expect("an active pin retires")
    }

    #[test]
    fn each_consecutive_retirement_doubles_the_hold_up_to_a_week() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "e2e-nobody-holds-this");
        let mut now = t0();
        let mut holds = Vec::new();
        for round in 1..=8 {
            assert_eq!(retire(&mut conn, pin.id, now), round);
            let hold = pin_hold(round);
            holds.push(hold.num_hours());

            // One minute short of the hold: still held, nothing re-admitted.
            let early = readmit_due_pins(&mut conn, now + hold - Duration::minutes(1), 25)
                .expect("early pass");
            assert!(
                early.readmitted.is_empty(),
                "round {round}: inside its hold"
            );
            assert_eq!((early.held, early.deferred), (1, 0));
            assert_eq!(reread(&mut conn, pin.id).status, "retired");

            // One minute past it: re-admitted, and the rung is KEPT — a cooldown
            // re-admission is a re-probe, not evidence.
            now = now + hold + Duration::minutes(1);
            let due = readmit_due_pins(&mut conn, now, 25).expect("due pass");
            assert_eq!(due.readmitted.len(), 1, "round {round}: hold elapsed");
            let row = reread(&mut conn, pin.id);
            assert_eq!(row.status, "active");
            assert_eq!(row.consecutive_retirements, round);
            assert_eq!(row.last_outcome.as_deref(), Some(PIN_OUTCOME_EXHAUSTED));
        }
        assert_eq!(holds, vec![6, 12, 24, 48, 96, 168, 168, 168]);
    }

    #[test]
    fn a_fetch_resets_the_ladder() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "arrives-late");
        retire(&mut conn, pin.id, t0());
        readmit_due_pins(&mut conn, t0() + Duration::hours(7), 25).expect("readmit");
        retire(&mut conn, pin.id, t0() + Duration::hours(8));
        readmit_due_pins(&mut conn, t0() + Duration::hours(21), 25).expect("readmit");
        assert_eq!(reread(&mut conn, pin.id).consecutive_retirements, 2);

        // The bytes arrive: the reconcile tick sees the head local.
        let local: HashSet<String> = ["arrives-late".to_string()].into_iter().collect();
        let active = list_active_pins(&mut conn).expect("active");
        let ids = pins_needing_fetched_settle(&active, &local);
        assert_eq!(ids, vec![pin.id]);
        assert_eq!(
            settle_fetched_pins(&mut conn, &ids, t0() + Duration::hours(22)).expect("settle"),
            1
        );
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.consecutive_retirements, 0);
        assert_eq!(row.last_outcome.as_deref(), Some(PIN_OUTCOME_FETCHED));

        // Settled: the next tick has nothing to write.
        let active = list_active_pins(&mut conn).expect("active");
        assert!(pins_needing_fetched_settle(&active, &local).is_empty());
        assert_eq!(settle_fetched_pins(&mut conn, &[], t0()).expect("noop"), 0);

        // The bytes go missing later and no peer has them: back to the first rung.
        assert_eq!(retire(&mut conn, pin.id, t0() + Duration::days(30)), 1);
    }

    #[test]
    fn a_fetched_pin_that_was_never_retired_is_settled_once() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "always-here");
        let local: HashSet<String> = ["always-here".to_string()].into_iter().collect();
        let active = list_active_pins(&mut conn).expect("active");
        let ids = pins_needing_fetched_settle(&active, &local);
        assert_eq!(ids, vec![pin.id], "last_outcome is still NULL");
        settle_fetched_pins(&mut conn, &ids, t0()).expect("settle");
        let active = list_active_pins(&mut conn).expect("active");
        assert!(pins_needing_fetched_settle(&active, &local).is_empty());
        // A pin whose head is NOT local is never settled.
        let other = pin_for(&mut conn, "not-here");
        let active = list_active_pins(&mut conn).expect("active");
        assert!(pins_needing_fetched_settle(&active, &local).is_empty());
        assert!(reread(&mut conn, other.id).last_outcome.is_none());
    }

    #[test]
    fn an_inventory_naming_the_head_readmits_at_once_and_resets_the_ladder() {
        let mut conn = test_conn();
        let named = pin_for(&mut conn, "named-head");
        let unnamed = pin_for(&mut conn, "unnamed-head");
        let mut now = t0();
        for _ in 0..3 {
            retire(&mut conn, named.id, now);
            retire(&mut conn, unnamed.id, now);
            now += Duration::days(2);
            readmit_due_pins(&mut conn, now, 25).expect("readmit");
        }
        retire(&mut conn, named.id, now);
        retire(&mut conn, unnamed.id, now);
        assert_eq!(reread(&mut conn, named.id).consecutive_retirements, 4);

        // One minute into a 48 h hold a peer advertises the head.
        let advertised: HashSet<&str> = ["named-head", "something-else"].into_iter().collect();
        let readmitted =
            readmit_named_pins(&mut conn, &advertised, now + Duration::minutes(1)).expect("named");
        assert_eq!(readmitted.len(), 1);
        assert_eq!(readmitted[0].id, named.id);
        let row = reread(&mut conn, named.id);
        assert_eq!(row.status, "active");
        assert_eq!(row.consecutive_retirements, 0, "ladder reset");

        let held = reread(&mut conn, unnamed.id);
        assert_eq!(held.status, "retired");
        assert_eq!(
            held.consecutive_retirements, 4,
            "an unnamed pin keeps its rung"
        );

        // Named and it STILL exhausts: first rung again, not the fifth.
        assert_eq!(retire(&mut conn, named.id, now + Duration::hours(1)), 1);
    }

    #[test]
    fn a_read_miss_on_a_retired_pin_inside_its_hold_does_not_reactivate_it() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "e2e-read-again");
        retire(&mut conn, pin.id, t0());
        let retired_at = reread(&mut conn, pin.id).updated_at;

        for n in 1..=3 {
            let demand = record_demand_pin(
                &mut conn,
                NewAcquisitionPin {
                    head_ref: "e2e-read-again".into(),
                    ..sample_pin(1)
                },
                t0() + Duration::hours(n),
            )
            .expect("demand");
            assert!(matches!(demand, DemandPin::HeldRetired(_)), "{demand:?}");
        }
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "retired", "demand does not undo the hold");
        assert_eq!(row.held_demand_count, 3, "the demand is counted");
        assert_eq!(row.consecutive_retirements, 1);
        assert_eq!(
            row.updated_at, retired_at,
            "counting demand must not restart the retirement clock"
        );
        assert!(list_active_pins(&mut conn).expect("active").is_empty());

        // Nor does it shorten the hold.
        let early = readmit_due_pins(&mut conn, t0() + Duration::hours(5), 25).expect("early");
        assert!(early.readmitted.is_empty());

        // The hold ends on its own schedule; the count clears with it.
        let due = readmit_due_pins(&mut conn, t0() + Duration::hours(7), 25).expect("due");
        assert_eq!(due.readmitted.len(), 1);
        assert_eq!(due.readmitted[0].held_demand_count, 3);
        let row = reread(&mut conn, pin.id);
        assert_eq!((row.status.as_str(), row.held_demand_count), ("active", 0));
    }

    #[test]
    fn a_read_miss_does_not_resurrect_a_removed_pin_but_a_repin_does() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "unpinned-by-its-person");
        set_pin_status(&mut conn, pin.id, "removed").expect("remove");

        let demand = record_demand_pin(
            &mut conn,
            NewAcquisitionPin {
                head_ref: "unpinned-by-its-person".into(),
                ..sample_pin(1)
            },
            t0(),
        )
        .expect("demand");
        assert!(matches!(demand, DemandPin::NotRevived(_)), "{demand:?}");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "removed");
        assert_eq!(row.held_demand_count, 1);

        // The substrate's own passes leave a removal alone too.
        assert!(retire_pin(&mut conn, pin.id, t0())
            .expect("retire")
            .is_none());
        let pass = readmit_due_pins(&mut conn, t0() + Duration::days(30), 25).expect("pass");
        assert!(pass.readmitted.is_empty());
        let advertised: HashSet<&str> = ["unpinned-by-its-person"].into_iter().collect();
        assert!(readmit_named_pins(&mut conn, &advertised, t0())
            .expect("named")
            .is_empty());
        assert_eq!(reread(&mut conn, pin.id).status, "removed");

        // The person pinning it again is the one thing that revives it.
        pin_for(&mut conn, "unpinned-by-its-person");
        assert_eq!(reread(&mut conn, pin.id).status, "active");
    }

    #[test]
    fn a_read_miss_authors_a_new_pin_and_leaves_an_active_one_as_it_is() {
        let mut conn = test_conn();
        let demanded = NewAcquisitionPin {
            head_ref: "fresh-head".into(),
            ..sample_pin(1)
        };
        let first = record_demand_pin(&mut conn, demanded.clone(), t0()).expect("demand");
        let DemandPin::Authored(pin) = first else {
            panic!("a never-seen head must author a pin: {first:?}")
        };
        assert_eq!(pin.status, "active");
        assert_eq!(pin.created_at, "2026-10-13T12:00:00Z");
        assert_eq!((pin.consecutive_retirements, pin.held_demand_count), (0, 0));

        // An explicit pin with its own priority and closure rule…
        upsert_pin(
            &mut conn,
            NewAcquisitionPin {
                head_ref: "fresh-head".into(),
                closure_rule_json: Some(r#"{"depth":2}"#.into()),
                ..sample_pin(7)
            },
        )
        .expect("explicit re-pin");
        // …is not overwritten by a later read-miss carrying the demand defaults.
        let again = record_demand_pin(&mut conn, demanded, t0()).expect("demand");
        assert!(matches!(again, DemandPin::AlreadyActive(_)), "{again:?}");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.priority, 7);
        assert_eq!(row.closure_rule_json.as_deref(), Some(r#"{"depth":2}"#));
        assert_eq!(row.held_demand_count, 0);
    }

    #[test]
    fn an_explicit_repin_overrides_the_hold_but_keeps_the_rung() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "wanted-again");
        retire(&mut conn, pin.id, t0());
        readmit_due_pins(&mut conn, t0() + Duration::hours(7), 25).expect("readmit");
        retire(&mut conn, pin.id, t0() + Duration::hours(8));

        pin_for(&mut conn, "wanted-again");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "active", "a person's re-pin is not held");
        assert_eq!(row.consecutive_retirements, 2, "a re-pin is not evidence");
        assert_eq!(retire(&mut conn, pin.id, t0() + Duration::hours(9)), 3);
    }

    #[test]
    fn readmission_is_bounded_per_tick_and_demanded_pins_go_first() {
        let mut conn = test_conn();
        // Sixty pins retired in the same minute — they all come due together.
        let ids: Vec<i32> = (0..60)
            .map(|i| {
                let pin = pin_for(&mut conn, &format!("e2e-{i:03}"));
                retire(&mut conn, pin.id, t0() + Duration::seconds(i));
                pin.id
            })
            .collect();
        // Somebody read the LAST one while it was held.
        record_demand_pin(
            &mut conn,
            NewAcquisitionPin {
                head_ref: "e2e-059".into(),
                ..sample_pin(1)
            },
            t0() + Duration::hours(1),
        )
        .expect("demand");

        let tick = t0() + Duration::hours(7);
        let first = readmit_due_pins(&mut conn, tick, 25).expect("tick 1");
        assert_eq!(
            (first.readmitted.len(), first.deferred, first.held),
            (25, 35, 0)
        );
        assert_eq!(
            first.readmitted[0].id, ids[59],
            "the demanded pin is re-admitted ahead of longer-overdue ones"
        );
        assert_eq!(
            first.readmitted[1..]
                .iter()
                .map(|p| p.id)
                .collect::<Vec<_>>(),
            ids[..24].to_vec(),
            "then longest overdue first"
        );
        assert_eq!(count_active_pins(&mut conn).expect("count"), 25);
        assert_eq!(
            count_pins_by_status(&mut conn, "retired").expect("count"),
            35
        );

        let second = readmit_due_pins(&mut conn, tick + Duration::minutes(1), 25).expect("tick 2");
        assert_eq!((second.readmitted.len(), second.deferred), (25, 10));
        let third = readmit_due_pins(&mut conn, tick + Duration::minutes(2), 25).expect("tick 3");
        assert_eq!((third.readmitted.len(), third.deferred), (10, 0));
        let fourth = readmit_due_pins(&mut conn, tick + Duration::minutes(3), 25).expect("tick 4");
        assert_eq!(
            (fourth.readmitted.len(), fourth.deferred, fourth.held),
            (0, 0, 0)
        );
        assert_eq!(count_active_pins(&mut conn).expect("count"), 60);
    }

    /// Upgrade: a database written by a build without the backoff columns. The
    /// migration's backfill puts an already-retired row on the first rung, a
    /// retired row it could not see (count 0) still serves a 6 h hold, and
    /// active/removed rows start at zero.
    #[test]
    fn rows_from_before_the_backoff_columns_upgrade_onto_the_first_rung() {
        use diesel::connection::SimpleConnection;

        let mut conn = test_conn();
        // Walk back to the schema before this change.
        let mut reverted = false;
        for _ in 0..8 {
            let version = conn
                .revert_last_migration(MIGRATIONS)
                .expect("revert")
                .to_string();
            if version.starts_with("20261013143107") {
                reverted = true;
                break;
            }
        }
        assert!(
            reverted,
            "the backoff migration must be among the latest eight"
        );
        conn.batch_execute(
            "INSERT INTO acquisition_pins (head_ref, status, updated_at) VALUES
                 ('old-retired', 'retired', '2026-10-13T06:30:00Z'),
                 ('old-active',  'active',  '2026-10-13T06:30:00Z'),
                 ('old-removed', 'removed', '2026-10-13T06:30:00Z');",
        )
        .expect("old-shape rows");
        conn.run_pending_migrations(MIGRATIONS).expect("upgrade");

        let all = list_all_pins(&mut conn).expect("list");
        let by_head = |h: &str| all.iter().find(|p| p.head_ref == h).expect("row").clone();
        let old_retired = by_head("old-retired");
        assert_eq!(old_retired.consecutive_retirements, 1);
        assert_eq!(
            old_retired.last_outcome.as_deref(),
            Some(PIN_OUTCOME_EXHAUSTED)
        );
        assert_eq!(by_head("old-active").consecutive_retirements, 0);
        assert!(by_head("old-active").last_outcome.is_none());
        assert_eq!(by_head("old-removed").consecutive_retirements, 0);

        // Retired 5.5 h before t0: still inside the 6 h hold it was already
        // serving, due a little later, exactly as on the old build.
        let early = readmit_due_pins(&mut conn, t0(), 25).expect("early");
        assert!(early.readmitted.is_empty());
        let due = readmit_due_pins(&mut conn, t0() + Duration::hours(1), 25).expect("due");
        assert_eq!(due.readmitted.len(), 1);
        // Its next failure steps to 12 h rather than repeating 6 h.
        assert_eq!(
            retire(&mut conn, old_retired.id, t0() + Duration::hours(2)),
            2
        );
        assert_eq!(
            pin_readmit_at(&reread(&mut conn, old_retired.id).updated_at, 2),
            Some(t0() + Duration::hours(14))
        );

        // A retired row the backfill never saw (count 0) and one with the
        // SQL-default clock format: neither is stranded, neither re-probes early.
        conn.batch_execute(
            "UPDATE acquisition_pins SET status = 'retired', consecutive_retirements = 0,
                    updated_at = '2026-10-13T12:00:00Z' WHERE head_ref = 'old-active';
             UPDATE acquisition_pins SET status = 'retired', consecutive_retirements = 0,
                    updated_at = '2026-10-13 12:00:00' WHERE head_ref = 'old-removed';",
        )
        .expect("zero-count retired rows");
        let pass = readmit_due_pins(&mut conn, t0() + Duration::hours(5), 25).expect("pass");
        assert_eq!(
            pass.readmitted
                .iter()
                .map(|p| p.head_ref.as_str())
                .collect::<Vec<_>>(),
            vec!["old-removed"],
            "an unreadable clock fails toward re-admission; a readable one serves 6 h"
        );
        assert_eq!(pass.held, 2);
        let pass = readmit_due_pins(&mut conn, t0() + Duration::hours(7), 25).expect("pass");
        assert_eq!(pass.readmitted.len(), 1);
        assert_eq!(pass.readmitted[0].head_ref, "old-active");
    }
}
