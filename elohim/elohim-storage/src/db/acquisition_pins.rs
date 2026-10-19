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

/// Ceiling of the re-admission hold: 6 h, 12 h, then 24 h for good.
///
/// A day, not a week, because retirement here is BOUNDED PROBING, not proof
/// that every provider was asked. The dispatcher's rotation advances once per
/// drain, so a want's retries can all land on the same peer (two peers and two
/// stable batches of 25); a spent budget means "asked `max(3, peers)` times",
/// and a holder may never have been asked. The backstop therefore has to stay
/// frequent enough to be the remedy for that: once a day at worst. The ladder
/// can grow again when exhaustion carries per-want provider coverage.
pub const PIN_READMIT_MAX_HOLD: chrono::Duration = chrono::Duration::hours(24);

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
    // 6 h << 2 already reaches the cap; clamping the shift keeps it in range.
    let secs = PIN_READMIT_COOLDOWN.num_seconds() << doublings.min(16);
    chrono::Duration::seconds(secs).min(PIN_READMIT_MAX_HOLD)
}

/// Closed label for the rung [`pin_hold`] returns — the `hold` label of
/// `elohim_acquisition_pin_backoff_steps_total`.
pub fn pin_hold_label(consecutive_retirements: i32) -> &'static str {
    match pin_hold(consecutive_retirements).num_hours() {
        ..=6 => "6h",
        7..=12 => "12h",
        _ => "24h",
    }
}

/// Every value [`pin_hold_label`] can return, in ladder order.
pub const PIN_HOLD_LABELS: [&str; 3] = ["6h", "12h", "24h"];

/// Parse a `retired_at` stamp (`current_timestamp()`'s RFC 3339,
/// `%Y-%m-%dT%H:%M:%SZ`). `None` for NULL or anything else.
pub fn parse_retired_at(retired_at: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(retired_at?)
        .ok()
        .map(|t| t.with_timezone(&chrono::Utc))
}

/// When a retired pin's own hold ([`pin_hold`]) ends, or `None` when its
/// `retired_at` is missing or unreadable.
///
/// This is the deadline for a pin NOBODY IS ASKING FOR. A read since the
/// retirement shortens it — to the next hold pass if the demand auto-pin
/// admitted the read, to the first rung if it did not ([`retired_hold_pass`],
/// [`record_demand_pin`], [`mark_read_since_retired`]).
///
/// `None` is NOT "due". The hold pass repairs such a row to a first-rung hold
/// measured from the moment it noticed — fail closed — rather than re-admitting
/// early on a clock it cannot read.
pub fn pin_readmit_at(
    retired_at: Option<&str>,
    consecutive_retirements: i32,
) -> Option<chrono::DateTime<chrono::Utc>> {
    parse_retired_at(retired_at).map(|t| t + pin_hold(consecutive_retirements))
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
/// A re-pin never opens a new `round`: as before the ladder, the pin keeps
/// whatever retry state its tracker holds — a re-pin that lands before the
/// reconcile which drops an exhausted tracker is retired again by it without a
/// fetch, and a client re-asserting an active pin cannot renew its budget.
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
            pins::read_since_retired.eq(0),
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
///
/// No status guard, and it stamps `updated_at` — which is why a hold is
/// computed from `retired_at` and never from `updated_at`: the author call
/// this follows is asynchronous and can land on a row that has since retired.
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
// below. Each runs in one IMMEDIATE transaction — the rows it decides on are
// the rows it writes, and what it returns is exactly what committed (or, on an
// error, nothing did) — and each takes `now` so the ladder is testable against
// a clock. Only the hold pass opens a new `round`: when it settles a pin, and
// when it re-admits one whose tracker is gone. A read, an inventory naming the
// head and an explicit re-pin keep the round, as on dev.

type Now = chrono::DateTime<chrono::Utc>;

/// Retire exhausted pins: `active` → `retired`, one rung up the ladder each,
/// `retired_at` stamped, `readmit_after` set to the end of the new rung.
///
/// `exhausted` pairs each pin id with the `round` whose retry budget was spent
/// (`AcquisitionState::exhausted_pins`). A pin whose row is no longer `active`
/// (un-pinned meanwhile), or whose `round` has moved on (the hold pass opened a
/// new one since that exhaustion was measured), is left alone — a stale
/// exhaustion says nothing about the round the pin is now in.
///
/// Returns `(pin id, new consecutive_retirements)` for each pin retired.
pub fn retire_pins(
    conn: &mut SqliteConnection,
    exhausted: &[(i32, i32)],
    now: Now,
) -> QueryResult<Vec<(i32, i32)>> {
    if exhausted.is_empty() {
        return Ok(Vec::new());
    }
    let stamped = stamp(now);
    conn.immediate_transaction(|conn| {
        let mut retired = Vec::new();
        for &(pin_id, round) in exhausted {
            let flipped = diesel::update(
                pins::acquisition_pins
                    .filter(pins::id.eq(pin_id))
                    .filter(pins::status.eq("active"))
                    .filter(pins::round.eq(round)),
            )
            .set((
                pins::status.eq("retired"),
                pins::consecutive_retirements.eq(pins::consecutive_retirements + 1),
                pins::last_outcome.eq(PIN_OUTCOME_EXHAUSTED),
                pins::retired_at.eq(&stamped),
                pins::updated_at.eq(&stamped),
            ))
            .execute(conn)?;
            if flipped > 0 {
                let rung: i32 = pins::acquisition_pins
                    .filter(pins::id.eq(pin_id))
                    .select(pins::consecutive_retirements)
                    .first(conn)?;
                diesel::update(pins::acquisition_pins.filter(pins::id.eq(pin_id)))
                    .set(pins::readmit_after.eq(stamp(now + pin_hold(rung))))
                    .execute(conn)?;
                retired.push((pin_id, rung));
            }
        }
        Ok(retired)
    })
}

/// [`retire_pins`] for one pin. `None` when it was not retired.
pub fn retire_pin(
    conn: &mut SqliteConnection,
    pin_id: i32,
    round: i32,
    now: Now,
) -> QueryResult<Option<i32>> {
    Ok(retire_pins(conn, &[(pin_id, round)], now)?
        .first()
        .map(|&(_, rung)| rung))
}

/// Why the hold pass re-admitted a pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadmitCause {
    /// Its own hold ([`pin_hold`]) elapsed.
    Hold,
    /// Its own hold had not elapsed, but a read the auto-pin did not admit
    /// marked it, and the first rung has.
    Read,
}

/// What one [`retired_hold_pass`] committed.
#[derive(Debug, Default)]
pub struct HoldPass {
    /// Due pins whose head turned out to be local: re-activated, ladder reset,
    /// `last_outcome = fetched`. Rows as read before the write.
    pub settled: Vec<AcquisitionPin>,
    /// Due pins re-admitted for a re-probe, with why. Rows as read before the
    /// write, so `consecutive_retirements` is the rung each one served.
    pub readmitted: Vec<(AcquisitionPin, ReadmitCause)>,
    /// Retired pins still held after the pass (including any just repaired).
    pub held: usize,
    /// Retired pins whose `retired_at` was missing or unreadable, now holding
    /// on the first rung from `now`. Rows as read before the repair.
    pub clocks_repaired: Vec<AcquisitionPin>,
}

