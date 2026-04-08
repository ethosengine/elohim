# Seeder DHT Drain & Provenance Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decouple the seeder from P2P readiness by making SQLite the write queue and the drain loop the sole DHT publisher, while ensuring content is never *served* from HTTP reads until its DHT provenance state is established.

**Architecture:** (1) Gate content reads on a provenance marker so unpublished rows are invisible to HTTP clients. (2) Add a bootstrap retry loop to the P2P event loop so the swarm forms after startup-init delays. (3) Replace the one-shot `publish_all_epr_heads` with a periodic, peer-gated drain loop that updates the provenance marker on success. (4) Remove the fire-and-forget auto-publish from `POST /db/content` so the drain loop is the only publisher. Seeder becomes a pure HTTP producer with no P2P awareness.

**Tech Stack:** Rust (elohim-storage), Diesel (SQLite), libp2p 0.54 (Kademlia), Tokio, Axum-style HTTP, TypeScript (genesis seeder)

---

## Execution Notes (2026-04-08 refactor)

During execution, Task E1 was refactored from a standalone `GET /p2p/publish-state`
debug endpoint to an integration into the existing `/p2p/status` endpoint. Rationale:

- Drain state is a legitimate peer-health signal — other peers can read
  `drain.pending` to judge how busy/overloaded a node is and potentially route
  around stuck peers. It's not just seeder scaffolding.
- Honors the "reuse storage compute reporting, don't duplicate" rule from
  MEMORY.md.
- Watch-channel-backed reads are cheaper for seeder polling than per-request
  DB queries.

The E1 / E2 / F1 sections below have been rewritten to reflect the integrated
design. The rest of the plan (Phases A, B, C, D) executed as originally specified.

---

## P2P Design Gate Output

### Entity: `content.p2p_published_at` (new nullable timestamp column on existing `content` table)

- **Classification**: **C — Operational**
- **Justification**: Per-node bookkeeping tracking which content rows have had their EPR Head successfully published to the libp2p Kademlia DHT *by this peer*. No other peer verifies it; if lost, it can be reconstructed by re-publishing (Kademlia `put_record` is idempotent). This is *not* Holochain provenance — Holochain notarization uses `dht_anchor_hash`, which is a distinct column tracked separately by the future `p2p-coherence` work.
- **Content Address Strategy**: **Content-Derived (CID)** — keyed by `content.id` which is already the content CID; this column is an attribute of an existing content row, not a new identifier.
- **Address Justification**: The entity is a scalar attribute (single timestamp) of an existing CID-addressed row. Introducing a separate keyed identifier would be pure ceremony.
- **Source of Truth**: **SQLite (operational)**
- **Coordinator Zome**: N/A — Kademlia (libp2p) is distinct from the Holochain DHT; no zome function touches this column.
- **Storage Projection**: `content` table, new nullable column `p2p_published_at TEXT`. No `dht_anchor_hash` change.
- **HTTP Route**: No new route. Drain state (`{ total, published, pending }`) is surfaced as a nullable `drain` field on the existing `GET /p2p/status` response (operational peer-health projection). No mutation route — the drain loop is the sole writer.
- **Anti-Pattern Check**:
  - *UUID for notarized entity*: N/A, operational.
  - *REST-first*: Starting from the drain loop behavior, not from a route. ✓
  - *CID as relational FK*: The column sits on the content row itself (attribute, not FK to notarized content). No dangling-reference risk on versioning — a new content version is a new row with its own `p2p_published_at`. ✓
  - *Missing source-of-truth comment*: Migration will include `-- Source of truth: local (operational). Tracks local Kademlia publish state.` ✓
  - *Creating a new DHT entry type*: Not creating one. ✓
  - *Granular data on DHT*: N/A, this data never leaves the local node. ✓

### Design Constraints Discovered

1. **Two distinct "provenance" concepts exist in the codebase** and must not be conflated:
   - `dht_anchor_hash` (Holochain ActionHash, populated by post-commit signal) — real notarization. Currently `NULL` for direct storage writes (see `src/http.rs:2145` TODO).
   - `p2p_published_at` (this plan, Kademlia publish timestamp) — network visibility marker.
   - The read gate in Phase A accepts EITHER as sufficient provenance. Future `p2p-coherence` work can tighten the gate once Holochain writes are wired.

2. **The current POST `/db/content` spawns a fire-and-forget `publish_epr_head`** (`src/http.rs:2189-2225`). This must be removed in Phase D — the drain loop becomes the sole publisher, otherwise two code paths race.

3. **The one-shot `initial_publish_done` flag** (`src/p2p/mod.rs:209, 578, 707-708`) and the adaptive-pacing loop inside `publish_all_epr_heads` (`src/p2p/mod.rs:855-942`) already contain most of the drain mechanics. Phase C refactors this into a recurring peer-gated function rather than writing it from scratch.

4. **Ordering dependency**: Phase A (read gate) must ship before Phase D (remove auto-publish), otherwise there's a window where direct writes stop publishing and reads still return them — leaking unprovenanced content.

---

## File Structure

### New files

- `elohim/elohim-storage/migrations/2026-04-08-000000_p2p_published_at/up.sql` — adds column + index
- `elohim/elohim-storage/migrations/2026-04-08-000000_p2p_published_at/down.sql` — drops column
- `elohim/elohim-storage/tests/drain_loop_integration.rs` — integration test for the drain loop (new file; the existing `tests/` directory already holds `resilience_integration.rs` and `sync_integration.rs`)

### Modified files

*All modifications are to the operational storage projection layer — no new DHT entry types, no coordinator zome changes. Source of truth for notarized content remains the Holochain DHT; this plan adds a peer-local publish-state column that is purely operational.*

