# EPR Phase 4 — Projector Controller + Topology Last-Mile Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve every `TODO(Phase 4 follow-up)` site across `services/{distribution,reciprocity,peer_topology}_view.rs` + `main.rs:1129` so the topology UI sprint's last-mile becomes pure rendering against real substrate data — no stubs, no glue debt.

**Architecture:** Reuse REA `economic_events` (Category A — DHT-notarized) with new `action='ack-projection'` for projector acks; project to a `projection_events` operational log (Category C). Backfill `device_archetype` + `superseded_by` columns on the existing `peer_identity_bindings` projection (A2 fields the projection currently drops). Add four thin helper modules (`imagodei_lookup`, `connectivity`, `device_capacity`, `peer_diversity`) so view services consume one-liners. Wire main.rs manifest registry layer-1.

**Tech Stack:** Rust (elohim-storage, holochain DNAs), Diesel migrations, libp2p 0.54 connected-peers snapshot, ts-rs codegen, JSON Schema (schema-first), Cucumber (a2o features).

**Spec / parent master:** `genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md` — Wave 1 sub-plan; topology last-mile bridge table at master plan §"Topology last-mile bridge."

**Worktree:** Run in a dedicated worktree per `superpowers:using-git-worktrees`. Branch from origin/dev. Single worktree for the whole plan; ff-merge to dev when done.

**Build flag:** `elohim-storage` requires `RUSTFLAGS='--cfg getrandom_backend="custom"'` per CLAUDE.md gotcha. Rust DNA workspaces use plain `cargo` (no flag override).

---

## P2P Design Gate Output

| Entity | Category | Source of Truth | Justification |
|---|---|---|---|
| `EconomicEvent` with `action='ack-projection'` | A — Notarized (REUSE existing entry type) | Holochain DHT (content_store zome) | An ack is a concrete REA event ("projector PROVIDED projection-service for this content"). EconomicEvent already carries provider/receiver/resource_classified_as. No new entry type — uses Lamad DNA capacity (~73/~100) without consuming any |
| `projection_events` (SQLite) | C — Operational | DHT via `rea_projection` signal stream | Append-only log derived from `economic_events WHERE action='ack-projection'`. Rebuildable from any peer's content_store storage projection |
| `device_archetype` column (added to `peer_identity_bindings`) | C — Operational backfill | DHT (AgentPeerBinding A2 link metadata, imagodei zome) | Field already exists on DHT entry; projection drops it today. ALTER TABLE + signal-stream backfill |
| `superseded_by` column (added to `peer_identity_bindings`) | C — Operational backfill | DHT (AgentPeerBinding A2 link metadata) | Same shape as device_archetype |
| `imagodei_lookup::resolve_display_name` | helper API (no entity) | Reads existing `humans` table (Category A projection of Human DHT entry) | Pure function over existing data |
| `connectivity::is_online` | helper API (no entity) | Ephemeral — libp2p `connected_peers()` snapshot at call time | Operational ephemeral; no persistence |
| `device_capacity::available_bytes_for` | helper API (no entity) | Reads existing `system_metrics` + `rea_commitments` (custody-blob outflow) | Pure aggregation function |
| `peer_diversity::diversity_hint_for` | helper API (no entity) | Reads `peer_identity_bindings.device_archetype` (after T1) + future geo column | Pure function |
| Manifest registry layer-1 (main.rs:1129) | C — Operational | Pillar manifest JSON files on disk | Operator-controlled config; no DHT presence |

**Anti-pattern check:** ✓ No new entry types created. ✓ No CID-as-FK (acks reference blob_hash, which is a hash, not a CID-of-content-entry). ✓ Source of truth declared inline at top of every new module file (per `2026-05-01-light-up-the-topology-design.md` source-of-truth declaration requirement). ✓ HTTP routes designed last — Phase 4 introduces no new HTTP routes; the existing distribution / reciprocity / peer-topology routes simply return non-stub data.

---

## File Structure

### New files
| Path | Responsibility |
|------|----------------|
| `elohim/elohim-storage/src/db/projection_events.rs` | Diesel model + writer for `projection_events` |
| `elohim/elohim-storage/migrations/2026-05-11-110000_projection_events/up.sql` | CREATE TABLE projection_events |
| `elohim/elohim-storage/migrations/2026-05-11-110000_projection_events/down.sql` | DROP TABLE projection_events |
| `elohim/elohim-storage/migrations/2026-05-11-110001_peer_bindings_archetype_superseded/up.sql` | ALTER TABLE peer_identity_bindings ADD COLUMN device_archetype TEXT NOT NULL DEFAULT 'node', ADD COLUMN superseded_by TEXT |
| `elohim/elohim-storage/migrations/2026-05-11-110001_peer_bindings_archetype_superseded/down.sql` | (no-op for SQLite — column drops require table recreate) |
| `elohim/elohim-storage/src/services/imagodei_lookup.rs` | `resolve_display_name(agent_cid: &str) -> Option<String>` over `humans` table |
| `elohim/elohim-storage/src/services/connectivity.rs` | `is_online(peer_id: &str, snapshot: &HashSet<String>) -> bool` from libp2p connected_peers snapshot |
| `elohim/elohim-storage/src/services/device_capacity.rs` | `available_bytes_for(human_id: &str) -> u64` = device totals minus committed |
| `elohim/elohim-storage/src/services/peer_diversity.rs` | `diversity_hint_for(replicas: &[ReplicaPeer]) -> DiversityHint` from archetype mix |
| `elohim/elohim-storage/src/p2p/projection_ack_handler.rs` | rea_projection signal handler that ingests ack-projection events into projection_events |
| `elohim/holochain/dna/elohim/zomes/content_store/src/projection_ack.rs` | Coordinator function `ack_projection(blob_hash: String)` that emits the ack-projection EconomicEvent |
| `elohim/elohim-storage/tests/phase4_projector_topology_integration.rs` | End-to-end: doorway acks projection → projection_events row appears → distribution_view returns non-stub `projector_count` + `projector_identities` |

### Modified files
| Path | What changes |
|------|--------------|
| `elohim/elohim-storage/src/db/peer_identity_bindings.rs` | Add `device_archetype` + `superseded_by` to `PeerIdentityBindingRow`; update queries; add backfill function from `AgentPeerBinding` signal-stream |
| `elohim/elohim-storage/src/services/distribution_view.rs` | Replace 5 TODO sites (lines 160, 164, 182, 256, 259) with real helper calls |
| `elohim/elohim-storage/src/services/reciprocity_view.rs` | Replace 3 TODO sites (lines 78, 159, 163) with helper calls; add `connected_peers` param to function signature |
| `elohim/elohim-storage/src/services/peer_topology_view.rs` | Replace 1 TODO site (line 180) — compute resilience_cliffs from sole-replica analysis |
| `elohim/elohim-storage/src/services/mod.rs` | Re-export 4 new helper modules |
| `elohim/elohim-storage/src/db/mod.rs` | Re-export `projection_events` module |
| `elohim/elohim-storage/src/views.rs` | None expected — DistributionDetails / ReciprocityRow / PeerTopologyView already declare the fields; only the values change |
| `elohim/elohim-storage/src/main.rs` | Wire projection_events writer into rea_projection signal handler at startup; wire pillar manifest layer-1 loader at line ~1129 |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/economic_event.rs` | Add `'ack-projection'` to validated action enum |
| `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` | Re-export `projection_ack` coordinator |
| `elohim/sdk/schemas/v1/economic-event-actions.schema.json` (or wherever the action enum lives) | Add `'ack-projection'` to enum if applicable |

---

## Task 1 — ALTER TABLE peer_identity_bindings: device_archetype + superseded_by

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-11-110001_peer_bindings_archetype_superseded/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-11-110001_peer_bindings_archetype_superseded/down.sql`
- Modify: `elohim/elohim-storage/src/db/peer_identity_bindings.rs`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (Diesel-generated; regenerate after migration)
- Test: `elohim/elohim-storage/tests/peer_bindings_phase4_columns.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/peer_bindings_phase4_columns.rs
use elohim_storage::db::peer_identity_bindings::{insert_binding, list_active_for_agent, PeerIdentityBindingRow};
use elohim_storage::db::test_helpers::{fresh_pool, now_iso};

#[test]
fn binding_carries_device_archetype_and_superseded_by() {
    let pool = fresh_pool();
    let mut conn = pool.get().expect("conn");

    let binding = PeerIdentityBindingRow {
        peer_id: "12D3KooWtest1".to_string(),
        agent_cid: "agent-matthew-desktop".to_string(),
        dht_anchor_hash: "u-action-hash-1".to_string(),
        valid_from: now_iso(),
        valid_until: None,
        observed_at: now_iso(),
        source: "dna".to_string(),
        device_archetype: "desktop".to_string(),
        superseded_by: None,
    };
    insert_binding(&mut conn, &binding).expect("insert");

    let active = list_active_for_agent(&mut conn, "agent-matthew-desktop", &now_iso())
        .expect("list");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].device_archetype, "desktop");
    assert_eq!(active[0].superseded_by, None);
}

#[test]
fn superseded_binding_is_excluded_from_list_active() {
    let pool = fresh_pool();
    let mut conn = pool.get().expect("conn");

    insert_binding(&mut conn, &PeerIdentityBindingRow {
        peer_id: "12D3KooWold".into(),
        agent_cid: "agent-matthew-desktop".into(),
        dht_anchor_hash: "u-action-hash-old".into(),
        valid_from: now_iso(),
        valid_until: None,
        observed_at: now_iso(),
        source: "dna".into(),
        device_archetype: "desktop".into(),
        superseded_by: Some("u-action-hash-new".into()),
    }).expect("insert old");

    insert_binding(&mut conn, &PeerIdentityBindingRow {
        peer_id: "12D3KooWnew".into(),
        agent_cid: "agent-matthew-desktop".into(),
        dht_anchor_hash: "u-action-hash-new".into(),
        valid_from: now_iso(),
        valid_until: None,
        observed_at: now_iso(),
        source: "dna".into(),
        device_archetype: "desktop".into(),
        superseded_by: None,
    }).expect("insert new");

    let active = list_active_for_agent(&mut conn, "agent-matthew-desktop", &now_iso())
        .expect("list");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].peer_id, "12D3KooWnew");
}
```

