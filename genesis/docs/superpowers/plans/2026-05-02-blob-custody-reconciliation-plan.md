# Blob Custody Reconciliation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the trinity reconciliation pattern (manifest / reality / diff) for blob custody so the topology UI can faithfully show "replicas grow toward target after a peer connects" and "commitments unhonored beyond grace are visible."

**Architecture:** New SQLite projection (`peer_blob_inventory`) populated by libp2p gossipsub on `elohim/inventory/blob`. New custody reconciliation controller computes diff between `rea_commitments(custody-blob)` (DHT manifest) and the projection (operational reality), acting on own commitments by kicking parallel-raced fetches and signaling on others' via `placement-gap` REA events. Blob GET handler falls back to the same shared fetch helper. Existing T03d action conventions remain the single substrate vocabulary; no new DHT entries.

**Tech Stack:** Rust (elohim-storage), Diesel (SQLite), libp2p 0.54 with gossipsub + request-response, MessagePack via rmp_serde, tokio async runtime, ts-rs for boundary types, schema-first JSON Schema validation against existing harness.

---

## P2P Design Gate — Source of Truth Declarations

Per the P2P design gate skill (`p2p-design-gate`), every entity introduced by this plan is classified A/A2/B/B2/C with explicit substrate routing.

| Entity | Category | Source of truth | DHT entry type? | HTTP route | Coordinator fn / signal projector |
|---|---|---|---|---|---|
| `peer_blob_inventory` row | C (operational) | libp2p gossipsub on topic `elohim/inventory/blob` | None | `/api/v1/diagnostics/inventory-parity` (read-only diagnostic), implicitly read by topology view aggregators | Projector: `apply_snapshot` / `apply_delta` (both in `db/peer_blob_inventory.rs`); rebuilt from gossip replay |
| `peer_inventory_cursor` row | C (operational, infrastructural) | Local commit timestamps + sequence high-watermarks observed during projection write | None | None | Mirrored to disk by projection writer; lives only to survive restart |
| `BlobInventorySnapshot` / `BlobInventoryDelta` (wire) | C (operational gossip) | Emitted live by inventory broadcaster | None | None (gossipsub-internal) | Source: `inventory_broadcaster.rs`; sink: `apply_snapshot`/`apply_delta` |
| `placement-gap` action on `economic_events` | C (observation event over manifest) | Reconciliation controller emits when own-receiver custody commitment goes unhonored beyond grace | None (free-text REA action; no schema enum needed per T03d precedent) | Implicitly readable via existing distribution/reciprocity view aggregators | Coordinator: `reconcile/custody.rs` reconcile pass |
| Custody reconciliation outcomes (kicks fired, gaps emitted) | C (operational metrics) | `ReconciliationMetrics` struct in process | None | Implicitly read by `/api/v1/diagnostics/inventory-parity` and existing metrics endpoints | Reset on restart; not persisted |

**Why no new DHT entries:** the manifest already exists (`rea_commitments(action='custody-blob')` from T03d). Reality is operational (libp2p). Diffs are computed; not stored. Per `project_dht_vs_libp2p_scoping` and `project_three_layer_truth_model`.

**HTTP route added:** only one — `/api/v1/diagnostics/inventory-parity` (T18, read-only). All other behavior is internal projection + controller wiring; the existing `GET /api/v1/blob/{hash}` route gains internal fallback behavior but the route itself is unchanged.

---

## File Structure

### New Rust files

| Path | Purpose |
|---|---|
| `elohim/elohim-storage/migrations/2026-05-02-110000_peer_blob_inventory/up.sql` | Create `peer_blob_inventory` table + indexes |
| `elohim/elohim-storage/migrations/2026-05-02-110000_peer_blob_inventory/down.sql` | Drop table + indexes |
| `elohim/elohim-storage/migrations/2026-05-02-110100_peer_inventory_cursor/up.sql` | Create `peer_inventory_cursor` table |
| `elohim/elohim-storage/migrations/2026-05-02-110100_peer_inventory_cursor/down.sql` | Drop cursor table |
| `elohim/elohim-storage/migrations/2026-05-02-120000_placement_gap_action/up.sql` | Index for `economic_events(action='placement-gap', resource_inventoried_as)` query |
| `elohim/elohim-storage/migrations/2026-05-02-120000_placement_gap_action/down.sql` | Drop the index |
| `elohim/elohim-storage/src/db/peer_blob_inventory.rs` | Diesel CRUD + `apply_snapshot` / `apply_delta` / `record_fetch_success` / `lookup_hosts` |
| `elohim/elohim-storage/src/p2p/inventory_gossip.rs` | Wire types (`BlobInventorySnapshot`, `BlobInventoryDelta`), MessagePack codecs, structural-verify, topic constant |
| `elohim/elohim-storage/src/p2p/inventory_broadcaster.rs` | Snapshot timer, delta emitter, sequence allocator, parity sweep |
| `elohim/elohim-storage/src/p2p/blob_fetch.rs` | Shared race-fetch helper (consult inventory, race candidates, verify, persist, emit `serve-blob`) |
| `elohim/elohim-storage/src/reconcile/custody.rs` | Reconciliation controller — multi-trigger reconcile pass |

### Modified Rust files

| Path | Change |
|---|---|
| `elohim/elohim-storage/src/db/diesel_schema.rs` | Add `table!` macros for `peer_blob_inventory` and `peer_inventory_cursor`; register in `allow_tables_to_appear_in_same_query!` if needed |
| `elohim/elohim-storage/src/db/models.rs` | Add `Queryable` + `Insertable` structs for both new tables |
| `elohim/elohim-storage/src/db/mod.rs` | `pub mod peer_blob_inventory;` |
| `elohim/elohim-storage/src/p2p/mod.rs` | Register `INVENTORY_TOPIC` subscription; wire receive arm; expose `is_connected` / `connected_peers` accessors; new `P2PCommand::FetchBlob { peer_id, hash, reply }` variant; new `P2PCommand::SnapshotRequest { peer_id }` variant |
| `elohim/elohim-storage/src/p2p/mod.rs` | Add `pub mod inventory_gossip;`, `pub mod inventory_broadcaster;`, `pub mod blob_fetch;` |
| `elohim/elohim-storage/src/reconcile/mod.rs` | `pub mod custody;` |
| `elohim/elohim-storage/src/http.rs` | Modify `GET /blob/{hash}` handler — call `blob_fetch::race_fetch` on local miss before returning 404. Add `/api/v1/diagnostics/inventory-parity` route. |
| `elohim/elohim-storage/src/config.rs` | Add operator presets: `inventory_broadcast_seconds`, `inventory_freshness_seconds`, `custody_sweep_seconds`, `placement_grace_seconds`, `placement_gap_cooldown_seconds`, `kick_fetch_per_peer_per_minute`, `fetch_blob_timeout_seconds`, `fetch_blob_parallelism` |

### Test files

Inline `#[cfg(test)] mod tests` in each new module. Plus:

| Path | Purpose |
|---|---|
| `elohim/elohim-storage/tests/blob_inventory_smoke.rs` | Integration smoke test — apply snapshot, query inventory, race fetch with mocked P2P channel |

---

## Phase ordering

Tasks are dependency-ordered. Each task lands one commit; the lib test count grows monotonically. Multi-peer integration scenarios run on Jenkins, not Eclipse Che.

| Order | Task | Depends on |
|---|---|---|
| 1 | T12 — `peer_blob_inventory` migration + Diesel | none (Phase 1 schemas are sibling; not dependencies) |
| 2 | T13 — Inventory gossip wire types | T12 (uses the row types) |
| 3 | T14 — Inventory projection writer | T12 + T13 |
| 4 | T15 — Inventory broadcast scheduler | T13 + T14 (delta emitter hooks into the same module that consumes via writer) |
| 5 | T16 — Custody reconciliation controller (incl. placement-gap migration) | T12 + T14 + T15 |
| 6 | T17 — GET-time blob fallback (shared race-fetch helper) | T12 + T16 (controller uses the helper for own-commitment kicks) |
| 7 | T18 — Filesystem parity sweep + diagnostic endpoint | T15 (lives in same module) |

---

## Task T12: peer_blob_inventory migration + Diesel schema + models

**Why this is here:** This is the Reality projection table. Source of truth: libp2p gossipsub. Category C operational projection rebuildable from gossip replay. The schema is the foundation everything else hangs on.

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-02-110000_peer_blob_inventory/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-02-110000_peer_blob_inventory/down.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-02-110100_peer_inventory_cursor/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-02-110100_peer_inventory_cursor/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (add `table!` macros)
- Modify: `elohim/elohim-storage/src/db/models.rs` (add row structs)
- Modify: `elohim/elohim-storage/src/db/mod.rs` (`pub mod peer_blob_inventory;`)
- Create: `elohim/elohim-storage/src/db/peer_blob_inventory.rs` (CRUD + tests)

- [ ] **Step 1: Write migration up.sql for peer_blob_inventory**

Create `elohim/elohim-storage/migrations/2026-05-02-110000_peer_blob_inventory/up.sql`:

```sql
-- T12 — peer_blob_inventory: Reality projection of who currently hosts what blob.
--
-- Source of truth: libp2p gossipsub messages on topic 'elohim/inventory/blob'.
-- Category C operational projection rebuildable from gossip replay.
-- Manifest counterpart: rea_commitments(action='custody-blob') (DHT-notarized via T03d).
--
-- Timestamps stored as TEXT (ISO-8601) per elohim-storage conventions.
-- source discriminates evidence quality:
--   'gossip-snapshot' — peer broadcast a full inventory snapshot
--   'gossip-delta'    — peer broadcast a single add (deltas with 'removed' don't write rows; they delete)
--   'fetch-success'   — this peer successfully fetched the blob from the named peer (strongest evidence)
-- sequence is the per-peer monotonic counter from the gossip wire; used for gap-detect at receive time.

CREATE TABLE peer_blob_inventory (
    peer_id      TEXT NOT NULL,
    blob_hash    TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    source       TEXT NOT NULL CHECK (source IN ('gossip-snapshot', 'gossip-delta', 'fetch-success')),
    sequence     INTEGER NOT NULL,
    PRIMARY KEY (peer_id, blob_hash)
);

CREATE INDEX idx_peer_blob_inventory_blob ON peer_blob_inventory(blob_hash);
CREATE INDEX idx_peer_blob_inventory_recent ON peer_blob_inventory(last_seen_at);
```

- [ ] **Step 2: Write migration down.sql for peer_blob_inventory**

Create `elohim/elohim-storage/migrations/2026-05-02-110000_peer_blob_inventory/down.sql`:

```sql
DROP INDEX IF EXISTS idx_peer_blob_inventory_recent;
DROP INDEX IF EXISTS idx_peer_blob_inventory_blob;
DROP TABLE IF EXISTS peer_blob_inventory;
```

- [ ] **Step 3: Write migration up.sql for peer_inventory_cursor**

Create `elohim/elohim-storage/migrations/2026-05-02-110100_peer_inventory_cursor/up.sql`:

```sql
-- T12 — peer_inventory_cursor: per-peer sequence high-watermark.
-- Survives restart so projection writer's gap-detect doesn't false-fire on restart.
-- One row per peer; updated on each successful apply_snapshot or apply_delta.

CREATE TABLE peer_inventory_cursor (
    peer_id        TEXT NOT NULL PRIMARY KEY,
    last_sequence  INTEGER NOT NULL,
    last_updated   TEXT NOT NULL
);
```

- [ ] **Step 4: Write migration down.sql for peer_inventory_cursor**

Create `elohim/elohim-storage/migrations/2026-05-02-110100_peer_inventory_cursor/down.sql`:

```sql
DROP TABLE IF EXISTS peer_inventory_cursor;
```

- [ ] **Step 5: Add table macros to diesel_schema.rs**

In `elohim/elohim-storage/src/db/diesel_schema.rs`, append (location: keep alphabetical-by-table-name in the section near `peer_identity_bindings`):

```rust
diesel::table! {
    peer_blob_inventory (peer_id, blob_hash) {
        peer_id      -> Text,
        blob_hash    -> Text,
        last_seen_at -> Text,
        source       -> Text,
        sequence     -> BigInt,
    }
}

diesel::table! {
    peer_inventory_cursor (peer_id) {
        peer_id       -> Text,
        last_sequence -> BigInt,
        last_updated  -> Text,
    }
}
```

