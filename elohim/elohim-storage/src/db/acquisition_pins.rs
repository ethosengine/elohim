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

/// Most retired pins the hold pass re-admits in one reconcile tick.
///
/// Pins retired in the same hour come due in the same hour; un-bounded, a
/// thousand of them moved `pull.total` by a thousand in one tick. Sized to
/// `p2p::acquisition::MAX_ACQUISITION_INFLIGHT` (asserted equal in that
/// module's tests): admitting more per tick than one dispatch round can put in
/// flight only parks the surplus in the queue while it already counts against
/// caught-up. Demand goes through this same door — a read-miss never flips a
/// status itself — so nothing a reader does can move `total` faster than this.
/// The inventory arm is NOT bounded by it: a peer naming a head is evidence,
/// and it re-admits immediately.
pub const MAX_PIN_READMITS_PER_TICK: usize = 25;

/// Of [`MAX_PIN_READMITS_PER_TICK`], the slots that go to the longest-overdue
/// pins whatever their demand, before demanded pins are preferred for the rest.
///
/// Without a reserve, 25 demanded pins coming due every tick would postpone an
/// undemanded due pin for as long as the demand lasted. With it, a due pin
/// behind `k` longer-overdue pins is re-admitted within `ceil((k + 1) / 10)`
/// ticks under any demand. Ten of 25: demand is a reader waiting now, so it
/// keeps the majority (15, plus any reserve the oldest pins do not use), while
/// the worst case for the whole alpha backlog (about 1100 due at once) stays
/// under two hours at the 60 s tick.
pub const PIN_READMITS_RESERVED_FOR_OLDEST: usize = 10;

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

/// Parse a `retired_at` stamp (`current_timestamp()`'s RFC 3339,
/// `%Y-%m-%dT%H:%M:%SZ`). `None` for NULL or anything else.
pub fn parse_retired_at(retired_at: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(retired_at?)
        .ok()
        .map(|t| t.with_timezone(&chrono::Utc))
}

/// When a retired pin's hold ends, or `None` when its `retired_at` is missing
/// or unreadable.
///
/// The hold is [`pin_hold`] for the pin's rung — unless somebody has read the
/// head while it was held (`demanded`), which CAPS it at the first rung
/// ([`PIN_READMIT_COOLDOWN`]). Demand does not end a hold the node has only
/// just concluded, but a head someone is asking for is re-probed within 6 h of
/// its last retirement however high its rung; heads nobody reads keep climbing
/// to a week. So a read-often, never-fillable pin re-probes at most once per
/// 6 h, and its ladder is not reset by the reading.
///
/// `None` is NOT "due". The hold pass ([`retired_hold_pass`]) repairs such a
/// row to a first-rung hold measured from the moment it noticed — fail closed —
/// rather than re-admitting early on a clock it cannot read.
pub fn pin_readmit_at(
    retired_at: Option<&str>,
    consecutive_retirements: i32,
    demanded: bool,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let hold = if demanded {
        PIN_READMIT_COOLDOWN
    } else {
        pin_hold(consecutive_retirements)
    };
    parse_retired_at(retired_at).map(|t| t + hold)
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
/// Every re-pin bumps `round`, active row or not: the in-memory retry state
/// for the pin is discarded at the next reconcile, and a retirement computed
/// from the exhaustion that preceded the re-pin no longer matches the row.
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
            pins::held_demand_count.eq(0),
            pins::round.eq(pins::round + 1),
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
// a clock. Every re-activation bumps `round`.

type Now = chrono::DateTime<chrono::Utc>;

/// Retire exhausted pins: `active` → `retired`, one rung up the ladder each,
/// `retired_at` stamped.
///
/// `exhausted` pairs each pin id with the `round` whose retry budget was spent
/// (`AcquisitionState::exhausted_pins`). A pin whose row is no longer `active`
/// (un-pinned meanwhile), or whose `round` has moved on (re-pinned, or named by
/// an inventory, since that exhaustion was measured), is left alone — a stale
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
    /// Its hold had not elapsed, but it was read while held and the first rung
    /// has.
    Demand,
}