- [ ] **Step 2: Run test to verify it fails (column does not exist yet)**

Run:
```
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --test peer_bindings_phase4_columns -- --test-threads=1 2>&1 | tail -30
```
Expected: FAIL — compile error "no field `device_archetype` on `PeerIdentityBindingRow`" OR runtime error "no such column: device_archetype".

- [ ] **Step 3: Write the migration up.sql**

```sql
-- migrations/2026-05-11-110001_peer_bindings_archetype_superseded/up.sql
-- Phase 4 (EPR delivery master): backfill device_archetype + superseded_by
-- onto the peer_identity_bindings projection. Both fields are A2 link
-- metadata on the AgentPeerBinding DHT entry (imagodei zome) — they live
-- in the DNA but the projection table dropped them. Source of truth:
-- DHT; this projection rebuildable from imagodei signal stream.

ALTER TABLE peer_identity_bindings
    ADD COLUMN device_archetype TEXT NOT NULL DEFAULT 'node';

ALTER TABLE peer_identity_bindings
    ADD COLUMN superseded_by TEXT;

-- Index for "active bindings only" queries.
CREATE INDEX IF NOT EXISTS idx_peer_bindings_active
    ON peer_identity_bindings (agent_cid)
    WHERE superseded_by IS NULL;
```

- [ ] **Step 4: Write down.sql**

```sql
-- migrations/2026-05-11-110001_peer_bindings_archetype_superseded/down.sql
-- SQLite does not support DROP COLUMN cleanly; for rollback, recreate the
-- table without these columns. Operator-only — not safe in prod with data.

DROP INDEX IF EXISTS idx_peer_bindings_active;

CREATE TABLE peer_identity_bindings_old AS
    SELECT peer_id, agent_cid, dht_anchor_hash, valid_from, valid_until, observed_at, source
    FROM peer_identity_bindings;
DROP TABLE peer_identity_bindings;
ALTER TABLE peer_identity_bindings_old RENAME TO peer_identity_bindings;
```

- [ ] **Step 5: Update PeerIdentityBindingRow struct**

```rust
// elohim/elohim-storage/src/db/peer_identity_bindings.rs
//! ## Source of Truth
//!
//! Operational (Category C) projection of imagodei zome's AgentPeerBinding
//! DHT entry. The DNA carries device_archetype and superseded_by as A2 link
//! metadata; this projection mirrors them via the imagodei signal stream.
//! No SQLite column here is authoritative — rebuildable from any peer's
//! content_store via the established backfill flow.

use diesel::prelude::*;
use crate::db::diesel_schema::peer_identity_bindings;

#[derive(Debug, Clone, Queryable, Insertable, Selectable, Identifiable)]
#[diesel(table_name = peer_identity_bindings)]
#[diesel(primary_key(peer_id, dht_anchor_hash))]
pub struct PeerIdentityBindingRow {
    pub peer_id: String,
    pub agent_cid: String,
    pub dht_anchor_hash: String,
    pub valid_from: String,
    pub valid_until: Option<String>,
    pub observed_at: String,
    pub source: String,
    pub device_archetype: String,        // Phase 4 — backfilled from DNA
    pub superseded_by: Option<String>,   // Phase 4 — backfilled from DNA
}
```

- [ ] **Step 6: Update list_active_for_agent to filter on superseded_by IS NULL**

```rust
pub fn list_active_for_agent(
    conn: &mut SqliteConnection,
    agent_cid_arg: &str,
    now_iso: &str,
) -> diesel::result::QueryResult<Vec<PeerIdentityBindingRow>> {
    use crate::db::diesel_schema::peer_identity_bindings::dsl::*;
    peer_identity_bindings
        .filter(agent_cid.eq(agent_cid_arg))
        .filter(superseded_by.is_null())
        .filter(valid_until.is_null().or(valid_until.gt(now_iso)))
        .load::<PeerIdentityBindingRow>(conn)
}
```

- [ ] **Step 7: Run test to verify it passes**

Run the same command from Step 2. Expected: PASS (2 tests).

- [ ] **Step 8: Run full peer_identity_bindings test suite to catch regressions**

```
cargo test --lib db::peer_identity_bindings 2>&1 | tail -30
```
Expected: all existing tests still pass.

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-11-110001_peer_bindings_archetype_superseded/ \
        elohim/elohim-storage/src/db/peer_identity_bindings.rs \
        elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/tests/peer_bindings_phase4_columns.rs
git commit -m "feat(storage): Phase 4 T1 — peer_bindings device_archetype + superseded_by

ALTER TABLE adds the two A2 fields the AgentPeerBinding DHT entry
carries but the projection drops. Backfill from DNA signal stream
lands in T1.5 (next commit). Source of truth stays the DHT; this is
operational projection (Category C) per p2p-design-gate."
```

---

## Task 2 — Add `ack-projection` to REA action enum + integrity validator

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/economic_event.rs`
- Modify: `elohim/sdk/schemas/v1/economic-event-actions.schema.json` (if it exists; otherwise the action validation is structural in Rust)
- Test: `elohim/holochain/tests/sweettest/src/tests/projection_ack.rs`

- [ ] **Step 1: Locate the REA EconomicEvent action validator**

Run:
```
grep -rn "custody-blob\|serve-blob" elohim/holochain/dna/elohim/zomes/content_store_integrity/src/ 2>&1 | head -10
```
Identify the validator function that whitelists known action strings. The `ack-projection` action follows the same pattern.

- [ ] **Step 2: Write the failing sweettest**

```rust
// elohim/holochain/tests/sweettest/src/tests/projection_ack.rs
use sweettest::*;
use crate::common::*;

#[tokio::test(flavor = "multi_thread")]
async fn ack_projection_economic_event_validates() {
    let (conductor, _agent, cell) = setup_one_agent("content_store").await;

    let event_input = EconomicEventInput {
        action: "ack-projection".to_string(),
        provider: "agent-doorway-projector".to_string(),
        receiver: "agent-matthew".to_string(),
        resource_classified_as: "sha256-bafkreitest".to_string(),
        resource_quantity_value: 1.0,
        resource_quantity_unit: "ack".to_string(),
        // ... other required fields per existing EconomicEventInput
    };

    let result: ActionHash = conductor
        .call(&cell.zome("content_store"), "create_economic_event", event_input)
        .await;

    // Validation passed if no panic; the entry exists.
    assert!(!result.get_raw_39().is_empty());
}
```

- [ ] **Step 3: Run sweettest to verify it fails**

```
cd elohim/holochain/tests/sweettest
cargo test --test projection_ack -- --nocapture 2>&1 | tail -30
```
Expected: FAIL — validator rejects unknown action `ack-projection`.

- [ ] **Step 4: Add `ack-projection` to validator's allowed-action enum**

