//! Placement gap CRUD — structured shefa signal surface.
//!
//! Rows are a projection of "contract says N stewards should hold X;
//! reality has M < N holding it." Rebuildable from shard_locations +
//! rea_commitments + humans.household_id at startup. Operational Category C.

use diesel::prelude::*;

use super::diesel_schema::placement_gaps;
use super::models::{NewPlacementGap, PlacementGapRow};
use crate::StorageError;

#[derive(Debug, Clone, Default)]
pub struct GapQuery {
    pub kind: Option<String>,
    pub content_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Insert a new gap row or bump `last_seen_at` + current-state fields on the
/// matching `(content_id, shard_hash, h_app_id, gap_kind)` row.
pub fn upsert_gap(conn: &mut SqliteConnection, gap: &NewPlacementGap) -> Result<(), StorageError> {
    // Try plain insert first; on unique-index conflict, update current-state.
    let inserted = diesel::insert_or_ignore_into(placement_gaps::table)
        .values(gap)
        .execute(conn)?;

    if inserted == 0 {
        diesel::update(
            placement_gaps::table
                .filter(placement_gaps::content_id.eq(gap.content_id))
                .filter(placement_gaps::shard_hash.eq(gap.shard_hash))
                .filter(placement_gaps::h_app_id.eq(gap.h_app_id))
                .filter(placement_gaps::gap_kind.eq(gap.gap_kind)),
        )
        .set((
            placement_gaps::achieved_steward_count.eq(gap.achieved_steward_count),
            placement_gaps::contract_coverage.eq(gap.contract_coverage),
            placement_gaps::last_seen_at.eq(gap.last_seen_at),
        ))
        .execute(conn)?;
    }

    Ok(())
}

pub fn list_gaps(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    query: GapQuery,
) -> Result<Vec<PlacementGapRow>, StorageError> {
    let mut q = placement_gaps::table
        .filter(placement_gaps::h_app_id.eq(h_app_id))
        .into_boxed();

    if let Some(kind) = query.kind {
        q = q.filter(placement_gaps::gap_kind.eq(kind));
    }
    if let Some(content_id) = query.content_id {
        q = q.filter(placement_gaps::content_id.eq(content_id));
    }

    q = q.order(placement_gaps::last_seen_at.desc());
    if let Some(lim) = query.limit {
        q = q.limit(lim);
    }
    if let Some(off) = query.offset {
        q = q.offset(off);
    }

    q.load::<PlacementGapRow>(conn).map_err(StorageError::from)
}

/// Remove rows whose `last_seen_at` is strictly less than the cutoff.
/// Returns the number of rows deleted.
pub fn gc_stale(conn: &mut SqliteConnection, cutoff_iso: &str) -> Result<usize, StorageError> {
    let n =
        diesel::delete(placement_gaps::table.filter(placement_gaps::last_seen_at.lt(cutoff_iso)))
            .execute(conn)?;
    Ok(n)
}

pub fn clear_for_content(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    content_id: &str,
) -> Result<usize, StorageError> {
    let n = diesel::delete(
        placement_gaps::table
            .filter(placement_gaps::h_app_id.eq(h_app_id))
            .filter(placement_gaps::content_id.eq(content_id)),
    )
    .execute(conn)?;
    Ok(n)
}
