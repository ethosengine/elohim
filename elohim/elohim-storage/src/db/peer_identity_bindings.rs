//! CRUD for the `peer_identity_bindings` table.
//!
//! Source of truth: Holochain DHT (imagodei AgentPeerBinding entry — Task A.2).
//! This table is a Category C operational projection rebuildable from signal replay.
//!
//! ## Primary operations
//!
//! - `upsert` — called by the ReconcileController when an AgentPeerBinding signal
//!   arrives (Task A.4 controller, Task A.8 signal handler).
//! - `lookup_active` — called by `HolochainBackedPeerIdentityMap::lookup` on the
//!   P2P hot path; returns the most recently observed active binding for a peer.

use crate::db::diesel_schema::peer_identity_bindings;
use crate::db::models::{NewPeerIdentityBindingRow, PeerIdentityBindingRow};
use crate::error::StorageError;
use diesel::prelude::*;

/// Upsert a peer identity binding row.
///
/// Uses INSERT OR REPLACE semantics keyed on `(peer_id, dht_anchor_hash)`.
/// If the same DHT entry is observed again (e.g. from a second source), the
/// row is replaced with the latest `observed_at` and `source`.
pub fn upsert(
    conn: &mut SqliteConnection,
    row: &NewPeerIdentityBindingRow,
) -> Result<(), StorageError> {
    diesel::replace_into(peer_identity_bindings::table)
        .values(row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("peer_identity_bindings upsert: {e}")))
}

/// Look up the most recently observed, currently-active binding for `peer_id`.
///
/// "Active" means: `valid_until IS NULL OR valid_until > now_iso`, AND
/// `superseded_by IS NULL`. The query returns at most one row — the one with
/// the latest `observed_at`. The partial index
/// `idx_peer_identity_bindings_current_per_agent` (keyed on `agent_cid`) covers
/// supersession-aware filtering for agent-side queries; the per-peer lookup
/// here uses the existing `idx_peer_identity_bindings_peer_id` index.
///
/// Returns `None` if no active binding exists for the peer.
pub fn lookup_active(
    conn: &mut SqliteConnection,
    peer_id: &str,
    now_iso: &str,
) -> Result<Option<PeerIdentityBindingRow>, StorageError> {
    use peer_identity_bindings::dsl;

    dsl::peer_identity_bindings
        .filter(dsl::peer_id.eq(peer_id))
        .filter(dsl::valid_until.is_null().or(dsl::valid_until.gt(now_iso)))
        .filter(dsl::superseded_by.is_null())
        .order(dsl::observed_at.desc())
        .first::<PeerIdentityBindingRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("peer_identity_bindings lookup: {e}")))
}