/// The per-tick pass over the retired set — only over the rows that are DUE.
///
/// A row is due when `now` is past its `readmit_after`, the deadline every
/// writer of a retired row keeps current: retirement sets it to the end of the
/// pin's own hold ([`pin_hold`]: 6 h, 12 h, then 24 h), and a read that was
/// not admitted ([`mark_read_since_retired`]) caps it at `retired_at` +
/// [`PIN_READMIT_COOLDOWN`]. (An admitted read does not wait for this pass: it
/// re-activates the pin in the read path, [`record_demand_pin`].) Every value
/// is normalised UTC text in whole seconds, compared as text against
/// `stamp(now - 1 ns)` inclusively. For a whole-second deadline that is exactly
/// `readmit_after < now` at full precision — dev's strict `now - retired_at >
/// cooldown`: a pin due at `18:00:00Z` is not admitted by a pass at exactly
/// `18:00:00Z` and is admitted by one at `18:00:00.250Z`. At rest two indexed existence checks on
/// `idx_acquisition_pins_status_readmit_after` find nothing, and the pass
/// opens no write transaction.
///
/// EVERY due row is handled in this pass — there is no per-tick bound, as
/// there was none before the hold ladder: a backlog that comes due together is
/// re-activated together, and pin priority then orders it in the dispatcher.
/// So "6 h for a read pin" and "24 h at most" are maximum waits up to one
/// reconcile tick.
///
/// For each due row:
///
/// 1. **Settle.** If its head is local — a foreground fetch, replication, or
///    any other ingestion supplied it while the pin was held — it has its
///    answer: `active`, ladder reset, `last_outcome = fetched`. A retired pin
///    whose content arrives EARLIER than its deadline is settled here when it
///    comes due, or sooner by the inventory and fetch paths; the pass does not
///    look at heads that are not due. If presence cannot be read at all, the
///    due rows are re-admitted as before the ladder, and the fetch settles
///    them.
/// 2. **Re-admit.** Otherwise `active` again, for a re-probe. The rung is kept
///    — a re-admission is not evidence — so a pin that is read and still
///    unfillable goes up a rung each time, and without another read it serves
///    that rung. Only a fetch or an inventory naming the head resets it.
///
/// A retired row with no `readmit_after` (one an older build retired) gets one
/// from its `retired_at` here; if `retired_at` is missing or unreadable too it
/// is NOT re-admitted: `retired_at` becomes `now` and it drops to the first
/// rung, holding [`PIN_READMIT_COOLDOWN`] from here — fail closed, once.
///
/// `tracked` names the pins that still hold an acquisition tracker. A re-probe
/// opens a new `round` (a fresh retry budget) only for a pin whose tracker is
/// gone. One that survived the hold — the process was suspended through it, or
/// every reconcile stopped before tracker maintenance — keeps its round and so
/// its exhausted tracker, dispatches nothing and is retired again, as before
/// the ladder.
pub fn retired_hold_pass(
    conn: &mut SqliteConnection,
    app_ctx: &crate::db::AppContext,
    now: Now,
    tracked: &std::collections::HashSet<i32>,
) -> QueryResult<HoldPass> {
    let stamped = stamp(now);
    // Due means `readmit_after < now` at full precision (see above).
    let due_by = stamp(now - chrono::Duration::nanoseconds(1));
    let retired = || pins::acquisition_pins.filter(pins::status.eq("retired"));
    let undated: bool = diesel::select(diesel::dsl::exists(
        retired().filter(pins::readmit_after.is_null()),
    ))
    .get_result(conn)?;
    let due: bool = diesel::select(diesel::dsl::exists(
        retired().filter(pins::readmit_after.le(&due_by)),
    ))
    .get_result(conn)?;
    if !undated && !due {
        return Ok(HoldPass {
            held: usize::try_from(count_pins_by_status(conn, "retired")?).unwrap_or(0),
            ..HoldPass::default()
        });
    }
    conn.immediate_transaction(|conn| {
        let mut pass = HoldPass::default();
        let retired_row = |id: i32| {
            pins::acquisition_pins
                .filter(pins::id.eq(id))
                .filter(pins::status.eq("retired"))
        };

        // Rows retired without a deadline: give them one.
        let undated: Vec<AcquisitionPin> = pins::acquisition_pins
            .filter(pins::status.eq("retired"))
            .filter(pins::readmit_after.is_null())
            .load(conn)?;
        for pin in undated {
            match pin_readmit_at(pin.retired_at.as_deref(), pin.consecutive_retirements) {
                Some(deadline) => {
                    diesel::update(retired_row(pin.id))
                        .set(pins::readmit_after.eq(stamp(deadline)))
                        .execute(conn)?;
                }
                None => {
                    diesel::update(retired_row(pin.id))
                        .set((
                            pins::retired_at.eq(&stamped),
                            pins::consecutive_retirements.eq(1),
                            pins::readmit_after.eq(stamp(now + PIN_READMIT_COOLDOWN)),
                        ))
                        .execute(conn)?;
                    pass.clocks_repaired.push(pin);
                }
            }
        }

        let due: Vec<AcquisitionPin> = pins::acquisition_pins
            .filter(pins::status.eq("retired"))
            .filter(pins::readmit_after.le(&due_by))
            .order(pins::readmit_after.asc())
            .load(conn)?;
        let local = if due.is_empty() {
            std::collections::HashSet::new()
        } else {
            let heads: Vec<String> = due.iter().map(|p| p.head_ref.clone()).collect();
            match crate::db::content_diesel::content_ids_present(conn, app_ctx, &heads) {
                Ok(local) => local,
                // A read failure here must not stop re-admission: re-admit
                // as before the ladder and let the fetch find the bytes.
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        due = due.len(),
                        "acquisition hold pass: local-presence check failed; re-admitting due pins"
                    );
                    std::collections::HashSet::new()
                }
            }
        };
        for pin in due {
            if local.contains(&pin.head_ref) {
                diesel::update(retired_row(pin.id))
                    .set((
                        pins::status.eq("active"),
                        pins::consecutive_retirements.eq(0),
                        pins::read_since_retired.eq(0),
                        pins::last_outcome.eq(PIN_OUTCOME_FETCHED),
                        pins::round.eq(pins::round + 1),
                        pins::updated_at.eq(&stamped),
                    ))
                    .execute(conn)?;
                pass.settled.push(pin);
                continue;
            }
            let own = pin_readmit_at(pin.retired_at.as_deref(), pin.consecutive_retirements);
            let cause = if own.is_some_and(|own| now > own) {
                ReadmitCause::Hold
            } else if pin.read_since_retired != 0 {
                ReadmitCause::Read
            } else {
                ReadmitCause::Hold
            };
            let round = if tracked.contains(&pin.id) {
                pin.round
            } else {
                pin.round + 1
            };
            diesel::update(retired_row(pin.id))
                .set((
                    pins::status.eq("active"),
                    pins::read_since_retired.eq(0),
                    pins::round.eq(round),
                    pins::updated_at.eq(&stamped),
                ))
                .execute(conn)?;
            pass.readmitted.push((pin, cause));
        }
        pass.held = usize::try_from(count_pins_by_status(conn, "retired")?).unwrap_or(0);
        Ok(pass)
    })
}

/// What an inventory naming did ([`inventory_named_pins`]).
#[derive(Debug, Default)]
pub struct NamedPass {
    /// Retired pins re-activated with their ladder reset. Rows as read before
    /// the write.
    pub readmitted: Vec<AcquisitionPin>,
    /// Pins that were already `active` (a re-probe in progress) and still
    /// carried a retirement count: count reset, nothing else touched.
    pub reset_active: Vec<AcquisitionPin>,
}

