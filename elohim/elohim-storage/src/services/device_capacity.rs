//! ## Source of Truth
//!
//! Helper API (Category C operational). Aggregates:
//!   total_bytes:     from services/system_metrics.rs (placeholder returns 0)
//!   committed_bytes: SUM(rea_commitments.resource_quantity_value)
//!                    WHERE action='custody-blob' AND provider IN (human's peers)
//! Returns total - committed (saturating to 0).

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{Float, Nullable};

use crate::db::DbPool;

// ---------------------------------------------------------------------------
// Test-only override store (always compiled — integration tests need access)
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Mutex;

static TEST_TOTALS: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// Override the total bytes for a given human_id in tests.
/// Must be called before `available_bytes_for`.
/// NOTE: Always compiled so integration tests (separate crates) can call it.
pub fn override_total_for_test(human_id: &str, total: u64) {
    let mut guard = TEST_TOTALS.lock().expect("TEST_TOTALS lock");
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(human_id.to_string(), total);
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute available storage bytes for a human:
///   available = device_capacity_total(human_id) - committed_bytes_for(human_id)
/// Saturates at 0 (never negative).
pub async fn available_bytes_for(pool: &DbPool, human_id_arg: &str) -> u64 {
    let total = device_capacity_total(human_id_arg);
    let committed = committed_bytes_for(pool, human_id_arg).await.unwrap_or(0);
    total.saturating_sub(committed)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn device_capacity_total(human_id_arg: &str) -> u64 {
    // Check test override first (noop in production since map stays None).
    {
        let guard = TEST_TOTALS.lock().expect("TEST_TOTALS lock");
        if let Some(map) = guard.as_ref() {
            if let Some(&v) = map.get(human_id_arg) {
                return v;
            }
        }
    }

    // TODO(topology M1 Task 2): when services/system_metrics.rs lands its
    // device_capacity_total_bytes(human_id) function, call it here.
    tracing::debug!(
        target = "phase4::device_capacity",
        human = %human_id_arg,
        "device_capacity_total: returning 0 — depends on topology M1 system_metrics module"
    );
    0
}

async fn committed_bytes_for(
    pool: &DbPool,
    human_id_arg: &str,
) -> diesel::result::QueryResult<u64> {
    use crate::db::diesel_schema::peer_identity_bindings::dsl as bind;
    use crate::db::diesel_schema::rea_commitments::dsl as rc;

    let mut conn = pool.get().map_err(|_| diesel::result::Error::NotFound)?;

    // Look up the human's active peer set.
    let my_peers: Vec<String> = bind::peer_identity_bindings
        .filter(bind::agent_cid.eq(human_id_arg))
        .filter(bind::superseded_by.is_null())
        .select(bind::peer_id)
        .load::<String>(&mut conn)?;

    if my_peers.is_empty() {
        return Ok(0);
    }

    let total: Option<f32> = rc::rea_commitments
        .filter(rc::action.eq("custody-blob"))
        .filter(rc::provider.eq_any(&my_peers))
        .select(sql::<Nullable<Float>>("SUM(resource_quantity_value)"))
        .first::<Option<f32>>(&mut conn)?;

    Ok(total.unwrap_or(0.0).max(0.0) as u64)
}