Also add both names to the `allow_tables_to_appear_in_same_query!` macro near the bottom of the file. Find it (search for `allow_tables_to_appear_in_same_query!`); append `peer_blob_inventory,` and `peer_inventory_cursor,` to the list.

- [ ] **Step 6: Add models to models.rs**

In `elohim/elohim-storage/src/db/models.rs`, append (near the `PeerIdentityBindingRow` definitions for symmetry):

```rust
// ============================================================================
// peer_blob_inventory (Category C — operational projection from libp2p gossip)
// ============================================================================
//
// Source of truth: libp2p gossipsub on topic 'elohim/inventory/blob'.
// Manifest counterpart: rea_commitments(action='custody-blob').

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = peer_blob_inventory)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PeerBlobInventoryRow {
    pub peer_id: String,
    pub blob_hash: String,
    pub last_seen_at: String,
    /// 'gossip-snapshot' | 'gossip-delta' | 'fetch-success'
    pub source: String,
    pub sequence: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = peer_blob_inventory)]
pub struct NewPeerBlobInventoryRow {
    pub peer_id: String,
    pub blob_hash: String,
    pub last_seen_at: String,
    pub source: String,
    pub sequence: i64,
}

// ============================================================================
// peer_inventory_cursor (Category C — operational sequence high-watermark)
// ============================================================================

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = peer_inventory_cursor)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PeerInventoryCursorRow {
    pub peer_id: String,
    pub last_sequence: i64,
    pub last_updated: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = peer_inventory_cursor)]
pub struct NewPeerInventoryCursorRow {
    pub peer_id: String,
    pub last_sequence: i64,
    pub last_updated: String,
}
```

- [ ] **Step 7: Register the new module in db/mod.rs**

In `elohim/elohim-storage/src/db/mod.rs`, add (alphabetical with peer_identity_bindings):

```rust
pub mod peer_blob_inventory;
```

- [ ] **Step 8: Write the CRUD module**

Create `elohim/elohim-storage/src/db/peer_blob_inventory.rs`:

```rust
//! CRUD for `peer_blob_inventory` and `peer_inventory_cursor`.
//!
//! Source of truth: libp2p gossipsub messages on topic 'elohim/inventory/blob'.
//! Category C operational projection rebuildable from gossip replay.
//! Manifest counterpart: rea_commitments(action='custody-blob').
//!
//! ## Sequence semantics
//!
//! - Snapshots are accepted regardless of sequence (recovery path from
//!   sequence-manipulation attacks). The receiver updates its high-watermark
//!   to the snapshot's sequence.
//! - Deltas check `sequence == stored_max + 1`. Gaps queue a snapshot-request.
//!   Replays drop silently.

use crate::db::diesel_schema::{peer_blob_inventory, peer_inventory_cursor};
use crate::db::models::{
    NewPeerBlobInventoryRow, NewPeerInventoryCursorRow, PeerBlobInventoryRow,
    PeerInventoryCursorRow,
};
use crate::error::StorageError;
use diesel::prelude::*;

/// Apply a full snapshot for a peer. Replaces all existing entries for this
/// peer with the new set; entries not in the new set are deleted.
/// Snapshots are accepted regardless of sequence.
pub fn apply_snapshot(
    conn: &mut SqliteConnection,
    peer_id: &str,
    hashes: &[String],
    sequence: i64,
    snapshot_at: &str,
) -> Result<(), StorageError> {
    conn.transaction(|conn| {
        // Delete all existing entries for this peer.
        diesel::delete(
            peer_blob_inventory::table.filter(peer_blob_inventory::peer_id.eq(peer_id)),
        )
        .execute(conn)?;

        // Insert the new set.
        for hash in hashes {
            let row = NewPeerBlobInventoryRow {
                peer_id: peer_id.to_string(),
                blob_hash: hash.clone(),
                last_seen_at: snapshot_at.to_string(),
                source: "gossip-snapshot".to_string(),
                sequence,
            };
            diesel::insert_into(peer_blob_inventory::table)
                .values(&row)
                .execute(conn)?;
        }

        // Update cursor. Snapshots always advance the cursor to their sequence.
        upsert_cursor(conn, peer_id, sequence, snapshot_at)?;

        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("apply_snapshot: {e}")))
}

/// Apply a delta for a peer. Returns `Ok(false)` if a gap was detected
/// (caller should request a snapshot); `Ok(true)` if applied; `Ok(false)`
/// silently for replays. Errors only on actual DB failures.
pub fn apply_delta(
    conn: &mut SqliteConnection,
    peer_id: &str,
    added: &[String],
    removed: &[String],
    sequence: i64,
    emitted_at: &str,
) -> Result<DeltaApplyOutcome, StorageError> {
    conn.transaction(|conn| {
        let stored_max = read_cursor_sequence(conn, peer_id)?;

        match stored_max {
            Some(max) if sequence <= max => {
                // Replay; drop silently.
                Ok::<DeltaApplyOutcome, diesel::result::Error>(DeltaApplyOutcome::Replay)
            }
            Some(max) if sequence != max + 1 => {
                // Gap detected; do not apply. Caller will request a snapshot.
                Ok::<DeltaApplyOutcome, diesel::result::Error>(DeltaApplyOutcome::Gap {
                    expected: max + 1,
                    received: sequence,
                })
            }
            // Either fresh peer (None) — accept as initial — or sequence == max + 1.
            _ => {
                for hash in added {
                    let row = NewPeerBlobInventoryRow {
                        peer_id: peer_id.to_string(),
                        blob_hash: hash.clone(),
                        last_seen_at: emitted_at.to_string(),
                        source: "gossip-delta".to_string(),
                        sequence,
                    };
                    diesel::replace_into(peer_blob_inventory::table)
                        .values(&row)
                        .execute(conn)?;
                }
                for hash in removed {
                    diesel::delete(
                        peer_blob_inventory::table
                            .filter(peer_blob_inventory::peer_id.eq(peer_id))
                            .filter(peer_blob_inventory::blob_hash.eq(hash)),
                    )
                    .execute(conn)?;
                }
                upsert_cursor(conn, peer_id, sequence, emitted_at)?;
                Ok(DeltaApplyOutcome::Applied)
            }
        }
    })
    .map_err(|e| StorageError::Database(format!("apply_delta: {e}")))
}

/// Record a successful direct fetch. Promotes the entry to source='fetch-success'
/// (the strongest evidence). Does NOT touch the cursor — fetch-success is
/// an out-of-band evidence path, not a gossip arrival.
pub fn record_fetch_success(
    conn: &mut SqliteConnection,
    peer_id: &str,
    blob_hash: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    // Read the current sequence for this (peer, blob) if any; preserve it.
    // For fresh entries, use 0 — fetch-success entries don't participate in
    // sequence-based gap detection.
    let existing_seq: Option<i64> = peer_blob_inventory::table
        .filter(peer_blob_inventory::peer_id.eq(peer_id))
        .filter(peer_blob_inventory::blob_hash.eq(blob_hash))
        .select(peer_blob_inventory::sequence)
        .first::<i64>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("record_fetch_success lookup: {e}")))?;

    let row = NewPeerBlobInventoryRow {
        peer_id: peer_id.to_string(),
        blob_hash: blob_hash.to_string(),
        last_seen_at: observed_at.to_string(),
        source: "fetch-success".to_string(),
        sequence: existing_seq.unwrap_or(0),
    };
    diesel::replace_into(peer_blob_inventory::table)
        .values(&row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("record_fetch_success upsert: {e}")))
}

/// Look up the set of peers known to host a blob, ordered by evidence
/// strength (fetch-success first, then by recency).
pub fn lookup_hosts(
    conn: &mut SqliteConnection,
    blob_hash: &str,
    fresh_after: &str,
) -> Result<Vec<PeerBlobInventoryRow>, StorageError> {
    use peer_blob_inventory::dsl;

    dsl::peer_blob_inventory
        .filter(dsl::blob_hash.eq(blob_hash))
        .filter(dsl::last_seen_at.gt(fresh_after))
        .order((
            // SQLite sorts strings lexicographically; 'fetch-success' < 'gossip-*'
            // alphabetically, but we want fetch-success first. Use a CASE expression
            // via order_by would require diesel sql_query; simpler: order by a
            // computed boolean. For now, fetch in two passes and merge.
            dsl::last_seen_at.desc(),
        ))
        .load::<PeerBlobInventoryRow>(conn)
        .map(|rows| {
            // Stable partition: fetch-success first, then the rest in last_seen_at desc order.
            let mut fetch_success: Vec<_> = rows
                .iter()
                .filter(|r| r.source == "fetch-success")
                .cloned()
                .collect();
            let rest: Vec<_> = rows
                .into_iter()
                .filter(|r| r.source != "fetch-success")
                .collect();
            fetch_success.extend(rest);
            fetch_success
        })
        .map_err(|e| StorageError::Database(format!("lookup_hosts: {e}")))
}

/// Outcome of `apply_delta`. Used by the caller to decide whether to request
/// a snapshot from the source peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaApplyOutcome {
    Applied,
    Replay,
    Gap { expected: i64, received: i64 },
}

fn upsert_cursor(
    conn: &mut SqliteConnection,
    peer_id: &str,
    sequence: i64,
    last_updated: &str,
) -> Result<(), diesel::result::Error> {
    let row = NewPeerInventoryCursorRow {
        peer_id: peer_id.to_string(),
        last_sequence: sequence,
        last_updated: last_updated.to_string(),
    };
    diesel::replace_into(peer_inventory_cursor::table)
        .values(&row)
        .execute(conn)
        .map(|_| ())
}

fn read_cursor_sequence(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Option<i64>, diesel::result::Error> {
    peer_inventory_cursor::table
        .filter(peer_inventory_cursor::peer_id.eq(peer_id))
        .select(peer_inventory_cursor::last_sequence)
        .first::<i64>(conn)
        .optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder().build(manager).expect("build pool");
        let mut conn = pool.get().expect("connection");
        run_migrations(&mut conn).expect("migrations");
        pool
    }

    #[test]
    fn snapshot_replaces_set() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(&mut conn, "peer_A", &["h1".into(), "h2".into()], 1, "2026-05-02T00:00:00Z")
            .unwrap();
        apply_snapshot(&mut conn, "peer_A", &["h2".into(), "h3".into()], 2, "2026-05-02T00:01:00Z")
            .unwrap();

        let rows = lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z").unwrap();
        assert!(rows.is_empty(), "h1 should be gone after second snapshot");

        let rows = lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peer_id, "peer_A");
        assert_eq!(rows[0].sequence, 2);
    }

    #[test]
    fn delta_applied_when_in_order() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // Initial snapshot establishes sequence 1.
        apply_snapshot(&mut conn, "peer_B", &["h1".into()], 1, "2026-05-02T00:00:00Z").unwrap();

        let outcome = apply_delta(
            &mut conn,
            "peer_B",
            &["h2".into()],
            &[],
            2,
            "2026-05-02T00:00:30Z",
        )
        .unwrap();
        assert_eq!(outcome, DeltaApplyOutcome::Applied);

        let rows = lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peer_id, "peer_B");
    }

    #[test]
    fn delta_gap_returns_gap_outcome() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(&mut conn, "peer_C", &["h1".into()], 1, "2026-05-02T00:00:00Z").unwrap();

        // Skip to sequence 5 — gap.
        let outcome = apply_delta(
            &mut conn,
            "peer_C",
            &["h2".into()],
            &[],
            5,
            "2026-05-02T00:00:30Z",
        )
        .unwrap();
        assert_eq!(
            outcome,
            DeltaApplyOutcome::Gap {
                expected: 2,
                received: 5
            }
        );

        // h2 should NOT be persisted.
        let rows = lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z").unwrap();
        assert!(rows.is_empty(), "delta with gap must not persist");
    }

    #[test]
    fn delta_replay_drops_silently() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(&mut conn, "peer_D", &["h1".into()], 5, "2026-05-02T00:00:00Z").unwrap();

        // Replay an old delta with sequence 3.
        let outcome = apply_delta(
            &mut conn,
            "peer_D",
            &["h2".into()],
            &[],
            3,
            "2026-05-02T00:00:30Z",
        )
        .unwrap();
        assert_eq!(outcome, DeltaApplyOutcome::Replay);

        let rows = lookup_hosts(&mut conn, "h2", "2026-05-01T00:00:00Z").unwrap();
        assert!(rows.is_empty(), "replay must not write");
    }

    #[test]
    fn record_fetch_success_promotes_source() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(&mut conn, "peer_E", &["h1".into()], 1, "2026-05-02T00:00:00Z").unwrap();

        record_fetch_success(&mut conn, "peer_E", "h1", "2026-05-02T00:01:00Z").unwrap();

        let rows = lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source, "fetch-success");
        assert_eq!(rows[0].last_seen_at, "2026-05-02T00:01:00Z");
    }

    #[test]
    fn lookup_hosts_orders_fetch_success_first() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        // Two peers gossip the same blob.
        apply_snapshot(&mut conn, "peer_F", &["h1".into()], 1, "2026-05-02T00:00:00Z").unwrap();
        apply_snapshot(&mut conn, "peer_G", &["h1".into()], 1, "2026-05-02T00:01:00Z").unwrap();

        // peer_F got promoted to fetch-success.
        record_fetch_success(&mut conn, "peer_F", "h1", "2026-05-02T00:00:30Z").unwrap();

        let rows = lookup_hosts(&mut conn, "h1", "2026-05-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].peer_id, "peer_F", "fetch-success peer must come first");
        assert_eq!(rows[0].source, "fetch-success");
        assert_eq!(rows[1].peer_id, "peer_G");
    }

    #[test]
    fn lookup_hosts_filters_stale() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        apply_snapshot(&mut conn, "peer_H", &["h1".into()], 1, "2026-05-01T00:00:00Z").unwrap();

        // Fresh-after threshold beyond the snapshot timestamp.
        let rows = lookup_hosts(&mut conn, "h1", "2026-05-02T00:00:00Z").unwrap();
        assert!(rows.is_empty(), "stale entries must not appear");
    }
}
```