/// A peer's inventory names these heads — fresh evidence that the answer may
/// have changed.
///
/// - A `retired` pin for a named head goes back to `active` at once, whatever
///   its rung and without waiting for the next pass, with its ladder reset.
///   Its `round` is NOT bumped: as before the ladder, the pin keeps whatever
///   retry state its tracker holds, so an advertisement that lands before the
///   reconcile which drops an exhausted tracker re-opens the ask without a
///   single extra fetch, and the next reconcile retires it again.
/// - An `active` pin for a named head that still carries a retirement count
///   (re-admitted for a re-probe) has the count reset, nothing else.
/// - `removed` pins are not touched.
pub fn inventory_named_pins(
    conn: &mut SqliteConnection,
    advertised: &std::collections::HashSet<&str>,
    now: Now,
) -> QueryResult<NamedPass> {
    if advertised.is_empty() {
        return Ok(NamedPass::default());
    }
    let stamped = stamp(now);
    conn.immediate_transaction(|conn| {
        let candidates: Vec<AcquisitionPin> = pins::acquisition_pins
            .filter(
                pins::status.eq("retired").or(pins::status
                    .eq("active")
                    .and(pins::consecutive_retirements.gt(0))),
            )
            .order(pins::id.asc())
            .load(conn)?;
        let mut pass = NamedPass::default();
        for pin in candidates {
            if !advertised.contains(pin.head_ref.as_str()) {
                continue;
            }
            if pin.status == "retired" {
                diesel::update(pins::acquisition_pins.filter(pins::id.eq(pin.id)))
                    .set((
                        pins::status.eq("active"),
                        pins::consecutive_retirements.eq(0),
                        pins::read_since_retired.eq(0),
                        pins::updated_at.eq(&stamped),
                    ))
                    .execute(conn)?;
                pass.readmitted.push(pin);
            } else {
                diesel::update(pins::acquisition_pins.filter(pins::id.eq(pin.id)))
                    .set((
                        pins::consecutive_retirements.eq(0),
                        pins::updated_at.eq(&stamped),
                    ))
                    .execute(conn)?;
                pass.reset_active.push(pin);
            }
        }
        Ok(pass)
    })
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
/// `last_outcome` reads `fetched`.
///
/// Returns the rows actually written, as read inside the transaction before
/// the write — a pin removed since the caller's census is not in it.
pub fn settle_fetched_pins(
    conn: &mut SqliteConnection,
    pin_ids: &[i32],
    now: Now,
) -> QueryResult<Vec<AcquisitionPin>> {
    if pin_ids.is_empty() {
        return Ok(Vec::new());
    }
    let stamped = stamp(now);
    conn.immediate_transaction(|conn| {
        let rows: Vec<AcquisitionPin> = pins::acquisition_pins
            .filter(pins::id.eq_any(pin_ids))
            .filter(pins::status.eq("active"))
            .order(pins::id.asc())
            .load(conn)?;
        let ids: Vec<i32> = rows.iter().map(|p| p.id).collect();
        diesel::update(pins::acquisition_pins.filter(pins::id.eq_any(&ids)))
            .set((
                pins::consecutive_retirements.eq(0),
                pins::last_outcome.eq(PIN_OUTCOME_FETCHED),
                pins::updated_at.eq(&stamped),
            ))
            .execute(conn)?;
        Ok(rows)
    })
}

/// What an admitted read-miss did to the pin ledger ([`record_demand_pin`]).
/// Each carries the row as it stands after the call.
#[derive(Debug)]
pub enum DemandPin {
    /// No pin existed for the head; one was born `active`.
    Authored(AcquisitionPin),
    /// The pin was `active`; it was refreshed as a re-pin refreshes it.
    Refreshed(AcquisitionPin),
    /// The pin was `retired` (or `paused`) and is `active` again.
    Reactivated(AcquisitionPin),
    /// The pin is `removed` — a person's decision. Nothing was written.
    NotRevived(AcquisitionPin),
}

/// Record demand for a head — the writer for a read-miss the demand auto-pin
/// ADMITTED (`services::demand_autopin`: past its per-id throttle, inside its
/// window of distinct ids, under its active-row ceiling — exactly the reads
/// that reached [`upsert_pin`] before the hold ladder).
///
/// It does what that `upsert_pin` did, with two exceptions. The write is the
/// same — closure rule and priority set to the demand defaults, `active`,
/// `updated_at` stamped, no new `round` — so a retired pin is re-activated in
/// the read path and the very next reconcile's census includes it, with
/// whatever retry state its tracker holds. The exceptions:
///
/// - the pin keeps its rung (`consecutive_retirements`): a read is not
///   evidence the answer changed, so if it is still unfillable it serves a
///   longer hold next time;
/// - `removed` stays removed. ONLY AN EXPLICIT RE-PIN (`POST /api/v1/pins`)
///   RESTORES A REMOVED PIN — an operator decision of 2026-10-14: the person
///   un-pinned it, and a read is not them pinning it again. A read for a
///   removed pin's content is then served only by the read path's foreground
///   fetch — nothing on a pure-iroh node, one peer otherwise — or by
///   replication happening to carry it.
///
/// A read the auto-pin does NOT admit never reaches this function; it only
/// marks a retired row ([`mark_read_since_retired`]), which caps its hold at
/// 6 h — the flat cooldown it had before the ladder. Read-driven
/// re-activations are therefore exactly the admitted reads: no more than
/// 256 distinct heads per 300 s window, and never past the ceiling.
pub fn record_demand_pin(
    conn: &mut SqliteConnection,
    new: NewAcquisitionPin,
    now: Now,
) -> QueryResult<DemandPin> {
    conn.immediate_transaction(|conn| {
        let identity = || {
            pins::acquisition_pins
                .filter(pins::agent_pub_key.eq(new.agent_pub_key.clone()))
                .filter(pins::head_ref.eq(new.head_ref.clone()))
                .filter(pins::kind.eq(new.kind.clone()))
        };
        let stamped = stamp(now);
        let Some(existing) = identity().first::<AcquisitionPin>(conn).optional()? else {
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
                .execute(conn)?;
            return identity().first(conn).map(DemandPin::Authored);
        };
        if existing.status == "removed" {
            return Ok(DemandPin::NotRevived(existing));
        }
        diesel::update(pins::acquisition_pins.filter(pins::id.eq(existing.id)))
            .set((
                pins::closure_rule_json.eq(new.closure_rule_json.clone()),
                pins::priority.eq(new.priority),
                pins::status.eq("active"),
                pins::read_since_retired.eq(0),
                pins::updated_at.eq(&stamped),
            ))
            .execute(conn)?;
        let row: AcquisitionPin = identity().first(conn)?;
        Ok(if existing.status == "active" {
            DemandPin::Refreshed(row)
        } else {
            DemandPin::Reactivated(row)
        })
    })
}

/// A read-miss the demand auto-pin did NOT admit (throttled, window full, at
/// the ceiling) named this head: if its pin is `retired`, mark the row as read
/// since it retired. A marked row's hold is capped at the first rung — its
/// `readmit_after` is pulled in to `retired_at` + [`PIN_READMIT_COOLDOWN`] —
/// so the read re-admits nothing early and cannot leave the pin waiting longer
/// than the flat 6 h it waited before the ladder.
///
/// Idempotent, and cheap on the path that is not throttled by anything: one
/// indexed read; a write only the first time a retired pin is read in a hold.
/// The deadline written is normalised UTC text, like every other. The write
/// repeats every
/// predicate of the read ([`mark_row_read`]), so callers racing on one row
/// write it once. Returns whether this call marked a row. Nothing else — no
/// pin, an active pin, a removed pin — is touched.
pub fn mark_read_since_retired(
    conn: &mut SqliteConnection,
    agent_pub_key: &str,
    head_ref: &str,
    kind: &str,
) -> QueryResult<bool> {
    match unmarked_retired_row(conn, agent_pub_key, head_ref, kind)? {
        Some(seen) => mark_row_read(conn, &seen),
        None => Ok(false),
    }
}

/// A retired, unmarked row as [`unmarked_retired_row`] read it: the deadline
/// marking should give it, and the retirement that deadline was computed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnmarkedRetired {
    pub id: i32,
    /// `retired_at` as read. The write applies only while the row still holds
    /// this retirement, so a deadline is never stamped onto a later one.
    pub retired_at: Option<String>,
    pub deadline: Option<String>,
}

