//! Handlers for PortalHost DHT signals — upsert and delete the `portal_hosts` projection.
//!
//! ## Design classification
//!
//! `portal_hosts` is Category A (notarized). The DHT entry is the source of truth.
//! This projection is rebuilt from signal replay; it is never the authority.
//!
//! ## Signal mapping
//!
//! | Signal field              | DB column              | Notes                          |
//! |---------------------------|------------------------|--------------------------------|
//! | `action_hash`             | `dht_anchor_hash`      | Create ActionHash — provenance |
//! | `human_action_hash`       | `human_id`             | ActionHash of Human DHT entry  |
//! | `host_url`                | `host_url`             | direct                         |
//! | `label`                   | `label`                | nullable                       |
//! | `added_at`                | `added_at`             | ISO 8601 from DHT timestamp    |
//! | `reach`                   | `reach`                | debug-formatted enum tag       |
//!
//! For removal, `original_action_hash` identifies the projection row (it is the
//! `dht_anchor_hash` from the preceding `PortalHostCreated` signal).

use crate::db::models::NewPortalHostRow;
use crate::db::{portal_hosts, DbPool};
use crate::reconcile::signal_stream::{PortalHostCreatedSignal, PortalHostRemovedSignal};
use crate::reconcile::ReconcileError;
use std::sync::Arc;
use tracing::{debug, warn};

/// Handle a `DnaSignal::PortalHostCreated` — upsert the projection row.
///
/// Uses INSERT OR REPLACE semantics so that signal replay is idempotent.
/// If the controller was constructed without a DB pool this method logs a
/// warning and returns `Ok(())` — preserving backward compatibility with
/// test controllers built via `ReconcileController::new()`.
pub fn on_portal_host_created(
    db_pool: &Option<Arc<DbPool>>,
    sig: PortalHostCreatedSignal,
) -> Result<(), ReconcileError> {
    let Some(pool) = db_pool else {
        warn!(
            host_url = %sig.host_url,
            "PortalHostCreated received but no db_pool wired — upsert skipped (use new_with_storage)"
        );
        return Ok(());
    };

    let mut conn = pool
        .get()
        .map_err(|e| ReconcileError::Pool(format!("{e}")))?;

    let row = NewPortalHostRow {
        // human_action_hash is the ActionHash of the Human DHT entry; the
        // portal_hosts table uses it as the foreign-key into `humans`.
        human_id: sig.human_action_hash,
        host_url: sig.host_url,
        label: sig.label,
        added_at: sig.added_at,
        last_reachable_at: None,
        reach: sig.reach,
        // action_hash is the Create ActionHash — stored as dht_anchor_hash for
        // DHT provenance and as the key used by PortalHostRemoved.
        dht_anchor_hash: sig.action_hash,
    };

    portal_hosts::upsert(&mut conn, &row).map_err(|e| ReconcileError::Sweep(e.to_string()))?;

    debug!(
        human_id = %row.human_id,
        host_url = %row.host_url,
        dht_anchor_hash = %row.dht_anchor_hash,
        "portal_hosts projection upserted"
    );

    Ok(())
}

/// Handle a `DnaSignal::PortalHostRemoved` — delete the projection row.
///
/// Uses `original_action_hash` (the Create ActionHash from the preceding
/// `PortalHostCreated` signal) as the lookup key; this matches the
/// `dht_anchor_hash` column that was written during upsert.
///
/// Returns `Ok(())` whether or not a row was found; a missing row means the
/// signal arrived before the creation signal was projected (race), or the row
/// was already cleaned up. Either way the desired post-state (no row) is reached.
pub fn on_portal_host_removed(
    db_pool: &Option<Arc<DbPool>>,
    sig: PortalHostRemovedSignal,
) -> Result<(), ReconcileError> {
    let Some(pool) = db_pool else {
        warn!(
            host_url = %sig.host_url,
            "PortalHostRemoved received but no db_pool wired — delete skipped (use new_with_storage)"
        );
        return Ok(());
    };

    let mut conn = pool
        .get()
        .map_err(|e| ReconcileError::Pool(format!("{e}")))?;

    let deleted = portal_hosts::delete_by_anchor_hash(&mut conn, &sig.original_action_hash)
        .map_err(|e| ReconcileError::Sweep(e.to_string()))?;

    if deleted == 0 {
        debug!(
            original_action_hash = %sig.original_action_hash,
            host_url = %sig.host_url,
            "PortalHostRemoved: no projection row found for anchor (idempotent)"
        );
    } else {
        debug!(
            original_action_hash = %sig.original_action_hash,
            host_url = %sig.host_url,
            deleted,
            "portal_hosts projection row deleted"
        );
    }

    Ok(())
}