- [ ] **Step 9: Build + run targeted tests**

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib peer_blob_inventory --quiet
```

Expected: all 7 new tests pass; full lib pass should be ≥1161 (was 1154 + 7 new tests).

- [ ] **Step 10: Run full lib pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --quiet 2>&1 | tail -5
```

Expected: `1161 passed; 0 failed; 1 ignored` or similar (count grows by exactly 7 from T11's 1154 baseline).

- [ ] **Step 11: Run clippy + fmt**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests -- -D warnings 2>&1 | tail -10
cargo fmt --check -p elohim-storage 2>&1 | head -10
```

Expected: clippy clean; fmt clean.

- [ ] **Step 12: Commit**

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology
git add elohim/elohim-storage/migrations/2026-05-02-110000_peer_blob_inventory/ \
        elohim/elohim-storage/migrations/2026-05-02-110100_peer_inventory_cursor/ \
        elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/src/db/models.rs \
        elohim/elohim-storage/src/db/mod.rs \
        elohim/elohim-storage/src/db/peer_blob_inventory.rs
git commit -m "feat(storage): T12 — peer_blob_inventory projection table + Diesel CRUD"
```

---

## Task T13: Inventory gossip wire types

**Why this is here:** The wire format defines what flows on `elohim/inventory/blob`. Snapshots are authoritative; deltas carry per-peer monotonic sequence for gap-detect. Both messages have a structural-non-empty signature field (Stage 1; Ed25519 verification is Stage 2).

**Files:**
- Create: `elohim/elohim-storage/src/p2p/inventory_gossip.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — add `pub mod inventory_gossip;`

- [ ] **Step 1: Write the test for snapshot codec round-trip (TDD)**

Create `elohim/elohim-storage/src/p2p/inventory_gossip.rs` with the test stub first:

```rust
//! Inventory gossip wire types and structural verification.
//!
//! Topic: `elohim/inventory/blob` (gossipsub).
//! Wire format: MessagePack via `rmp_serde`.
//!
//! ## Why two messages
//!
//! - `BlobInventorySnapshot` is the authoritative full-state replacement.
//!   Receivers replace their per-peer entries with the snapshot's set.
//! - `BlobInventoryDelta` is the event-driven add/remove.
//!   Receivers track per-peer sequence; gap-detect requests a snapshot.
//!
//! ## Stage 1 signature
//!
//! Both messages carry a `signature: Vec<u8>` field that is structurally
//! non-empty (a single null byte is sufficient at Stage 1). Stage 2 will
//! enforce Ed25519 verification over canonical bytes; the structural-non-empty
//! gate is a forward-compatible placeholder.

use serde::{Deserialize, Serialize};

/// Gossipsub topic for blob inventory broadcasts. Wire-level keeps the
/// `blob` identifier even though the broader vocabulary uses `quilt`/`pantry`
/// per the storage-vocabulary memory pin.
pub const INVENTORY_TOPIC: &str = "elohim/inventory/blob";

/// Periodic full-state snapshot. Replaces the receiver's per-peer entries
/// with the snapshot's set. Accepted regardless of sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobInventorySnapshot {
    /// Multibase-encoded libp2p PeerId of the broadcaster.
    pub peer_id: String,
    /// Set of blob hashes the peer currently hosts.
    pub hashes: Vec<String>,
    /// Microseconds since epoch — when the snapshot was computed.
    pub snapshot_at: i64,
    /// Per-peer monotonic counter. Snapshots advance the receiver's high-watermark.
    pub sequence: u64,
    /// Structural non-empty signature (Stage 1). Ed25519 in Stage 2.
    pub signature: Vec<u8>,
}

/// Event-driven add/remove. Receivers apply against their per-peer set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobInventoryDelta {
    pub peer_id: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Microseconds since epoch — when the delta was emitted.
    pub emitted_at: i64,
    /// Per-peer monotonic counter. Receivers gap-detect on `expected_next` mismatch.
    pub sequence: u64,
    /// Structural non-empty signature (Stage 1). Ed25519 in Stage 2.
    pub signature: Vec<u8>,
}

/// Reasons a wire message can fail structural verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    EmptyPeerId,
    EmptySignature,
    EmptyDelta,
    InvalidHashFormat(String),
}

impl BlobInventorySnapshot {
    /// Encode to MessagePack bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Decode from MessagePack bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Structural verification — Stage 1 gate.
    ///
    /// Enforces:
    /// - peer_id is non-empty
    /// - signature is non-empty (Stage 1; Ed25519 in Stage 2)
    /// - blob hashes look like sha256 hex (64 hex chars) — defensive only
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.peer_id.is_empty() {
            return Err(VerifyError::EmptyPeerId);
        }
        if self.signature.is_empty() {
            return Err(VerifyError::EmptySignature);
        }
        for hash in &self.hashes {
            if !is_blob_hash_shaped(hash) {
                return Err(VerifyError::InvalidHashFormat(hash.clone()));
            }
        }
        Ok(())
    }
}

impl BlobInventoryDelta {
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Structural verification — Stage 1 gate.
    ///
    /// In addition to the snapshot rules, deltas must carry at least one
    /// add or remove. Empty deltas are protocol violations.
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.peer_id.is_empty() {
            return Err(VerifyError::EmptyPeerId);
        }
        if self.signature.is_empty() {
            return Err(VerifyError::EmptySignature);
        }
        if self.added.is_empty() && self.removed.is_empty() {
            return Err(VerifyError::EmptyDelta);
        }
        for hash in self.added.iter().chain(self.removed.iter()) {
            if !is_blob_hash_shaped(hash) {
                return Err(VerifyError::InvalidHashFormat(hash.clone()));
            }
        }
        Ok(())
    }
}

/// Sha256 hex shape check: 64 lowercase hex chars (defensive structural rule).
fn is_blob_hash_shaped(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> BlobInventorySnapshot {
        BlobInventorySnapshot {
            peer_id: "12D3KooWtest1".to_string(),
            hashes: vec!["a".repeat(64), "b".repeat(64)],
            snapshot_at: 1_700_000_000_000_000,
            sequence: 42,
            signature: vec![0x00],
        }
    }

    fn sample_delta() -> BlobInventoryDelta {
        BlobInventoryDelta {
            peer_id: "12D3KooWtest1".to_string(),
            added: vec!["c".repeat(64)],
            removed: vec!["a".repeat(64)],
            emitted_at: 1_700_000_001_000_000,
            sequence: 43,
            signature: vec![0x00],
        }
    }

    #[test]
    fn snapshot_round_trips() {
        let snapshot = sample_snapshot();
        let bytes = snapshot.to_bytes().unwrap();
        let decoded = BlobInventorySnapshot::from_bytes(&bytes).unwrap();
        assert_eq!(snapshot, decoded);
    }

    #[test]
    fn delta_round_trips() {
        let delta = sample_delta();
        let bytes = delta.to_bytes().unwrap();
        let decoded = BlobInventoryDelta::from_bytes(&bytes).unwrap();
        assert_eq!(delta, decoded);
    }

    #[test]
    fn snapshot_verify_passes_well_formed() {
        assert_eq!(sample_snapshot().verify_structural(), Ok(()));
    }

    #[test]
    fn snapshot_verify_rejects_empty_peer_id() {
        let mut s = sample_snapshot();
        s.peer_id = String::new();
        assert_eq!(s.verify_structural(), Err(VerifyError::EmptyPeerId));
    }

    #[test]
    fn snapshot_verify_rejects_empty_signature() {
        let mut s = sample_snapshot();
        s.signature.clear();
        assert_eq!(s.verify_structural(), Err(VerifyError::EmptySignature));
    }

    #[test]
    fn snapshot_verify_rejects_malformed_hash() {
        let mut s = sample_snapshot();
        s.hashes.push("notahex!".to_string());
        assert!(matches!(
            s.verify_structural(),
            Err(VerifyError::InvalidHashFormat(_))
        ));
    }

    #[test]
    fn delta_verify_passes_well_formed() {
        assert_eq!(sample_delta().verify_structural(), Ok(()));
    }

    #[test]
    fn delta_verify_rejects_empty_payload() {
        let mut d = sample_delta();
        d.added.clear();
        d.removed.clear();
        assert_eq!(d.verify_structural(), Err(VerifyError::EmptyDelta));
    }

    #[test]
    fn delta_verify_rejects_malformed_hash() {
        let mut d = sample_delta();
        d.added.push("notahex!".to_string());
        assert!(matches!(
            d.verify_structural(),
            Err(VerifyError::InvalidHashFormat(_))
        ));
    }

    #[test]
    fn topic_constant_matches_spec() {
        assert_eq!(INVENTORY_TOPIC, "elohim/inventory/blob");
    }
}
```

- [ ] **Step 2: Register the module in p2p/mod.rs**

In `elohim/elohim-storage/src/p2p/mod.rs`, find the section where other p2p submodules are declared (search for `pub mod identity_handshake;` or similar) and add:

```rust
pub mod inventory_gossip;
```

- [ ] **Step 3: Run targeted tests**

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib inventory_gossip --quiet
```

Expected: 10 tests pass.

- [ ] **Step 4: Run full lib pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --quiet 2>&1 | tail -5
```

Expected: lib count grows by 10 from T12's level (now ~1171 passing).

- [ ] **Step 5: Clippy + fmt**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests -- -D warnings 2>&1 | tail -5
cargo fmt --check -p elohim-storage 2>&1 | head -5
```