/// The read half of [`mark_read_since_retired`]: the head's pin if it is
/// retired and not yet marked, with the deadline marking it should give it.
pub(crate) fn unmarked_retired_row(
    conn: &mut SqliteConnection,
    agent_pub_key: &str,
    head_ref: &str,
    kind: &str,
) -> QueryResult<Option<UnmarkedRetired>> {
    let row: Option<(i32, Option<String>, Option<String>)> = pins::acquisition_pins
        .filter(pins::agent_pub_key.eq(agent_pub_key))
        .filter(pins::head_ref.eq(head_ref))
        .filter(pins::kind.eq(kind))
        .filter(pins::status.eq("retired"))
        .filter(pins::read_since_retired.eq(0))
        .select((pins::id, pins::retired_at, pins::readmit_after))
        .first(conn)
        .optional()?;
    Ok(row.map(|(id, retired_at, readmit_after)| {
        let cap = parse_retired_at(retired_at.as_deref()).map(|t| stamp(t + PIN_READMIT_COOLDOWN));
        // The earlier of its own deadline and the cap. Both are
        // `current_timestamp()` stamps, so they order as strings.
        let deadline = match (readmit_after, cap) {
            (Some(own), Some(cap)) => Some(own.min(cap)),
            (own, cap) => cap.or(own),
        };
        UnmarkedRetired {
            id,
            retired_at,
            deadline,
        }
    }))
}

