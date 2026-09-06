//! `standing_generations` + `standing_generation_aggregate`.
//!
//! Accountable-correction contract §7. A GENERATION = one (evaluator, PINNED
//! policy bytes by CID) projection. Per-act application is tracked per
//! generation, so a second evaluator is never suppressed by the first and a
//! policy change produces a NEW generation rather than silently re-weighting
//! history.
//!
//! Policy bytes are pinned in the row, not read from the live registry at
//! replay time: `ManifestDebitWeightPolicy::from_registry` reads whatever the
//! registry holds *now*, which makes a replay non-deterministic the moment the
//! manifest moves.

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;

use crate::db::diesel_schema::{standing_generation_aggregate, standing_generations};

/// Being replayed; not yet served.
pub const GEN_BUILDING: &str = "building";
/// The generation `standing_view` currently reflects.
pub const GEN_PUBLISHED: &str = "published";
/// A published generation being replayed again. READERS SEE THIS — "rebuilding"
/// is a visible state, not a silent gap (§7).
pub const GEN_REBUILDING: &str = "rebuilding";

#[derive(Debug, Clone, PartialEq, Queryable, Selectable)]
#[diesel(table_name = standing_generations)]
pub struct GenerationRow {
    pub generation_id: i32,
    pub evaluator_pubkey: Vec<u8>,
    pub policy_manifest_cid: String,
    pub policy_bytes: String,
    pub status: String,
    pub created_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = standing_generation_aggregate)]
pub struct GenerationAggregateRow {
    pub generation_id: i32,
    pub evaluator_pubkey: Vec<u8>,
    pub subject_pubkey: Vec<u8>,
    pub debit_weight_sum: i32,
    pub last_signal_at_micros: Option<i64>,
}

/// Open a new generation in `building`. The id is assigned by SQLite.
pub fn create_generation(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
    policy_manifest_cid: &str,
    policy_bytes: &str,
    status: &str,
    now: &str,
) -> Result<i32, DieselError> {
    use crate::db::diesel_schema::standing_generations::dsl as t;
    diesel::insert_into(t::standing_generations)
        .values((
            t::evaluator_pubkey.eq(evaluator),
            t::policy_manifest_cid.eq(policy_manifest_cid),
            t::policy_bytes.eq(policy_bytes),
            t::status.eq(status),
            t::created_at.eq(now),
        ))
        .execute(conn)?;
    t::standing_generations
        .select(t::generation_id)
        .order_by(t::generation_id.desc())
        .first::<i32>(conn)
}

pub fn fetch_generation(
    conn: &mut SqliteConnection,
    generation_id: i32,
) -> Result<Option<GenerationRow>, DieselError> {
    use crate::db::diesel_schema::standing_generations::dsl as t;
    t::standing_generations
        .filter(t::generation_id.eq(generation_id))
        .first::<GenerationRow>(conn)
        .optional()
}

/// The generation an evaluator currently SERVES, if any.
pub fn published_generation(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
) -> Result<Option<GenerationRow>, DieselError> {
    use crate::db::diesel_schema::standing_generations::dsl as t;
    t::standing_generations
        .filter(t::evaluator_pubkey.eq(evaluator))
        .filter(t::status.eq(GEN_PUBLISHED))
        .order_by(t::generation_id.desc())
        .first::<GenerationRow>(conn)
        .optional()
}

/// The generation an evaluator is currently BUILDING or REBUILDING, if any.
pub fn in_flight_generation(
    conn: &mut SqliteConnection,
    evaluator: &[u8],
) -> Result<Option<GenerationRow>, DieselError> {
    use crate::db::diesel_schema::standing_generations::dsl as t;
    t::standing_generations
        .filter(t::evaluator_pubkey.eq(evaluator))
        .filter(t::status.eq(GEN_BUILDING).or(t::status.eq(GEN_REBUILDING)))
        .order_by(t::generation_id.desc())
        .first::<GenerationRow>(conn)
        .optional()
}

pub fn set_status(
    conn: &mut SqliteConnection,
    generation_id: i32,
    status: &str,
    published_at: Option<&str>,
) -> Result<(), DieselError> {
    use crate::db::diesel_schema::standing_generations::dsl as t;
    diesel::update(t::standing_generations.filter(t::generation_id.eq(generation_id)))
        .set((
            t::status.eq(status),
            t::published_at.eq(published_at.map(|s| s.to_string())),
        ))
        .execute(conn)?;
    Ok(())
}

pub fn fetch_aggregate(
    conn: &mut SqliteConnection,
    generation_id: i32,
    evaluator: &[u8],
    subject: &[u8],
) -> Result<Option<GenerationAggregateRow>, DieselError> {
    use crate::db::diesel_schema::standing_generation_aggregate::dsl as t;
    t::standing_generation_aggregate
        .filter(t::generation_id.eq(generation_id))
        .filter(t::evaluator_pubkey.eq(evaluator))
        .filter(t::subject_pubkey.eq(subject))
        .first::<GenerationAggregateRow>(conn)
        .optional()
}

pub fn upsert_aggregate(
    conn: &mut SqliteConnection,
    row: &GenerationAggregateRow,
) -> Result<(), DieselError> {
    use crate::db::diesel_schema::standing_generation_aggregate::dsl as t;
    diesel::insert_into(t::standing_generation_aggregate)
        .values(row)
        .on_conflict((t::generation_id, t::evaluator_pubkey, t::subject_pubkey))
        .do_update()
        .set(row)
        .execute(conn)?;
    Ok(())
}

pub fn list_aggregates(
    conn: &mut SqliteConnection,
    generation_id: i32,
) -> Result<Vec<GenerationAggregateRow>, DieselError> {
    use crate::db::diesel_schema::standing_generation_aggregate::dsl as t;
    t::standing_generation_aggregate
        .filter(t::generation_id.eq(generation_id))
        .order_by((t::evaluator_pubkey.asc(), t::subject_pubkey.asc()))
        .load::<GenerationAggregateRow>(conn)
}

pub fn clear_aggregates(
    conn: &mut SqliteConnection,
    generation_id: i32,
) -> Result<usize, DieselError> {
    use crate::db::diesel_schema::standing_generation_aggregate::dsl as t;
    diesel::delete(t::standing_generation_aggregate.filter(t::generation_id.eq(generation_id)))
        .execute(conn)
}
