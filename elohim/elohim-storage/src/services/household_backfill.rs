//! One-shot startup pass: for humans rows with null household_id, fill from an
//! external mapping sourced from DHT humans entries. Legacy rows whose DHT
//! entry carries no householdId remain null.

use diesel::prelude::*;

use crate::db::diesel_schema::humans;
use crate::db::DbPool;
use crate::StorageError;

/// `mapping` is a vec of (human_id, household_id) pairs, typically produced by
/// reading the current humans DHT entries at boot.
///
/// Only rows with a NULL household_id are updated — rows that already have a
/// household_id set are left untouched. The function is idempotent: running it
/// twice with the same mapping produces the same result.
///
/// Returns the number of rows actually updated.
pub fn run_once(
    pool: &DbPool,
    mapping: Vec<(String, String)>,
) -> Result<usize, StorageError> {
    if mapping.is_empty() {
        tracing::debug!("household_backfill: empty mapping, nothing to do");
        return Ok(0);
    }

    let mut conn = pool.get().map_err(|e| StorageError::Internal(e.to_string()))?;
    let mut filled = 0usize;

    for (human_id, household_id) in mapping {
        let n = diesel::update(
            humans::table
                .filter(humans::id.eq(&human_id))
                .filter(humans::household_id.is_null()),
        )
        .set(humans::household_id.eq(&household_id))
        .execute(&mut conn)?;

        filled += n;
    }

    tracing::info!(filled, "household_backfill complete");
    Ok(filled)
}