/// Delete all bindings for a peer (used in tests and reconciliation resets).
#[cfg(test)]
pub fn delete_all_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<usize, StorageError> {
    use peer_identity_bindings::dsl;

    diesel::delete(dsl::peer_identity_bindings.filter(dsl::peer_id.eq(peer_id)))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("peer_identity_bindings delete: {e}")))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewPeerIdentityBindingRow;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:pib_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn binding(
        peer_id: &str,
        agent_cid: &str,
        dht_anchor_hash: &str,
        valid_from: &str,
        valid_until: Option<&str>,
        observed_at: &str,
        source: &str,
    ) -> NewPeerIdentityBindingRow {
        NewPeerIdentityBindingRow {
            peer_id: peer_id.to_string(),
            agent_cid: agent_cid.to_string(),
            dht_anchor_hash: dht_anchor_hash.to_string(),
            valid_from: valid_from.to_string(),
            valid_until: valid_until.map(|s| s.to_string()),
            observed_at: observed_at.to_string(),
            source: source.to_string(),
            device_archetype: "node".to_string(),
            superseded_by: None,
        }
    }

    #[test]
    fn upsert_and_lookup_active_binding() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let row = binding(
            "12D3KooWXyz",
            "bafyreiabc",
            "uhCAkABCDEF",
            "2026-01-01T00:00:00Z",
            None,
            "2026-04-24T00:00:00Z",
            "dht",
        );
        upsert(&mut conn, &row).expect("upsert");

        let result =
            lookup_active(&mut conn, "12D3KooWXyz", "2026-04-24T12:00:00Z").expect("lookup");
        assert!(result.is_some(), "expected active binding");
        let found = result.unwrap();
        assert_eq!(found.agent_cid, "bafyreiabc");
        assert_eq!(found.source, "dht");
    }

    #[test]
    fn lookup_returns_none_for_unknown_peer() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let result =
            lookup_active(&mut conn, "12D3KooWUnknown", "2026-04-24T12:00:00Z").expect("lookup");
        assert!(result.is_none(), "expected None for unknown peer");
    }

    #[test]
    fn lookup_excludes_expired_binding() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Binding valid 2026-01-01 → 2026-02-01; query now = 2026-04-24 (past expiry)
        let row = binding(
            "12D3KooWXyz",
            "bafyreiabc",
            "uhCAkABCDEF",
            "2026-01-01T00:00:00Z",
            Some("2026-02-01T00:00:00Z"),
            "2026-04-24T00:00:00Z",
            "dht",
        );
        upsert(&mut conn, &row).expect("upsert");

        let result =
            lookup_active(&mut conn, "12D3KooWXyz", "2026-04-24T12:00:00Z").expect("lookup");
        assert!(result.is_none(), "expired binding must not be returned");
    }

    #[test]
    fn lookup_returns_most_recently_observed_among_multiple() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Two bindings for the same peer, different DHT anchors (e.g. key rotation)
        upsert(
            &mut conn,
            &binding(
                "12D3KooWXyz",
                "bafyreiabc",
                "anchor-1",
                "2026-01-01T00:00:00Z",
                None,
                "2026-04-20T00:00:00Z",
                "dht",
            ),
        )
        .expect("upsert 1");
        upsert(
            &mut conn,
            &binding(
                "12D3KooWXyz",
                "bafyreinew",
                "anchor-2",
                "2026-04-01T00:00:00Z",
                None,
                "2026-04-24T00:00:00Z",
                "gossip",
            ),
        )
        .expect("upsert 2");

        let result =
            lookup_active(&mut conn, "12D3KooWXyz", "2026-04-24T12:00:00Z").expect("lookup");
        assert!(result.is_some());
        // Most recently observed should win
        assert_eq!(result.unwrap().agent_cid, "bafyreinew");
    }

    #[test]
    fn lookup_excludes_superseded_binding() {
        // Two rows for the same peer:
        //   - older row, NOT superseded → must be the active binding
        //   - newer row, superseded_by = Some(...) → must NOT be returned
        // even though it has the latest observed_at.
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // Older row: un-superseded, observed earlier.
        let older = NewPeerIdentityBindingRow {
            peer_id: "12D3KooWXyz".to_string(),
            agent_cid: "bafyreiabc".to_string(),
            dht_anchor_hash: "anchor-current".to_string(),
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: None,
            observed_at: "2026-04-20T00:00:00Z".to_string(),
            source: "dht".to_string(),
            device_archetype: "node".to_string(),
            superseded_by: None,
        };
        upsert(&mut conn, &older).expect("upsert older");

        // Newer row: superseded, observed later. Must be excluded by the filter.
        let newer_superseded = NewPeerIdentityBindingRow {
            peer_id: "12D3KooWXyz".to_string(),
            agent_cid: "bafyreiabc".to_string(),
            dht_anchor_hash: "anchor-stale".to_string(),
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: None,
            observed_at: "2026-04-24T00:00:00Z".to_string(),
            source: "dht".to_string(),
            device_archetype: "node".to_string(),
            superseded_by: Some("uhCkk-supersedes-this".to_string()),
        };
        upsert(&mut conn, &newer_superseded).expect("upsert newer-superseded");

        let result =
            lookup_active(&mut conn, "12D3KooWXyz", "2026-04-24T12:00:00Z").expect("lookup");
        let row = result.expect("expected an active (un-superseded) binding");
        assert_eq!(
            row.dht_anchor_hash, "anchor-current",
            "lookup_active must skip superseded rows even when their observed_at is later"
        );
        assert!(
            row.superseded_by.is_none(),
            "active row must have superseded_by == NULL"
        );
    }
}