Expected: both clean.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p/inventory_gossip.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(storage): T13 — inventory gossip wire types (snapshot + delta) with structural verify"
```

---

## Task T14: Inventory projection writer

**Why this is here:** Glue between gossip arrival and the SQLite projection. Subscribes to `elohim/inventory/blob`, decodes, structural-verifies, dispatches to `apply_snapshot` or `apply_delta`. On gap-detect, sends a `SnapshotRequest` command via the swarm channel.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — add receive arm for INVENTORY_TOPIC; add `P2PCommand::SnapshotRequest { peer_id }`; add module exposing the writer function for testability

- [ ] **Step 1: Add the SnapshotRequest P2PCommand variant**

Find the `P2PCommand` enum in `elohim/elohim-storage/src/p2p/mod.rs` (search for `enum P2PCommand`). Add a new variant:

```rust
/// Request a fresh `BlobInventorySnapshot` from the named peer.
/// Issued by the projection writer when it detects a sequence gap.
SnapshotRequest {
    peer_id: libp2p::PeerId,
},
```

The dispatcher arm in the event loop should:
- For now, log at debug and drop (a Stage-1 placeholder). Stage 2 will route this as a libp2p request-response message. The placeholder is acceptable because the next periodic snapshot from the source peer will close the gap naturally — `SnapshotRequest` is an optimization for fast recovery, not a correctness requirement.

Find the `match cmd` block that processes `P2PCommand` variants (search for the existing `P2PCommand::FetchShard` arm) and add:

```rust
P2PCommand::SnapshotRequest { peer_id } => {
    debug!(
        target: "elohim_storage::inventory",
        peer_id = %peer_id,
        "SnapshotRequest queued; Stage 1 placeholder — relying on next periodic snapshot"
    );
}
```

- [ ] **Step 2: Subscribe to the inventory topic at swarm init**

Find where other gossipsub topics are subscribed (search for `gossipsub.subscribe`). Add (alongside `RECOVERY_REVOCATION_TOPIC` or similar):

```rust
{
    let topic = libp2p::gossipsub::IdentTopic::new(crate::p2p::inventory_gossip::INVENTORY_TOPIC);
    if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
        warn!(target: "elohim_storage::inventory", error = ?e, "Failed to subscribe to inventory topic");
    }
}
```

- [ ] **Step 3: Add the receive arm for INVENTORY_TOPIC**

Find the gossipsub Message handler arm (search for `GossipsubEvent::Message` or `gossipsub::Event::Message`). Inside the `if message.topic.as_str() == ...` cascade, add a new branch above (or below, as fits) the existing recovery topic branch:

```rust
} else if message.topic.as_str() == crate::p2p::inventory_gossip::INVENTORY_TOPIC {
    // Try snapshot first, then delta. We don't have a wire-level discriminator;
    // distinguishing relies on serde — snapshots have `hashes` (no `added`/`removed`)
    // and deltas have `added`/`removed` (no `hashes`). serde will fail one and accept
    // the other.

    use crate::p2p::inventory_gossip::{BlobInventorySnapshot, BlobInventoryDelta};

    if let Ok(snapshot) = BlobInventorySnapshot::from_bytes(&message.data) {
        if let Err(e) = snapshot.verify_structural() {
            warn!(
                target: "elohim_storage::inventory",
                from = %propagation_source,
                error = ?e,
                "Inventory snapshot failed structural verify — dropped"
            );
        } else if let Some(pool) = self.db_pool.as_ref() {
            match pool.get() {
                Ok(mut conn) => {
                    let now_iso = chrono::Utc::now()
                        .format("%Y-%m-%dT%H:%M:%SZ")
                        .to_string();
                    let _ = micros_to_iso(snapshot.snapshot_at)
                        .or(Some(now_iso.clone()));
                    let when = micros_to_iso(snapshot.snapshot_at).unwrap_or(now_iso);
                    match crate::db::peer_blob_inventory::apply_snapshot(
                        &mut conn,
                        &snapshot.peer_id,
                        &snapshot.hashes,
                        snapshot.sequence as i64,
                        &when,
                    ) {
                        Ok(()) => debug!(
                            target: "elohim_storage::inventory",
                            from = %propagation_source,
                            peer_id = %snapshot.peer_id,
                            count = snapshot.hashes.len(),
                            sequence = snapshot.sequence,
                            "Inventory snapshot applied"
                        ),
                        Err(e) => warn!(
                            target: "elohim_storage::inventory",
                            from = %propagation_source,
                            error = %e,
                            "apply_snapshot failed"
                        ),
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::inventory",
                    error = %e,
                    "inventory: db pool exhausted"
                ),
            }
        }
    } else if let Ok(delta) = BlobInventoryDelta::from_bytes(&message.data) {
        if let Err(e) = delta.verify_structural() {
            warn!(
                target: "elohim_storage::inventory",
                from = %propagation_source,
                error = ?e,
                "Inventory delta failed structural verify — dropped"
            );
        } else if let Some(pool) = self.db_pool.as_ref() {
            match pool.get() {
                Ok(mut conn) => {
                    let when = micros_to_iso(delta.emitted_at)
                        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
                    match crate::db::peer_blob_inventory::apply_delta(
                        &mut conn,
                        &delta.peer_id,
                        &delta.added,
                        &delta.removed,
                        delta.sequence as i64,
                        &when,
                    ) {
                        Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Applied) => {
                            debug!(
                                target: "elohim_storage::inventory",
                                peer_id = %delta.peer_id,
                                sequence = delta.sequence,
                                added = delta.added.len(),
                                removed = delta.removed.len(),
                                "Inventory delta applied"
                            );
                        }
                        Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Replay) => {
                            debug!(
                                target: "elohim_storage::inventory",
                                peer_id = %delta.peer_id,
                                sequence = delta.sequence,
                                "Inventory delta replay — dropped silently"
                            );
                        }
                        Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Gap { expected, received }) => {
                            warn!(
                                target: "elohim_storage::inventory",
                                peer_id = %delta.peer_id,
                                expected,
                                received,
                                "Inventory delta gap — requesting snapshot"
                            );
                            // Best-effort: send the snapshot-request command.
                            // Parse the peer_id string into a libp2p::PeerId.
                            if let Ok(pid) = delta.peer_id.parse::<libp2p::PeerId>() {
                                let cmd = P2PCommand::SnapshotRequest { peer_id: pid };
                                let _ = self.cmd_tx.try_send(cmd);
                            }
                        }
                        Err(e) => warn!(
                            target: "elohim_storage::inventory",
                            error = %e,
                            "apply_delta failed"
                        ),
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::inventory",
                    error = %e,
                    "inventory: db pool exhausted"
                ),
            }
        }
    } else {
        debug!(
            target: "elohim_storage::inventory",
            from = %propagation_source,
            "Inventory message decoded as neither snapshot nor delta — dropped"
        );
    }
}
```

If `micros_to_iso` doesn't already exist as a helper in this module or a parent, add it near the top of `p2p/mod.rs`:

```rust
fn micros_to_iso(micros: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_micros(micros)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}
```

(Verify with `grep` whether `from_timestamp_micros` is already in scope — if Holochain timestamps already have a similar conversion, prefer that over adding a duplicate.)

- [ ] **Step 4: Add a unit test for the writer dispatch logic**

This is hard to test end-to-end without the full p2p event loop. Instead, exercise the dispatch by calling `apply_snapshot` and `apply_delta` directly in a new integration test file (the existing dispatch logic's correctness is exercised via the T12 unit tests for `apply_snapshot`/`apply_delta`; this integration-level test demonstrates the receive-side path under test conditions).

Create `elohim/elohim-storage/tests/inventory_writer_smoke.rs`:

```rust
//! Smoke test for the inventory projection writer's apply paths.
//! Exercises `apply_snapshot` and `apply_delta` against an in-memory pool,
//! mirroring what the live receive arm in p2p/mod.rs does.

use elohim_storage::db::peer_blob_inventory::{
    apply_delta, apply_snapshot, lookup_hosts, DeltaApplyOutcome,
};
use elohim_storage::db::{run_migrations, DbPool};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;

fn test_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder().build(manager).expect("build pool");
    let mut conn = pool.get().expect("connection");
    run_migrations(&mut conn).expect("migrations");
    pool
}

#[test]
fn writer_applies_snapshot_then_in_order_deltas() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    apply_snapshot(
        &mut conn,
        "12D3KooWtest1",
        &["a".repeat(64), "b".repeat(64)],
        1,
        "2026-05-02T00:00:00Z",
    )
    .expect("snapshot");

    let outcome = apply_delta(
        &mut conn,
        "12D3KooWtest1",
        &["c".repeat(64)],
        &[],
        2,
        "2026-05-02T00:00:30Z",
    )
    .expect("delta");
    assert_eq!(outcome, DeltaApplyOutcome::Applied);

    let hosts = lookup_hosts(&mut conn, &"c".repeat(64), "2026-05-01T00:00:00Z").unwrap();
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].peer_id, "12D3KooWtest1");
}

#[test]
fn writer_detects_gap_on_out_of_order_delta() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    apply_snapshot(
        &mut conn,
        "12D3KooWtest2",
        &["a".repeat(64)],
        1,
        "2026-05-02T00:00:00Z",
    )
    .expect("snapshot");

    let outcome = apply_delta(
        &mut conn,
        "12D3KooWtest2",
        &["b".repeat(64)],
        &[],
        5,
        "2026-05-02T00:01:00Z",
    )
    .expect("delta");
    match outcome {
        DeltaApplyOutcome::Gap { expected, received } => {
            assert_eq!(expected, 2);
            assert_eq!(received, 5);
        }
        other => panic!("expected Gap, got {other:?}"),
    }
}
```

- [ ] **Step 5: Build and run targeted tests**

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test inventory_writer_smoke --quiet
```

Expected: 2 tests pass.

- [ ] **Step 6: Run full lib + integration tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --quiet 2>&1 | tail -3
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test inventory_writer_smoke --quiet 2>&1 | tail -3
```

Expected: lib still ~1171; new integration smoke test passes.

- [ ] **Step 7: Clippy + fmt**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests -- -D warnings 2>&1 | tail -5
cargo fmt --check -p elohim-storage 2>&1 | head -5
```

Expected: both clean.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/tests/inventory_writer_smoke.rs
git commit -m "feat(storage): T14 — inventory projection writer (gossip→SQL with gap-detect + snapshot-request)"
```

---

## Task T15: Inventory broadcast scheduler

**Why this is here:** Owns the snapshot timer and the delta emitter. Reads operator presets for archetype-tunable cadence. Maintains the per-peer (this-peer) sequence allocator.

**Files:**
- Create: `elohim/elohim-storage/src/p2p/inventory_broadcaster.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — `pub mod inventory_broadcaster;` and spawn the broadcaster task
- Modify: `elohim/elohim-storage/src/config.rs` — add `inventory_broadcast_seconds: Option<u64>` field

- [ ] **Step 1: Add config field**

In `elohim/elohim-storage/src/config.rs`, find the `Config` struct (search for `pub struct Config`). Add the field next to existing operator presets:

```rust
/// Cadence for inventory snapshot broadcasts on `elohim/inventory/blob`.
/// Defaults are archetype-driven (see `inventory_broadcast_seconds_default`).
/// Operator preset; 4-layer override pattern (archetype → policy.toml → env/CLI → admin trigger).
pub inventory_broadcast_seconds: Option<u64>,
```

In the `Default` impl, add `inventory_broadcast_seconds: None,` at the right line.

In the env-var parsing block (search for `INVENTORY_` or follow the existing pattern for `BLOB_PANTRY_MAX_BYTES` etc.), add:

```rust
if let Ok(v) = std::env::var("INVENTORY_BROADCAST_SECONDS") {
    if let Ok(n) = v.parse::<u64>() {
        config.inventory_broadcast_seconds = Some(n);
    }
}
```

Add a helper function at the bottom of `config.rs`:

```rust
/// Default snapshot broadcast cadence per archetype.
/// `None` means broadcasting is disabled by default for this archetype.
pub fn inventory_broadcast_seconds_default(archetype: Option<&str>) -> Option<u64> {
    match archetype {
        Some("node") => Some(60),
        Some("desktop") => Some(300),
        Some("mobile") => None,
        Some("steward") => Some(60),
        _ => Some(60),
    }
}
```

- [ ] **Step 2: Write the broadcaster module skeleton with TDD test**

Create `elohim/elohim-storage/src/p2p/inventory_broadcaster.rs`:

