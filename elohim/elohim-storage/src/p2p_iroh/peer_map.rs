//! Phase 10 — cross-stack peer identity mapping.
//!
//! Bridges libp2p `PeerId` ↔ iroh `NodeId` for the same logical agent
//! during the hybrid window. Backed by the
//! `cross_stack_peer_map` table (operational projection, Category C).
//!
//! Identity is keyed by `agent_cid` — the canonical imagodei identity
//! handle that does not change across transports. Either `peer_id` or
//! `node_id` may be `NULL` if only one side has observed the peer; both
//! populated means we've seen the same `agent_cid` on both stacks.
//!
//! Writes are upserts on `agent_cid`. Conflicting `peer_id` or `node_id`
//! (the same transport identity claimed by two `agent_cid`s) is a
//! protocol violation and surfaces as a unique-index constraint
//! violation — handlers should log and reject the second binding.

use diesel::prelude::*;
use diesel::sql_types::Text;

use crate::db::diesel_schema::cross_stack_peer_map;
use crate::error::StorageError;

/// Row shape — both transport identities optional, agent_cid required.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = cross_stack_peer_map)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct CrossStackPeerMapRow {
    pub agent_cid: String,
    pub peer_id: Option<String>,
    pub node_id: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

/// Record a libp2p `PeerId` observation for an agent. Upserts on
/// `agent_cid`. If a row already exists, advances `last_seen_at` and
/// (if currently NULL) sets `peer_id`.
pub fn record_libp2p(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    peer_id: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    use cross_stack_peer_map as t;
    conn.transaction(|conn| {
        let existing: Option<CrossStackPeerMapRow> = t::table
            .filter(t::agent_cid.eq(agent_cid))
            .first(conn)
            .optional()?;
        match existing {
            Some(_) => {
                diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
                    .set((t::peer_id.eq(peer_id), t::last_seen_at.eq(observed_at)))
                    .execute(conn)?;
            }
            None => {
                diesel::insert_into(t::table)
                    .values((
                        t::agent_cid.eq(agent_cid),
                        t::peer_id.eq(Some(peer_id)),
                        t::node_id.eq::<Option<&str>>(None),
                        t::first_seen_at.eq(observed_at),
                        t::last_seen_at.eq(observed_at),
                    ))
                    .execute(conn)?;
            }
        }
        Ok::<_, diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("record_libp2p: {e}")))
}

/// Record an iroh `NodeId` observation for an agent. Upserts on
/// `agent_cid`. Mirror of [`record_libp2p`] for the iroh side.
pub fn record_iroh(
    conn: &mut SqliteConnection,
    agent_cid: &str,
    node_id: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    use cross_stack_peer_map as t;
    conn.transaction(|conn| {
        let existing: Option<CrossStackPeerMapRow> = t::table
            .filter(t::agent_cid.eq(agent_cid))
            .first(conn)
            .optional()?;
        match existing {
            Some(_) => {
                diesel::update(t::table.filter(t::agent_cid.eq(agent_cid)))
                    .set((t::node_id.eq(node_id), t::last_seen_at.eq(observed_at)))
                    .execute(conn)?;
            }
            None => {
                diesel::insert_into(t::table)
                    .values((
                        t::agent_cid.eq(agent_cid),
                        t::peer_id.eq::<Option<&str>>(None),
                        t::node_id.eq(Some(node_id)),
                        t::first_seen_at.eq(observed_at),
                        t::last_seen_at.eq(observed_at),
                    ))
                    .execute(conn)?;
            }
        }
        Ok::<_, diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("record_iroh: {e}")))
}

/// Resolve the iroh `NodeId` (if known) for a libp2p `PeerId`.
pub fn iroh_for_libp2p(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Option<String>, StorageError> {
    use cross_stack_peer_map as t;
    let nid: Option<Option<String>> = t::table
        .filter(t::peer_id.eq(peer_id))
        .select(t::node_id)
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("iroh_for_libp2p: {e}")))?;
    Ok(nid.flatten())
}

/// Resolve the libp2p `PeerId` (if known) for an iroh `NodeId`.
pub fn libp2p_for_iroh(
    conn: &mut SqliteConnection,
    node_id: &str,
) -> Result<Option<String>, StorageError> {
    use cross_stack_peer_map as t;
    let pid: Option<Option<String>> = t::table
        .filter(t::node_id.eq(node_id))
        .select(t::peer_id)
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("libp2p_for_iroh: {e}")))?;
    Ok(pid.flatten())
}

// Workaround so unused imports don't warn when Diesel macros are minimally exercised.
#[allow(dead_code)]
fn _phantom_ty(_t: Text) {}