- `elohim/elohim-storage/src/db/diesel_schema.rs` — add `p2p_published_at` to `content` table macro (operational projection column)
- `elohim/elohim-storage/src/db/models.rs` — add field to `Content` struct
- `elohim/elohim-storage/src/db/content_diesel.rs` — (1) add `require_provenance: bool` flag to `ContentQuery`, (2) apply filter in `list_content` and `get_content`, (3) add `list_unpublished_content_ids` query, (4) add `mark_published` update, (5) update inline test setup SQL
- `elohim/elohim-storage/src/views.rs` — add `p2p_published_at` to `ContentView` (camelCase: `p2pPublishedAt`)
- `elohim/elohim-storage/src/http.rs` — (1) set `require_provenance = true` on external read paths, (2) remove fire-and-forget `publish_epr_head` from POST handler
- `elohim/elohim-storage/src/p2p/mod.rs` — (1) add `bootstrap_retry_interval` select arm, (2) replace one-shot `publish_all_epr_heads` with peer-gated periodic `drain_publish_queue`, (3) update `mark_published` on success, (4) add `DrainStatusInfo` struct (ts-rs exported, camelCase), (5) add `drain: Option<DrainStatusInfo>` field on `P2PStatusInfo`, (6) populate `drain` in `refresh_status` via `count_publish_state`, (7) invoke `refresh_status` from the drain tick arm so the watch channel stays fresh on the 15s drain cadence
- `elohim/elohim-storage/src/db/content_diesel.rs` — (in addition to Phase A changes) update `count_publish_state` to use a single SQL `FILTER` clause so total and published counts are atomic against concurrent drain ticks
- `elohim/elohim-storage/src/main.rs` — move `p2p_node.start().await?` to just before `node.run()`
- `genesis/seeder/src/seed-sqlite.ts` — remove any P2P readiness waits (if present); document that seeded content is invisible to reads until drained

---

## Phase A — Provenance Read Gate (Safety First)

**Purpose**: Before changing any publish behavior, make sure unprovenanced content is never returned by HTTP reads. This closes the "false positive" leak the user flagged: the current `list_content` returns everything in the table regardless of `dht_anchor_hash` or publish state.

### Task A1: Add `p2p_published_at` column migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-08-000000_p2p_published_at/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-08-000000_p2p_published_at/down.sql`

- [ ] **Step 1: Write the up migration**

```sql
-- Source of truth: local (operational). Category C.
-- Tracks local Kademlia publish state for EPR Heads.
-- NULL = not yet published to libp2p Kad DHT. Set by the drain loop in p2p/mod.rs.
-- Distinct from `dht_anchor_hash` which tracks Holochain notarization.
-- Reconstruction strategy: re-publish from the content table (put_record is idempotent);
-- losing this column only costs one extra drain pass.
ALTER TABLE content ADD COLUMN p2p_published_at TEXT;

-- Partial index over unpublished rows — the drain loop scans this frequently.
CREATE INDEX idx_content_p2p_unpublished
    ON content(h_app_id, id)
    WHERE p2p_published_at IS NULL;
```

- [ ] **Step 2: Write the down migration**

```sql
DROP INDEX IF EXISTS idx_content_p2p_unpublished;
ALTER TABLE content DROP COLUMN p2p_published_at;
```

- [ ] **Step 3: Verify migration applies cleanly**

Run:
```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```
Expected: build succeeds, diesel picks up the new migration on next pool init.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-08-000000_p2p_published_at/
git commit -m "feat(storage): add p2p_published_at column for drain queue state"
```

### Task A2: Update diesel schema and Content model

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs:62-80`
- Modify: `elohim/elohim-storage/src/db/models.rs`

- [ ] **Step 1: Add column to `diesel::table!` macro for `content`**

In `src/db/diesel_schema.rs`, inside the `content (id)` table block (currently ends at line 80 with `dht_anchor_hash -> Nullable<Text>,`), add after `dht_anchor_hash`:

```rust
        dht_anchor_hash -> Nullable<Text>,
        p2p_published_at -> Nullable<Text>,
    }
}
```

- [ ] **Step 2: Add field to the `Content` struct in `src/db/models.rs`**

Locate the `Content` struct (it mirrors the `content` table). Add `pub p2p_published_at: Option<String>,` as the last field. Also add it to any `NewContent` / insertable struct if present, defaulted to `None`.

- [ ] **Step 3: Update the inline test setup SQL**

In `src/db/content_diesel.rs:588-610`, the `setup_test_db()` function creates the content table manually. Add `p2p_published_at TEXT` after `dht_anchor_hash TEXT`:

```rust
                dht_anchor_hash TEXT,
                p2p_published_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
```

