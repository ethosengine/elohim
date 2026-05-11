//! ## Source of Truth
//!
//! Helper API for view services. Reads `humans` table (operational
//! projection of Human DHT entry, Category A in imagodei zome). DHT
//! remains authoritative — this is a read-only convenience over the
//! existing projection.

use diesel::prelude::*;

use crate::db::DbPool;

/// Resolve the display name for an agent CID by looking up the `humans` table.
///
/// Returns `Some(display_name)` if a human row exists, `None` otherwise.
/// Non-fatal: any DB error returns `None` with a debug log.
pub async fn resolve_display_name(pool: &DbPool, agent_cid_arg: &str) -> Option<String> {
    use crate::db::diesel_schema::humans::dsl as h;

    let mut conn = pool.get().ok()?;
    h::humans
        .filter(h::id.eq(agent_cid_arg))
        .select(h::display_name)
        .first::<String>(&mut conn)
        .optional()
        .ok()
        .flatten()
}