```rust
// elohim/holochain/dna/elohim/zomes/content_store_integrity/src/economic_event.rs
//
// (Locate the function — likely `validate_action` or similar — and extend
// the allowed-action match arm. Pattern from existing entries:)
fn validate_action(action: &str) -> ValidateCallbackResult {
    match action {
        "custody-blob" | "serve-blob" | "ack-projection" => ValidateCallbackResult::Valid,
        _ => ValidateCallbackResult::Invalid(format!("unknown action: {}", action)),
    }
}
```

- [ ] **Step 5: Run sweettest to verify it passes**

Run the same command from Step 3. Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/economic_event.rs \
        elohim/holochain/tests/sweettest/src/tests/projection_ack.rs
# If the action enum lives in JSON schema:
git add elohim/sdk/schemas/v1/economic-event-actions.schema.json
git commit -m "feat(content-store): Phase 4 T2 — accept ack-projection EconomicEvent action

Doorway projectors emit EconomicEvent { action: 'ack-projection',
provider: <doorway agent_cid>, resource_classified_as: <blob_hash> }
on each successful projection. REUSES existing EconomicEvent entry
type — no new DHT entry type, Lamad DNA stays at ~73/~100. P2P design
gate per master plan §'P2P Design Gate Output' table."
```

---

## Task 3 — `projection_events` table + Diesel writer

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-11-110000_projection_events/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-11-110000_projection_events/down.sql`
- Create: `elohim/elohim-storage/src/db/projection_events.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (regenerate)
- Test: `elohim/elohim-storage/tests/projection_events_writer.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/projection_events_writer.rs
use chrono::Utc;
use elohim_storage::db::projection_events::{
    append_projection_event, list_recent_for_blob, ProjectionEventRow,
};
use elohim_storage::db::test_helpers::fresh_pool;

#[test]
fn append_and_read_back_projection_event() {
    let pool = fresh_pool();
    let mut conn = pool.get().expect("conn");

    let now = Utc::now().to_rfc3339();
    append_projection_event(&mut conn, &ProjectionEventRow {
        blob_hash: "sha256-test1".into(),
        projector_agent_cid: "agent-doorway-1".into(),
        emitted_at: now.clone(),
        source_action_hash: "u-source-1".into(),
    }).expect("append");

    let recent = list_recent_for_blob(&mut conn, "sha256-test1", 10).expect("list");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].projector_agent_cid, "agent-doorway-1");
}

#[test]
fn list_recent_returns_newest_first_and_respects_limit() {
    let pool = fresh_pool();
    let mut conn = pool.get().expect("conn");

    for (i, agent) in ["agent-1", "agent-2", "agent-3"].iter().enumerate() {
        append_projection_event(&mut conn, &ProjectionEventRow {
            blob_hash: "sha256-test2".into(),
            projector_agent_cid: agent.to_string(),
            emitted_at: format!("2026-05-{:02}T00:00:00Z", i + 1),
            source_action_hash: format!("u-source-{}", i),
        }).expect("append");
    }

    let recent = list_recent_for_blob(&mut conn, "sha256-test2", 2).expect("list");
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].projector_agent_cid, "agent-3");
    assert_eq!(recent[1].projector_agent_cid, "agent-2");
}
```

- [ ] **Step 2: Run test to verify it fails (module/table does not exist)**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --test projection_events_writer 2>&1 | tail -30
```
Expected: FAIL — compile error "could not find module projection_events" OR "no such table".

- [ ] **Step 3: Write the migration up.sql**

```sql
-- migrations/2026-05-11-110000_projection_events/up.sql
-- Phase 4 — append-only operational log of doorway projector acks.
-- Source of truth: DHT (EconomicEvent entry, content_store zome,
-- action='ack-projection'). This table is rebuildable from any peer's
-- content_store storage projection by replaying the rea_projection signal
-- stream filtered to action='ack-projection'. P2P design gate Category C.

CREATE TABLE projection_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    blob_hash TEXT NOT NULL,
    projector_agent_cid TEXT NOT NULL,
    emitted_at TEXT NOT NULL,
    source_action_hash TEXT NOT NULL UNIQUE  -- dedup by source DHT action
);

CREATE INDEX idx_projection_events_blob_emitted
    ON projection_events (blob_hash, emitted_at DESC);
```

- [ ] **Step 4: Write down.sql**

```sql
-- migrations/2026-05-11-110000_projection_events/down.sql
DROP INDEX IF EXISTS idx_projection_events_blob_emitted;
DROP TABLE IF EXISTS projection_events;
```

- [ ] **Step 5: Write the projection_events module**

```rust
// elohim/elohim-storage/src/db/projection_events.rs
//! ## Source of Truth
//!
//! Operational (Category C) append-only log derived from the DHT-notarized
//! EconomicEvent entry (action='ack-projection'). Rebuildable from any
//! peer's content_store via the rea_projection signal stream. No SQLite
//! row here is authoritative.

use diesel::prelude::*;
use crate::db::diesel_schema::projection_events;

#[derive(Debug, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = projection_events)]
pub struct ProjectionEventRow {
    pub blob_hash: String,
    pub projector_agent_cid: String,
    pub emitted_at: String,
    pub source_action_hash: String,
}

pub fn append_projection_event(
    conn: &mut SqliteConnection,
    row: &ProjectionEventRow,
) -> diesel::result::QueryResult<()> {
    use crate::db::diesel_schema::projection_events::dsl::*;
    diesel::insert_into(projection_events)
        .values(row)
        .on_conflict(source_action_hash)
        .do_nothing()
        .execute(conn)?;
    Ok(())
}

pub fn list_recent_for_blob(
    conn: &mut SqliteConnection,
    blob_hash_arg: &str,
    limit: i64,
) -> diesel::result::QueryResult<Vec<ProjectionEventRow>> {
    use crate::db::diesel_schema::projection_events::dsl::*;
    projection_events
        .filter(blob_hash.eq(blob_hash_arg))
        .order(emitted_at.desc())
        .limit(limit)
        .load(conn)
}

pub fn distinct_projectors_for_blob(
    conn: &mut SqliteConnection,
    blob_hash_arg: &str,
) -> diesel::result::QueryResult<Vec<String>> {
    use crate::db::diesel_schema::projection_events::dsl::*;
    projection_events
        .filter(blob_hash.eq(blob_hash_arg))
        .select(projector_agent_cid)
        .distinct()
        .load(conn)
}
```

- [ ] **Step 6: Re-export from db/mod.rs**

```rust
// elohim/elohim-storage/src/db/mod.rs (append)
pub mod projection_events;
```

- [ ] **Step 7: Regenerate diesel schema if needed**

```
cd elohim/elohim-storage
diesel print-schema > src/db/diesel_schema.rs
```
(Or equivalent project workflow per existing migrations.)

- [ ] **Step 8: Run test to verify it passes**

Run from Step 2. Expected: PASS (2 tests).

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-11-110000_projection_events/ \
        elohim/elohim-storage/src/db/projection_events.rs \
        elohim/elohim-storage/src/db/mod.rs \
        elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/tests/projection_events_writer.rs
git commit -m "feat(storage): Phase 4 T3 — projection_events append-only log

Operational (Category C) log of ack-projection EconomicEvents. Three
helper functions: append_projection_event, list_recent_for_blob (newest
first, limit-bounded), distinct_projectors_for_blob (set of acking
projectors). Rebuildable from DHT via rea_projection signal stream."
```

---

## Task 4 — Wire rea_projection signal handler to write projection_events

**Files:**
- Create: `elohim/elohim-storage/src/p2p/projection_ack_handler.rs`
- Modify: `elohim/elohim-storage/src/main.rs` (wire handler into rea_projection signal stream at startup)
- Test: `elohim/elohim-storage/tests/projection_ack_signal_e2e.rs`

- [ ] **Step 1: Locate the existing rea_projection signal handler entry point**

Run:
```
grep -rn "rea_projection\|RealProjectionSignal\|EconomicEventReceived" elohim/elohim-storage/src/ 2>&1 | head -20
```
Find the dispatcher that already routes EconomicEvent signals to the existing economic_events table writer. The new handler hangs off the same dispatch.

- [ ] **Step 2: Write the failing integration test**

```rust
// tests/projection_ack_signal_e2e.rs
use elohim_storage::db::projection_events::list_recent_for_blob;
use elohim_storage::db::test_helpers::fresh_pool;
use elohim_storage::p2p::projection_ack_handler::handle_projection_ack;
use elohim_storage::p2p::rea_projection_dispatch::EconomicEventSignal;