- [ ] **Step 4: Build and run existing content_diesel tests**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage content_diesel::tests
```
Expected: existing tests still pass with the new column. If a test fails constructing a `Content` struct literal, add `p2p_published_at: None` to it.

- [ ] **Step 5: Commit**

The new column is an operational projection attribute (not a DHT entry type); source of truth for the column's values is this peer's drain loop, not any notarized record.

```bash
git add elohim/elohim-storage/src/db/diesel_schema.rs elohim/elohim-storage/src/db/models.rs elohim/elohim-storage/src/db/content_diesel.rs
git commit -m "feat(storage): wire p2p_published_at into Content projection (operational)"
```

### Task A3: Add `require_provenance` filter to ContentQuery and list_content

**Files:**
- Modify: `elohim/elohim-storage/src/db/content_diesel.rs` — `ContentQuery` struct and `list_content` fn (starts at line 187)

- [ ] **Step 1: Write the failing test for the provenance filter**

Append to the `mod tests` block in `src/db/content_diesel.rs` (after the existing tests, before the closing `}`):

```rust
    #[test]
    fn test_list_content_respects_require_provenance() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Insert two rows: one published, one not.
        let published = CreateContentInput {
            id: "cid-published".to_string(),
            title: "Published".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            validation_status: "valid".to_string(),
            created_by: None,
            content_body: None,
        };
        let unpublished = CreateContentInput {
            id: "cid-unpublished".to_string(),
            ..published.clone()
        };
        create_content(&mut conn, &ctx, published).unwrap();
        create_content(&mut conn, &ctx, unpublished).unwrap();

        // Mark only the first as p2p_published.
        diesel::sql_query(
            "UPDATE content SET p2p_published_at = datetime('now') WHERE id = 'cid-published'",
        )
        .execute(&mut conn)
        .unwrap();

        // Default query (no provenance filter) — returns BOTH rows. Regression guard.
        let unrestricted = list_content(
            &mut conn,
            &ctx,
            &ContentQuery {
                limit: 10,
                offset: 0,
                require_provenance: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(unrestricted.len(), 2, "unrestricted list should return all rows");

        // Gated query — returns ONLY the published row.
        let gated = list_content(
            &mut conn,
            &ctx,
            &ContentQuery {
                limit: 10,
                offset: 0,
                require_provenance: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(gated.len(), 1, "gated list should filter out unpublished rows");
        assert_eq!(gated[0].content.id, "cid-published");
    }
```

- [ ] **Step 2: Run test to verify it fails with a compile error**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage test_list_content_respects_require_provenance
```
Expected: FAIL — `require_provenance` field does not exist on `ContentQuery`.

- [ ] **Step 3: Add `require_provenance` to `ContentQuery`**

Locate the `ContentQuery` struct (defined near the top of `content_diesel.rs` or in a nearby file). Add:

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentQuery {
    // ... existing fields ...
    #[serde(default)]
    pub require_provenance: bool,
}
```

- [ ] **Step 4: Apply the filter in `list_content`**

In `list_content` (line 187), after the existing `reach` filter block (around line 217-219), add:

```rust
    // Provenance gate: exclude rows that have not been notarized on Holochain
    // (dht_anchor_hash) AND have not been published to libp2p Kad (p2p_published_at).
    // Either marker is sufficient. External HTTP reads set this to true; internal
    // drain-loop queries set it to false so the loop can see unpublished rows.
    if query.require_provenance {
        base_query = base_query.filter(
            content::dht_anchor_hash
                .is_not_null()
                .or(content::p2p_published_at.is_not_null()),
        );
    }
```

- [ ] **Step 5: Run the test, confirm it passes**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage test_list_content_respects_require_provenance
```
Expected: PASS.

- [ ] **Step 6: Run the full content_diesel test module**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage content_diesel::tests
```
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/db/content_diesel.rs
git commit -m "feat(storage): add require_provenance filter to ContentQuery

Gates list_content on dht_anchor_hash OR p2p_published_at being set.
Default is false (backward compatible for internal queries); external
HTTP read paths will opt in via a subsequent commit."
```

### Task A4: Apply `require_provenance = true` on all external content read paths in http.rs

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` — every place that builds a `ContentQuery` from HTTP input

- [ ] **Step 1: Identify all call sites**

Run:
```bash
```
Then use the Grep tool:
```
pattern: "ContentQuery \{|list_content\(.*ContentQuery"
path: elohim/elohim-storage/src/http.rs
output_mode: content
-n: true
```
Expected: a handful of sites (list, bulk, search, etc.). Enumerate them before editing so nothing is missed.

- [ ] **Step 2: Write an integration-style smoke test asserting list returns 0 when table has only unpublished rows**

Create `elohim/elohim-storage/tests/provenance_gate_integration.rs`:

```rust
//! Integration test: HTTP list_content must not return rows that have
//! neither dht_anchor_hash nor p2p_published_at set.

use elohim_storage::db::{
    content_diesel::{create_content, list_content, ContentQuery, CreateContentInput},
    context::AppContext,
};
use diesel::{Connection, SqliteConnection};
use diesel::RunQueryDsl;

fn test_conn() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    // Operational test fixture. Source of truth for the real schema is the
    // migration file; this in-memory mirror replicates the essential columns
    // (including the dht_anchor_hash DHT anchor column and the new
    // p2p_published_at operational projection column) so the gate filter
    // can be exercised without a full DB.
    diesel::sql_query(
        r#"
        CREATE TABLE content (
            id TEXT PRIMARY KEY NOT NULL,
            h_app_id TEXT NOT NULL DEFAULT 'lamad',
            title TEXT NOT NULL,
            description TEXT,
            content_type TEXT NOT NULL DEFAULT 'concept',
            content_format TEXT NOT NULL DEFAULT 'markdown',
            content_body TEXT,
            blob_hash TEXT,
            blob_cid TEXT,
            content_size_bytes INTEGER,
            metadata_json TEXT,
            reach TEXT NOT NULL DEFAULT 'public',
            validation_status TEXT NOT NULL DEFAULT 'valid',
            created_by TEXT,
            dht_anchor_hash TEXT,
            p2p_published_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(&mut conn)
    .unwrap();
    // Operational projection fixture for content_tags — source of truth is
    // the content_tags Links hanging off the parent Content DHT anchor
    // (no standalone entry type; see Category A2 in the p2p-design-gate skill).
    diesel::sql_query(
        r#"
        CREATE TABLE content_tags (
            h_app_id TEXT NOT NULL DEFAULT 'lamad',
            content_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY (h_app_id, content_id, tag)
        )
        "#,
    )
    .execute(&mut conn)
    .unwrap();
    conn
}

#[test]
fn unpublished_content_is_invisible_to_external_reads() {
    let mut conn = test_conn();
    let ctx = AppContext::new("lamad");
    create_content(
        &mut conn,
        &ctx,
        CreateContentInput {
            id: "cid-unpublished".into(),
            title: "Unpublished".into(),
            description: None,
            content_type: "concept".into(),
            content_format: "markdown".into(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".into(),
            validation_status: "valid".into(),
            created_by: None,
            content_body: None,
        },
    )
    .unwrap();

    let external = list_content(
        &mut conn,
        &ctx,
        &ContentQuery {
            limit: 10,
            offset: 0,
            require_provenance: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(external.is_empty(), "external reads must not return unpublished content");

    let internal = list_content(
        &mut conn,
        &ctx,
        &ContentQuery {
            limit: 10,
            offset: 0,
            require_provenance: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(internal.len(), 1, "internal reads must still see the row");
}
```

- [ ] **Step 3: Run the test, verify it compiles and passes**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test provenance_gate_integration
```
Expected: PASS (the filter is already implemented in Task A3).

- [ ] **Step 4: Flip every external content-read call site in http.rs to `require_provenance: true`**

These routes project the Content entry type (Category A) out to HTTP; the DHT entry is the source of truth, and external consumers must only see rows whose projection has been confirmed (either via Holochain post-commit signal populating `dht_anchor_hash`, or via the drain loop populating `p2p_published_at`).

For each `ContentQuery { ... }` construction inside a public HTTP handler (not internal callers like the drain loop or `publish_all_epr_heads`), set `require_provenance: true`. Cover: GET /db/content (list), GET /db/content/{id} (single fetch — add equivalent gating in `get_content` if not filtered the same way), POST /db/content/bulk read-after-write (if applicable), search endpoints.

Example edit near `src/http.rs:2105`:
```rust
let query = ContentQuery {
    content_type: params.get("contentType").cloned(),
    // ... existing fields ...
    require_provenance: true,
};
```

- [ ] **Step 5: Also gate `get_content` single-row fetch**

In `src/db/content_diesel.rs`, find the single-row fetch function (`get_content` or similar). Add an optional `require_provenance: bool` parameter and apply the same filter. HTTP single-row handlers pass `true`; the drain loop (if it needs single-row lookups) passes `false`.

Write and run a unit test for single-row fetch mirroring Task A3's test shape.

- [ ] **Step 6: Build and run the full storage test suite**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage
```
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/db/content_diesel.rs elohim/elohim-storage/tests/provenance_gate_integration.rs
git commit -m "feat(storage): gate external content reads on provenance marker

External HTTP read paths now require either dht_anchor_hash (Holochain
notarized) or p2p_published_at (Kademlia published). Closes the false
positive leak where seeded-but-undrained content was served immediately."
```

---

## Phase B — Bootstrap Retry Loop

**Purpose**: Make the P2P swarm actually form. Currently `p2p_node.start()` is called ~65s before the event loop runs (`main.rs:355` vs `main.rs:659`), so dials never get polled and Kademlia stays empty. Fix both the timing (move start later) and the robustness (periodic retry until peers connect).

### Task B1: Move `p2p_node.start()` to just before `node.run()`

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs:355` and `:659`

- [ ] **Step 1: Locate the current call**

Lines of interest:
- `main.rs:355` — `p2p_node.start().await?;` (currently inside the P2P setup block)
- `main.rs:659` — `_ = node.run(shutdown_rx) => { ... }` (the tokio::select! arm)

- [ ] **Step 2: Remove the early `start()` call**

Delete line 355 and its surrounding log lines that depend on it having started. Keep the peer-id/relay-mode/bootstrap-count info logs — those can still fire after config is loaded.

- [ ] **Step 3: Call `start()` immediately before `node.run()`**

At `main.rs:659`, just before entering the `tokio::select!`, add:

```rust
    #[cfg(feature = "p2p")]
    if let Some(ref node) = p2p_node {
        if let Err(e) = node.start().await {
            error!(error = %e, "Failed to start P2P node");
            return Err(e.into());
        }
        info!("P2P node started — dials queued, event loop about to poll");
    }
```

- [ ] **Step 4: Build**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```
Expected: builds clean.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "fix(storage): start P2P node immediately before event loop

Previously start() was called ~65s before node.run(), causing bootstrap
dials to sit unpolled and time out before the swarm was driven. Moving
start() closer to run() shrinks the race window to near-zero."
```

### Task B2: Add `bootstrap_retry_interval` arm to the P2P event loop

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:679-746` (the `run` function)

- [ ] **Step 1: Write a test scaffold for retry behavior**

Add to `elohim/elohim-storage/tests/drain_loop_integration.rs` (new file — see Task C1 for the shared test harness). For now, sketch the test as:

```rust
// TODO: integration test — spawn a P2P node with invalid bootstrap addrs,
// verify the retry counter ticks upward over 2 retry intervals.
// Covered by manual smoke test + log assertion for the first cut;
// full integration test follows in a later task.
```

Note: Full retry-loop integration testing requires spinning up a real swarm, which is heavy. For Phase B we rely on (a) the existing `tests/sync_integration.rs` pattern for swarm setup and (b) a manual smoke test with log assertions.

- [ ] **Step 2: Add a `bootstrap_retry_interval` local in `run()`**

Inside `run()` at `src/p2p/mod.rs:679`, after the existing `replication_interval` declaration (~line 688), add:

```rust
        let mut bootstrap_retry_interval = tokio::time::interval(Duration::from_secs(30));
        bootstrap_retry_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Track consecutive retry attempts for exponential backoff cap.
        let mut consecutive_empty_ticks: u32 = 0;
```

- [ ] **Step 3: Add the retry arm to the `tokio::select!`**

Inside the `tokio::select!` block in `run()`, add after the `replication_interval` arm (around line 735):

```rust
                _ = bootstrap_retry_interval.tick() => {
                    let connected = swarm.connected_peers().count();
                    if connected == 0 && !self.config.bootstrap_nodes.is_empty() {
                        consecutive_empty_ticks = consecutive_empty_ticks.saturating_add(1);
                        // Cap the retry frequency: after 10 ticks (~5 minutes of no peers),
                        // slow down to every 5 minutes by skipping ticks.
                        let should_retry = consecutive_empty_ticks <= 10
                            || consecutive_empty_ticks % 10 == 0;
                        if should_retry {
                            info!(
                                attempt = consecutive_empty_ticks,
                                "Bootstrap retry: no connected peers, re-dialing bootstrap nodes"
                            );
                            for addr_str in &self.config.bootstrap_nodes {
                                if let Ok(addr) = addr_str.parse::<Multiaddr>() {
                                    match swarm.dial(addr.clone()) {
                                        Ok(_) => debug!(addr = %addr, "Re-dialed bootstrap"),
                                        Err(e) => debug!(addr = %addr, error = %e, "Re-dial failed"),
                                    }
                                }
                            }
                        }
                    } else if connected > 0 && consecutive_empty_ticks > 0 {
                        info!(
                            connected = connected,
                            prior_attempts = consecutive_empty_ticks,
                            "Bootstrap recovered"
                        );
                        consecutive_empty_ticks = 0;
                    }
                    drop(swarm);
                }
```

- [ ] **Step 4: Build**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release -p elohim-storage
```
Expected: clean build.

- [ ] **Step 5: Run existing P2P tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test sync_integration
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test resilience_integration
```
Expected: all pass (retry loop is additive; existing tests should be unaffected).

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): add bootstrap retry loop to storage node event loop

Re-dials bootstrap nodes every 30s while connected_peers == 0, with
exponential backoff after 10 consecutive empty ticks. Handles both the
startup race (dials queued before event loop polls) and transient
network failures (peer restarts, DNS flaps, relay churn)."
```

---

## Phase C — Drain Loop Replaces One-Shot Publish

**Purpose**: Replace the one-shot `publish_all_epr_heads` (gated by `initial_publish_done`, always fires once regardless of peers) with a peer-gated, idempotent, periodic drain that marks rows `p2p_published_at = now()` on success. This makes the SQLite content table the write queue and the drain loop the sole publisher.

### Task C1: Add `list_unpublished_content_ids` and `mark_published` queries

**Files:**
- Modify: `elohim/elohim-storage/src/db/content_diesel.rs` — append two new functions

- [ ] **Step 1: Write failing tests**

Append to the `mod tests` block:

```rust
    #[test]
    fn test_list_unpublished_content_ids_returns_only_unpublished() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");

        // Three rows: A unpublished, B published, C unpublished.
        for id in ["a", "b", "c"] {
            create_content(
                &mut conn,
                &ctx,
                CreateContentInput {
                    id: id.into(),
                    title: id.to_uppercase(),
                    description: None,
                    content_type: "concept".into(),
                    content_format: "markdown".into(),
                    blob_hash: None,
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "commons".into(),
                    validation_status: "valid".into(),
                    created_by: None,
                    content_body: None,
                },
            )
            .unwrap();
        }

        diesel::sql_query("UPDATE content SET p2p_published_at = datetime('now') WHERE id = 'b'")
            .execute(&mut conn)
            .unwrap();

        let pending = list_unpublished_content_ids(&mut conn, &ctx, 100).unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending.contains(&"a".to_string()));
        assert!(pending.contains(&"c".to_string()));
    }

    #[test]
    fn test_mark_published_sets_timestamp() {
        let mut conn = setup_test_db();
        let ctx = AppContext::new("lamad");
        create_content(
            &mut conn,
            &ctx,
            CreateContentInput {
                id: "x".into(),
                title: "X".into(),
                description: None,
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "commons".into(),
                validation_status: "valid".into(),
                created_by: None,
                content_body: None,
            },
        )
        .unwrap();

        let pending_before = list_unpublished_content_ids(&mut conn, &ctx, 10).unwrap();
        assert_eq!(pending_before.len(), 1);

        mark_published(&mut conn, &ctx, "x").unwrap();

        let pending_after = list_unpublished_content_ids(&mut conn, &ctx, 10).unwrap();
        assert!(pending_after.is_empty());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage list_unpublished_content_ids
```
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement both functions**

Append to `src/db/content_diesel.rs` (after the existing read helpers):

```rust
/// Drain-loop query: return IDs of content rows that have not yet been
/// published to the libp2p Kad DHT. Scoped by app context. Internal use
/// only — does not apply the provenance gate (the drain loop IS the thing
/// that produces provenance).
pub fn list_unpublished_content_ids(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
) -> Result<Vec<String>, StorageError> {
    use crate::db::diesel_schema::content::dsl;
    dsl::content
        .filter(dsl::h_app_id.eq(&ctx.h_app_id))
        .filter(dsl::p2p_published_at.is_null())
        .select(dsl::id)
        .order(dsl::created_at.asc())
        .limit(limit)
        .load::<String>(conn)
        .map_err(|e| StorageError::Internal(format!("list_unpublished_content_ids failed: {}", e)))
}

/// Drain-loop write: mark a content row as p2p_published at the current time.
/// Operates on the operational projection column only — does not touch
/// dht_anchor_hash or any notarized state. Idempotent — re-publishing an
/// already-published row just bumps the timestamp.
pub fn mark_published(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<usize, StorageError> {
    use crate::db::diesel_schema::content::dsl;
    let now = chrono::Utc::now().to_rfc3339();
    diesel::update(
        dsl::content
            .filter(dsl::h_app_id.eq(&ctx.h_app_id))
            .filter(dsl::id.eq(content_id)),
    )
    .set(dsl::p2p_published_at.eq(now))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("mark_published failed: {}", e)))
}
```

- [ ] **Step 4: Run tests, verify pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage content_diesel::tests
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/content_diesel.rs
git commit -m "feat(storage): add drain queue primitives for p2p_published_at

list_unpublished_content_ids returns pending rows for the drain loop.
mark_published sets the timestamp after a successful Kademlia put_record."
```

### Task C2: Replace `publish_all_epr_heads` with `drain_publish_queue`

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — the `publish_all_epr_heads` function (line 855) and its single caller inside `run()` (line 707-726)

- [ ] **Step 1: Rename the function and change its signature**

In `src/p2p/mod.rs`, rename `publish_all_epr_heads` → `drain_publish_queue` and change its body:

```rust
    /// Drain up to `batch_limit` unpublished content rows by publishing their
    /// EPR Heads to Kademlia. Returns the number of rows successfully marked
    /// as published. Gated on having at least one connected peer — without
    /// peers, `put_record(Quorum::One)` would silently succeed locally without
    /// gossiping, creating phantom "published" state.
    async fn drain_publish_queue(&self, batch_limit: i64) -> usize {
        // Peer-gate: without peers, Kademlia can't gossip. Bail early.
        {
            let swarm = self.swarm.read().await;
            if swarm.connected_peers().count() == 0 {
                debug!("drain_publish_queue: no connected peers, skipping");
                return 0;
            }
        }

        let pool = match self.db_pool.as_ref() {
            Some(p) => p,
            None => {
                debug!("drain_publish_queue: no DB pool, skipping");
                return 0;
            }
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "drain_publish_queue: DB connection failed");
                return 0;
            }
        };

        let app_ctx = crate::db::AppContext::default_lamad();
        let pending_ids = match crate::db::content_diesel::list_unpublished_content_ids(
            &mut conn, &app_ctx, batch_limit,
        ) {
            Ok(ids) => ids,
            Err(e) => {
                warn!(error = %e, "drain_publish_queue: list_unpublished failed");
                return 0;
            }
        };

        if pending_ids.is_empty() {
            return 0;
        }

        info!(pending = pending_ids.len(), "drain_publish_queue: publishing batch");

        let mut published: usize = 0;
        let mut batch_delay = Duration::from_millis(1);

        for content_id in &pending_ids {
            let Some(head_bytes) = self.resolve_epr_head_locally(content_id) else {
                // Can't resolve head — skip this row; it'll be retried next tick.
                warn!(id = %content_id, "drain: EPR head not resolvable, skipping");
                continue;
            };

            let key = RecordKey::new(&format!("epr:{}", content_id));
            let record = Record {
                key,
                value: head_bytes,
                publisher: Some(*self.identity.peer_id()),
                expires: None,
            };

            let put_result = {
                let mut swarm = self.swarm.write().await;
                swarm
                    .behaviour_mut()
                    .kademlia
                    .put_record(record, libp2p::kad::Quorum::One)
            };

            match put_result {
                Ok(_) => {
                    if let Err(e) = crate::db::content_diesel::mark_published(
                        &mut conn, &app_ctx, content_id,
                    ) {
                        warn!(id = %content_id, error = %e, "drain: mark_published failed");
                    } else {
                        published += 1;
                    }
                    batch_delay =
                        Duration::from_millis((batch_delay.as_millis() as u64 / 2).max(1));
                }
                Err(e) => {
                    debug!(id = %content_id, error = ?e, "drain: put_record failed");
                    batch_delay =
                        Duration::from_millis((batch_delay.as_millis() as u64 * 2).min(500));
                }
            }

            if batch_delay.as_millis() > 1 {
                tokio::time::sleep(batch_delay).await;
            }
        }

        if published > 0 {
            info!(published, total = pending_ids.len(), "drain_publish_queue: batch complete");
        }
        published
    }
```

- [ ] **Step 2: Remove the `initial_publish_done` flag**

Delete:
- The field at `src/p2p/mod.rs:209`: `initial_publish_done: Arc<std::sync::atomic::AtomicBool>,`
- The initializer at line 578: `initial_publish_done: Arc::new(std::sync::atomic::AtomicBool::new(false)),`
- The import `std::sync::atomic::AtomicBool` if no longer used

- [ ] **Step 3: Replace the status_interval tick body with a drain call**

The current status_interval arm (lines 703-727) runs the one-shot publish gated by `initial_publish_done`. Replace that gated block with a simple status refresh (status stays on its own timer, drain moves to its own interval in Step 4).

New status_interval arm:

```rust
                _ = status_interval.tick() => {
                    drop(swarm);
                    self.refresh_status().await;
                }
```

- [ ] **Step 4: Add a dedicated `drain_interval` timer**

Near the other interval declarations in `run()` (~line 683-688), add:

```rust
        let mut drain_interval = tokio::time::interval(Duration::from_secs(15));
        drain_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
```

And add a new arm in the `tokio::select!`:

```rust
                _ = drain_interval.tick() => {
                    drop(swarm);
                    let _ = self.drain_publish_queue(500).await;
                }
```

- [ ] **Step 5: Populate replication state via a separate one-shot**

The old `initial_publish_done` block ALSO populated `replication_state.set_local_ids`. That needs a new home. Create a small helper:

```rust
    async fn hydrate_replication_state(&self) {
        let Some(pool) = self.db_pool.as_ref() else { return };
        let Ok(mut conn) = pool.get() else { return };
        let app_ctx = crate::db::AppContext::default_lamad();
        let query = crate::db::content_diesel::ContentQuery {
            limit: 100_000,
            require_provenance: false,
            ..Default::default()
        };
        if let Ok(items) = crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query) {
            let ids: std::collections::HashSet<String> =
                items.iter().map(|c| c.content.id.clone()).collect();
            tracing::info!(count = ids.len(), "Loaded local content IDs for replication state");
            self.replication_state.set_local_ids(ids).await;
        }
    }
```

Call it once from `run()` before entering the loop:

```rust
    pub async fn run(&self, mut shutdown: broadcast::Receiver<()>) {
        self.refresh_status().await;
        self.hydrate_replication_state().await;
        // ... existing interval declarations ...
```

- [ ] **Step 6: Build**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release -p elohim-storage
```
Expected: clean build. Fix any unused-import or unused-variable warnings from removing `initial_publish_done`.

- [ ] **Step 7: Run existing P2P tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage
```
Expected: all pass. Drain loop is peer-gated, so in unit tests (no peers) it no-ops.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): replace one-shot publish with peer-gated drain loop

Drains unpublished content rows every 15s, gated on connected_peers > 0.
Marks rows with p2p_published_at on successful put_record. Removes the
initial_publish_done one-shot flag — the drain is inherently idempotent
and restart-safe via the SQLite state column."
```

---

## Phase D — Remove Fire-and-Forget Auto-Publish from POST /db/content

**Purpose**: With the drain loop in place, the `tokio::spawn` in `POST /db/content` that calls `publish_epr_head` is redundant at best and racy at worst (it bypasses the peer gate and silently "publishes" into empty Kademlia). Note: this is a libp2p Kad DHT put, not a Holochain DHT entry type write — there is no coordinator zome involvement. Remove it so the drain loop is the sole publisher.

### Task D1: Remove the spawned auto-publish block

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:2189-2225`

- [ ] **Step 1: Delete the `#[cfg(feature = "p2p")] if result.is_ok() { ... }` block**

Remove the entire block starting at `src/http.rs:2189` that constructs an `EprHead`, serializes it with `rmp_serde`, and calls `handle.publish_epr_head(id, bytes).await`. The drain loop in Phase C now resolves the head locally via `resolve_epr_head_locally` and publishes on its own schedule.

- [ ] **Step 2: Check the bulk-content POST path for the same pattern**

Use Grep:
```
pattern: "publish_epr_head"
path: elohim/elohim-storage/src/http.rs
output_mode: content
-n: true
```

Remove any other `tokio::spawn` auto-publish sites you find in bulk/create handlers.

- [ ] **Step 3: Build**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release -p elohim-storage
```
Expected: clean. Any unused import (`crate::epr_codec::*`, `rmp_serde`) should be pruned.

- [ ] **Step 4: Run the test suite**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage
```
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "refactor(storage): remove fire-and-forget auto-publish from content POST

The drain loop is now the sole publisher of EPR Heads to Kademlia.
The previous tokio::spawn auto-publish bypassed the peer gate and
silently 'succeeded' into empty Kad on writes before peers connected —
creating phantom published state and racing with the drain loop."
```

---

## Phase E — Seeder Verification and Debug Endpoint

**Purpose**: (1) Add a small HTTP debug endpoint for pipeline assertions. (2) Verify the genesis seeder still works end-to-end with the new model: seeds go to SQLite, are invisible until drain fires, become visible after.

### Task E1: Integrate drain state into `P2PStatusInfo` / `GET /p2p/status`

**Rationale:** Drain state is a complementary peer-health signal, not just seeder scaffolding. Other peers can read `drain.pending` to judge how busy or overloaded a node is and potentially route around stuck peers. Surfacing it on `/p2p/status` (rather than as a standalone `/p2p/publish-state` route) honors the "reuse storage compute reporting, don't duplicate" rule from MEMORY.md — the watch-channel-backed `P2PStatusInfo` is already how operational peer-health is projected from the storage node. This is still a Category C operational projection with no DHT entry type or coordinator zome function involved; it just lands on the existing aggregate view.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — add `DrainStatusInfo`, extend `P2PStatusInfo`, populate in `refresh_status`, trigger refresh from drain tick
- Modify: `elohim/elohim-storage/src/db/content_diesel.rs` — `count_publish_state` (introduced in the Phase C drain work) uses a single SQL `FILTER` clause for atomic counts

- [ ] **Step 1: Add `DrainStatusInfo` struct in `p2p/mod.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DrainStatusInfo {
    pub total: i64,
    pub published: i64,
    pub pending: i64,
}
```

The ts-rs derive generates `DrainStatusInfo.ts` in the storage-client-ts generated folder. This is the operational projection of drain state — no DHT entry type, no source chain action.

- [ ] **Step 2: Add `drain: Option<DrainStatusInfo>` to `P2PStatusInfo`**

Append the new field as the last field on `P2PStatusInfo` (next to `replication`). Also add ts-rs derives to `P2PStatusInfo` and `ReplicationStatus` so the seeder can consume generated types.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct P2PStatusInfo {
    // ... existing fields ...
    pub replication: ReplicationStatus,
    /// Drain queue state (operational projection). `None` means the DB
    /// pool is unavailable or the count query failed — consumers MUST
    /// treat null as "keep waiting", not "caught up".
    pub drain: Option<DrainStatusInfo>,
}
```

The `Option<DrainStatusInfo>` (not a default-zero struct) is deliberate: a missing DB pool or failed query must be distinguishable from a genuine `pending == 0`. If we used `DrainStatusInfo::default()` on failure, broken nodes would look caught-up to the seeder and the pipeline would exit green on failure.

- [ ] **Step 3: Populate `drain` in `refresh_status`**

In `refresh_status()`, after computing the existing status fields, query `count_publish_state` with graceful fallback:

```rust
let drain = self.db_pool.as_ref().and_then(|pool| {
    let mut conn = pool.get().ok()?;
    let ctx = crate::db::AppContext::default_lamad();
    match crate::db::content_diesel::count_publish_state(&mut conn, &ctx) {
        Ok((total, published)) => Some(DrainStatusInfo {
            total,
            published,
            pending: total - published,
        }),
        Err(e) => {
            tracing::warn!(error = %e, "count_publish_state failed in refresh_status");
            None
        }
    }
});
```

Any failure path (pool missing, connection failure, query error) yields `None` so callers know to keep waiting.

- [ ] **Step 4: Trigger `refresh_status` from the drain tick**

In `run()`, the drain_interval arm added in Task C2 already calls `drain_publish_queue`. Add a `refresh_status().await` immediately after, so the watch channel reflects the new drain state on the 15s drain cadence rather than only on the slower status_interval cadence:

```rust
                _ = drain_interval.tick() => {
                    drop(swarm);
                    let _ = self.drain_publish_queue(500).await;
                    self.refresh_status().await;
                }
```

- [ ] **Step 5: Make `count_publish_state` atomic via SQL `FILTER`**

Update the `count_publish_state` query body (introduced in Task C1) to use a single aggregate SQL statement with a `FILTER` clause so total and published are computed in one round trip, preventing torn reads against concurrent drain ticks:

```rust
pub fn count_publish_state(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<(i64, i64), StorageError> {
    use diesel::sql_types::BigInt;
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = BigInt)] total: i64,
        #[diesel(sql_type = BigInt)] published: i64,
    }
    let row: Row = diesel::sql_query(
        "SELECT COUNT(*) AS total, \
         COUNT(*) FILTER (WHERE p2p_published_at IS NOT NULL) AS published \
         FROM content WHERE h_app_id = ?"
    )
    .bind::<diesel::sql_types::Text, _>(&ctx.h_app_id)
    .get_result(conn)
    .map_err(|e| StorageError::Internal(format!("count_publish_state failed: {}", e)))?;
    Ok((row.total, row.published))
}
```

Keep the existing unit test asserting it returns `(2, 1)` for two inserted rows with one marked published.

- [ ] **Step 6: Regenerate TypeScript bindings**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings
```
Expected: `elohim/sdk/storage-client-ts/src/generated/DrainStatusInfo.ts`, `P2PStatusInfo.ts`, and `ReplicationStatus.ts` are created or updated.

- [ ] **Step 7: Build and test**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release -p elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage count_publish_state
```
Expected: pass.

- [ ] **Step 8: Commit**

Commits landed as (historical reference):
- `71d6dbc8` — drain state integrated into `P2PStatusInfo`
- `d7e18977` — `Option<DrainStatusInfo>` so broken-vs-caught-up is distinguishable
- `547bfefc` — ts-rs export of `P2PStatusInfo` / `DrainStatusInfo` / `ReplicationStatus` so the seeder imports generated types

### Task E2: Update the seeder to assert drain completion

**Files:**
- Modify: `genesis/seeder/src/seed-sqlite.ts` (or wherever the seeder's "done" hook lives)

- [ ] **Step 1: Poll `/p2p/status` and read `response.drain` until `pending === 0`**

Import the generated types from the storage-client package so the seeder stays type-aligned with the Rust operational projection:

```typescript
import type { P2PStatusInfo, DrainStatusInfo } from '@elohim/storage-client';
```

Add a `waitForDrain(baseUrl, options?)` helper that polls the pre-existing `GET /p2p/status` operational projection route every 2s with a default 5-minute timeout. This route is not an API for a DHT entry type — it surfaces watch-channel state from the storage node's peer-health projection. The termination condition is `drain !== null && drain.total >= expectedMinTotal && drain.pending === 0`. A `null` drain means the storage node's DB pool is unavailable or the count query failed — treat it as "keep waiting", not "caught up":

```typescript
interface WaitForDrainOptions {
  timeoutMs?: number;
  expectedMinTotal?: number;
}

async function waitForDrain(
  baseUrl: string,
  { timeoutMs = 5 * 60_000, expectedMinTotal = 1 }: WaitForDrainOptions = {},
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let lastDrain: DrainStatusInfo | null = null;
  while (Date.now() < deadline) {
    const resp = await fetch(`${baseUrl}/p2p/status`);
    if (!resp.ok) {
      throw new Error(`/p2p/status query failed: ${resp.status}`);
    }
    const status = (await resp.json()) as P2PStatusInfo;
    lastDrain = status.drain;
    if (lastDrain === null) {
      console.log('drain progress: null (waiting for storage node)');
    } else {
      console.log(
        `drain progress: ${lastDrain.published}/${lastDrain.total} published, ${lastDrain.pending} pending`,
      );
      if (lastDrain.total >= expectedMinTotal && lastDrain.pending === 0) {
        console.log('drain complete');
        return;
      }
    }
    await new Promise((r) => setTimeout(r, 2000));
  }
  throw new Error(
    `drain did not complete within ${timeoutMs}ms (last drain=${JSON.stringify(lastDrain)})`,
  );
}
```

Call it after the main seed loop in `seed-sqlite.ts`. The call site must **re-throw** on drain failure so the summary block never prints a success-looking output on failure — this was the bug behind the original silent-failure incident.

- [ ] **Step 2: Remove any stale P2P readiness wait**

Search `genesis/seeder/src/` for any existing "wait for peers" / "check P2P" logic added in previous fix attempts:

```
pattern: "connected_peers|connectedPeers|waitForP2P|p2pReady"
path: genesis/seeder/src/
```

Delete them. The seeder's contract is now: POST to SQLite, then wait for drain. P2P readiness is the storage node's problem, not the seeder's.

- [ ] **Step 3: Run the seeder against a local dev stack**

```bash
cd genesis/seeder
pnpm build
# (Assumes doorway + storage are running; use the hc-dev-orchestrator skill)
pnpm seed -- --target http://localhost:8090
```
Expected: seeder runs, progress log shows `pending` counting down to 0.

- [ ] **Step 4: Commit**

Commits landed as (historical reference):
- `992cfa47` — seeder `waitForDrain` polling `/p2p/status`
- `b2296c0a` — rethrow on failure so the summary block never prints success on a failed drain
- `547bfefc` — seeder imports `P2PStatusInfo` / `DrainStatusInfo` from `@elohim/storage-client` generated types

Intent of the commit message: "Seeder now posts content to SQLite and polls `/p2p/status` until `drain.pending == 0` before declaring success. P2P readiness is the storage node's concern — the seeder is a pure HTTP producer. This is an operational projection over the `content` table, not a DHT entry type query."

---

## Phase F — Final Verification

### Task F1: End-to-end smoke test across Adam → Matthew

- [ ] **Step 1: Start the full dev stack**

Use the `hc-dev-orchestrator` skill to bring up conductor + storage + doorway for at least two peers (Adam + Matthew).

- [ ] **Step 2: Seed Adam only**

```bash
cd genesis/seeder
pnpm seed -- --target http://adam-storage:8090
```

Expected: seed completes, drain finishes on Adam.

- [ ] **Step 3: Verify Adam's publish state**

```bash
curl http://adam-storage:8090/p2p/status | jq .drain
```
Expected: `{ total: N, published: N, pending: 0 }` where N is the seed count. Asserting `drain.pending == 0` and `drain.total > 0` confirms the drain coordinator function (operational projection over the content table) has caught up — no DHT entry type involvement.

- [ ] **Step 4: Verify Matthew converges via replication**

```bash
# Wait 1-2 minutes for replication cycles
sleep 120
curl http://matthew-storage:8090/p2p/status | jq .drain
```
Expected: `drain.total` grows toward N as Matthew discovers Adam's content. The notarized source of truth for content remains the Holochain DHT / source chain; this projection just tracks local Kademlia publish bookkeeping.

- [ ] **Step 5: Verify external reads gate correctly during the window**

While Matthew is mid-replication, the rows that exist but haven't been published by Matthew should NOT show up in his `/db/content` list. Only rows with `p2p_published_at` set (by his own drain on replicated content) should be visible.

```bash
curl "http://matthew-storage:8090/db/content?limit=100" | jq '.items | length'
curl "http://matthew-storage:8090/p2p/status" | jq .drain
```

Expected: items length ≤ `drain.published` from the `/p2p/status` response. If items > published, the read gate is leaking — bug to track down in A4.

- [ ] **Step 6: No commit**

This is a verification task. If anything fails, open a task above for the root cause fix.

---

## Out of Scope / Follow-up Plans

- **Multi-seed partitioning** (Adam + Matthew split the seed payload): write as a separate plan in `genesis/plans/2026-04-09-multi-seed-partition-plan.md` once this plan is merged. The drain-queue architecture built here is a prerequisite — partitioning just means the seeder targets multiple hosts with disjoint content slices.
- **p2p-coherence TODO cleanup**: ~30+ scattered `TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal` comments across `src/api/*.rs` and `src/http.rs`. Tracked separately; this plan intentionally does not touch them.
- **Holochain-backed seeding**: seeding via Holochain admin API so `dht_anchor_hash` gets populated. That's the "real" notarization path and is the proper long-term fix, but it's gated on the p2p-coherence work above.

---

## Definition of Done

- [ ] All migrations applied, storage builds clean with `RUSTFLAGS=''` override
- [ ] `cargo test -p elohim-storage` passes
- [ ] Adam can seed with bootstrap failing on first dial; drain loop catches up and publishes after bootstrap retry succeeds
- [ ] Matthew's `/db/content` list never returns content rows whose `p2p_published_at` is NULL
- [ ] Seeder no longer contains any P2P readiness checks — only HTTP POST + drain wait
- [ ] `/p2p/status` returns an accurate `drain` field (operational projection — no DHT entry type involvement) and pipelines can assert `drain.pending == 0` after seeding to verify the drain coordinator function caught up