/// The write half of [`mark_read_since_retired`], guarded on every predicate
/// the read checked — the same retirement included — so it marks a row at
/// most once per hold whoever races, and a read that stalled across a
/// re-activation and a new retirement marks nothing: it was not a read of the
/// new hold.
pub(crate) fn mark_row_read(
    conn: &mut SqliteConnection,
    seen: &UnmarkedRetired,
) -> QueryResult<bool> {
    let row = pins::acquisition_pins
        .filter(pins::id.eq(seen.id))
        .filter(pins::status.eq("retired"))
        .filter(pins::read_since_retired.eq(0));
    let marked = match (&seen.retired_at, &seen.deadline) {
        (Some(retired_at), Some(deadline)) => {
            diesel::update(row.filter(pins::retired_at.eq(retired_at)))
                .set((
                    pins::read_since_retired.eq(1),
                    pins::readmit_after.eq(deadline),
                ))
                .execute(conn)?
        }
        (Some(retired_at), None) => diesel::update(row.filter(pins::retired_at.eq(retired_at)))
            .set(pins::read_since_retired.eq(1))
            .execute(conn)?,
        (None, Some(deadline)) => diesel::update(row.filter(pins::retired_at.is_null()))
            .set((
                pins::read_since_retired.eq(1),
                pins::readmit_after.eq(deadline),
            ))
            .execute(conn)?,
        (None, None) => diesel::update(row.filter(pins::retired_at.is_null()))
            .set(pins::read_since_retired.eq(1))
            .execute(conn)?,
    };
    Ok(marked > 0)
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

    fn demand_for(head: &str) -> NewAcquisitionPin {
        NewAcquisitionPin {
            head_ref: head.into(),
            ..sample_pin(1)
        }
    }

    fn pin_for(conn: &mut SqliteConnection, head: &str) -> AcquisitionPin {
        upsert_pin(conn, demand_for(head)).expect("upsert")
    }

    fn reread(conn: &mut SqliteConnection, id: i32) -> AcquisitionPin {
        get_pin_by_id(conn, id).expect("query").expect("row")
    }

    /// Retire `pin_id` at `at`, naming the round its row is in, and return the
    /// rung it landed on.
    fn retire(conn: &mut SqliteConnection, pin_id: i32, at: chrono::DateTime<Utc>) -> i32 {
        let round = reread(conn, pin_id).round;
        retire_pin(conn, pin_id, round, at)
            .expect("retire")
            .expect("an active pin retires")
    }

    fn hold_pass(conn: &mut SqliteConnection, now: chrono::DateTime<Utc>) -> HoldPass {
        retired_hold_pass(
            conn,
            &crate::db::AppContext::default_lamad(),
            now,
            &std::collections::HashSet::new(),
        )
        .expect("hold pass")
    }

    /// An admitted read-miss for `head` at `at`.
    fn read_miss(conn: &mut SqliteConnection, head: &str, at: chrono::DateTime<Utc>) -> DemandPin {
        record_demand_pin(conn, demand_for(head), at).expect("demand")
    }

    fn make_local(conn: &mut SqliteConnection, id: &str) {
        let result = crate::db::content_diesel::bulk_create_content(
            conn,
            &crate::db::AppContext::default_lamad(),
            vec![crate::db::content_diesel::CreateContentInput {
                id: id.to_string(),
                title: id.to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "public".to_string(),
                created_by: None,
                tags: vec![],
                dht_anchor_hash: None,
                content_body: Some("body".to_string()),
            }],
        )
        .expect("bulk_create_content");
        assert_eq!(result.inserted, 1);
    }

    #[test]
    fn each_consecutive_retirement_doubles_the_hold_up_to_a_day() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "e2e-nobody-holds-this");
        let mut now = t0();
        let mut holds = Vec::new();
        for rung in 1..=6 {
            assert_eq!(retire(&mut conn, pin.id, now), rung);
            let hold = pin_hold(rung);
            holds.push(hold.num_hours());

            // One minute short of the hold: still held, nothing re-admitted.
            let early = hold_pass(&mut conn, now + hold - Duration::minutes(1));
            assert!(early.readmitted.is_empty(), "rung {rung}: inside its hold");
            assert_eq!(early.held, 1);
            assert_eq!(reread(&mut conn, pin.id).status, "retired");

            // One minute past it: re-admitted, and the rung is KEPT — a cooldown
            // re-admission is a re-probe, not evidence.
            now = now + hold + Duration::minutes(1);
            let before = reread(&mut conn, pin.id).round;
            let due = hold_pass(&mut conn, now);
            assert_eq!(due.readmitted.len(), 1, "rung {rung}: hold elapsed");
            assert_eq!(due.readmitted[0].1, ReadmitCause::Hold);
            let row = reread(&mut conn, pin.id);
            assert_eq!(row.status, "active");
            assert_eq!(row.consecutive_retirements, rung);
            assert_eq!(row.round, before + 1, "re-admission opens a new round");
            assert_eq!(row.last_outcome.as_deref(), Some(PIN_OUTCOME_EXHAUSTED));
        }
        assert_eq!(holds, vec![6, 12, 24, 24, 24, 24]);
        assert_eq!(
            (1..=4).map(pin_hold_label).collect::<Vec<_>>(),
            vec!["6h", "12h", "24h", "24h"]
        );
        assert_eq!(PIN_HOLD_LABELS, ["6h", "12h", "24h"]);
    }

    #[test]
    fn a_retirement_names_its_round_and_a_stale_one_is_refused() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "reprobed-in-between");
        let measured = reread(&mut conn, pin.id).round;
        retire(&mut conn, pin.id, t0());

        // Its hold ends: re-admitted for a re-probe, in a new round.
        hold_pass(&mut conn, t0() + Duration::hours(7));
        assert_eq!(reread(&mut conn, pin.id).round, measured + 1);
        assert!(
            retire_pin(&mut conn, pin.id, measured, t0() + Duration::hours(7))
                .expect("retire")
                .is_none(),
            "an exhaustion measured in an earlier round cannot retire the pin"
        );
        assert_eq!(reread(&mut conn, pin.id).status, "active");

        // The current round's exhaustion does, and dates the next hold.
        assert_eq!(
            retire_pins(
                &mut conn,
                &[(pin.id, measured + 1), (9999, 0)],
                t0() + Duration::hours(8)
            )
            .expect("retire"),
            vec![(pin.id, 2)],
            "only what was retired is reported"
        );
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.retired_at.as_deref(), Some("2026-10-13T20:00:00Z"));
        assert_eq!(row.readmit_after.as_deref(), Some("2026-10-14T08:00:00Z"));
    }

    /// A re-pin never opens a new round, as before the ladder: a client
    /// re-asserting an active pin cannot renew its budget, and a re-pin of a
    /// retired pin keeps whatever retry state its tracker holds.
    #[test]
    fn a_repin_never_opens_a_new_round() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "asserted-again");
        for _ in 0..3 {
            assert_eq!(pin_for(&mut conn, "asserted-again").round, pin.round);
        }
        assert_eq!(
            retire_pin(&mut conn, pin.id, pin.round, t0()).expect("retire"),
            Some(1)
        );
        let back = pin_for(&mut conn, "asserted-again");
        assert_eq!((back.status.as_str(), back.round), ("active", pin.round));
    }

    #[test]
    fn a_fetch_resets_the_ladder() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "arrives-late");
        retire(&mut conn, pin.id, t0());
        hold_pass(&mut conn, t0() + Duration::hours(7));
        retire(&mut conn, pin.id, t0() + Duration::hours(8));
        hold_pass(&mut conn, t0() + Duration::hours(21));
        assert_eq!(reread(&mut conn, pin.id).consecutive_retirements, 2);

        // The bytes arrive: the reconcile tick sees the head local.
        let local: HashSet<String> = ["arrives-late".to_string()].into_iter().collect();
        let active = list_active_pins(&mut conn).expect("active");
        let ids = pins_needing_fetched_settle(&active, &local);
        assert_eq!(ids, vec![pin.id]);
        let settled =
            settle_fetched_pins(&mut conn, &ids, t0() + Duration::hours(22)).expect("settle");
        assert_eq!(settled.len(), 1);
        assert_eq!(settled[0].consecutive_retirements, 2, "the row as it was");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.consecutive_retirements, 0);
        assert_eq!(row.last_outcome.as_deref(), Some(PIN_OUTCOME_FETCHED));

        // Settled: the next tick has nothing to write.
        let active = list_active_pins(&mut conn).expect("active");
        assert!(pins_needing_fetched_settle(&active, &local).is_empty());
        assert!(settle_fetched_pins(&mut conn, &[], t0())
            .expect("noop")
            .is_empty());

        // The bytes go missing later and no peer has them: back to the first rung.
        assert_eq!(retire(&mut conn, pin.id, t0() + Duration::days(30)), 1);
    }

    #[test]
    fn settling_reports_only_the_rows_it_wrote() {
        let mut conn = test_conn();
        let here = pin_for(&mut conn, "always-here");
        let gone = pin_for(&mut conn, "unpinned-meanwhile");
        let local: HashSet<String> = ["always-here", "unpinned-meanwhile"]
            .into_iter()
            .map(String::from)
            .collect();
        let active = list_active_pins(&mut conn).expect("active");
        let mut ids = pins_needing_fetched_settle(&active, &local);
        ids.sort_unstable();
        assert_eq!(ids, vec![here.id, gone.id], "last_outcome is still NULL");

        // Un-pinned between the census and the write.
        set_pin_status(&mut conn, gone.id, "removed").expect("remove");
        let settled = settle_fetched_pins(&mut conn, &ids, t0()).expect("settle");
        assert_eq!(
            settled.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![here.id]
        );
        assert!(reread(&mut conn, gone.id).last_outcome.is_none());
        assert_eq!(reread(&mut conn, gone.id).status, "removed");

        let active = list_active_pins(&mut conn).expect("active");
        assert!(pins_needing_fetched_settle(&active, &local).is_empty());
        // A pin whose head is NOT local is never settled.
        let other = pin_for(&mut conn, "not-here");
        let active = list_active_pins(&mut conn).expect("active");
        assert!(pins_needing_fetched_settle(&active, &local).is_empty());
        assert!(reread(&mut conn, other.id).last_outcome.is_none());
    }

    #[test]
    fn content_that_becomes_local_while_its_pin_is_retired_settles_the_pin() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "fetched-in-the-foreground");
        let other = pin_for(&mut conn, "still-missing");
        for at in [0, 7, 20] {
            retire(&mut conn, pin.id, t0() + Duration::hours(at));
            retire(&mut conn, other.id, t0() + Duration::hours(at));
            hold_pass(&mut conn, t0() + Duration::hours(at) + Duration::hours(25));
        }
        let now = t0() + Duration::days(10);
        retire(&mut conn, pin.id, now);
        retire(&mut conn, other.id, now);
        let before = reread(&mut conn, pin.id);
        assert_eq!(
            (before.status.as_str(), before.consecutive_retirements),
            ("retired", 4)
        );

        // Something other than the pull queue supplies the bytes.
        make_local(&mut conn, "fetched-in-the-foreground");

        // One minute into a 24 h hold: the pass looks only at due rows, so the
        // pin is not settled yet (the inventory and fetch paths may get there
        // first).
        let early = hold_pass(&mut conn, now + Duration::minutes(1));
        assert!(early.settled.is_empty() && early.readmitted.is_empty());
        assert_eq!(early.held, 2);
        assert_eq!(reread(&mut conn, pin.id).status, "retired");

        // When it comes due it is settled, not re-probed; the pin whose head
        // is still missing is re-probed.
        let pass = hold_pass(&mut conn, now + Duration::hours(24) + Duration::minutes(1));
        assert_eq!(pass.settled.len(), 1);
        assert_eq!(pass.settled[0].id, pin.id);
        assert_eq!(pass.readmitted.len(), 1);
        assert_eq!(pass.readmitted[0].0.id, other.id);
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "active");
        assert_eq!(row.consecutive_retirements, 0);
        assert_eq!(row.last_outcome.as_deref(), Some(PIN_OUTCOME_FETCHED));
        assert_eq!(row.round, before.round + 1);
        assert_eq!(reread(&mut conn, other.id).consecutive_retirements, 4);
    }

    #[test]
    fn an_inventory_naming_the_head_readmits_at_once_and_resets_the_ladder() {
        let mut conn = test_conn();
        let named = pin_for(&mut conn, "named-head");
        let unnamed = pin_for(&mut conn, "unnamed-head");
        let reprobing = pin_for(&mut conn, "named-while-reprobing");
        let removed = pin_for(&mut conn, "named-but-removed");
        set_pin_status(&mut conn, removed.id, "removed").expect("remove");
        let mut now = t0();
        for _ in 0..3 {
            for id in [named.id, unnamed.id, reprobing.id] {
                retire(&mut conn, id, now);
            }
            now += Duration::days(2);
            hold_pass(&mut conn, now);
        }
        retire(&mut conn, named.id, now);
        retire(&mut conn, unnamed.id, now);
        let before = reread(&mut conn, named.id);
        assert_eq!(before.consecutive_retirements, 4);
        // `reprobing` is active, mid re-probe, still carrying three retirements.
        let reprobing_before = reread(&mut conn, reprobing.id);
        assert_eq!(
            (
                reprobing_before.status.as_str(),
                reprobing_before.consecutive_retirements
            ),
            ("active", 3)
        );

        // One minute into a 24 h hold a peer advertises the heads.
        let advertised: HashSet<&str> = [
            "named-head",
            "named-while-reprobing",
            "named-but-removed",
            "something-else",
        ]
        .into_iter()
        .collect();
        let pass = inventory_named_pins(&mut conn, &advertised, now + Duration::minutes(1))
            .expect("named");
        assert_eq!(
            pass.readmitted.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![named.id]
        );
        assert_eq!(
            pass.reset_active.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![reprobing.id]
        );
        let row = reread(&mut conn, named.id);
        assert_eq!(row.status, "active");
        assert_eq!(row.consecutive_retirements, 0, "ladder reset");
        assert_eq!(
            row.round, before.round,
            "the same round: the tracker is kept, as before the ladder"
        );

        // The pin already re-probing loses its count but keeps its round — an
        // advertiser that cannot serve must not renew its budget every round.
        let row = reread(&mut conn, reprobing.id);
        assert_eq!(
            (row.status.as_str(), row.consecutive_retirements),
            ("active", 0)
        );
        assert_eq!(row.round, reprobing_before.round);

        let held = reread(&mut conn, unnamed.id);
        assert_eq!(held.status, "retired");
        assert_eq!(
            held.consecutive_retirements, 4,
            "an unnamed pin keeps its rung"
        );
        assert_eq!(reread(&mut conn, removed.id).status, "removed");

        // Naming again changes nothing more.
        let again = inventory_named_pins(&mut conn, &advertised, now).expect("named");
        assert!(again.readmitted.is_empty() && again.reset_active.is_empty());

        // Named and it STILL exhausts: first rung again, not the fifth.
        assert_eq!(retire(&mut conn, named.id, now + Duration::hours(1)), 1);
    }

    /// An ADMITTED read does what it did before the ladder: it re-activates the
    /// retired pin at once, in the read path — keeping its rung.
    #[test]
    fn an_admitted_read_reactivates_a_retired_pin_at_once_and_keeps_its_rung() {
        let mut conn = test_conn();
        let read = pin_for(&mut conn, "e2e-read-again");
        let mut now = t0();
        for _ in 0..2 {
            retire(&mut conn, read.id, now);
            now += Duration::days(2);
            hold_pass(&mut conn, now);
        }
        // Rung 3: the 24 h rung.
        assert_eq!(retire(&mut conn, read.id, now), 3);
        let retired = reread(&mut conn, read.id);

        let demand = read_miss(&mut conn, "e2e-read-again", now + Duration::minutes(2));
        assert!(matches!(demand, DemandPin::Reactivated(_)), "{demand:?}");
        let row = reread(&mut conn, read.id);
        assert_eq!(row.status, "active");
        assert_eq!(row.consecutive_retirements, 3, "the rung is kept");
        assert_eq!(
            row.round, retired.round,
            "no new round, as before the ladder"
        );
        assert_eq!(row.retired_at, retired.retired_at);
        assert_eq!(row.updated_at, "2026-10-17T12:02:00Z");

        // Nothing is left for the hold pass.
        let pass = hold_pass(&mut conn, now + Duration::minutes(3));
        assert!(pass.readmitted.is_empty() && pass.settled.is_empty());

        // Still unfillable: one rung up, and without another read it serves
        // its 24 h in full.
        let again = now + Duration::minutes(10);
        assert_eq!(retire(&mut conn, read.id, again), 4);
        assert!(hold_pass(&mut conn, again + Duration::hours(23))
            .readmitted
            .is_empty());
        let own = hold_pass(&mut conn, again + Duration::hours(25));
        assert_eq!(own.readmitted.len(), 1);
        assert_eq!(own.readmitted[0].1, ReadmitCause::Hold);
    }

    /// A read dev's autopin would NOT have admitted (throttled, window full,
    /// ceiling) only marks the retired row as read. A marked row's hold is
    /// capped at the first rung: re-admitted no later than `retired_at + 6 h`,
    /// which is when dev's flat cooldown would have — and no sooner.
    #[test]
    fn a_non_admitted_read_marks_the_row_and_caps_its_hold_at_six_hours() {
        let mut conn = test_conn();
        let read = pin_for(&mut conn, "read-but-throttled");
        let unread = pin_for(&mut conn, "nobody-reads");
        let mut now = t0();
        for _ in 0..2 {
            retire(&mut conn, read.id, now);
            retire(&mut conn, unread.id, now);
            now += Duration::days(2);
            hold_pass(&mut conn, now);
        }
        assert_eq!(retire(&mut conn, read.id, now), 3);
        assert_eq!(retire(&mut conn, unread.id, now), 3);
        let retired = reread(&mut conn, read.id);
        let mark = |conn: &mut SqliteConnection, head: &str| {
            mark_read_since_retired(conn, "local-device", head, "item").expect("mark")
        };

        assert!(
            mark(&mut conn, "read-but-throttled"),
            "the first read marks the row"
        );
        for _ in 0..3 {
            assert!(
                !mark(&mut conn, "read-but-throttled"),
                "already marked: nothing more is written"
            );
        }
        let row = reread(&mut conn, read.id);
        assert_eq!(row.status, "retired");
        assert_eq!(row.read_since_retired, 1);
        assert_eq!(row.retired_at, retired.retired_at);
        assert_eq!(row.updated_at, retired.updated_at);
        assert_eq!(row.round, retired.round);

        // Nothing it does not apply to is touched: no pin, an active pin, a
        // removed pin.
        let active = pin_for(&mut conn, "active-head");
        let removed = pin_for(&mut conn, "removed-head");
        set_pin_status(&mut conn, removed.id, "removed").expect("remove");
        assert!(!mark(&mut conn, "never-pinned"));
        assert!(!mark(&mut conn, "active-head"));
        assert!(!mark(&mut conn, "removed-head"));
        assert_eq!(reread(&mut conn, active.id).read_since_retired, 0);
        assert_eq!(reread(&mut conn, removed.id).read_since_retired, 0);
        assert_eq!(list_all_pins(&mut conn).expect("list").len(), 4);

        // 5 h 59 m after retirement: held. The mark does not re-admit.
        let early = hold_pass(&mut conn, now + Duration::minutes(359));
        assert!(early.readmitted.is_empty());
        assert_eq!(early.held, 2);

        // 6 h 1 m: the marked pin is re-admitted — rung kept, mark cleared —
        // and the pin nobody read stays on its 24 h.
        let capped = hold_pass(&mut conn, now + Duration::minutes(361));
        assert_eq!(capped.readmitted.len(), 1);
        assert_eq!(capped.readmitted[0].0.id, read.id);
        assert_eq!(capped.readmitted[0].1, ReadmitCause::Read);
        let row = reread(&mut conn, read.id);
        assert_eq!(
            (row.status.as_str(), row.consecutive_retirements),
            ("active", 3)
        );
        assert_eq!(row.read_since_retired, 0);
        assert_eq!(reread(&mut conn, unread.id).status, "retired");
        let own = hold_pass(&mut conn, now + Duration::hours(25));
        assert_eq!(own.readmitted.len(), 1);
        assert_eq!(own.readmitted[0].1, ReadmitCause::Hold);
    }

    /// Two callers that both read the row as unmarked before either writes:
    /// the guarded write marks it once.
    #[test]
    fn racing_read_marks_write_the_row_once() {
        let pool = crate::test_util::test_pool();
        let mut a = pool.get().expect("conn a");
        let mut b = pool.get().expect("conn b");
        let pin = pin_for(&mut a, "read-twice-at-once");
        retire(&mut a, pin.id, t0());
        let seen_a = unmarked_retired_row(&mut a, "local-device", "read-twice-at-once", "item")
            .expect("read a")
            .expect("unmarked");
        let seen_b = unmarked_retired_row(&mut b, "local-device", "read-twice-at-once", "item")
            .expect("read b")
            .expect("unmarked");
        assert_eq!(seen_a, seen_b);
        assert_eq!(seen_a.deadline.as_deref(), Some("2026-10-13T18:00:00Z"));
        assert!(mark_row_read(&mut a, &seen_a).expect("write a"));
        assert!(
            !mark_row_read(&mut b, &seen_b).expect("write b"),
            "the second write finds the row already marked"
        );
        // An admitted read that re-activated it first also stops a mark.
        let other = pin_for(&mut a, "demanded-then-read");
        retire(&mut a, other.id, t0());
        let seen = unmarked_retired_row(&mut a, "local-device", "demanded-then-read", "item")
            .expect("read")
            .expect("unmarked");
        read_miss(&mut b, "demanded-then-read", t0());
        assert!(!mark_row_read(&mut a, &seen).expect("write"));
    }

    /// A read that stalled between its read and its write, across a
    /// re-activation and a new retirement, marks nothing: the deadline it
    /// computed belongs to the old hold.
    #[test]
    fn a_stalled_mark_does_not_stamp_an_old_deadline_on_a_new_retirement() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().expect("conn");
        let pin = pin_for(&mut conn, "stalled-read");
        retire(&mut conn, pin.id, t0());
        let seen = unmarked_retired_row(&mut conn, "local-device", "stalled-read", "item")
            .expect("read")
            .expect("unmarked");
        // Inventory re-activates it, a fresh round exhausts, it retires again.
        let advertised: std::collections::HashSet<&str> = ["stalled-read"].into_iter().collect();
        inventory_named_pins(&mut conn, &advertised, t0() + Duration::hours(1)).expect("named");
        retire(&mut conn, pin.id, t0() + Duration::hours(2));
        let before = reread(&mut conn, pin.id);
        assert!(!mark_row_read(&mut conn, &seen).expect("write"));
        let after = reread(&mut conn, pin.id);
        assert_eq!(after.read_since_retired, 0);
        assert_eq!(after.readmit_after, before.readmit_after);
        assert_eq!(after.readmit_after.as_deref(), Some("2026-10-13T20:00:00Z"));
    }

    /// The due check keeps whole seconds; a pass a fraction of a second past
    /// the deadline admits the pin, as dev's `now - retired_at > 6h` did.
    #[test]
    fn a_pass_within_the_deadline_second_admits_the_pin() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().expect("conn");
        let pin = pin_for(&mut conn, "due-on-the-second");
        retire(&mut conn, pin.id, t0());
        let deadline = t0() + PIN_READMIT_COOLDOWN;
        // Exactly on the deadline is not past it: dev's comparison is strict.
        assert!(hold_pass(&mut conn, deadline).readmitted.is_empty());
        assert_eq!(reread(&mut conn, pin.id).status, "retired");
        let pass = hold_pass(&mut conn, deadline + Duration::milliseconds(250));
        assert_eq!(pass.readmitted.len(), 1);
        assert_eq!(reread(&mut conn, pin.id).status, "active");
    }

    /// A due pin whose tracker survived the hold keeps its round, so the
    /// tracker — and its spent budget — is kept; one without a tracker opens a
    /// new round.
    #[test]
    fn a_surviving_tracker_keeps_its_round_through_readmission() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().expect("conn");
        let kept = pin_for(&mut conn, "tracker-survived");
        let fresh = pin_for(&mut conn, "tracker-evicted");
        retire(&mut conn, kept.id, t0());
        retire(&mut conn, fresh.id, t0());
        let (kept_round, fresh_round) = (
            reread(&mut conn, kept.id).round,
            reread(&mut conn, fresh.id).round,
        );
        let tracked: std::collections::HashSet<i32> = [kept.id].into_iter().collect();
        let pass = retired_hold_pass(
            &mut conn,
            &crate::db::AppContext::default_lamad(),
            t0() + Duration::hours(7),
            &tracked,
        )
        .expect("hold pass");
        assert_eq!(pass.readmitted.len(), 2);
        assert_eq!(reread(&mut conn, kept.id).round, kept_round);
        assert_eq!(reread(&mut conn, fresh.id).round, fresh_round + 1);
    }

    /// The pass touches only due rows, checks their presence in chunks far
    /// under SQLite's variable limit, and handles every due row in one pass.
    #[test]
    fn a_large_due_backlog_is_handled_in_one_pass_with_chunked_presence() {
        let mut conn = test_conn();
        let n = 1_201;
        for i in 0..n {
            let p = pin_for(&mut conn, &format!("backlog-{i:04}"));
            retire(&mut conn, p.id, t0());
        }
        // Not due yet: nothing at all.
        let quiet = hold_pass(&mut conn, t0() + Duration::hours(5));
        assert!(quiet.readmitted.is_empty() && quiet.settled.is_empty());
        assert_eq!(quiet.held, n);

        // Every other head turned up locally meanwhile.
        let local: Vec<String> = (0..n)
            .step_by(2)
            .map(|i| format!("backlog-{i:04}"))
            .collect();
        let result = crate::db::content_diesel::bulk_create_content(
            &mut conn,
            &crate::db::AppContext::default_lamad(),
            local
                .iter()
                .map(|id| crate::db::content_diesel::CreateContentInput {
                    id: id.clone(),
                    title: id.clone(),
                    description: None,
                    content_type: "concept".to_string(),
                    content_format: "markdown".to_string(),
                    blob_hash: None,
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "public".to_string(),
                    created_by: None,
                    tags: vec![],
                    dht_anchor_hash: None,
                    content_body: Some("body".to_string()),
                })
                .collect(),
        )
        .expect("bulk_create_content");
        assert_eq!(result.inserted as usize, local.len());
        let all: Vec<String> = (0..n).map(|i| format!("backlog-{i:04}")).collect();
        assert_eq!(
            crate::db::content_diesel::content_ids_present(
                &mut conn,
                &crate::db::AppContext::default_lamad(),
                &all
            )
            .expect("presence")
            .len(),
            local.len(),
            "more ids than one chunk"
        );

        let pass = hold_pass(&mut conn, t0() + Duration::hours(7));
        assert_eq!(pass.settled.len(), local.len());
        assert_eq!(pass.readmitted.len(), n - local.len());
        assert_eq!(pass.held, 0);
        assert_eq!(count_active_pins(&mut conn).expect("count"), n as i64);
    }

    /// If presence cannot be read, the due rows are re-admitted as before the
    /// ladder; the pass is not rolled back.
    #[test]
    fn a_failing_presence_check_still_readmits_the_due_rows() {
        use diesel::connection::SimpleConnection;

        let mut conn = test_conn();
        for i in 0..3 {
            let p = pin_for(&mut conn, &format!("due-{i}"));
            retire(&mut conn, p.id, t0());
        }
        conn.batch_execute("ALTER TABLE content RENAME TO content_unreachable")
            .expect("break the presence query");
        let pass = hold_pass(&mut conn, t0() + Duration::hours(7));
        assert!(pass.settled.is_empty());
        assert_eq!(pass.readmitted.len(), 3);
        assert_eq!(count_active_pins(&mut conn).expect("count"), 3);
    }

    #[test]
    fn a_read_miss_does_not_resurrect_a_removed_pin_but_a_repin_does() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "unpinned-by-its-person");
        set_pin_status(&mut conn, pin.id, "removed").expect("remove");
        let removed = reread(&mut conn, pin.id);

        let demand = read_miss(&mut conn, "unpinned-by-its-person", t0());
        assert!(matches!(demand, DemandPin::NotRevived(_)), "{demand:?}");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "removed");
        assert_eq!(row.updated_at, removed.updated_at, "nothing was written");

        // The substrate's own passes leave a removal alone too, however long
        // it has been.
        assert!(retire_pin(&mut conn, pin.id, removed.round, t0())
            .expect("retire")
            .is_none());
        let pass = hold_pass(&mut conn, t0() + Duration::days(30));
        assert!(pass.readmitted.is_empty() && pass.settled.is_empty());
        let advertised: HashSet<&str> = ["unpinned-by-its-person"].into_iter().collect();
        let named = inventory_named_pins(&mut conn, &advertised, t0()).expect("named");
        assert!(named.readmitted.is_empty() && named.reset_active.is_empty());
        assert_eq!(reread(&mut conn, pin.id).status, "removed");

        // The person pinning it again is the one thing that revives it.
        pin_for(&mut conn, "unpinned-by-its-person");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "active");
        assert_eq!(row.round, removed.round);
    }

    /// An admitted read on a head with no pin authors one; on an active pin it
    /// refreshes it exactly as the re-pin it used to be did — demand defaults
    /// for priority and closure rule, `updated_at` stamped, no new round.
    #[test]
    fn a_read_miss_authors_a_new_pin_and_refreshes_an_existing_one_as_before() {
        let mut conn = test_conn();
        let first = read_miss(&mut conn, "fresh-head", t0());
        let DemandPin::Authored(pin) = first else {
            panic!("a never-seen head must author a pin: {first:?}")
        };
        assert_eq!(pin.status, "active");
        assert_eq!(pin.created_at, "2026-10-13T12:00:00Z");
        assert_eq!(pin.consecutive_retirements, 0);

        upsert_pin(
            &mut conn,
            NewAcquisitionPin {
                closure_rule_json: Some(r#"{"depth":2}"#.into()),
                priority: 7,
                ..demand_for("fresh-head")
            },
        )
        .expect("explicit re-pin");
        let again = read_miss(&mut conn, "fresh-head", t0() + Duration::days(1));
        assert!(matches!(again, DemandPin::Refreshed(_)), "{again:?}");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.priority, 1);
        assert!(row.closure_rule_json.is_none());
        assert_eq!(row.status, "active");
        assert_eq!(row.updated_at, "2026-10-14T12:00:00Z");
        assert_eq!(row.round, pin.round);
    }

    #[test]
    fn an_explicit_repin_overrides_the_hold_but_keeps_the_rung() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "wanted-again");
        retire(&mut conn, pin.id, t0());
        hold_pass(&mut conn, t0() + Duration::hours(7));
        retire(&mut conn, pin.id, t0() + Duration::hours(8));
        let retired = reread(&mut conn, pin.id);

        pin_for(&mut conn, "wanted-again");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "active", "a person's re-pin is not held");
        assert_eq!(row.consecutive_retirements, 2, "a re-pin is not evidence");
        assert_eq!(
            row.round, retired.round,
            "no new round, as before the ladder"
        );
        assert_eq!(retire(&mut conn, pin.id, t0() + Duration::hours(9)), 3);
    }

    /// `set_commitment_cid` stamps `updated_at` with no status guard, and its
    /// caller is asynchronous: it can land on a row that has since retired.
    /// The hold must not move.
    #[test]
    fn unrelated_bookkeeping_on_a_retired_row_does_not_restart_its_hold() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "late-backfill");
        retire(&mut conn, pin.id, t0());
        let retired = reread(&mut conn, pin.id);

        set_commitment_cid(&mut conn, pin.id, "uhCkk-late").expect("backfill");
        let row = reread(&mut conn, pin.id);
        assert_ne!(
            row.updated_at, retired.updated_at,
            "the backfill stamps updated_at now"
        );
        assert_eq!(row.retired_at.as_deref(), Some("2026-10-13T12:00:00Z"));

        assert!(hold_pass(&mut conn, t0() + Duration::hours(5))
            .readmitted
            .is_empty());
        assert_eq!(
            hold_pass(&mut conn, t0() + Duration::hours(7))
                .readmitted
                .len(),
            1
        );
    }

    /// A retired row whose `retired_at` is missing or unreadable is not
    /// re-admitted early: it is repaired to a first-rung hold measured from the
    /// pass that noticed, once.
    #[test]
    fn an_unreadable_retirement_clock_fails_closed_to_the_base_hold() {
        use diesel::connection::SimpleConnection;

        let mut conn = test_conn();
        let null_clock = pin_for(&mut conn, "retired-by-an-older-build");
        let sql_clock = pin_for(&mut conn, "sql-default-format");
        conn.batch_execute(
            "UPDATE acquisition_pins SET status = 'retired', consecutive_retirements = 4,
                    retired_at = NULL WHERE head_ref = 'retired-by-an-older-build';
             UPDATE acquisition_pins SET status = 'retired', consecutive_retirements = 0,
                    retired_at = '2026-10-01 12:00:00' WHERE head_ref = 'sql-default-format';",
        )
        .expect("rows with no readable clock");

        let pass = hold_pass(&mut conn, t0());
        assert!(
            pass.readmitted.is_empty(),
            "never re-admitted on an unreadable clock"
        );
        assert_eq!(pass.clocks_repaired.len(), 2);
        assert_eq!(pass.held, 2);
        for id in [null_clock.id, sql_clock.id] {
            let row = reread(&mut conn, id);
            assert_eq!(row.status, "retired");
            assert_eq!(row.retired_at.as_deref(), Some("2026-10-13T12:00:00Z"));
            assert_eq!(row.consecutive_retirements, 1);
        }

        // Repaired once: the next pass has nothing to repair, and the hold is
        // six hours from the repair.
        let pass = hold_pass(&mut conn, t0() + Duration::minutes(359));
        assert!(pass.clocks_repaired.is_empty() && pass.readmitted.is_empty());
        let pass = hold_pass(&mut conn, t0() + Duration::minutes(361));
        assert_eq!(pass.readmitted.len(), 2);
    }

    /// Upgrade: a database written by a build without the backoff columns.
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
                 ('old-offset',  'retired', '2026-10-13T08:30:00+02:00'),
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
        assert_eq!(
            old_retired.retired_at.as_deref(),
            Some("2026-10-13T06:30:00Z")
        );
        assert_eq!(
            old_retired.readmit_after.as_deref(),
            Some("2026-10-13T12:30:00Z"),
            "its 6 h deadline, backfilled"
        );
        // Written with an offset: normalised to UTC in the code's format, so
        // its deadline orders correctly against every other stamp.
        let old_offset = by_head("old-offset");
        assert_eq!(
            old_offset.retired_at.as_deref(),
            Some("2026-10-13T06:30:00Z")
        );
        assert_eq!(
            old_offset.readmit_after.as_deref(),
            Some("2026-10-13T12:30:00Z")
        );
        for head in ["old-retired", "old-active", "old-removed"] {
            assert_eq!(by_head(head).round, 0, "{head}");
            assert_eq!(by_head(head).read_since_retired, 0, "{head}");
        }
        for head in ["old-active", "old-removed"] {
            assert_eq!(by_head(head).consecutive_retirements, 0, "{head}");
            assert!(by_head(head).last_outcome.is_none(), "{head}");
            assert!(by_head(head).retired_at.is_none(), "{head}");
            assert!(by_head(head).readmit_after.is_none(), "{head}");
        }

        // Retired 5.5 h before t0: still inside the 6 h hold it was already
        // serving, due a little later, exactly as on the old build.
        assert!(hold_pass(&mut conn, t0()).readmitted.is_empty());
        let due = hold_pass(&mut conn, t0() + Duration::hours(1));
        assert_eq!(
            due.readmitted.len(),
            2,
            "both due at 12:30 UTC, whatever offset the clock was written with"
        );
        // Its next failure steps to 12 h rather than repeating 6 h.
        assert_eq!(
            retire(&mut conn, old_retired.id, t0() + Duration::hours(2)),
            2
        );
        let row = reread(&mut conn, old_retired.id);
        assert_eq!(
            pin_readmit_at(row.retired_at.as_deref(), row.consecutive_retirements),
            Some(t0() + Duration::hours(14))
        );
    }
}