```rust
//! Inventory broadcaster — snapshot timer + delta emitter + sequence allocator.
//!
//! Responsibilities:
//! - Periodically (per archetype-tunable cadence) compute the local set of
//!   hosted blob hashes and publish a `BlobInventorySnapshot` to
//!   `elohim/inventory/blob`.
//! - On local blob add/remove events, emit a `BlobInventoryDelta` (with a
//!   small batching window to coalesce bursts).
//! - Allocate per-this-peer monotonic sequence numbers for both message types.
//!
//! Source of truth for "what blobs do I host": the local blob store; the
//! enumeration is delegated to `LocalInventory` so it can be mocked in tests.

use crate::p2p::inventory_gossip::{BlobInventoryDelta, BlobInventorySnapshot};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Trait for enumerating the local blob inventory. Production: walks the
/// blob store. Tests: returns a fixed set.
pub trait LocalInventory: Send + Sync {
    fn current_hashes(&self) -> Vec<String>;
}

/// Per-this-peer monotonic sequence allocator.
#[derive(Debug, Clone, Default)]
pub struct SequenceAllocator {
    inner: Arc<AtomicU64>,
}

impl SequenceAllocator {
    pub fn new(initial: u64) -> Self {
        Self {
            inner: Arc::new(AtomicU64::new(initial)),
        }
    }

    /// Allocate the next sequence number.
    pub fn next(&self) -> u64 {
        self.inner.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn current(&self) -> u64 {
        self.inner.load(Ordering::SeqCst)
    }
}

/// Build a snapshot for the given peer id with the given inventory.
pub fn build_snapshot<I: LocalInventory>(
    peer_id: &str,
    inventory: &I,
    seq: &SequenceAllocator,
    now_micros: i64,
) -> BlobInventorySnapshot {
    BlobInventorySnapshot {
        peer_id: peer_id.to_string(),
        hashes: inventory.current_hashes(),
        snapshot_at: now_micros,
        sequence: seq.next(),
        signature: vec![0x00], // Stage 1 structural non-empty
    }
}

/// Build a delta for the given add/remove batch.
pub fn build_delta(
    peer_id: &str,
    added: Vec<String>,
    removed: Vec<String>,
    seq: &SequenceAllocator,
    now_micros: i64,
) -> BlobInventoryDelta {
    BlobInventoryDelta {
        peer_id: peer_id.to_string(),
        added,
        removed,
        emitted_at: now_micros,
        sequence: seq.next(),
        signature: vec![0x00], // Stage 1 structural non-empty
    }
}

/// Compute the resolved cadence for this peer's archetype, honoring the
/// 4-layer override pattern (archetype default ← policy.toml ← env/CLI ←
/// admin trigger). Returns `None` to mean "broadcasting disabled."
pub fn resolved_cadence(
    archetype: Option<&str>,
    config_override: Option<u64>,
) -> Option<u64> {
    if let Some(seconds) = config_override {
        if seconds == 0 {
            return None; // explicit "0" means disabled
        }
        return Some(seconds);
    }
    crate::config::inventory_broadcast_seconds_default(archetype)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockInventory(Vec<String>);
    impl LocalInventory for MockInventory {
        fn current_hashes(&self) -> Vec<String> {
            self.0.clone()
        }
    }

    #[test]
    fn sequence_allocator_increments() {
        let alloc = SequenceAllocator::new(0);
        assert_eq!(alloc.next(), 1);
        assert_eq!(alloc.next(), 2);
        assert_eq!(alloc.next(), 3);
        assert_eq!(alloc.current(), 3);
    }

    #[test]
    fn snapshot_includes_inventory_and_advances_sequence() {
        let inv = MockInventory(vec!["a".repeat(64), "b".repeat(64)]);
        let alloc = SequenceAllocator::new(10);
        let snapshot = build_snapshot("12D3KooWtest", &inv, &alloc, 1_700_000_000_000_000);

        assert_eq!(snapshot.peer_id, "12D3KooWtest");
        assert_eq!(snapshot.hashes.len(), 2);
        assert_eq!(snapshot.sequence, 11);
        assert_eq!(snapshot.signature, vec![0x00]);
    }

    #[test]
    fn delta_carries_added_and_removed() {
        let alloc = SequenceAllocator::new(0);
        let delta = build_delta(
            "12D3KooWtest",
            vec!["a".repeat(64)],
            vec!["b".repeat(64)],
            &alloc,
            1_700_000_001_000_000,
        );

        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.removed.len(), 1);
        assert_eq!(delta.sequence, 1);
    }

    #[test]
    fn resolved_cadence_uses_override_when_present() {
        assert_eq!(resolved_cadence(Some("node"), Some(120)), Some(120));
    }

    #[test]
    fn resolved_cadence_falls_back_to_archetype_default() {
        assert_eq!(resolved_cadence(Some("node"), None), Some(60));
        assert_eq!(resolved_cadence(Some("desktop"), None), Some(300));
        assert_eq!(resolved_cadence(Some("mobile"), None), None);
        assert_eq!(resolved_cadence(Some("steward"), None), Some(60));
    }

    #[test]
    fn resolved_cadence_zero_override_disables() {
        assert_eq!(resolved_cadence(Some("node"), Some(0)), None);
    }
}
```

- [ ] **Step 3: Register the module**

In `elohim/elohim-storage/src/p2p/mod.rs`, add:

```rust
pub mod inventory_broadcaster;
```

- [ ] **Step 4: Run targeted tests**

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib inventory_broadcaster --quiet
```

Expected: 6 tests pass.

- [ ] **Step 5: Run full lib pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --quiet 2>&1 | tail -3
```

Expected: lib count grows by 6 from T14's level (now ~1177).

- [ ] **Step 6: Clippy + fmt**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests -- -D warnings 2>&1 | tail -5
cargo fmt --check -p elohim-storage 2>&1 | head -5
```

Expected: both clean.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/p2p/inventory_broadcaster.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/src/config.rs
git commit -m "feat(storage): T15 — inventory broadcast scheduler (snapshot/delta builders + sequence allocator + archetype-tunable cadence)"
```

**Note on the runtime broadcaster task:** The actual periodic-spawn loop (calling `build_snapshot` + publishing on the gossipsub topic at the resolved cadence) lives in the next-phase wiring. T15 lands the building blocks; the spawn-loop is one short call site that runs at swarm init time and is most easily tested at the integration level (Jenkins). The unit tests above cover the message construction and cadence-resolution logic — the bits where bugs hide.

---

## Task T16: Custody reconciliation controller

**Why this is here:** The diff engine. Multi-trigger reconcile pass. Acts on own commitments by kicking fetches; signals on others' by emitting `placement-gap` REA events. Connectivity API and reconciliation metrics fold in here.

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-02-120000_placement_gap_action/up.sql` and `down.sql`
- Create: `elohim/elohim-storage/src/reconcile/custody.rs`
- Modify: `elohim/elohim-storage/src/reconcile/mod.rs` (`pub mod custody;`)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — add `is_connected` and `connected_peers` accessors; add `ReconciliationMetrics` struct
- Modify: `elohim/elohim-storage/src/config.rs` — add `custody_sweep_seconds`, `placement_grace_seconds`, `placement_gap_cooldown_seconds`, `kick_fetch_per_peer_per_minute`, `inventory_freshness_seconds`

- [ ] **Step 1: Write placement-gap migration**

Create `elohim/elohim-storage/migrations/2026-05-02-120000_placement_gap_action/up.sql`:

```sql
-- T16 — placement-gap action convention.
--
-- Emitted by the custody reconciliation controller when a custody-blob
-- commitment goes unhonored beyond grace. The action joins
-- project-blob / serve-blob / custody-blob (T03d) as the operational REA
-- vocabulary. It is an observation event (lives in economic_events), not
-- a commitment.
--
-- Convention:
--   action='placement-gap'
--   provider=<custodian-cid>           — the peer expected to host
--   receiver=<content-steward-cid>     — the peer who holds the commitment
--   resource_inventoried_as=<blob_hash>
--   output_of=<custody-blob commitment action_hash>  — links the gap to its commitment
--
-- The composite index from T03d (idx_economic_events_action_resource) already
-- covers this query pattern; this migration adds an output_of index for the
-- "all gaps for commitment X" query.

CREATE INDEX IF NOT EXISTS idx_economic_events_output_of
    ON economic_events(output_of);
```

Create `elohim/elohim-storage/migrations/2026-05-02-120000_placement_gap_action/down.sql`:

```sql
DROP INDEX IF EXISTS idx_economic_events_output_of;
```

- [ ] **Step 2: Add config fields**

In `elohim/elohim-storage/src/config.rs`, add to the `Config` struct:

```rust
/// Periodic full reconcile-pass cadence for the custody controller.
pub custody_sweep_seconds: u64,
/// How long a custody commitment can be unhonored before placement-gap fires.
pub placement_grace_seconds: u64,
/// Minimum time between repeated placement-gap events for the same commitment.
pub placement_gap_cooldown_seconds: u64,
/// Rate limit on reconciliation-driven fetches per peer.
pub kick_fetch_per_peer_per_minute: u32,
/// TTL for peer_blob_inventory entries before they're considered stale.
pub inventory_freshness_seconds: u64,
```

In the `Default` impl:

```rust
custody_sweep_seconds: 120,
placement_grace_seconds: 300,
placement_gap_cooldown_seconds: 1800,
kick_fetch_per_peer_per_minute: 10,
inventory_freshness_seconds: 600,
```

Env-var parsing (mirror existing pattern):

```rust
if let Ok(v) = std::env::var("CUSTODY_SWEEP_SECONDS") {
    if let Ok(n) = v.parse::<u64>() {
        config.custody_sweep_seconds = n;
    }
}
if let Ok(v) = std::env::var("PLACEMENT_GRACE_SECONDS") {
    if let Ok(n) = v.parse::<u64>() {
        config.placement_grace_seconds = n;
    }
}
if let Ok(v) = std::env::var("PLACEMENT_GAP_COOLDOWN_SECONDS") {
    if let Ok(n) = v.parse::<u64>() {
        config.placement_gap_cooldown_seconds = n;
    }
}
if let Ok(v) = std::env::var("KICK_FETCH_PER_PEER_PER_MINUTE") {
    if let Ok(n) = v.parse::<u32>() {
        config.kick_fetch_per_peer_per_minute = n;
    }
}
if let Ok(v) = std::env::var("INVENTORY_FRESHNESS_SECONDS") {
    if let Ok(n) = v.parse::<u64>() {
        config.inventory_freshness_seconds = n;
    }
}
```

- [ ] **Step 3: Add connectivity API + ReconciliationMetrics on P2PNode**

In `elohim/elohim-storage/src/p2p/mod.rs`, find the `P2PNode` struct definition. Add a method block near the existing `peer_metrics` accessor:

```rust
impl P2PNode {
    /// Whether the named peer has an active libp2p connection right now.
    /// Backed by the existing `peer_metrics` DashMap (entries are created
    /// on connect and removed on disconnect).
    pub fn is_connected(&self, peer_id: &libp2p::PeerId) -> bool {
        self.peer_metrics
            .get(&peer_id.to_string())
            .map(|m| m.is_connected)
            .unwrap_or(false)
    }

    /// Snapshot of currently connected peers.
    pub fn connected_peers(&self) -> Vec<libp2p::PeerId> {
        self.peer_metrics
            .iter()
            .filter(|m| m.is_connected)
            .filter_map(|m| m.key().parse().ok())
            .collect()
    }

