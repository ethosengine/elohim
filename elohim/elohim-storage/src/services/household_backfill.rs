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
pub fn run_once(pool: &DbPool, mapping: Vec<(String, String)>) -> Result<usize, StorageError> {
    if mapping.is_empty() {
        tracing::debug!("household_backfill: empty mapping, nothing to do");
        return Ok(0);
    }

    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;
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

/// Backfill from a DHT-sourced `(member_cid, household_cid)` mapping, matching
/// rows the way the live reconcile path does.
///
/// The runtime stamp in [`crate::reconcile`] `on_membership_projected` matches a
/// human by `humans.id == member_cid` OR `humans.agent_pub_key == member_cid`
/// with the `agent:` prefix stripped — because production rows are slug-keyed
/// (`humans.id = "human-alice-n1"`) while the membership entry carries the
/// `agent:<pubkey>` form. The id-only [`run_once`] above misses those rows, so
/// the DHT replayer (`holochain_humans_replayer`) uses THIS variant to stay
/// byte-identical with the live stamp.
///
/// `household_cid` is stored verbatim as `humans.household_id` — the SAME value
/// the controller writes (`collective_cid`, e.g. `collective:<action_hash>`) —
/// so backfill and live-stamp never diverge.
///
/// Only `humans.household_id IS NULL` rows are touched (idempotent, never
/// overwrites a create-time or live-stamped value). Returns the rows updated.
pub fn run_once_by_membership(
    pool: &DbPool,
    mapping: Vec<(String, String)>,
) -> Result<usize, StorageError> {
    if mapping.is_empty() {
        tracing::debug!("household_backfill(by_membership): empty mapping, nothing to do");
        return Ok(0);
    }

    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    let mut filled = 0usize;

    for (member_cid, household_cid) in mapping {
        // Mirror the controller: match the slug id OR the raw agent key.
        let member_agent_key = member_cid
            .strip_prefix("agent:")
            .unwrap_or(&member_cid)
            .to_string();

        let n = diesel::update(
            humans::table
                .filter(
                    humans::id
                        .eq(&member_cid)
                        .or(humans::agent_pub_key.eq(&member_agent_key)),
                )
                .filter(humans::household_id.is_null()),
        )
        .set(humans::household_id.eq(&household_cid))
        .execute(&mut conn)?;

        filled += n;
    }

    tracing::info!(filled, "household_backfill(by_membership) complete");
    Ok(filled)
}