#[tokio::test]
async fn ack_projection_signal_writes_projection_event() {
    let pool = fresh_pool();

    let signal = EconomicEventSignal {
        action: "ack-projection".into(),
        provider: "agent-doorway-1".into(),
        receiver: "agent-matthew".into(),
        resource_classified_as: "sha256-test-blob".into(),
        action_hash: "u-source-1".into(),
        emitted_at: "2026-05-11T12:00:00Z".into(),
    };

    handle_projection_ack(&pool, &signal).await.expect("handle");

    let mut conn = pool.get().expect("conn");
    let recent = list_recent_for_blob(&mut conn, "sha256-test-blob", 10).expect("list");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].projector_agent_cid, "agent-doorway-1");
}

#[tokio::test]
async fn non_ack_projection_action_is_ignored() {
    let pool = fresh_pool();

    let signal = EconomicEventSignal {
        action: "custody-blob".into(),  // NOT ack-projection
        provider: "agent-bob".into(),
        receiver: "agent-matthew".into(),
        resource_classified_as: "sha256-other".into(),
        action_hash: "u-source-2".into(),
        emitted_at: "2026-05-11T12:00:00Z".into(),
    };

    handle_projection_ack(&pool, &signal).await.expect("handle");

    let mut conn = pool.get().expect("conn");
    let recent = list_recent_for_blob(&mut conn, "sha256-other", 10).expect("list");
    assert!(recent.is_empty(), "non-ack-projection events must not write to projection_events");
}
```

- [ ] **Step 3: Run test to verify it fails**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --test projection_ack_signal_e2e 2>&1 | tail -30
```
Expected: FAIL — module `projection_ack_handler` does not exist.

- [ ] **Step 4: Write the handler**

```rust
// elohim/elohim-storage/src/p2p/projection_ack_handler.rs
//! ## Source of Truth
//!
//! Operational handler (Category C) that mirrors EconomicEvent signals
//! with action='ack-projection' into the projection_events log.
//! The DHT EconomicEvent entry remains authoritative.

use thiserror::Error;
use crate::db::projection_events::{append_projection_event, ProjectionEventRow};
use crate::db::DbPool;
use crate::p2p::rea_projection_dispatch::EconomicEventSignal;

#[derive(Debug, Error)]
pub enum ProjectionAckError {
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("pool error: {0}")]
    Pool(String),
}

const ACK_PROJECTION_ACTION: &str = "ack-projection";

pub async fn handle_projection_ack(
    pool: &DbPool,
    signal: &EconomicEventSignal,
) -> Result<(), ProjectionAckError> {
    if signal.action != ACK_PROJECTION_ACTION {
        return Ok(());
    }

    let mut conn = pool.get().map_err(|e| ProjectionAckError::Pool(e.to_string()))?;
    append_projection_event(&mut conn, &ProjectionEventRow {
        blob_hash: signal.resource_classified_as.clone(),
        projector_agent_cid: signal.provider.clone(),
        emitted_at: signal.emitted_at.clone(),
        source_action_hash: signal.action_hash.clone(),
    })?;

    tracing::debug!(
        target = "phase4::projection_ack",
        blob_hash = %signal.resource_classified_as,
        projector = %signal.provider,
        "recorded projection ack"
    );

    Ok(())
}
```

- [ ] **Step 5: Wire into main.rs at the existing rea_projection dispatch site**

Find the dispatch site (located in Step 1) and add a call to `handle_projection_ack` alongside the existing economic_events writer. Both run on every EconomicEvent signal; the handler self-filters by `action`.

- [ ] **Step 6: Run test to verify it passes**

Run from Step 3. Expected: PASS (2 tests).

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/p2p/projection_ack_handler.rs \
        elohim/elohim-storage/src/main.rs \
        elohim/elohim-storage/tests/projection_ack_signal_e2e.rs
git commit -m "feat(storage): Phase 4 T4 — wire projection_ack signal handler

EconomicEvent signals with action='ack-projection' now mirror into
projection_events. Other actions (custody-blob, serve-blob) flow
through the existing dispatch unchanged. Source of truth: DHT
EconomicEvent entry."
```

---

## Task 5 — Helper module: `imagodei_lookup::resolve_display_name`

**Files:**
- Create: `elohim/elohim-storage/src/services/imagodei_lookup.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Test: `elohim/elohim-storage/tests/imagodei_lookup.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/imagodei_lookup.rs
use elohim_storage::db::test_helpers::{fresh_pool, insert_human_for_test};
use elohim_storage::services::imagodei_lookup::resolve_display_name;

#[tokio::test]
async fn resolves_display_name_for_known_agent() {
    let pool = fresh_pool();
    insert_human_for_test(&pool, "agent-matthew", "Matthew Manager").await;

    let name = resolve_display_name(&pool, "agent-matthew").await;
    assert_eq!(name, Some("Matthew Manager".to_string()));
}

#[tokio::test]
async fn returns_none_for_unknown_agent() {
    let pool = fresh_pool();
    let name = resolve_display_name(&pool, "agent-unknown").await;
    assert_eq!(name, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test --test imagodei_lookup 2>&1 | tail -20
```
Expected: FAIL — module does not exist.

- [ ] **Step 3: Write the helper**

```rust
// elohim/elohim-storage/src/services/imagodei_lookup.rs
//! ## Source of Truth
//!
//! Helper API for view services. Reads `humans` table (operational
//! projection of Human DHT entry, Category A in imagodei zome). DHT
//! remains authoritative — this is a read-only convenience over the
//! existing projection.

use crate::db::DbPool;

pub async fn resolve_display_name(pool: &DbPool, agent_cid_arg: &str) -> Option<String> {
    use crate::db::diesel_schema::humans::dsl::*;
    use diesel::prelude::*;

    let mut conn = pool.get().ok()?;
    humans
        .filter(id.eq(agent_cid_arg))
        .select(display_name)
        .first::<Option<String>>(&mut conn)
        .ok()
        .flatten()
}
```

- [ ] **Step 4: Re-export from services/mod.rs**

```rust
// elohim/elohim-storage/src/services/mod.rs (append)
pub mod imagodei_lookup;
```

- [ ] **Step 5: Run test to verify it passes**

Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/imagodei_lookup.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/tests/imagodei_lookup.rs
git commit -m "feat(storage): Phase 4 T5 — imagodei_lookup::resolve_display_name helper

Pure read-only convenience over the humans table. Topology last-mile:
reciprocity_view + peer_topology_view consume this to populate
display_name fields without per-call SQL plumbing."
```

---

## Task 6 — Helper module: `connectivity::is_online`

**Files:**
- Create: `elohim/elohim-storage/src/services/connectivity.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Test: `elohim/elohim-storage/tests/connectivity_helper.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/connectivity_helper.rs
use std::collections::HashSet;
use elohim_storage::services::connectivity::{is_online, any_online_in};

#[test]
fn is_online_true_when_peer_in_snapshot() {
    let snapshot: HashSet<String> = ["12D3KooWa".into(), "12D3KooWb".into()].into();
    assert!(is_online("12D3KooWa", &snapshot));
}

#[test]
fn is_online_false_when_peer_not_in_snapshot() {
    let snapshot: HashSet<String> = ["12D3KooWa".into()].into();
    assert!(!is_online("12D3KooWb", &snapshot));
}

#[test]
fn any_online_in_returns_true_if_any_peer_present() {
    let snapshot: HashSet<String> = ["12D3KooWb".into()].into();
    assert!(any_online_in(&["12D3KooWa", "12D3KooWb", "12D3KooWc"], &snapshot));
}

#[test]
fn any_online_in_returns_false_when_none_match() {
    let snapshot: HashSet<String> = ["12D3KooWx".into()].into();
    assert!(!any_online_in(&["12D3KooWa", "12D3KooWb"], &snapshot));
}
```

- [ ] **Step 2: Run test → FAIL**

- [ ] **Step 3: Write the helper**

```rust
// elohim/elohim-storage/src/services/connectivity.rs
//! ## Source of Truth
//!
//! Operational helper (Category C). Ephemeral — consumes a snapshot
//! of libp2p `connected_peers()` taken by the caller at query time.
//! No persistence. Reads, never writes.

use std::collections::HashSet;

pub fn is_online(peer_id: &str, snapshot: &HashSet<String>) -> bool {
    snapshot.contains(peer_id)
}

pub fn any_online_in(peers: &[&str], snapshot: &HashSet<String>) -> bool {
    peers.iter().any(|p| snapshot.contains(*p))
}
```

- [ ] **Step 4: Re-export + Step 5: Run test → PASS + Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/connectivity.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/tests/connectivity_helper.rs
git commit -m "feat(storage): Phase 4 T6 — connectivity::is_online helper