    /// Read-only view of the reconciliation metrics counter struct.
    pub fn reconciliation_metrics(&self) -> ReconciliationMetricsSnapshot {
        ReconciliationMetricsSnapshot {
            reconcile_passes_total: self
                .reconciliation_metrics
                .reconcile_passes_total
                .load(std::sync::atomic::Ordering::Relaxed),
            kicks_fired_total: self
                .reconciliation_metrics
                .kicks_fired_total
                .load(std::sync::atomic::Ordering::Relaxed),
            placement_gaps_emitted_total: self
                .reconciliation_metrics
                .placement_gaps_emitted_total
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}
```

Verify `peer_metrics` entries have an `is_connected: bool` field. If they don't, add it (and set it to `true` on `ConnectionEstablished` and `false` on `ConnectionClosed` in the existing event handlers — find these by grepping `ConnectionEstablished`).

Add the metrics struct (alongside `peer_metrics` definition):

```rust
#[derive(Debug, Default)]
pub struct ReconciliationMetrics {
    pub reconcile_passes_total: std::sync::atomic::AtomicU64,
    pub kicks_fired_total: std::sync::atomic::AtomicU64,
    pub placement_gaps_emitted_total: std::sync::atomic::AtomicU64,
}

#[derive(Debug, Clone, Copy)]
pub struct ReconciliationMetricsSnapshot {
    pub reconcile_passes_total: u64,
    pub kicks_fired_total: u64,
    pub placement_gaps_emitted_total: u64,
}
```

Add a field to `P2PNode`:

```rust
pub reconciliation_metrics: std::sync::Arc<ReconciliationMetrics>,
```

Initialize it in the `P2PNode::new` (or whichever constructor) — `reconciliation_metrics: std::sync::Arc::new(ReconciliationMetrics::default()),`.

- [ ] **Step 4: Write the controller module with TDD tests**

Create `elohim/elohim-storage/src/reconcile/custody.rs`:

```rust
//! Custody reconciliation controller.
//!
//! Multi-trigger reconcile pass:
//! - Gossip arrival (`BlobInventorySnapshot` / `BlobInventoryDelta` for peer X)
//! - Connection event (`ConnectionEstablished` for peer X)
//! - Periodic sweep (timer; cadence per `custody_sweep_seconds`)
//!
//! Each trigger calls `reconcile_pass`, which is idempotent.
//!
//! For each `custody-blob` commitment in `rea_commitments`:
//! - If this peer is the provider AND blob_hash is missing locally:
//!   query `peer_blob_inventory` for hosts; if any candidates exist,
//!   kick a fetch via `blob_fetch::race_fetch` (T17).
//! - If this peer is the receiver AND last_seen for the provider is older
//!   than `placement_grace_seconds`: emit a `placement-gap` REA event
//!   (with cooldown).
//!
//! Local store presence is queried via the trait `LocalBlobStore` so the
//! reconcile pass can be unit-tested without a real blob store.

use crate::db::diesel_schema::{economic_events, peer_blob_inventory, rea_commitments};
use crate::db::models::{NewEconomicEventRow, RewardCommitmentRow};
use crate::error::StorageError;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

/// Trait for checking whether a blob exists in this peer's local store.
/// Production: queries the blob_store. Tests: returns a fixed set.
pub trait LocalBlobStore: Send + Sync {
    fn has(&self, blob_hash: &str) -> bool;
}

/// Trait for kicking a fetch. Production: calls `blob_fetch::race_fetch` (T17).
/// Tests: records the kick request.
pub trait FetchKicker: Send + Sync {
    fn kick(&self, blob_hash: &str, candidates: Vec<String>);
}

/// Outcome of a single reconcile pass; useful for tests + metrics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconcileOutcome {
    pub kicks_fired: u32,
    pub placement_gaps_emitted: u32,
    pub commitments_examined: u32,
}

/// Run one reconcile pass over the custody-blob commitments visible in this
/// peer's projection. Idempotent.
pub fn reconcile_pass(
    conn: &mut SqliteConnection,
    self_cid: &str,
    local_store: &dyn LocalBlobStore,
    fetch_kicker: &dyn FetchKicker,
    placement_grace_seconds: u64,
    placement_gap_cooldown_seconds: u64,
    inventory_freshness_seconds: u64,
    now: DateTime<Utc>,
) -> Result<ReconcileOutcome, StorageError> {
    let mut outcome = ReconcileOutcome::default();

    let custody_rows = rea_commitments::table
        .filter(rea_commitments::action.eq("custody-blob"))
        .load::<RewardCommitmentRow>(conn)
        .map_err(|e| StorageError::Database(format!("load custody-blob commitments: {e}")))?;

    let stale_before = (now
        - chrono::Duration::seconds(inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let unhonored_before = (now
        - chrono::Duration::seconds(placement_grace_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let cooldown_after = (now
        - chrono::Duration::seconds(placement_gap_cooldown_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    for commitment in custody_rows {
        outcome.commitments_examined += 1;
        let Some(blob_hash) = commitment.resource_classified_as.as_ref() else {
            continue;
        };

        if commitment.provider == self_cid {
            // Own commitment — act if missing.
            if !local_store.has(blob_hash) {
                let candidates = crate::db::peer_blob_inventory::lookup_hosts(
                    conn,
                    blob_hash,
                    &stale_before,
                )?;
                if !candidates.is_empty() {
                    let candidate_peers: Vec<String> =
                        candidates.into_iter().map(|c| c.peer_id).collect();
                    fetch_kicker.kick(blob_hash, candidate_peers);
                    outcome.kicks_fired += 1;
                }
            }
        } else if commitment.receiver == self_cid {
            // Other peer's commitment to me — observe; signal on stale.
            let last_seen: Option<String> = peer_blob_inventory::table
                .filter(peer_blob_inventory::peer_id.eq(&commitment.provider))
                .filter(peer_blob_inventory::blob_hash.eq(blob_hash))
                .select(peer_blob_inventory::last_seen_at)
                .first::<String>(conn)
                .optional()
                .map_err(|e| {
                    StorageError::Database(format!("lookup last_seen for placement-gap: {e}"))
                })?;

            let unhonored = match last_seen {
                None => true, // never observed
                Some(ts) => ts.as_str() < unhonored_before.as_str(),
            };

            if !unhonored {
                continue;
            }

            // Cooldown: don't re-emit if a recent placement-gap event for the
            // same commitment exists.
            let recent_gap = economic_events::table
                .filter(economic_events::action.eq("placement-gap"))
                .filter(economic_events::output_of.eq(&commitment.id))
                .filter(economic_events::has_point_in_time.gt(&cooldown_after))
                .count()
                .get_result::<i64>(conn)
                .map_err(|e| {
                    StorageError::Database(format!("count recent placement-gap: {e}"))
                })?;

            if recent_gap > 0 {
                continue;
            }

            // Emit placement-gap event.
            let event_id = uuid::Uuid::new_v4().to_string();
            let new_event = NewEconomicEventRow {
                id: event_id,
                h_app_id: commitment.h_app_id.clone(),
                action: "placement-gap".to_string(),
                provider: commitment.provider.clone(),
                receiver: commitment.receiver.clone(),
                resource_conforms_to: commitment.resource_conforms_to.clone(),
                resource_inventoried_as: Some(blob_hash.clone()),
                resource_classified_as_json: None,
                resource_quantity_value: None,
                resource_quantity_unit: None,
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_point_in_time: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                has_duration: None,
                input_of: None,
                output_of: Some(commitment.id.clone()),
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: None,
                state: "observed".to_string(),
                note: None,
                metadata_json: None,
                dht_anchor_hash: None,
                created_at: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                verified_at: None,
            };
            diesel::insert_into(economic_events::table)
                .values(&new_event)
                .execute(conn)
                .map_err(|e| {
                    StorageError::Database(format!("insert placement-gap event: {e}"))
                })?;
            outcome.placement_gaps_emitted += 1;
        }
    }

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::rea_commitments;
    use crate::db::models::NewReaCommitmentRow;
    use crate::db::peer_blob_inventory::apply_snapshot;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::sync::Mutex;

    fn test_pool() -> DbPool {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder().build(manager).expect("build pool");
        let mut conn = pool.get().expect("connection");
        run_migrations(&mut conn).expect("migrations");
        pool
    }

    struct StaticStore(Vec<String>);
    impl LocalBlobStore for StaticStore {
        fn has(&self, hash: &str) -> bool {
            self.0.iter().any(|h| h == hash)
        }
    }

    struct RecordingKicker {
        kicks: Mutex<Vec<(String, Vec<String>)>>,
    }
    impl FetchKicker for RecordingKicker {
        fn kick(&self, blob_hash: &str, candidates: Vec<String>) {
            self.kicks
                .lock()
                .unwrap()
                .push((blob_hash.to_string(), candidates));
        }
    }

    fn insert_custody_commitment(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        receiver: &str,
        blob_hash: &str,
    ) {
        let row = NewReaCommitmentRow {
            id: id.to_string(),
            h_app_id: "test".to_string(),
            action: "custody-blob".to_string(),
            provider: provider.to_string(),
            receiver: receiver.to_string(),
            resource_conforms_to: None,
            resource_classified_as: Some(blob_hash.to_string()),
            resource_quantity_value: Some(1024.0),
            resource_quantity_unit: Some("bytes".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state: "active".to_string(),
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: Some("hash1".to_string()),
            created_at: "2026-05-02T00:00:00Z".to_string(),
        };
        diesel::insert_into(rea_commitments::table)
            .values(&row)
            .execute(conn)
            .unwrap();
    }

    #[test]
    fn own_commitment_with_local_blob_no_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &"a".repeat(64));

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec!["a".repeat(64)]),
            &kicker,
            300,
            1800,
            600,
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 0);
        assert!(kicker.kicks.lock().unwrap().is_empty());
    }

    #[test]
    fn own_commitment_with_missing_blob_kicks_when_candidate_exists() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_hash);
        apply_snapshot(
            &mut conn,
            "peer_X",
            &[blob_hash.clone()],
            1,
            "2026-05-02T00:00:00Z",
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]), // local store empty
            &kicker,
            300,
            1800,
            600,
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks.len(), 1);
        assert_eq!(kicks[0].0, blob_hash);
        assert_eq!(kicks[0].1, vec!["peer_X".to_string()]);
    }

    #[test]
    fn own_commitment_with_no_candidates_no_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &"a".repeat(64));

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            300,
            1800,
            600,
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 0);
    }

    #[test]
    fn others_commitment_unhonored_emits_placement_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            300,
            1800,
            600,
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 1);
        assert_eq!(outcome.kicks_fired, 0);

        // Verify the event landed.
        use crate::db::diesel_schema::economic_events;
        let count: i64 = economic_events::table
            .filter(economic_events::action.eq("placement-gap"))
            .filter(economic_events::output_of.eq("c1"))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn others_commitment_with_fresh_inventory_no_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        // Fresh inventory entry from other_cid.
        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(&mut conn, "other_cid", &[blob_hash.clone()], 1, &when).unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            300,
            1800,
            600,
            now,
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 0);
    }

    #[test]
    fn placement_gap_cooldown_suppresses_repeat() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        // First pass: emit gap.
        let now = chrono::Utc::now();
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let _ = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            300,
            1800,
            600,
            now,
        )
        .unwrap();

        // Second pass within cooldown — should suppress.
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            300,
            1800,
            600,
            now + chrono::Duration::seconds(60),
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 0);
    }
}
```

- [ ] **Step 5: Register the module**

In `elohim/elohim-storage/src/reconcile/mod.rs`, add:

```rust
pub mod custody;
```

- [ ] **Step 6: Run targeted tests**

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib reconcile::custody --quiet
```

Expected: 6 tests pass.

- [ ] **Step 7: Run full lib pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --quiet 2>&1 | tail -3
```

Expected: lib count grows by 6 from T15's level (now ~1183).

- [ ] **Step 8: Clippy + fmt**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests -- -D warnings 2>&1 | tail -5
cargo fmt --check -p elohim-storage 2>&1 | head -5
```

Expected: both clean.

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-02-120000_placement_gap_action/ \
        elohim/elohim-storage/src/reconcile/custody.rs \
        elohim/elohim-storage/src/reconcile/mod.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/src/config.rs
git commit -m "feat(storage): T16 — custody reconciliation controller (multi-trigger; act on own / signal on others' via placement-gap)"
```

---

## Task T17: GET-time blob fallback + shared race-fetch helper

**Why this is here:** The user-visible recovery path. Both the HTTP blob handler and T16's controller-driven kicks call the same `race_fetch` helper. Per-peer timeout, racing N=3 candidates in parallel, hash verification, persists locally on first verified hit, emits `serve-blob` REA event.

**Files:**
- Create: `elohim/elohim-storage/src/p2p/blob_fetch.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — add `P2PCommand::FetchBlob { peer_id, hash, reply }` variant; register submodule
- Modify: `elohim/elohim-storage/src/http.rs` — modify `GET /blob/{hash}` route to call helper on local miss
- Modify: `elohim/elohim-storage/src/config.rs` — add `fetch_blob_timeout_seconds`, `fetch_blob_parallelism`

- [ ] **Step 1: Add config fields**

In `elohim/elohim-storage/src/config.rs`:

```rust
pub fetch_blob_timeout_seconds: u64,
pub fetch_blob_parallelism: usize,
```