/// What one [`retired_hold_pass`] committed.
#[derive(Debug, Default)]
pub struct HoldPass {
    /// Retired pins whose head turned out to be local: re-activated, ladder
    /// reset, `last_outcome = fetched`. Rows as read before the write.
    pub settled: Vec<AcquisitionPin>,
    /// Pins re-admitted for a re-probe, with why. Rows as read before the
    /// write, so `consecutive_retirements` is the rung each one served.
    pub readmitted: Vec<(AcquisitionPin, ReadmitCause)>,
    /// Pins that are due but that this tick's bound left for a later one.
    pub deferred: usize,
    /// Pins still inside their hold (including any just repaired).
    pub held: usize,
    /// Retired pins whose `retired_at` was missing or unreadable, now holding
    /// on the first rung from `now`. Rows as read before the repair.
    pub clocks_repaired: Vec<AcquisitionPin>,
}

/// The per-tick pass over the retired set. In order:
///
/// 1. **Settle.** A retired pin whose head is local — a foreground fetch,
///    replication, or any other ingestion supplied it while the pin was held —
///    has its answer: `active`, ladder reset, `last_outcome = fetched`. Not
///    bounded by `limit`; nothing is fetched for it.
/// 2. **Repair.** A retired pin with no readable `retired_at` cannot be timed.
///    It is NOT re-admitted: `retired_at` becomes `now` and the pin drops to
///    the first rung, so it holds [`PIN_READMIT_COOLDOWN`] from here — fail
///    closed, once (the repaired row is readable from then on).
/// 3. **Re-admit.** Pins whose hold has ended ([`pin_readmit_at`], demand-capped)
///    go back to `active`, at most `limit` of them: the `reserved` longest
///    overdue first whatever their demand, then demanded pins, then the rest
///    by how long overdue ([`PIN_READMITS_RESERVED_FOR_OLDEST`]).
///
/// A re-admission keeps `consecutive_retirements` — it is a re-probe, not
/// evidence. Only a fetch or an inventory naming the head resets the ladder.
///
/// bounded-work: one read of the retired set, one presence query over its
/// heads, and at most `limit` + settled + repaired row writes.
pub fn retired_hold_pass(
    conn: &mut SqliteConnection,
    app_ctx: &crate::db::AppContext,
    now: Now,
    limit: usize,
    reserved: usize,
) -> QueryResult<HoldPass> {
    let stamped = stamp(now);
    conn.immediate_transaction(|conn| {
        let retired = list_pins_by_status(conn, "retired")?;
        if retired.is_empty() {
            return Ok(HoldPass::default());
        }
        let heads: Vec<String> = retired.iter().map(|p| p.head_ref.clone()).collect();
        let local = crate::db::content_diesel::content_ids_present(conn, app_ctx, &heads)?;
        let mut pass = HoldPass::default();
        let mut due: Vec<(Now, ReadmitCause, AcquisitionPin)> = Vec::new();
        let retired_row = |id: i32| {
            pins::acquisition_pins
                .filter(pins::id.eq(id))
                .filter(pins::status.eq("retired"))
        };
        for pin in retired {
            if local.contains(&pin.head_ref) {
                diesel::update(retired_row(pin.id))
                    .set((
                        pins::status.eq("active"),
                        pins::consecutive_retirements.eq(0),
                        pins::held_demand_count.eq(0),
                        pins::last_outcome.eq(PIN_OUTCOME_FETCHED),
                        pins::round.eq(pins::round + 1),
                        pins::updated_at.eq(&stamped),
                    ))
                    .execute(conn)?;
                pass.settled.push(pin);
                continue;
            }
            let demanded = pin.held_demand_count > 0;
            let Some(readmit_at) = pin_readmit_at(
                pin.retired_at.as_deref(),
                pin.consecutive_retirements,
                demanded,
            ) else {
                diesel::update(retired_row(pin.id))
                    .set((
                        pins::retired_at.eq(&stamped),
                        pins::consecutive_retirements.eq(1),
                    ))
                    .execute(conn)?;
                pass.clocks_repaired.push(pin);
                pass.held += 1;
                continue;
            };
            if now <= readmit_at {
                pass.held += 1;
                continue;
            }
            // Due. By its own hold, or only because demand capped it?
            let own = pin_readmit_at(
                pin.retired_at.as_deref(),
                pin.consecutive_retirements,
                false,
            );
            let cause = match own {
                Some(own) if now > own => ReadmitCause::Hold,
                _ => ReadmitCause::Demand,
            };
            due.push((readmit_at, cause, pin));
        }

        // Longest overdue first; the first `reserved` are taken as they come.
        due.sort_by(|(a_at, _, a), (b_at, _, b)| a_at.cmp(b_at).then(a.id.cmp(&b.id)));
        let mut rest = due.split_off(reserved.min(limit).min(due.len()));
        // Stable: within each demand class the overdue order is kept.
        rest.sort_by_key(|(_, _, p)| p.held_demand_count == 0);
        due.extend(rest);
        pass.deferred = due.len().saturating_sub(limit);
        for (_, cause, pin) in due.into_iter().take(limit) {
            diesel::update(retired_row(pin.id))
                .set((
                    pins::status.eq("active"),
                    pins::held_demand_count.eq(0),
                    pins::round.eq(pins::round + 1),
                    pins::updated_at.eq(&stamped),
                ))
                .execute(conn)?;
            pass.readmitted.push((pin, cause));
        }
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
///   its rung and outside the per-tick bound, with its ladder reset.
/// - An `active` pin for a named head that still carries a retirement count
///   (re-admitted for a re-probe) has the count reset. Its `round` is NOT
///   bumped: a peer that advertises a head on every inventory round but cannot
///   serve it must not hand the pin a fresh retry budget each time.
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
                        pins::held_demand_count.eq(0),
                        pins::round.eq(pins::round + 1),
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

/// What a read-miss did to the pin ledger ([`record_demand_pin`]). Each carries
/// the row as it stands after the call.
#[derive(Debug)]
pub enum DemandPin {
    /// No pin existed for the head; one was born `active`.
    Authored(AcquisitionPin),
    /// An `active` pin already wants the head. Nothing written.
    AlreadyActive(AcquisitionPin),
    /// The pin is `retired` and less than [`PIN_READMIT_COOLDOWN`] has passed
    /// since it retired (or its clock is unreadable). Demand counted; the pin
    /// holds until the first rung ends, then the hold pass re-admits it.
    HeldRetired(AcquisitionPin),
    /// The pin is `retired` and the first rung has already ended. Demand
    /// counted; the next hold pass re-admits it (within its per-tick bound).
    DueOnDemand(AcquisitionPin),
    /// The pin is `removed` (or `paused`) — a person's decision. Demand
    /// counted on the row; the pin is not revived.
    NotRevived(AcquisitionPin),
}

/// Record demand for a head — the read-miss writer (`services::demand_autopin`).
///
/// Unlike [`upsert_pin`] this never changes the status of a row that exists:
///
/// - `retired`: the demand is counted, and a counted demand CAPS the pin's hold
///   at the first rung ([`pin_readmit_at`]). The read does not flip the row
///   itself — the hold pass does, on its next tick, so reads cannot move
///   `pull.total` faster than that pass's bound — and it does not reset the
///   ladder. Why a cap and not "ignore": the read path's own foreground fetch
///   is not a substitute for the acquisition round. It exists only where the
///   HTTP server has a libp2p handle (not on a pure-iroh node), and it asks
///   exactly one peer — the first connected — for the head and for each
///   shard. A different peer holding the bytes is found only by the pull
///   queue, which is what re-admission restarts.
/// - `removed` stays removed: the person un-pinned it, and a read is not them
///   pinning it again. Before this function a read-miss went through
///   [`upsert_pin`] and silently re-activated it. The consequence is plain: a
///   read for a removed pin's content is served ONLY by that foreground fetch
///   — nothing on a pure-iroh node, one peer otherwise — and never by the pull
///   queue, until the person re-pins.
/// - `active` is left alone, so a read-miss no longer overwrites an explicit
///   pin's priority and closure rule with the demand defaults.
///
/// No timestamp on an existing row is touched.
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
        let Some(existing) = identity().first::<AcquisitionPin>(conn).optional()? else {
            let stamped = stamp(now);
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
        if existing.status == "active" {
            return Ok(DemandPin::AlreadyActive(existing));
        }
        diesel::update(pins::acquisition_pins.filter(pins::id.eq(existing.id)))
            .set(pins::held_demand_count.eq(pins::held_demand_count + 1))
            .execute(conn)?;
        let row: AcquisitionPin = identity().first(conn)?;
        if row.status != "retired" {
            return Ok(DemandPin::NotRevived(row));
        }
        let first_rung_over =
            pin_readmit_at(row.retired_at.as_deref(), row.consecutive_retirements, true)
                .is_some_and(|at| now > at);
        Ok(if first_rung_over {
            DemandPin::DueOnDemand(row)
        } else {
            DemandPin::HeldRetired(row)
        })
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

    /// The hold pass with the production bound and reserve.
    fn hold_pass(conn: &mut SqliteConnection, now: chrono::DateTime<Utc>) -> HoldPass {
        retired_hold_pass(
            conn,
            &crate::db::AppContext::default_lamad(),
            now,
            MAX_PIN_READMITS_PER_TICK,
            PIN_READMITS_RESERVED_FOR_OLDEST,
        )
        .expect("hold pass")
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
    fn each_consecutive_retirement_doubles_the_hold_up_to_a_week() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "e2e-nobody-holds-this");
        let mut now = t0();
        let mut holds = Vec::new();
        for rung in 1..=8 {
            assert_eq!(retire(&mut conn, pin.id, now), rung);
            let hold = pin_hold(rung);
            holds.push(hold.num_hours());

            // One minute short of the hold: still held, nothing re-admitted.
            let early = hold_pass(&mut conn, now + hold - Duration::minutes(1));
            assert!(early.readmitted.is_empty(), "rung {rung}: inside its hold");
            assert_eq!((early.held, early.deferred), (1, 0));
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
        assert_eq!(holds, vec![6, 12, 24, 48, 96, 168, 168, 168]);
    }

    #[test]
    fn a_retirement_names_its_round_and_a_stale_one_is_refused() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "repinned-in-between");
        let measured = reread(&mut conn, pin.id).round;

        // The person pins it again after the exhaustion was measured.
        pin_for(&mut conn, "repinned-in-between");
        assert_eq!(reread(&mut conn, pin.id).round, measured + 1);
        assert!(
            retire_pin(&mut conn, pin.id, measured, t0())
                .expect("retire")
                .is_none(),
            "an exhaustion measured in an earlier round cannot retire the pin"
        );
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "active");
        assert_eq!(row.consecutive_retirements, 0);
        assert!(row.retired_at.is_none());

        // The current round's exhaustion does.
        assert_eq!(
            retire_pins(&mut conn, &[(pin.id, measured + 1), (9999, 0)], t0()).expect("retire"),
            vec![(pin.id, 1)],
            "only what was retired is reported"
        );
        assert_eq!(
            reread(&mut conn, pin.id).retired_at.as_deref(),
            Some("2026-10-13T12:00:00Z")
        );
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

        // One minute into a 48 h hold.
        let pass = hold_pass(&mut conn, now + Duration::minutes(1));
        assert_eq!(pass.settled.len(), 1);
        assert_eq!(pass.settled[0].id, pin.id);
        assert!(pass.readmitted.is_empty());
        assert_eq!(pass.held, 1, "the pin whose head is still missing holds");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "active");
        assert_eq!(row.consecutive_retirements, 0);
        assert_eq!(row.last_outcome.as_deref(), Some(PIN_OUTCOME_FETCHED));
        assert_eq!(row.round, before.round + 1);
        assert_eq!(reread(&mut conn, other.id).status, "retired");
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

        // One minute into a 48 h hold a peer advertises the heads.
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
            row.round,
            before.round + 1,
            "a new round: fresh retry budget"
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

    /// Demand caps the hold at the first rung. A pin on the 48 h rung that is
    /// read while held is re-admitted once 6 h have passed since it retired —
    /// not before, and not 48 h later — and its ladder is not reset.
    #[test]
    fn a_read_miss_caps_a_retired_pins_hold_at_six_hours_without_resetting_the_ladder() {
        let mut conn = test_conn();
        let read = pin_for(&mut conn, "e2e-read-again");
        let unread = pin_for(&mut conn, "e2e-nobody-reads");
        let mut now = t0();
        for _ in 0..3 {
            retire(&mut conn, read.id, now);
            retire(&mut conn, unread.id, now);
            now += Duration::days(2);
            hold_pass(&mut conn, now);
        }
        // Rung 4: a 48 h hold.
        assert_eq!(retire(&mut conn, read.id, now), 4);
        assert_eq!(retire(&mut conn, unread.id, now), 4);
        let retired = reread(&mut conn, read.id);

        // Read two hours in: inside the first rung. Counted, held.
        let demand = record_demand_pin(
            &mut conn,
            demand_for("e2e-read-again"),
            now + Duration::hours(2),
        )
        .expect("demand");
        assert!(matches!(demand, DemandPin::HeldRetired(_)), "{demand:?}");
        let row = reread(&mut conn, read.id);
        assert_eq!((row.status.as_str(), row.held_demand_count), ("retired", 1));
        assert_eq!(
            row.retired_at, retired.retired_at,
            "demand does not move the clock"
        );
        assert_eq!(row.updated_at, retired.updated_at);
        assert_eq!(row.round, retired.round);

        // 5 h 59 m after retirement: the first rung has not ended.
        let early = hold_pass(&mut conn, now + Duration::minutes(359));
        assert!(early.readmitted.is_empty());
        assert_eq!(early.held, 2);
        assert_eq!(reread(&mut conn, read.id).status, "retired");

        // 6 h 1 m: the demanded pin is re-admitted — by demand, not by its own
        // 48 h hold — and the pin nobody read stays held.
        let capped = hold_pass(&mut conn, now + Duration::minutes(361));
        assert_eq!(capped.readmitted.len(), 1);
        assert_eq!(capped.readmitted[0].0.id, read.id);
        assert_eq!(capped.readmitted[0].1, ReadmitCause::Demand);
        assert_eq!(capped.held, 1);
        let row = reread(&mut conn, read.id);
        assert_eq!(row.status, "active");
        assert_eq!(
            row.consecutive_retirements, 4,
            "demand does not reset the ladder"
        );
        assert_eq!(row.held_demand_count, 0);
        assert_eq!(row.round, retired.round + 1);
        assert_eq!(reread(&mut conn, unread.id).status, "retired");

        // It exhausts again: one rung UP, and the 6 h cap restarts from there.
        let again = now + Duration::hours(7);
        assert_eq!(retire(&mut conn, read.id, again), 5);
        let demand = record_demand_pin(
            &mut conn,
            demand_for("e2e-read-again"),
            again + Duration::hours(5),
        )
        .expect("demand");
        assert!(matches!(demand, DemandPin::HeldRetired(_)), "{demand:?}");
        assert!(hold_pass(&mut conn, again + Duration::hours(5))
            .readmitted
            .is_empty());

        // The pin nobody read serves its whole 48 h.
        assert!(hold_pass(&mut conn, now + Duration::hours(47))
            .readmitted
            .iter()
            .all(|(p, _)| p.id != unread.id));
        let own = hold_pass(&mut conn, now + Duration::hours(49));
        assert!(own
            .readmitted
            .iter()
            .any(|(p, cause)| p.id == unread.id && *cause == ReadmitCause::Hold));
    }

    /// The other side of the boundary: the first read arrives after the first
    /// rung has already ended. The next hold pass re-admits the pin.
    #[test]
    fn a_read_miss_after_the_first_rung_is_readmitted_by_the_next_pass() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "read-late");
        retire(&mut conn, pin.id, t0());
        hold_pass(&mut conn, t0() + Duration::hours(7));
        assert_eq!(retire(&mut conn, pin.id, t0() + Duration::hours(8)), 2);
        let retired_at = t0() + Duration::hours(8);

        // Nobody has read it: seven hours into a 12 h hold it is held.
        assert!(hold_pass(&mut conn, retired_at + Duration::hours(7))
            .readmitted
            .is_empty());

        let demand = record_demand_pin(
            &mut conn,
            demand_for("read-late"),
            retired_at + Duration::hours(7),
        )
        .expect("demand");
        assert!(matches!(demand, DemandPin::DueOnDemand(_)), "{demand:?}");
        // The read itself flips nothing — the pass is the one door.
        assert_eq!(reread(&mut conn, pin.id).status, "retired");

        let pass = hold_pass(
            &mut conn,
            retired_at + Duration::hours(7) + Duration::minutes(1),
        );
        assert_eq!(pass.readmitted.len(), 1);
        assert_eq!(pass.readmitted[0].1, ReadmitCause::Demand);
        let row = reread(&mut conn, pin.id);
        assert_eq!(
            (row.status.as_str(), row.consecutive_retirements),
            ("active", 2)
        );
    }

    #[test]
    fn a_read_miss_does_not_resurrect_a_removed_pin_but_a_repin_does() {
        let mut conn = test_conn();
        let pin = pin_for(&mut conn, "unpinned-by-its-person");
        set_pin_status(&mut conn, pin.id, "removed").expect("remove");
        let removed = reread(&mut conn, pin.id);

        let demand = record_demand_pin(&mut conn, demand_for("unpinned-by-its-person"), t0())
            .expect("demand");
        assert!(matches!(demand, DemandPin::NotRevived(_)), "{demand:?}");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.status, "removed");
        assert_eq!(row.held_demand_count, 1);

        // The substrate's own passes leave a removal alone too, however long
        // it has been and whatever demand it has counted.
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
        assert_eq!((row.status.as_str(), row.held_demand_count), ("active", 0));
        assert_eq!(row.round, removed.round + 1);
    }

    #[test]
    fn a_read_miss_authors_a_new_pin_and_leaves_an_active_one_as_it_is() {
        let mut conn = test_conn();
        let first = record_demand_pin(&mut conn, demand_for("fresh-head"), t0()).expect("demand");
        let DemandPin::Authored(pin) = first else {
            panic!("a never-seen head must author a pin: {first:?}")
        };
        assert_eq!(pin.status, "active");
        assert_eq!(pin.created_at, "2026-10-13T12:00:00Z");
        assert_eq!((pin.consecutive_retirements, pin.held_demand_count), (0, 0));

        // An explicit pin with its own priority and closure rule…
        let explicit = upsert_pin(
            &mut conn,
            NewAcquisitionPin {
                closure_rule_json: Some(r#"{"depth":2}"#.into()),
                priority: 7,
                ..demand_for("fresh-head")
            },
        )
        .expect("explicit re-pin");
        // …is not touched at all by a later read-miss carrying the demand
        // defaults: not its priority, not its rule, not its timestamps.
        let again = record_demand_pin(
            &mut conn,
            demand_for("fresh-head"),
            t0() + Duration::days(1),
        )
        .expect("demand");
        assert!(matches!(again, DemandPin::AlreadyActive(_)), "{again:?}");
        let row = reread(&mut conn, pin.id);
        assert_eq!(row.priority, 7);
        assert_eq!(row.closure_rule_json.as_deref(), Some(r#"{"depth":2}"#));
        assert_eq!(row.held_demand_count, 0);
        assert_eq!(row.updated_at, explicit.updated_at);
        assert_eq!(row.round, explicit.round);
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
        assert_eq!(row.round, retired.round + 1);
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

    #[test]
    fn readmission_is_bounded_per_tick() {
        let mut conn = test_conn();
        // Sixty pins retired in the same minute — they all come due together.
        let ids: Vec<i32> = (0..60)
            .map(|i| {
                let pin = pin_for(&mut conn, &format!("e2e-{i:03}"));
                retire(&mut conn, pin.id, t0() + Duration::seconds(i));
                pin.id
            })
            .collect();

        let tick = t0() + Duration::hours(7);
        let first = hold_pass(&mut conn, tick);
        assert_eq!(
            (first.readmitted.len(), first.deferred, first.held),
            (25, 35, 0)
        );
        assert_eq!(
            first
                .readmitted
                .iter()
                .map(|(p, _)| p.id)
                .collect::<Vec<_>>(),
            ids[..25].to_vec(),
            "longest overdue first"
        );
        assert_eq!(count_active_pins(&mut conn).expect("count"), 25);
        assert_eq!(
            count_pins_by_status(&mut conn, "retired").expect("count"),
            35
        );

        let second = hold_pass(&mut conn, tick + Duration::minutes(1));
        assert_eq!((second.readmitted.len(), second.deferred), (25, 10));
        let third = hold_pass(&mut conn, tick + Duration::minutes(2));
        assert_eq!((third.readmitted.len(), third.deferred), (10, 0));
        let fourth = hold_pass(&mut conn, tick + Duration::minutes(3));
        assert_eq!(
            (fourth.readmitted.len(), fourth.deferred, fourth.held),
            (0, 0, 0)
        );
        assert_eq!(count_active_pins(&mut conn).expect("count"), 60);
    }

    /// Demanded pins are preferred, but ten slots a tick go to the longest
    /// overdue whatever their demand — so demand cannot postpone an undemanded
    /// due pin for as long as it lasts.
    #[test]
    fn the_oldest_due_pins_keep_a_reserve_under_sustained_demand() {
        let mut conn = test_conn();
        // Twelve pins nobody reads, retired first (so: longest overdue)…
        let quiet: Vec<i32> = (0..12)
            .map(|i| {
                let pin = pin_for(&mut conn, &format!("quiet-{i:02}"));
                retire(&mut conn, pin.id, t0() + Duration::seconds(i));
                pin.id
            })
            .collect();
        // …and forty that are read, retired an hour later.
        let read: Vec<i32> = (0..40)
            .map(|i| {
                let head = format!("read-{i:02}");
                let pin = pin_for(&mut conn, &head);
                let at = t0() + Duration::hours(1) + Duration::seconds(i);
                retire(&mut conn, pin.id, at);
                record_demand_pin(&mut conn, demand_for(&head), at + Duration::minutes(5))
                    .expect("demand");
                pin.id
            })
            .collect();

        let tick = t0() + Duration::hours(8);
        let first = hold_pass(&mut conn, tick);
        let first_ids: Vec<i32> = first.readmitted.iter().map(|(p, _)| p.id).collect();
        assert_eq!(first_ids.len(), 25);
        assert_eq!(
            first_ids[..10],
            quiet[..10],
            "the reserve: ten longest overdue"
        );
        assert_eq!(
            first_ids[10..],
            read[..15],
            "then demand, longest overdue first"
        );
        assert_eq!(first.deferred, 27);

        // The last two quiet pins are now the longest overdue: next tick's reserve.
        let second = hold_pass(&mut conn, tick + Duration::minutes(1));
        let second_ids: Vec<i32> = second.readmitted.iter().map(|(p, _)| p.id).collect();
        assert_eq!(second_ids[..2], quiet[10..]);
        assert_eq!(second_ids[2..], read[15..38]);
        assert_eq!(second.deferred, 2);
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

        // Demand cannot time it either: held, not due.
        let demand =
            record_demand_pin(&mut conn, demand_for("sql-default-format"), t0()).expect("demand");
        assert!(matches!(demand, DemandPin::HeldRetired(_)), "{demand:?}");

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
        for head in ["old-retired", "old-active", "old-removed"] {
            assert_eq!(by_head(head).held_demand_count, 0, "{head}");
            assert_eq!(by_head(head).round, 0, "{head}");
        }
        for head in ["old-active", "old-removed"] {
            assert_eq!(by_head(head).consecutive_retirements, 0, "{head}");
            assert!(by_head(head).last_outcome.is_none(), "{head}");
            assert!(by_head(head).retired_at.is_none(), "{head}");
        }

        // Retired 5.5 h before t0: still inside the 6 h hold it was already
        // serving, due a little later, exactly as on the old build.
        assert!(hold_pass(&mut conn, t0()).readmitted.is_empty());
        let due = hold_pass(&mut conn, t0() + Duration::hours(1));
        assert_eq!(due.readmitted.len(), 1);
        // Its next failure steps to 12 h rather than repeating 6 h.
        assert_eq!(
            retire(&mut conn, old_retired.id, t0() + Duration::hours(2)),
            2
        );
        let row = reread(&mut conn, old_retired.id);
        assert_eq!(
            pin_readmit_at(
                row.retired_at.as_deref(),
                row.consecutive_retirements,
                false
            ),
            Some(t0() + Duration::hours(14))
        );
    }
}