Ephemeral helper consuming a libp2p connected_peers snapshot. View
services pass the snapshot down so per-row online checks are O(1)."
```

---

## Task 7 — Helper module: `device_capacity::available_bytes_for`

**Files:**
- Create: `elohim/elohim-storage/src/services/device_capacity.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Test: `elohim/elohim-storage/tests/device_capacity_helper.rs`

**Note:** This task depends on `services/system_metrics.rs` being either created (per topology M1 plan Task 2) OR stubbed-acceptable here. If system_metrics.rs already exists (verify in Step 1), use it. Otherwise, this task includes a minimal `device_capacity_total_bytes(pool, human_id)` that reads from a placeholder constant + leaves a TODO referencing topology M1 Task 2.

- [ ] **Step 1: Verify whether `services/system_metrics.rs` exists**

Run:
```
ls elohim/elohim-storage/src/services/system_metrics.rs 2>&1
```
If it exists, use its `device_capacity_total_bytes` function. If not, this task uses a placeholder that returns a fixed test value and emits a `tracing::debug!` referencing the topology M1 Task 2 dependency.

- [ ] **Step 2: Write the failing test**

```rust
// tests/device_capacity_helper.rs
use elohim_storage::db::test_helpers::{fresh_pool, insert_commitment_for_test};
use elohim_storage::services::device_capacity::available_bytes_for;

#[tokio::test]
async fn available_equals_total_minus_committed() {
    let pool = fresh_pool();

    // Inject a fake total of 1_000_000 bytes for human-matthew via the
    // device_capacity test override (or via system_metrics if it exists).
    elohim_storage::services::device_capacity::override_total_for_test(
        "human-matthew",
        1_000_000,
    );

    insert_commitment_for_test(
        &pool,
        "human-matthew",  // provider peer maps to human via existing binding helper
        300_000,
    ).await;

    let available = available_bytes_for(&pool, "human-matthew").await;
    assert_eq!(available, 700_000);
}
```

- [ ] **Step 3: Run test → FAIL**

- [ ] **Step 4: Write the helper**

```rust
// elohim/elohim-storage/src/services/device_capacity.rs
//! ## Source of Truth
//!
//! Helper API (Category C operational). Aggregates:
//!   total_bytes:    from services/system_metrics.rs (or placeholder)
//!   committed_bytes: SUM(rea_commitments.resource_quantity_value)
//!                    WHERE action='custody-blob' AND provider IN (human's peers)
//! Returns total - committed (saturating to 0).

use diesel::prelude::*;
use diesel::dsl::sql;
use diesel::sql_types::{Float, Nullable};

use crate::db::DbPool;

#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use once_cell::sync::Lazy;

#[cfg(test)]
static TEST_TOTALS: Lazy<Mutex<HashMap<String, u64>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
pub fn override_total_for_test(human_id: &str, total: u64) {
    TEST_TOTALS.lock().unwrap().insert(human_id.to_string(), total);
}

pub async fn available_bytes_for(pool: &DbPool, human_id_arg: &str) -> u64 {
    let total = device_capacity_total(human_id_arg);
    let committed = committed_bytes_for(pool, human_id_arg).await.unwrap_or(0);
    total.saturating_sub(committed)
}

fn device_capacity_total(human_id_arg: &str) -> u64 {
    #[cfg(test)]
    if let Some(&v) = TEST_TOTALS.lock().unwrap().get(human_id_arg) {
        return v;
    }

    // TODO(topology M1 Task 2): when services/system_metrics.rs lands,
    // call its device_capacity_total_bytes(human_id) here.
    tracing::debug!(
        target = "phase4::device_capacity",
        human = %human_id_arg,
        "device_capacity_total: returning 0 — depends on topology M1 system_metrics module"
    );
    0
}

async fn committed_bytes_for(pool: &DbPool, human_id_arg: &str) -> diesel::result::QueryResult<u64> {
    use crate::db::diesel_schema::peer_identity_bindings::dsl as bind;
    use crate::db::diesel_schema::rea_commitments::dsl as rc;

    let mut conn = pool.get().map_err(|_| diesel::result::Error::NotFound)?;

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
```

- [ ] **Step 5: Re-export + Step 6: Run test → PASS + Step 7: Commit**

```bash
git add elohim/elohim-storage/src/services/device_capacity.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/tests/device_capacity_helper.rs
git commit -m "feat(storage): Phase 4 T7 — device_capacity::available_bytes_for helper

Computes total - committed bytes per human. Total wired to 0 today
with TODO(topology M1 Task 2) pointing at services/system_metrics.rs;
this preserves the substrate seam without blocking Phase 4 on a
parallel sprint."
```

---

## Task 8 — Helper module: `peer_diversity::diversity_hint_for`

**Files:**
- Create: `elohim/elohim-storage/src/services/peer_diversity.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Test: `elohim/elohim-storage/tests/peer_diversity_helper.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/peer_diversity_helper.rs
use elohim_storage::services::peer_diversity::diversity_hint_for;
use elohim_storage::views::{DeviceArchetype, DiversityHint, ReplicaPeer};

fn replica(peer_id: &str, archetype: DeviceArchetype) -> ReplicaPeer {
    ReplicaPeer {
        peer_id: peer_id.into(),
        device_archetype: archetype,
        last_seen_seconds: 0,
        hop_hint: None,
        household_id: None,
        region_tier: None,
    }
}

#[test]
fn no_replicas_returns_none() {
    assert_eq!(diversity_hint_for(&[]), DiversityHint::None);
}

#[test]
fn single_archetype_returns_low_diversity() {
    let replicas = vec![
        replica("p1", DeviceArchetype::Desktop),
        replica("p2", DeviceArchetype::Desktop),
    ];
    assert_eq!(diversity_hint_for(&replicas), DiversityHint::Low);
}

#[test]
fn three_or_more_archetypes_returns_strong_diversity() {
    let replicas = vec![
        replica("p1", DeviceArchetype::Desktop),
        replica("p2", DeviceArchetype::Mobile),
        replica("p3", DeviceArchetype::Steward),
        replica("p4", DeviceArchetype::Node),
    ];
    assert_eq!(diversity_hint_for(&replicas), DiversityHint::Strong);
}

#[test]
fn two_archetypes_returns_moderate() {
    let replicas = vec![
        replica("p1", DeviceArchetype::Desktop),
        replica("p2", DeviceArchetype::Mobile),
    ];
    assert_eq!(diversity_hint_for(&replicas), DiversityHint::Moderate);
}
```

- [ ] **Step 2: Run test → FAIL**

- [ ] **Step 3: Write the helper**

```rust
// elohim/elohim-storage/src/services/peer_diversity.rs
//! ## Source of Truth
//!
//! Helper API (Category C operational). Pure function over replica
//! peer set; archetype field is itself an operational projection of
//! AgentPeerBinding A2 link metadata. No persistence.

use std::collections::HashSet;
use crate::views::{DeviceArchetype, DiversityHint, ReplicaPeer};

pub fn diversity_hint_for(replicas: &[ReplicaPeer]) -> DiversityHint {
    if replicas.is_empty() {
        return DiversityHint::None;
    }

    let unique_archetypes: HashSet<&DeviceArchetype> =
        replicas.iter().map(|r| &r.device_archetype).collect();

    match unique_archetypes.len() {
        0 => DiversityHint::None,
        1 => DiversityHint::Low,
        2 => DiversityHint::Moderate,
        _ => DiversityHint::Strong,
    }
}
```

(If the `DiversityHint` enum lacks the `Low` / `Moderate` / `Strong` variants, this task includes adding them to `views.rs` — but verify in Step 1. If they exist, use as-is.)

- [ ] **Step 4: Re-export + Step 5: Run test → PASS + Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/peer_diversity.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/tests/peer_diversity_helper.rs
git commit -m "feat(storage): Phase 4 T8 — peer_diversity::diversity_hint_for helper

Pure function: archetype-mix → DiversityHint enum. Distribution view
consumes this to replace the stubbed DiversityHint::None."
```

---

## Task 9 — Wire distribution_view.rs (resolve 5 TODO sites)

**Files:**
- Modify: `elohim/elohim-storage/src/services/distribution_view.rs`
- Test: extend `elohim/elohim-storage/tests/projection_ack_signal_e2e.rs` with view-level assertion

- [ ] **Step 1: Write the failing integration assertion**

Append to `tests/projection_ack_signal_e2e.rs`:

```rust
use elohim_storage::services::distribution_view::{compose_distribution_summary, DistributionContext};
use elohim_storage::views::DiversityHint;

#[tokio::test]
async fn distribution_summary_returns_real_projector_count_after_ack() {
    let pool = fresh_pool();

    // Pre-condition: blob has 1 replica entry.
    elohim_storage::db::test_helpers::insert_replica_for_test(&pool, "sha256-blob-A", "12D3KooWreplica").await;

    // Act: 2 different doorway projectors ack the projection.
    handle_projection_ack(&pool, &EconomicEventSignal {
        action: "ack-projection".into(),
        provider: "agent-doorway-1".into(),
        receiver: "agent-matthew".into(),
        resource_classified_as: "sha256-blob-A".into(),
        action_hash: "u-source-A1".into(),
        emitted_at: "2026-05-11T12:00:00Z".into(),
    }).await.unwrap();
    handle_projection_ack(&pool, &EconomicEventSignal {
        action: "ack-projection".into(),
        provider: "agent-doorway-2".into(),
        receiver: "agent-matthew".into(),
        resource_classified_as: "sha256-blob-A".into(),
        action_hash: "u-source-A2".into(),
        emitted_at: "2026-05-11T12:01:00Z".into(),
    }).await.unwrap();

    // Assert: distribution view returns projector_count=2 (not stubbed 0).
    let summary = compose_distribution_summary(
        &pool, "sha256-blob-A", DistributionContext::Visitor
    ).await.expect("compose");

    assert_eq!(summary.projector_count, 2, "must reflect distinct projectors from projection_events");
}
```

- [ ] **Step 2: Run test → FAIL** (current code has `let projector_count: u32 = 0;` stub)

- [ ] **Step 3: Replace TODO sites in `distribution_view.rs`**

In `compose_distribution_summary` (around line 159-165):
```rust
// Replace:
//   let projector_count: u32 = 0;
//   let diversity_hint = DiversityHint::None;
// With:
let projector_count = crate::db::projection_events::distinct_projectors_for_blob(
    &mut conn, blob_hash
)
    .map(|v| v.len() as u32)
    .unwrap_or(0);

// diversity_hint computed AFTER replica_peers is loaded — move the call
// down or restructure to compute it from the seen_peers set + binding lookup.
// For T9, compute over an empty replica list as a placeholder iff replica_peers
// hasn't been loaded yet in compose_distribution_summary; the FULL call uses
// load_replica_peers_full from the details function. Acceptable trade-off:
// summary's diversity_hint reads peer_identity_bindings directly for any peer in
// seen_peers and counts archetypes.
let archetypes_in_replicas = {
    use crate::db::diesel_schema::peer_identity_bindings::dsl as bind;
    let seen_vec: Vec<&str> = seen_peers.iter().map(String::as_str).collect();
    bind::peer_identity_bindings
        .filter(bind::peer_id.eq_any(&seen_vec))
        .filter(bind::superseded_by.is_null())
        .select(bind::device_archetype)
        .distinct()
        .load::<String>(&mut conn)
        .unwrap_or_default()
};
let diversity_hint = match archetypes_in_replicas.len() {
    0 => DiversityHint::None,
    1 => DiversityHint::Low,
    2 => DiversityHint::Moderate,
    _ => DiversityHint::Strong,
};
```

In the `Steward` arm (around line 182):
```rust
// Replace:
//   let any_projector = false;
// With:
let any_projector = crate::db::projection_events::distinct_projectors_for_blob(
    &mut conn, blob_hash
)
    .map(|projectors| projectors.iter().any(|p| my_agent_cids.contains(p.as_str())))
    .unwrap_or(false);
```
Where `my_agent_cids` is derived from `bindings.iter().map(|b| b.agent_cid.clone()).collect::<HashSet<_>>()`.

In `compose_distribution_details` (around line 256-260):
```rust
// Replace:
//   let projector_identities: Vec<ProjectorIdentity> = vec![];
// With:
let projector_identities: Vec<ProjectorIdentity> = {
    let projector_cids = crate::db::projection_events::distinct_projectors_for_blob(
        &mut conn, blob_hash
    ).unwrap_or_default();
    let mut out = Vec::with_capacity(projector_cids.len());
    for cid in projector_cids {
        let display_name = crate::services::imagodei_lookup::resolve_display_name(pool, &cid).await;
        out.push(ProjectorIdentity {
            agent_cid: cid,
            display_name,
            // last_signal_ack_seconds populated from projection_events.emitted_at
            // — see existing ProjectorIdentity field shape
            ..Default::default()
        });
    }
    out
};

// Replace:
//   let recent_projection_events: Vec<JsonVal> = vec![];
// With:
let recent_projection_events: Vec<JsonVal> = {
    let recent = crate::db::projection_events::list_recent_for_blob(
        &mut conn, blob_hash, 20
    ).unwrap_or_default();
    recent.into_iter().map(|r| JsonVal(serde_json::json!({
        "blobHash": r.blob_hash,
        "projectorAgentCid": r.projector_agent_cid,
        "emittedAt": r.emitted_at,
        "sourceActionHash": r.source_action_hash,
    }))).collect()
};
```

Verify `ProjectorIdentity` has the named fields by reading `views.rs`. Adjust to actual struct shape.

- [ ] **Step 4: Run test → PASS** (the new assertion); also re-run prior projection_ack_signal_e2e tests.

- [ ] **Step 5: Run full distribution_view test suite**

```
cargo test distribution_view 2>&1 | tail -30
```
Expected: all existing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/distribution_view.rs \
        elohim/elohim-storage/tests/projection_ack_signal_e2e.rs
git commit -m "feat(storage): Phase 4 T9 — distribution_view consumes projection_events

Replaces 5 TODO(Phase 4 follow-up) sites:
- projector_count: distinct projectors from projection_events
- diversity_hint: archetype-mix over peer_identity_bindings
- any_projector (steward arm): membership check vs my agent_cids
- projector_identities: from distinct_projectors + display_name lookup
- recent_projection_events: from list_recent_for_blob(limit=20)

Topology distribution badge now renders real data."
```

---

## Task 10 — Wire reciprocity_view.rs (resolve 3 TODO sites)

**Files:**
- Modify: `elohim/elohim-storage/src/services/reciprocity_view.rs`
- Test: `elohim/elohim-storage/tests/reciprocity_view_phase4.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/reciprocity_view_phase4.rs
use std::collections::HashSet;
use elohim_storage::db::test_helpers::*;
use elohim_storage::services::reciprocity_view::aggregate_reciprocity_view;

#[tokio::test]
async fn reciprocity_view_resolves_display_name_and_online() {
    let pool = fresh_pool();

    insert_human_for_test(&pool, "agent-jessica", "Jessica").await;
    insert_binding_for_test(&pool, "agent-jessica", "12D3KooWjessica", "desktop").await;
    insert_binding_for_test(&pool, "agent-matthew", "12D3KooWmatthew", "desktop").await;
    insert_commitment_for_test_typed(&pool, "12D3KooWmatthew", "agent-jessica", 100_000).await;

    let bindings = elohim_storage::db::peer_identity_bindings::list_active_for_agent(
        &mut pool.get().unwrap(),
        "agent-matthew",
        &chrono::Utc::now().to_rfc3339(),
    ).expect("bindings");

    let connected: HashSet<String> = ["12D3KooWjessica".into()].into();

    let view = aggregate_reciprocity_view(
        &pool, "agent-matthew", &bindings, &connected,
    ).await.expect("aggregate");

    assert!(!view.outflow.is_empty(), "expected outflow row to jessica");
    let row = &view.outflow[0];
    assert_eq!(row.display_name, Some("Jessica".to_string()));
    assert_eq!(row.online, Some(true));
}

#[tokio::test]
async fn reciprocity_capacity_returns_total_minus_committed() {
    let pool = fresh_pool();
    elohim_storage::services::device_capacity::override_total_for_test("agent-matthew", 1_000_000);

    insert_binding_for_test(&pool, "agent-matthew", "12D3KooWmatthew", "desktop").await;
    insert_commitment_for_test_typed(&pool, "12D3KooWmatthew", "agent-other", 250_000).await;

    let bindings = elohim_storage::db::peer_identity_bindings::list_active_for_agent(
        &mut pool.get().unwrap(),
        "agent-matthew",
        &chrono::Utc::now().to_rfc3339(),
    ).unwrap();

    let view = aggregate_reciprocity_view(
        &pool, "agent-matthew", &bindings, &HashSet::new(),
    ).await.unwrap();

    assert_eq!(view.capacity_available_bytes, 750_000);
}
```

- [ ] **Step 2: Run test → FAIL** (signature mismatch — function doesn't accept `connected_peers`)

- [ ] **Step 3: Update `aggregate_reciprocity_view` signature + replace TODO sites**

```rust
// elohim/elohim-storage/src/services/reciprocity_view.rs