In Default:

```rust
fetch_blob_timeout_seconds: 5,
fetch_blob_parallelism: 3,
```

Env-var parsing:

```rust
if let Ok(v) = std::env::var("FETCH_BLOB_TIMEOUT_SECONDS") {
    if let Ok(n) = v.parse::<u64>() {
        config.fetch_blob_timeout_seconds = n;
    }
}
if let Ok(v) = std::env::var("FETCH_BLOB_PARALLELISM") {
    if let Ok(n) = v.parse::<usize>() {
        if n > 0 {
            config.fetch_blob_parallelism = n;
        }
    }
}
```

- [ ] **Step 2: Add P2PCommand::FetchBlob variant**

In `elohim/elohim-storage/src/p2p/mod.rs`, add to the `P2PCommand` enum:

```rust
/// Fetch a blob from a specific peer. Used by the race-fetch helper.
FetchBlob {
    peer_id: libp2p::PeerId,
    hash: String,
    reply: tokio::sync::oneshot::Sender<Result<Vec<u8>, String>>,
},
```

In the dispatcher arm, route to the existing shard-fetch path (or add a new request-response handler if blob fetch differs from shard fetch). At Stage 1 it's acceptable to dispatch via the existing FetchShard handler if the wire shape is identical:

```rust
P2PCommand::FetchBlob { peer_id, hash, reply } => {
    // Stage 1: route through the existing shard-request infrastructure.
    // Stage 2 may add a dedicated blob-fetch protocol if shape diverges.
    let request = crate::p2p::shard_protocol::ShardRequest::Get { hash: hash.clone() };
    // ... send request, await response, forward to reply oneshot ...
    // Concrete implementation depends on existing FetchShard arm — copy that pattern,
    // changing only that this variant carries an explicit peer_id (not "first connected").
}
```

(Implementer: read the existing `P2PCommand::FetchShard` arm to see the exact pattern; replicate it but with an explicit-peer behavior.)

- [ ] **Step 3: Write the race-fetch helper module**

Create `elohim/elohim-storage/src/p2p/blob_fetch.rs`:

```rust
//! Shared blob-fetch helper. Used by both:
//! - HTTP blob handler (`GET /blob/{hash}`) on local miss
//! - Custody reconciliation controller (T16) for own-commitment kicks
//!
//! Strategy:
//! 1. Look up candidate peers in `peer_blob_inventory`, ordered by evidence
//!    strength (fetch-success first, then by recency).
//! 2. Filter to currently-connected peers.
//! 3. Race candidates in parallel batches of `fetch_blob_parallelism` (default 3).
//!    First reply that returns `Ok(bytes)` AND verifies the content hash wins.
//!    Pending replies in the batch are dropped; failed batch advances to the next.
//! 4. On verified success: persist locally, record fetch-success in
//!    `peer_blob_inventory`, emit `serve-blob` REA event.
//!
//! Hash verification: sha256-hex matches the requested hash.

use crate::config::Config;
use crate::db::peer_blob_inventory::{lookup_hosts, record_fetch_success};
use crate::error::StorageError;
use chrono::Utc;
use diesel::SqliteConnection;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Outcome of a race-fetch.
#[derive(Debug)]
pub enum FetchOutcome {
    /// Bytes fetched and verified.
    Hit { bytes: Vec<u8>, source_peer: String },
    /// All candidates exhausted; no peer served verified bytes.
    Miss,
    /// Inventory had no candidates to try.
    NoCandidates,
}

/// Race a fetch across the candidates known to host the blob.
/// Returns the verified bytes on first hit, or Miss/NoCandidates if no peer served.
///
/// `cmd_tx` is the swarm command channel; `is_connected` filters the candidate
/// list to peers actively connected.
pub async fn race_fetch(
    blob_hash: &str,
    candidates: Vec<String>, // pre-fetched from peer_blob_inventory
    cmd_tx: &mpsc::Sender<crate::p2p::P2PCommand>,
    is_connected: impl Fn(&str) -> bool,
    parallelism: usize,
    per_peer_timeout: Duration,
) -> FetchOutcome {
    let connected: Vec<String> = candidates
        .into_iter()
        .filter(|p| is_connected(p))
        .collect();

    if connected.is_empty() {
        return FetchOutcome::NoCandidates;
    }

    let mut iter = connected.into_iter();
    loop {
        let batch: Vec<String> = iter.by_ref().take(parallelism).collect();
        if batch.is_empty() {
            return FetchOutcome::Miss;
        }

        // Spawn a per-peer fetch task; collect oneshot replies.
        let mut handles = Vec::with_capacity(batch.len());
        for peer_id_str in batch {
            let Ok(peer_id) = peer_id_str.parse::<libp2p::PeerId>() else {
                continue;
            };
            let cmd_tx = cmd_tx.clone();
            let hash = blob_hash.to_string();
            let timeout = per_peer_timeout;
            let peer_label = peer_id_str.clone();
            handles.push(tokio::spawn(async move {
                let (reply_tx, reply_rx) = oneshot::channel();
                if cmd_tx
                    .send(crate::p2p::P2PCommand::FetchBlob {
                        peer_id,
                        hash: hash.clone(),
                        reply: reply_tx,
                    })
                    .await
                    .is_err()
                {
                    return (peer_label, Err("swarm channel closed".to_string()));
                }
                match tokio::time::timeout(timeout, reply_rx).await {
                    Ok(Ok(Ok(bytes))) => (peer_label, Ok(bytes)),
                    Ok(Ok(Err(e))) => (peer_label, Err(e)),
                    Ok(Err(_)) => (peer_label, Err("oneshot dropped".to_string())),
                    Err(_) => (peer_label, Err("timeout".to_string())),
                }
            }));
        }

        // Wait for first verified hit; return immediately.
        for handle in handles {
            if let Ok((peer, result)) = handle.await {
                if let Ok(bytes) = result {
                    if verify_blob_hash(&bytes, blob_hash) {
                        return FetchOutcome::Hit {
                            bytes,
                            source_peer: peer,
                        };
                    }
                }
            }
        }
        // All in batch failed (timeout, error, or hash mismatch); try next batch.
    }
}

/// Persist verified bytes to the local blob store, record fetch-success in
/// peer_blob_inventory, and emit a `serve-blob` REA event.
pub fn finalize_fetch_success(
    conn: &mut SqliteConnection,
    blob_hash: &str,
    source_peer: &str,
    bytes: &[u8],
    self_cid: &str,
    blob_store_persist: impl FnOnce(&[u8]) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    blob_store_persist(bytes)?;

    let now_iso = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    record_fetch_success(conn, source_peer, blob_hash, &now_iso)?;

    // Emit serve-blob REA event.
    use crate::db::diesel_schema::economic_events;
    use crate::db::models::NewEconomicEventRow;

    let event = NewEconomicEventRow {
        id: uuid::Uuid::new_v4().to_string(),
        h_app_id: "elohim".to_string(),
        action: "serve-blob".to_string(),
        provider: source_peer.to_string(),
        receiver: self_cid.to_string(),
        resource_conforms_to: None,
        resource_inventoried_as: Some(blob_hash.to_string()),
        resource_classified_as_json: None,
        resource_quantity_value: Some(bytes.len() as f32),
        resource_quantity_unit: Some("bytes".to_string()),
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_point_in_time: now_iso.clone(),
        has_duration: None,
        input_of: None,
        output_of: None, // The matching custody-blob commitment hash isn't always known at this layer.
        lamad_event_type: None,
        content_id: None,
        contributor_presence_id: None,
        path_id: None,
        triggered_by: None,
        state: "completed".to_string(),
        note: None,
        metadata_json: None,
        dht_anchor_hash: None,
        created_at: now_iso,
        verified_at: None,
    };
    diesel::insert_into(economic_events::table)
        .values(&event)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("insert serve-blob event: {e}")))
}

/// Verify the bytes' sha256 hex matches the requested hash.
pub fn verify_blob_hash(bytes: &[u8], expected_hex: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual_hex = hex::encode(hasher.finalize());
    actual_hex == expected_hex.to_lowercase()
}

/// Wire the helper's parameters from the runtime Config.
pub fn fetch_params_from_config(config: &Config) -> (usize, Duration) {
    (
        config.fetch_blob_parallelism,
        Duration::from_secs(config.fetch_blob_timeout_seconds),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known_blob() -> (Vec<u8>, String) {
        let bytes = b"hello world".to_vec();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hex = hex::encode(hasher.finalize());
        (bytes, hex)
    }

    #[test]
    fn verify_blob_hash_accepts_match() {
        let (bytes, hex) = known_blob();
        assert!(verify_blob_hash(&bytes, &hex));
    }

    #[test]
    fn verify_blob_hash_rejects_mismatch() {
        let (bytes, _) = known_blob();
        assert!(!verify_blob_hash(&bytes, &"a".repeat(64)));
    }

    #[test]
    fn verify_blob_hash_handles_uppercase_expected() {
        let (bytes, hex) = known_blob();
        let upper = hex.to_uppercase();
        assert!(verify_blob_hash(&bytes, &upper));
    }
}
```

- [ ] **Step 4: Register the module**

In `elohim/elohim-storage/src/p2p/mod.rs`:

```rust
pub mod blob_fetch;
```

Verify `sha2`, `hex`, and `uuid` crates are in `Cargo.toml`. If `uuid` is missing, add `uuid = { version = "1", features = ["v4"] }`. If `sha2` and `hex` are missing, add `sha2 = "0.10"` and `hex = "0.4"`. (Likely already present; check first with `grep` in the workspace `Cargo.toml`.)

- [ ] **Step 5: Modify HTTP blob handler to use the helper**

In `elohim/elohim-storage/src/http.rs`, find the `GET /blob/{hash}` handler (search for `fn get_blob` or `/blob/{hash}`). Locate the local-miss branch where it currently returns 404. Replace it with:

```rust
// Local miss — try peer fallback before returning 404.
let candidates = {
    let mut conn = state.db_pool.get()?;
    let stale_before = (chrono::Utc::now()
        - chrono::Duration::seconds(state.config.inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    crate::db::peer_blob_inventory::lookup_hosts(&mut conn, &hash, &stale_before)
        .map(|rows| rows.into_iter().map(|r| r.peer_id).collect::<Vec<_>>())
        .unwrap_or_default()
};

if candidates.is_empty() {
    return Ok(http_404_response(&hash));
}

let (parallelism, per_peer_timeout) =
    crate::p2p::blob_fetch::fetch_params_from_config(&state.config);
let p2p_node = state.p2p_node.clone();
let cmd_tx = p2p_node.cmd_tx.clone();
let is_connected = move |peer: &str| {
    peer.parse::<libp2p::PeerId>()
        .map(|pid| p2p_node.is_connected(&pid))
        .unwrap_or(false)
};

let outcome = crate::p2p::blob_fetch::race_fetch(
    &hash,
    candidates,
    &cmd_tx,
    is_connected,
    parallelism,
    per_peer_timeout,
)
.await;

match outcome {
    crate::p2p::blob_fetch::FetchOutcome::Hit { bytes, source_peer } => {
        // Persist + emit serve-blob.
        let bytes_for_response = bytes.clone();
        let mut conn = state.db_pool.get()?;
        let self_cid = state.config.self_cid.clone().unwrap_or_default();
        let _ = crate::p2p::blob_fetch::finalize_fetch_success(
            &mut conn,
            &hash,
            &source_peer,
            &bytes,
            &self_cid,
            |b| state.blob_store.put(&hash, b).map_err(|e| StorageError::Database(e.to_string())),
        );
        Ok(http_200_blob_response(bytes_for_response))
    }
    crate::p2p::blob_fetch::FetchOutcome::Miss
    | crate::p2p::blob_fetch::FetchOutcome::NoCandidates => Ok(http_404_response(&hash)),
}
```

The exact names `http_200_blob_response`, `http_404_response`, `state.blob_store.put`, and `state.config.self_cid` need to match what the existing handler uses. Adapt the variable names to the existing function. (Implementer: read the surrounding handler code for the response builder pattern and the blob-store persist pattern.)

If `state.config.self_cid: Option<String>` doesn't exist yet, this is the right place to add it — it's needed for `serve-blob` event provenance. Add to `Config`:

```rust
/// CID of this peer's steward (its agent's content-addressed identity).
/// Used as `receiver` field in serve-blob REA events emitted on successful fetch.
pub self_cid: Option<String>,
```

- [ ] **Step 6: Run targeted unit tests for the helper**

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib blob_fetch --quiet
```

Expected: 3 tests pass (the verify_blob_hash unit tests).

The race_fetch logic itself is async + needs a swarm-channel mock; the integration smoke test (next step) covers the broader flow.

- [ ] **Step 7: Run full lib pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --quiet 2>&1 | tail -3
```

Expected: lib count grows by 3 from T16's level (now ~1186).

- [ ] **Step 8: Clippy + fmt**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests -- -D warnings 2>&1 | tail -5
cargo fmt --check -p elohim-storage 2>&1 | head -5
```

Expected: both clean. If clippy flags ownership/lifetime issues with the closure in `race_fetch`, refactor minimally to satisfy.

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-storage/src/p2p/blob_fetch.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/src/config.rs
git commit -m "feat(storage): T17 — GET-time blob fallback with shared race-fetch helper (parallelism + hash verify + serve-blob emission)"
```

---

## Task T18: Filesystem parity sweep + diagnostic endpoint

**Why this is here:** Defends the failure mode `project_inventory_exchange_not_byte_replication` warns about: gossip can run cleanly while bytes never replicate. The parity sweep periodically checks the local peer's gossiped set against the actual filesystem set; mismatch raises an operational signal. The diagnostic endpoint exposes the current parity state for the topology UI's "operational health" surface.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/inventory_broadcaster.rs` — add parity-check function + last-parity-state cache
- Modify: `elohim/elohim-storage/src/http.rs` — add `GET /api/v1/diagnostics/inventory-parity` route

- [ ] **Step 1: Add parity-check function to broadcaster module**

Append to `elohim/elohim-storage/src/p2p/inventory_broadcaster.rs`:

```rust
/// Filesystem parity check: compare the gossiped set against the actual
/// set in the local store. Returns the symmetric difference for diagnosis.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ParityReport {
    /// Hashes the broadcaster gossiped but the local store doesn't have.
    pub gossiped_but_missing: Vec<String>,
    /// Hashes the local store has but the broadcaster didn't gossip.
    pub local_but_not_gossiped: Vec<String>,
    /// Total filesystem entries.
    pub filesystem_count: usize,
    /// Total gossiped entries.
    pub gossiped_count: usize,
    /// ISO 8601 — when the report was computed.
    pub checked_at: String,
}

/// Compute the parity report between a freshly-walked inventory and a
/// previously-gossiped set.
pub fn compute_parity<I: LocalInventory>(
    local_store: &I,
    last_gossiped: &[String],
    now_iso: &str,
) -> ParityReport {
    let local: std::collections::HashSet<String> =
        local_store.current_hashes().into_iter().collect();
    let gossiped: std::collections::HashSet<String> = last_gossiped.iter().cloned().collect();

    let gossiped_but_missing: Vec<String> = gossiped.difference(&local).cloned().collect();
    let local_but_not_gossiped: Vec<String> = local.difference(&gossiped).cloned().collect();

    ParityReport {
        gossiped_but_missing,
        local_but_not_gossiped,
        filesystem_count: local.len(),
        gossiped_count: gossiped.len(),
        checked_at: now_iso.to_string(),
    }
}

#[cfg(test)]
mod parity_tests {
    use super::*;

    struct StaticInv(Vec<String>);
    impl LocalInventory for StaticInv {
        fn current_hashes(&self) -> Vec<String> {
            self.0.clone()
        }
    }

    #[test]
    fn parity_clean_when_sets_match() {
        let inv = StaticInv(vec!["a".repeat(64), "b".repeat(64)]);
        let last_gossiped = vec!["a".repeat(64), "b".repeat(64)];
        let report = compute_parity(&inv, &last_gossiped, "2026-05-02T00:00:00Z");

        assert!(report.gossiped_but_missing.is_empty());
        assert!(report.local_but_not_gossiped.is_empty());
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 2);
    }

    #[test]
    fn parity_detects_gossiped_but_missing() {
        let inv = StaticInv(vec!["a".repeat(64)]);
        let last_gossiped = vec!["a".repeat(64), "b".repeat(64)];
        let report = compute_parity(&inv, &last_gossiped, "2026-05-02T00:00:00Z");

        assert_eq!(report.gossiped_but_missing.len(), 1);
        assert_eq!(report.gossiped_but_missing[0], "b".repeat(64));
    }

    #[test]
    fn parity_detects_local_but_not_gossiped() {
        let inv = StaticInv(vec!["a".repeat(64), "c".repeat(64)]);
        let last_gossiped = vec!["a".repeat(64)];
        let report = compute_parity(&inv, &last_gossiped, "2026-05-02T00:00:00Z");

        assert_eq!(report.local_but_not_gossiped.len(), 1);
        assert_eq!(report.local_but_not_gossiped[0], "c".repeat(64));
    }
}
```

- [ ] **Step 2: Add the diagnostic endpoint**

In `elohim/elohim-storage/src/http.rs`, find the routes assembly (search for `Router::new()`). Add a route handler:

```rust
async fn get_inventory_parity(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    use axum::Json;

    // For Stage 1: the broadcaster's last-known gossiped set is read from a
    // shared cache (which the broadcaster updates on each snapshot emit).
    // For now, query the local inventory and use it as both sides — Stage 1
    // simplification; Stage 2 will track the last-actually-gossiped set in
    // a P2PNode-side cache populated by the broadcaster.

    let local_hashes: Vec<String> = state.blob_store.list_hashes().unwrap_or_default();
    let last_gossiped: Vec<String> = state
        .p2p_node
        .last_gossiped_inventory()
        .unwrap_or_else(|| local_hashes.clone());

    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Adapter to use compute_parity with the existing blob_store.list_hashes.
    struct StoreAdapter(Vec<String>);
    impl crate::p2p::inventory_broadcaster::LocalInventory for StoreAdapter {
        fn current_hashes(&self) -> Vec<String> {
            self.0.clone()
        }
    }

    let report = crate::p2p::inventory_broadcaster::compute_parity(
        &StoreAdapter(local_hashes),
        &last_gossiped,
        &now_iso,
    );

    Json(report)
}
```

Add the route registration in the assembly:

```rust
.route("/api/v1/diagnostics/inventory-parity", axum::routing::get(get_inventory_parity))
```

If `state.blob_store.list_hashes()` and `state.p2p_node.last_gossiped_inventory()` don't exist yet:
- For `list_hashes`: implementer should add a `pub fn list_hashes(&self) -> Result<Vec<String>, ...>` to the existing `BlobStore` impl that walks the local pantry directory.
- For `last_gossiped_inventory`: implementer should add a `RwLock<Vec<String>>` field to `P2PNode` that the broadcaster updates on each `build_snapshot` call. Read accessor: `pub fn last_gossiped_inventory(&self) -> Option<Vec<String>> { self.last_gossiped_inventory.read().ok().map(|g| g.clone()) }`.

Both additions are small (~10 lines each); fold them in here.

- [ ] **Step 3: Run targeted tests**

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib parity --quiet
```

Expected: 3 tests pass.

- [ ] **Step 4: Run full lib pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --quiet 2>&1 | tail -3
```

Expected: lib count grows by 3 from T17's level (now ~1189).

- [ ] **Step 5: Clippy + fmt**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests -- -D warnings 2>&1 | tail -5
cargo fmt --check -p elohim-storage 2>&1 | head -5
```

Expected: both clean.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p/inventory_broadcaster.rs \
        elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): T18 — filesystem parity sweep + /api/v1/diagnostics/inventory-parity diagnostic"
```

---

## Final verification (after all 7 tasks land)

- [ ] Full lib + integration pass:

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --quiet 2>&1 | tail -3
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --tests --quiet 2>&1 | tail -3
```

Expected: lib ≥ 1189 passing; integration tests (including new `inventory_writer_smoke`) pass.

- [ ] Clippy + fmt clean:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --tests -- -D warnings 2>&1 | tail -5
cargo fmt --check -p elohim-storage 2>&1 | head -5
```

- [ ] Migration round-trip (verify down.sql actually reverts):

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib migration_roundtrip --quiet
```

If a roundtrip test doesn't already exist in the test harness, this verification can be deferred to Jenkins.

- [ ] Schema-contract pass:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract --quiet 2>&1 | tail -3
```

Expected: 80 passing (Phase 1 schemas; Phase 2 didn't add new view schemas).

- [ ] Codegen drift check:

```bash
cd /projects/elohim/.claude/worktrees/light-up-topology
git diff --stat -- 'elohim/sdk/storage-client-ts/src/generated/' 'app/elohim-app/src/app/generated/' 'app/elohim-library/projects/elohim-service/src/generated/'
```

Expected: empty (Phase 2 didn't touch view schemas).

## Multi-peer integration (deferred to Jenkins)

The following scenarios verify on a real multi-peer cluster. They are documented as Jenkins-pipeline test cases, NOT to be implemented in Eclipse Che.

- **Flow 1 — replica grows after peer connects.**
  Setup: peer J has a `custody-blob` commitment for blob X; peer M hosts blob X locally; both peers running.
  Steps: bring peer J online; assert that within `inventory_broadcast_seconds + custody_sweep_seconds`, peer J fetches blob X from peer M; assert `serve-blob` event lands; assert `peer_blob_inventory(peer_J, X)` shows source='fetch-success'.

- **Flow 2 — placement-gap surfaces when custodian goes offline.**
  Setup: peer J has a commitment receiving from peer C as custodian; peer C was online and gossiping.
  Steps: stop peer C; wait `placement_grace_seconds + cushion`; assert peer J emits a `placement-gap` event; assert distribution-summary view aggregator surfaces the unhonored count.

- **Flow 3 — GET-time fallback (the user-visible recovery path).**
  Setup: peer J doesn't have blob X; peer M hosts blob X; peer J has gossiped inventory listing peer M as host.
  Steps: GET blob X via doorway → peer J; assert response is the bytes; assert peer J persisted locally; assert `serve-blob` event lands.

## Summary

7 tasks land the trinity reconciliation pattern for blob custody. Each task lands one commit. Lib test count grows from 1154 (T11 baseline) to ≥ 1189 (35 new tests across T12-T18). No new DHT entry types. One new SQLite table (`peer_blob_inventory`) plus a small auxiliary cursor table. Three new operator presets at minimum (cadence, freshness, sweep). REA action vocabulary extended by `placement-gap` and `serve-blob`. The view aggregators (Phase 4 of the parent sprint) consume these tables and emit topology UI badges; with this Phase 2 substrate landed, the visibility surface becomes truthful.

## Test plan

- [ ] T12: peer_blob_inventory migration applies; round-trip works; CRUD ops pass; sequence semantics correct.
- [ ] T13: snapshot/delta wire types round-trip via MessagePack; structural-verify rejects malformed messages.
- [ ] T14: snapshot apply replaces per-peer set; delta apply respects sequence; gap-detect queues a snapshot request.
- [ ] T15: scheduler emits at archetype cadence; sequence is monotonic; cadence-resolution honors the 4-layer override pattern.
- [ ] T16: reconcile pass is idempotent; act-on-own kicks via FetchKicker; signal-on-others' emits placement-gap with cooldown; counts grow as expected.
- [ ] T17: HTTP blob handler returns local hit; on local miss races candidates with parallelism=3; verifies hash; persists locally; emits serve-blob; returns 404 on all-miss.
- [ ] T18: parity sweep detects mismatch in either direction; diagnostic endpoint serves a valid ParityReport JSON.

## Related

- [Blob Custody Reconciliation — Design](../specs/2026-05-02-blob-custody-reconciliation-design.md) — the design this plan implements
- [Light Up the Topology — Operational Visibility Sprint Design](../specs/2026-05-01-light-up-the-topology-design.md) — parent sprint; resumes after this Phase 2 lands
- T03d action conventions (in the parent sprint plan) — `custody-blob` / `project-blob` / `serve-blob` are the manifest layer this plan's reality + diff layers operate against