pub async fn aggregate_reciprocity_view(
    pool: &DbPool,
    agent_cid: &str,
    bindings: &[PeerIdentityBindingRow],
    connected_peers: &HashSet<String>,  // NEW Phase 4 param
) -> Result<ReciprocityView, ReciprocityViewError> {
    // ... existing setup unchanged ...

    let outflow = compute_flow_rows(&mut conn, &my_peers, FlowDirection::Outflow, pool, connected_peers).await?;
    let inflow = compute_flow_rows(&mut conn, &my_peers, FlowDirection::Inflow, pool, connected_peers).await?;

    // ... existing net_hosted_bytes math unchanged ...

    Ok(ReciprocityView {
        agent_cid: agent_cid.to_string(),
        inflow,
        outflow,
        net_hosted_bytes,
        capacity_available_bytes: crate::services::device_capacity::available_bytes_for(
            pool, agent_cid
        ).await as i64,  // verify field type — view declares i64 or u64
    })
}
```

In `compute_flow_rows`, replace `display_name: None` and `online: None`:
```rust
rows.push(ReciprocityRow {
    counterparty_household_id: counterparty.clone(),
    display_name: crate::services::imagodei_lookup::resolve_display_name(pool, &counterparty).await,
    committed_bytes,
    delivered_bytes,
    honored_percent,
    online: Some(crate::services::connectivity::any_online_in(
        // Look up counterparty's peer set via bindings — small SQL helper
        &lookup_peers_for_agent(&mut conn, &counterparty),
        connected_peers,
    )),
});
```

(Add private `fn lookup_peers_for_agent(conn, agent_cid) -> Vec<&str>` if not already present.)

- [ ] **Step 4: Update all callers of `aggregate_reciprocity_view`**

Run:
```
grep -rn "aggregate_reciprocity_view" elohim/elohim-storage/src/ 2>&1
```
Update each call site to pass the connected_peers snapshot. The snapshot is taken in the libp2p swarm dispatcher (similar pattern to `peer_topology_view.rs` — see how `view_federation` snapshots there). For HTTP-route call sites where no swarm is available, pass `HashSet::new()` (online will be Some(false) for all, which is correct for read-from-cold-cache scenarios).

- [ ] **Step 5: Run test → PASS**; re-run prior reciprocity_view tests.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/reciprocity_view.rs \
        elohim/elohim-storage/tests/reciprocity_view_phase4.rs \
        # any updated call sites
git commit -m "feat(storage): Phase 4 T10 — reciprocity_view consumes Phase 4 helpers

Replaces 3 TODO(Phase 4 follow-up) sites:
- capacity_available_bytes: device_capacity::available_bytes_for
- display_name: imagodei_lookup::resolve_display_name (per row)
- online: connectivity::any_online_in (per row, against snapshot)

Function signature gains connected_peers param; HTTP callers pass
empty set (correct semantics for cold-cache reads), libp2p callers
pass swarm.connected_peers() snapshot."
```

---

## Task 11 — Wire peer_topology_view.rs (resolve_cliffs from sole-replica analysis)

**Files:**
- Modify: `elohim/elohim-storage/src/services/peer_topology_view.rs`
- Test: `elohim/elohim-storage/tests/peer_topology_phase4.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/peer_topology_phase4.rs
use elohim_storage::db::test_helpers::*;
use elohim_storage::services::peer_topology_view::aggregate_peer_topology_view;

#[tokio::test]
async fn peer_topology_emits_resilience_cliff_when_sole_replica() {
    let pool = fresh_pool();
    let federator = elohim_storage::services::federator::Federator::stub_for_test();

    // Setup: matthew owns 1 CID hosted by ONLY one peer (jessica).
    insert_binding_for_test(&pool, "agent-matthew", "12D3KooWmatthew", "desktop").await;
    insert_binding_for_test(&pool, "agent-jessica", "12D3KooWjessica", "desktop").await;
    insert_replica_with_owner(&pool, "sha256-cidA", "12D3KooWjessica", "agent-matthew").await;

    let view = aggregate_peer_topology_view(&pool, &federator, "agent-matthew")
        .await.expect("aggregate");

    assert!(!view.resilience_cliffs.is_empty(),
        "expected at least one resilience cliff (cidA has sole replica jessica)");
}
```

- [ ] **Step 2: Run test → FAIL** (cliffs vector is hard-coded `vec![]`)

- [ ] **Step 3: Implement sole-replica analysis at the marked line ~180**

```rust
// elohim/elohim-storage/src/services/peer_topology_view.rs

// Replace:
//   let resilience_cliffs = vec![];
// With:
let resilience_cliffs = compute_resilience_cliffs(&mut conn, agent_cid)?;
```

Add the helper:
```rust
fn compute_resilience_cliffs(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> Result<Vec<ResilienceCliff>, PeerTopologyError> {
    use crate::db::diesel_schema::peer_blob_inventory::dsl as inv;
    use crate::db::diesel_schema::content::dsl as c;

    // 1) CIDs owned by this agent (via content table or REA commitments)
    let my_cids: Vec<String> = c::content
        .filter(c::owner_agent_cid.eq(agent_cid))  // verify exact column name
        .select(c::blob_hash)
        .load::<String>(conn)?;

    if my_cids.is_empty() {
        return Ok(vec![]);
    }

    // 2) For each CID, count distinct replicas in peer_blob_inventory
    let mut cliffs = vec![];
    for cid in my_cids {
        let replica_count: i64 = inv::peer_blob_inventory
            .filter(inv::blob_hash.eq(&cid))
            .select(diesel::dsl::count_distinct(inv::peer_id))
            .first(conn)?;
        if replica_count <= 1 {
            cliffs.push(ResilienceCliff {
                blob_hash: cid,
                replica_count: replica_count as u32,
                kind: ResilienceCliffKind::SoleReplica,
            });
        }
    }
    Ok(cliffs)
}
```

(Verify `ResilienceCliff` struct + `ResilienceCliffKind` enum exist in `views.rs`. If not, add minimal definitions per the topology design spec.)

- [ ] **Step 4: Run test → PASS**

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/peer_topology_view.rs \
        elohim/elohim-storage/tests/peer_topology_phase4.rs
git commit -m "feat(storage): Phase 4 T11 — peer_topology resilience_cliffs from substrate

Sole-replica detection: for each owned CID, count distinct replicas
in peer_blob_inventory. replica_count <= 1 emits a SoleReplica
ResilienceCliff entry. Replaces TODO(Phase 4 follow-up) at line 180."
```

---

## Task 12 — Wire main.rs:1129 manifest registry layer-1 (load pillar manifests from disk)

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs`
- Test: `elohim/elohim-storage/tests/manifest_registry_layer1.rs`

- [ ] **Step 1: Identify the disk path convention for pillar manifests**

Run:
```
ls elohim/sdk/domains/lamad/manifest.json elohim/sdk/domains/shefa/manifest.json 2>&1
grep -rn "domains/lamad/manifest.json\|domains/shefa/manifest.json\|pillar.*manifest" elohim/elohim-storage/src/ 2>&1 | head -10
```
Confirm the path convention. Existing manifests live at `elohim/sdk/domains/{pillar}/manifest.json`.

- [ ] **Step 2: Write the failing test**

```rust
// tests/manifest_registry_layer1.rs
use elohim_storage::services::write_through::WriteThroughState;

#[test]
fn write_through_layer1_loaded_from_pillar_manifests() {
    let manifest_dir = std::env::temp_dir().join("phase4_manifest_test");
    std::fs::create_dir_all(manifest_dir.join("lamad")).unwrap();
    std::fs::write(
        manifest_dir.join("lamad/manifest.json"),
        r#"{"writeThrough": {"epr": "on"}}"#,
    ).unwrap();

    let layer1 = elohim_storage::services::manifest_registry::load_pillar_manifest_layer1(
        &manifest_dir
    ).expect("load");

    assert_eq!(layer1.get("lamad").and_then(|m| m.get("epr")), Some(&"on".to_string()));
}
```

- [ ] **Step 3: Run test → FAIL** (function does not exist)

- [ ] **Step 4: Implement `load_pillar_manifest_layer1` in `services/manifest_registry.rs`**

```rust
// elohim/elohim-storage/src/services/manifest_registry.rs (add at appropriate location)

/// Phase 4 T12: load layer-1 write-through state from pillar manifests on disk.
/// Replaces the empty HashMap stub at main.rs:1129.
pub fn load_pillar_manifest_layer1(
    manifest_dir: &Path,
) -> Result<HashMap<String, HashMap<String, String>>, ManifestRegistryError> {
    let mut layer1 = HashMap::new();
    if !manifest_dir.exists() {
        tracing::warn!(
            target = "phase4::manifest_layer1",
            dir = ?manifest_dir,
            "manifest directory not present; layer-1 stays empty"
        );
        return Ok(layer1);
    }
    for entry in std::fs::read_dir(manifest_dir)? {
        let entry = entry?;
        let pillar_name = entry.file_name().to_string_lossy().to_string();
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let body = std::fs::read_to_string(&manifest_path)?;
        let json: serde_json::Value = serde_json::from_str(&body)?;
        if let Some(wt) = json.get("writeThrough").and_then(|v| v.as_object()) {
            let mut pillar_map = HashMap::new();
            for (k, v) in wt {
                if let Some(s) = v.as_str() {
                    pillar_map.insert(k.clone(), s.to_string());
                }
            }
            layer1.insert(pillar_name, pillar_map);
        }
    }
    Ok(layer1)
}
```

- [ ] **Step 5: Wire into main.rs at the previously stubbed line**

In `main.rs` around line 1129, replace:
```rust
let manifest_layer = std::collections::HashMap::new();
```
With:
```rust
let manifest_dir = std::env::var("ELOHIM_PILLAR_MANIFEST_DIR")
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|_| {
        // Default: relative to repo root from CWD or via env override
        std::path::PathBuf::from("elohim/sdk/domains")
    });
let manifest_layer = elohim_storage::services::manifest_registry::load_pillar_manifest_layer1(
    &manifest_dir
).unwrap_or_else(|e| {
    tracing::warn!(
        target = "phase4::manifest_layer1",
        error = ?e,
        "failed to load pillar manifest layer-1; layer stays empty"
    );
    std::collections::HashMap::new()
});
```

Update the surrounding TODO comment to remove the Phase 4 reference.

- [ ] **Step 6: Run test → PASS**; verify main.rs still compiles.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/services/manifest_registry.rs \
        elohim/elohim-storage/src/main.rs \
        elohim/elohim-storage/tests/manifest_registry_layer1.rs
git commit -m "feat(storage): Phase 4 T12 — manifest registry layer-1 loads from disk

Removes the empty-HashMap stub at main.rs:1129. Reads
elohim/sdk/domains/{pillar}/manifest.json files at startup;
ELOHIM_PILLAR_MANIFEST_DIR env override for tests / non-default
deployments. Failure modes degrade gracefully to empty layer
(same behaviour as before, but with operator-visible warning)."
```

---

## Task 13 — End-to-end Phase 4 integration test

**Files:**
- Create: `elohim/elohim-storage/tests/phase4_projector_topology_integration.rs`

- [ ] **Step 1: Write the failing test**

```rust
// tests/phase4_projector_topology_integration.rs
//! Phase 4 closure test: substrate → topology surfaces.
//!
//! Asserts that the full chain — coordinator emits ack-projection
//! EconomicEvent → rea_projection signal handler writes projection_events
//! → distribution_view returns non-stub data — works end-to-end.

use std::collections::HashSet;
use elohim_storage::db::test_helpers::*;

#[tokio::test]
async fn end_to_end_projector_ack_surfaces_in_distribution_view() {
    let pool = fresh_pool();

    // Seed a content row + 3 replicas of mixed archetypes.
    insert_human_for_test(&pool, "agent-matthew", "Matthew").await;
    insert_human_for_test(&pool, "agent-doorway-1", "Doorway 1").await;
    insert_binding_for_test(&pool, "agent-replica-a", "12D3KooWa", "desktop").await;
    insert_binding_for_test(&pool, "agent-replica-b", "12D3KooWb", "mobile").await;
    insert_binding_for_test(&pool, "agent-replica-c", "12D3KooWc", "steward").await;
    insert_replica_for_test(&pool, "sha256-blobX", "12D3KooWa").await;
    insert_replica_for_test(&pool, "sha256-blobX", "12D3KooWb").await;
    insert_replica_for_test(&pool, "sha256-blobX", "12D3KooWc").await;

    // Doorway acks projection.
    elohim_storage::p2p::projection_ack_handler::handle_projection_ack(
        &pool,
        &elohim_storage::p2p::rea_projection_dispatch::EconomicEventSignal {
            action: "ack-projection".into(),
            provider: "agent-doorway-1".into(),
            receiver: "agent-matthew".into(),
            resource_classified_as: "sha256-blobX".into(),
            action_hash: "u-action-X".into(),
            emitted_at: "2026-05-11T12:00:00Z".into(),
        },
    ).await.unwrap();

    // Compose Visitor distribution summary.
    let summary = elohim_storage::services::distribution_view::compose_distribution_summary(
        &pool, "sha256-blobX",
        elohim_storage::services::distribution_view::DistributionContext::Visitor,
    ).await.unwrap();

    assert_eq!(summary.replica_count, 3, "3 replicas in inventory");
    assert_eq!(summary.projector_count, 1, "1 distinct doorway acked");
    assert_eq!(summary.diversity_hint, elohim_storage::views::DiversityHint::Strong,
        "3 distinct archetypes (desktop/mobile/steward) = Strong diversity");

    // Compose details — projector_identities should resolve display names.
    let details = elohim_storage::services::distribution_view::compose_distribution_details(
        &pool, "sha256-blobX",
        elohim_storage::services::distribution_view::DistributionContext::Visitor,
    ).await.unwrap();

    assert_eq!(details.projector_identities.len(), 1);
    assert_eq!(details.projector_identities[0].agent_cid, "agent-doorway-1");
    assert_eq!(details.projector_identities[0].display_name, Some("Doorway 1".into()));
    assert_eq!(details.recent_projection_events.len(), 1);
}
```

- [ ] **Step 2: Run test — should PASS** if T1-T11 are all correctly wired.

- [ ] **Step 3: Run full storage test suite to catch any regressions**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test --lib --tests 2>&1 | tail -50
```
Expected: all tests pass.

- [ ] **Step 4: Run clippy + fmt**

```
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -20
cargo fmt --check
```

- [ ] **Step 5: Run schema codegen + contract test**

```
cd /projects/elohim
pnpm run schema:codegen:ts 2>&1 | tail -10
cd elohim/elohim-storage && cargo test --test schema_contract 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/tests/phase4_projector_topology_integration.rs
git commit -m "test(storage): Phase 4 T13 — end-to-end substrate→topology integration

Closure test: coordinator emits ack-projection EconomicEvent →
projection_events row appears → distribution_view returns
projector_count + diversity_hint + projector_identities (with
resolved display_name) + recent_projection_events.

Phase 4 substrate complete; topology UI sprint's last-mile is now
pure rendering against real data. All TODO(Phase 4 follow-up)
markers across services/{distribution,reciprocity,peer_topology}_view.rs
+ main.rs:1129 are resolved."
```

---

## Self-Review

- ✅ Every `TODO(Phase 4 follow-up)` site in distribution_view.rs (5), reciprocity_view.rs (3), peer_topology_view.rs (1), and main.rs (1) has a task that resolves it.
- ✅ P2P design gate output declared at top — every entity classified, REUSE EconomicEvent decision documented.
- ✅ No new DHT entry types proposed (Lamad ~73/100 unchanged).
- ✅ Helper API (T5-T8) absorbs glue per master plan D2 (b) recommendation — topology sprint's view-rendering becomes one-liners.
- ✅ T7 device_capacity has documented dependency on topology M1's services/system_metrics.rs with graceful degradation.
- ✅ T13 end-to-end closure test reproduces the topology sprint's first-mile render path.
- ✅ Each task is TDD-shaped: failing test → minimal impl → passing test → commit.
- ✅ `RUSTFLAGS` override for elohim-storage cargo invocations included where applicable.
- ✅ `CARGO_TARGET_DIR` pool path included for cargo invocations (per session-start guidance).
- ✅ Closing condition aligns with master plan Wave 1 deliverable.

## Execution Handoff

After this plan saves, the master sprint orchestrator dispatches:
- **One Sonnet implementation agent** via `superpowers:subagent-driven-development`
- Working in a single dedicated worktree (per master plan D6 — Wave 1 has only one sub-plan)
- Task-by-task with two-stage review per dispatch
- Forbids: scope creep, dep version changes, destructive git ops, signature changes without workspace-grep of callers
- BLOCKED report on any obstacle that requires editing code outside this plan's named files
