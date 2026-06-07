---
title: Slice 1 — Acquisition Pull Queue: reconcile rails, DevicePin, .pull wire, ladder rungs 2–3 — Implementation Plan
id: epr-acquisition-slice1-pull-queue-plan
status: Draft
class: protocol-canonical
domain: D5
sprint: acquisition-family-slice-1
cites:
  - epr-acquisition-pull-queue-design | the spec this plan implements — Slice 1 of its §13 slicing (reconcile rails §4.1, acquisition stream §4.2-4.4, DevicePin §1.1, rungs 2-3 §8); gate record in its §3/§12 covers every entity here | sha256:fc4a0cdd9828a377 | path: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md
  - .claude/memory-kit/gap-items/specs__2026-06-07-epr-acquisition-pull-queue-design.json
  - genesis/plans/2026-04-06-identity-driven-replication-plan.md
  - genesis/plans/2026-03-22-epr-body-plane-plan.md
  - epr-slice3-route-claims-plan | the house-style precedent — sibling §12.6-slice plan whose task structure, gate ceremony, and commit cadence this plan mirrors | sha256:5fcfaef3bd0e911b | path: genesis/docs/superpowers/plans/2026-06-06-epr-slice3-route-claims-plan.md
---

# Slice 1 — Acquisition Pull Queue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the spec's Slice 1 — the shared `reconcile_rails` module, the sibling acquisition
reconcile stream, the `acquisition_pins` DevicePin store + local routes, the `.pull` wire counts +
`wait-for-pull` tooling, and ladder rungs 2–3 on the link surface (gap-items #1–#6 of
`specs__2026-06-07-epr-acquisition-pull-queue-design.json`).

**Architecture:** The acquisition stream is a *sibling* of the running replication loop (Approach B,
operator-adjudicated): the proven state-machine rails are extracted into `reconcile_rails::GapTracker`,
`ReplicationState` becomes a thin wrapper over it (public API and all existing tests unchanged), and the
new `AcquisitionState` consumes the same tracker per-pin. Pins are Category-B local rows (airplane-mode
creatable — no conductor, no peers, no p2p feature required); the queue state is Category-C in-memory,
recomputed on restart. Per-item done-signal is sha256-verified bytes in the local projection (the
existing GetContent → `bulk_create_content` path); counts surface as `P2PStatusInfo.pull`
(`Option` — null means keep-waiting, never caught-up) following the View Schema Contract.

**Tech stack:** Rust (hyper hand-rolled dispatch in `http.rs`, Diesel/SQLite, libp2p shard protocol,
ts-rs), TypeScript (seeder tooling, Angular 19 host component), JSON Schema (view contract), Gherkin (a2o).

**P2P design gate:** run and adjudicated at spec time — see the spec's §3 (entity table) and §12
(gate record). The only new table (`acquisition_pins`) is **Category B agent-scoped local** with the
source-of-truth comment in its migration (the declaring site); `/api/v1/pins` deliberately has no
DHT entry type behind it (the pin's notarized shadow is the Slice-2 `provide-content` Commitment on
the EXISTING Mishpat entry type). No new DHT entry types anywhere in this plan.

**Env/test discipline:** everything here runs on `household-nodes` (in-process two-node harness for
the e2e). Build with the cargo pool slot:
`export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev`
(ambient `RUSTFLAGS` stays as the system sets it for elohim-storage). Container has NO nextest —
use plain `cargo test`. Never pipe gate exit codes.

**Slice-1 scope rails (from the spec — do not exceed):**
- `kind='cluster'` pins are REJECTED with 501 + named reason (closure resolver is Slice 3).
- No `provide-content` commitment, no sync-back, no scorer arm (Slice 2).
- No new shard wire vocabulary (striping is a follow-on spec; ≤16MB whole-content fetch only —
  larger items surface `failed: transport-unavailable`, honest per spec §4.2).
- Elements stay stateless (elohim-elements gospel): menu composition + action handling live in the
  Angular HOST component/service, never in `<elohim-epr-link>`.

---

## File structure (locked decomposition)

| File | Responsibility |
|---|---|
| Create `elohim/elohim-storage/src/p2p/reconcile_rails.rs` | `GapTracker` (generic gap state machine) + `DispatchBudget` (slot backpressure helper) |
| Modify `elohim/elohim-storage/src/p2p/replication.rs` | `ReplicationState` delegates to `GapTracker`; public API unchanged |
| Create `elohim/elohim-storage/src/p2p/acquisition.rs` | `AcquisitionState` (per-pin trackers, rollup), `PullStatusInfo` (ts-rs) |
| Create `elohim/elohim-storage/migrations/2026-06-07-000000_acquisition_pins/{up,down}.sql` | DevicePin table (Category B local; source-of-truth comment) |
| Modify `elohim/elohim-storage/src/db/diesel_schema.rs` | `acquisition_pins` table! macro |
| Modify `elohim/elohim-storage/src/db/models.rs` | `AcquisitionPin` + `NewAcquisitionPin` Diesel models |
| Create `elohim/elohim-storage/src/db/acquisition_pins.rs` | CRUD: upsert / list-active / set-status / presence diff helper |
| Modify `elohim/elohim-storage/src/db/mod.rs` | `pub mod acquisition_pins;` |
| Create `elohim/elohim-views/src/acquisition.rs` | `PinView`, `CreatePinInputView`, `PinPullStatusView` (ts-rs, camelCase) |
| Modify `elohim/elohim-views/src/lib.rs` | `pub mod acquisition;` |
| Modify `elohim/elohim-storage/src/p2p/mod.rs` | acquisition intervals + dispatch + completion hooks + `pull` in `refresh_status` + `P2PStatusInfo.pull` |
| Modify `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json` | add optional `pull` property |
| Modify `elohim/elohim-storage/src/http.rs` | `/api/v1/pins` GET/POST/DELETE match arms + handlers |
| Create `genesis/seeder/src/wait-for-pull.ts` | tri-state poller (mirror of wait-for-drain) |
| Modify `app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts` | rungs 2–3 menu items + handlers |
| Create `app/elohim-app/src/app/elohim/services/acquisition.service.ts` | capability detection + pin POST (Tauri-direct) / cache-warm (browser) |
| Create `elohim/elohim-storage/tests/acquisition_pins_http.rs` | airplane-mode property + route contract |
| Create `genesis/a2o/features/delivery/acquisition-pins.feature` | story-first scenarios |

---

### Task 1: `reconcile_rails::GapTracker` — extract the state machine

**Files:**
- Create: `elohim/elohim-storage/src/p2p/reconcile_rails.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (add `pub mod reconcile_rails;` next to `pub mod replication;`)

- [ ] **Step 1: Write the failing test** (in the new module, bottom)

```rust
// elohim/elohim-storage/src/p2p/reconcile_rails.rs
//! Shared reconcile-stream rails (spec §4.1, R-E): the gap state machine and
//! dispatch budget used by BOTH the replication stream (whole-inventory,
//! node-policy) and the acquisition stream (desired-set, user-declared).
//! ONE controller pattern governs all reconcile streams — a parallel bespoke
//! fetcher is a coherence violation.

use std::collections::{HashMap, HashSet};

/// Generic gap state machine: known-local / pending / completed / failed(retries).
/// Retry discipline is retry-on-NEXT-cycle (never immediate re-queue — the
/// freeze-at-partial battle-scar, see replication.rs mark_failed docs).
#[derive(Debug, Default)]
pub struct GapTracker {
    local_ids: HashSet<String>,
    pending: HashSet<String>,
    completed: HashSet<String>,
    failed: HashMap<String, u32>,
    caught_up: bool,
    max_retries: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desired_set_reconcile_diffs_wants_against_local() {
        let mut t = GapTracker::new(3);
        t.set_local_ids(["a".into()].into_iter().collect());
        let gaps = t.reconcile_desired(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(gaps.len(), 2);
        assert!(gaps.contains(&"b".to_string()) && gaps.contains(&"c".to_string()));
        let s = t.counts();
        assert_eq!((s.pending, s.completed, s.failed), (2, 0, 0));
        assert!(!s.caught_up);
    }

    #[test]
    fn exhausted_retries_drop_out_of_reconcile() {
        let mut t = GapTracker::new(1);
        t.reconcile_desired(vec!["x".into()]);
        t.mark_failed("x");
        // next cycle: fail_count=1 >= max_retries=1 → not re-queued
        let gaps = t.reconcile_desired(vec!["x".into()]);
        assert!(gaps.is_empty());
        assert_eq!(t.counts().failed, 1);
    }

    #[test]
    fn dispatch_budget_caps_inflight() {
        assert_eq!(DispatchBudget::new(50).available(47), 3);
        assert_eq!(DispatchBudget::new(50).available(50), 0);
        assert_eq!(DispatchBudget::new(50).available(60), 0); // saturating
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cd /projects/elohim/elohim/elohim-storage
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev
cargo test --lib reconcile_rails 2>&1 | tail -5
```
Expected: compile error (methods not defined).

- [ ] **Step 3: Implement `GapTracker` + `DispatchBudget`**

```rust
/// Snapshot counts for status surfaces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GapCounts {
    pub pending: usize,
    pub completed: usize,
    pub failed: usize,
    pub caught_up: bool,
}

impl GapTracker {
    pub fn new(max_retries: u32) -> Self {
        Self { max_retries, ..Default::default() }
    }

    pub fn counts(&self) -> GapCounts {
        GapCounts {
            pending: self.pending.len(),
            completed: self.completed.len(),
            failed: self.failed.len(),
            caught_up: self.caught_up,
        }
    }

    /// Register IDs already present locally. `flip_caught_up_if_restored`
    /// preserves replication.rs's restored-pod semantics; the acquisition
    /// stream passes `false` (a pin is never caught-up before reconcile).
    pub fn set_local_ids_with(&mut self, ids: HashSet<String>, flip_caught_up_if_restored: bool) {
        let had_content = !ids.is_empty();
        self.local_ids = ids;
        if flip_caught_up_if_restored && had_content && self.pending.is_empty() {
            self.caught_up = true;
        }
    }

    pub fn set_local_ids(&mut self, ids: HashSet<String>) {
        self.set_local_ids_with(ids, false);
    }

    /// Inventory-driven entry (replication): remote advertises, we diff.
    pub fn discover(&mut self, remote_ids: Vec<String>) -> Vec<String> {
        self.enqueue_missing(remote_ids)
    }

    /// Desired-set-driven entry (acquisition): WE declare, then diff.
    /// Identical machine, different direction of declaration (spec §4.2).
    pub fn reconcile_desired(&mut self, want_ids: Vec<String>) -> Vec<String> {
        self.enqueue_missing(want_ids)
    }

    fn enqueue_missing(&mut self, ids: Vec<String>) -> Vec<String> {
        let mut new_gaps = Vec::new();
        for id in ids {
            if self.local_ids.contains(&id)
                || self.completed.contains(&id)
                || self.pending.contains(&id)
            {
                continue;
            }
            if self.failed.get(&id).copied().unwrap_or(0) >= self.max_retries {
                continue;
            }
            self.pending.insert(id.clone());
            new_gaps.push(id);
        }
        if !new_gaps.is_empty() {
            self.caught_up = false;
        }
        new_gaps
    }

    pub fn mark_completed(&mut self, id: &str) {
        self.pending.remove(id);
        self.failed.remove(id);
        self.completed.insert(id.to_string());
        self.local_ids.insert(id.to_string());
    }

    /// Removed from pending, NOT re-queued — the next reconcile/discover
    /// cycle re-includes it while fail_count < max_retries (R-E).
    pub fn mark_failed(&mut self, id: &str) {
        self.pending.remove(id);
        *self.failed.entry(id.to_string()).or_insert(0) += 1;
    }

    pub fn update_caught_up(&mut self) {
        self.caught_up = self.pending.is_empty();
    }

    /// True if this tracker still wants `id` (pending now or retryable later).
    pub fn wants(&self, id: &str) -> bool {
        self.pending.contains(id)
    }
}

/// Slot-backpressure helper (R-E): dispatch rate becomes a natural function
/// of peer response speed — mirrors drain_gap_queue's MAX_REPLICATION_INFLIGHT.
#[derive(Debug, Clone, Copy)]
pub struct DispatchBudget {
    max_inflight: usize,
}

impl DispatchBudget {
    pub fn new(max_inflight: usize) -> Self {
        Self { max_inflight }
    }
    pub fn available(&self, in_flight: usize) -> usize {
        self.max_inflight.saturating_sub(in_flight)
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --lib reconcile_rails 2>&1 | tail -5
```
Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/reconcile_rails.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(storage): reconcile_rails — shared GapTracker + DispatchBudget (acquisition spec §4.1)"
```

---

### Task 2: `ReplicationState` delegates to `GapTracker` (zero behavior change)

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/replication.rs` (replace `ReplicationInner` internals; KEEP `ReplicationStatus`, all `pub async fn` signatures, and every existing test untouched)

- [ ] **Step 1: Swap the inner state for the shared tracker**

Replace the `ReplicationInner` struct and the bodies of the impl (lines 30–153) with delegation —
the file keeps `ReplicationStatus` (ts-rs) and the `ReplicationState` API verbatim:

```rust
use super::reconcile_rails::GapTracker;

const MAX_RETRIES: u32 = 3;

/// Thread-safe replication state manager — thin wrapper over the shared
/// reconcile_rails::GapTracker (spec §4.1: one controller pattern, extracted).
#[derive(Debug, Clone)]
pub struct ReplicationState {
    inner: Arc<RwLock<GapTracker>>,
}

impl ReplicationState {
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(GapTracker::new(MAX_RETRIES))) }
    }

    pub async fn status(&self) -> ReplicationStatus {
        let c = self.inner.read().await.counts();
        ReplicationStatus {
            pending: c.pending,
            completed: c.completed,
            failed: c.failed,
            caught_up: c.caught_up,
        }
    }

    pub async fn set_local_ids(&self, ids: HashSet<String>) {
        // restored-pod semantics preserved (flip caught_up when restored)
        self.inner.write().await.set_local_ids_with(ids, true);
    }

    pub async fn discover(&self, remote_ids: Vec<String>) -> Vec<String> {
        self.inner.write().await.discover(remote_ids)
    }

    pub async fn mark_completed(&self, id: &str) {
        self.inner.write().await.mark_completed(id);
    }

    pub async fn mark_failed(&self, id: &str) {
        self.inner.write().await.mark_failed(id);
    }

    pub async fn update_caught_up(&self) {
        self.inner.write().await.update_caught_up();
    }
}
```

Keep the doc comments from the original methods (move them onto the wrappers — they carry the
battle-scars: restored-pod caught-up rationale on `set_local_ids`, retry-on-next-cycle on `mark_failed`).
Delete `ReplicationInner` and the now-unused `HashMap` import if dead.

- [ ] **Step 2: Run the EXISTING replication tests unchanged**

```bash
cargo test --lib replication 2>&1 | tail -5
```
Expected: all 7 existing tests pass (`replication_state_discovers_gaps`, `..._marks_completed`,
`..._retries_failures`, `restored_pod_...`, `fresh_pod_...`, `pending_gaps_...`, `..._skips_known_local_ids`).
They are the extraction's regression harness — none may be edited.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/p2p/replication.rs
git commit -m "refactor(storage): ReplicationState delegates to shared GapTracker — API + tests unchanged"
```

---

### Task 3: `acquisition_pins` table + Diesel layer (the DevicePin, Category B)

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-06-07-000000_acquisition_pins/up.sql`, `down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (table! + `allow_tables_to_appear_in_same_query!` list)
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Create: `elohim/elohim-storage/src/db/acquisition_pins.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (`pub mod acquisition_pins;`)

> Migration timestamp `2026-06-07-000000`: per `feedback_diesel_migration_timestamp_collision`,
> verify no other migration shares this exact stamp before committing (`ls migrations | grep 2026-06-07`).

- [ ] **Step 1: Write the migration**

```sql
-- up.sql
-- DevicePin — the airplane-mode-durable local want declaration (Category B, agent-scoped).
-- Source of truth: local (agent-scoped device pin; roams via export, not gossip).
-- No dht_anchor_hash by design: the pin's notarized shadow is a provide-content
-- Commitment written at sync-back (Slice 2), NOT this row.
-- Spec: 2026-06-07-epr-acquisition-pull-queue-design.md §1.1, §3.
CREATE TABLE acquisition_pins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_pub_key TEXT NOT NULL DEFAULT 'local-device',
    head_ref TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'item' CHECK (kind IN ('item', 'cluster')),
    closure_rule_json TEXT,
    priority INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'removed')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (agent_pub_key, head_ref, kind)
);
CREATE INDEX idx_acquisition_pins_status ON acquisition_pins(status);
```

```sql
-- down.sql
DROP INDEX IF EXISTS idx_acquisition_pins_status;
DROP TABLE IF EXISTS acquisition_pins;
```

- [ ] **Step 2: diesel_schema.rs + models.rs**

```rust
// diesel_schema.rs — append (and add `acquisition_pins,` to allow_tables_to_appear_in_same_query!)
// Migration: 2026-06-07-000000_acquisition_pins
diesel::table! {
    acquisition_pins (id) {
        id -> Integer,
        agent_pub_key -> Text,
        head_ref -> Text,
        kind -> Text,
        closure_rule_json -> Nullable<Text>,
        priority -> Integer,
        status -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

```rust
// models.rs — append
#[derive(Debug, Clone, Queryable, Identifiable, Serialize)]
#[diesel(table_name = crate::db::diesel_schema::acquisition_pins)]
pub struct AcquisitionPin {
    pub id: i32,
    pub agent_pub_key: String,
    pub head_ref: String,
    pub kind: String,
    pub closure_rule_json: Option<String>,
    pub priority: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::diesel_schema::acquisition_pins)]
pub struct NewAcquisitionPin {
    pub agent_pub_key: String,
    pub head_ref: String,
    pub kind: String,
    pub closure_rule_json: Option<String>,
    pub priority: i32,
}
```

- [ ] **Step 3: Write failing CRUD tests + implementation**

```rust
// elohim/elohim-storage/src/db/acquisition_pins.rs
//! DevicePin CRUD (Category B local store). Airplane-mode property: nothing
//! here touches p2p, the conductor, or the network — pure local SQLite.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::acquisition_pins::dsl as pins;
use super::models::{AcquisitionPin, NewAcquisitionPin};

/// Idempotent upsert on the composite identity (agent, head, kind) —
/// re-pinning an existing pin refreshes priority/closure and revives a
/// 'removed' pin to 'active' (gate: agent-scoped composite address, spec §3).
pub fn upsert_pin(
    conn: &mut SqliteConnection,
    new: NewAcquisitionPin,
) -> QueryResult<AcquisitionPin> {
    diesel::insert_into(pins::acquisition_pins)
        .values(&new)
        .on_conflict((pins::agent_pub_key, pins::head_ref, pins::kind))
        .do_update()
        .set((
            pins::closure_rule_json.eq(new.closure_rule_json.clone()),
            pins::priority.eq(new.priority),
            pins::status.eq("active"),
            pins::updated_at.eq(diesel::dsl::sql::<diesel::sql_types::Text>("datetime('now')")),
        ))
        .execute(conn)?;
    pins::acquisition_pins
        .filter(pins::agent_pub_key.eq(&new.agent_pub_key))
        .filter(pins::head_ref.eq(&new.head_ref))
        .filter(pins::kind.eq(&new.kind))
        .first(conn)
}

pub fn list_active_pins(conn: &mut SqliteConnection) -> QueryResult<Vec<AcquisitionPin>> {
    pins::acquisition_pins
        .filter(pins::status.eq("active"))
        .order(pins::priority.desc())
        .load(conn)
}

pub fn list_all_pins(conn: &mut SqliteConnection) -> QueryResult<Vec<AcquisitionPin>> {
    pins::acquisition_pins.order(pins::id.asc()).load(conn)
}

/// Soft-remove (un-pin). Slice 2 adds revocation of the provide commitment
/// on this path; bytes GC is device storage policy, never automatic here.
pub fn set_pin_status(
    conn: &mut SqliteConnection,
    pin_id: i32,
    new_status: &str,
) -> QueryResult<usize> {
    diesel::update(pins::acquisition_pins.filter(pins::id.eq(pin_id)))
        .set((
            pins::status.eq(new_status),
            pins::updated_at.eq(diesel::dsl::sql::<diesel::sql_types::Text>("datetime('now')")),
        ))
        .execute(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::test_conn; // existing in-memory + migrations helper; if absent, mirror the pattern used by db tests in this crate

    #[test]
    fn upsert_is_idempotent_on_composite_identity() {
        let mut conn = test_conn();
        let pin = |p: i32| NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            head_ref: "epr:album-1".into(),
            kind: "item".into(),
            closure_rule_json: None,
            priority: p,
        };
        let a = upsert_pin(&mut conn, pin(1)).unwrap();
        let b = upsert_pin(&mut conn, pin(5)).unwrap();
        assert_eq!(a.id, b.id, "same composite identity = same row");
        assert_eq!(b.priority, 5);
        assert_eq!(list_active_pins(&mut conn).unwrap().len(), 1);
    }

    #[test]
    fn removed_pins_drop_out_of_active_and_revive_on_repin() {
        let mut conn = test_conn();
        let created = upsert_pin(&mut conn, NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            head_ref: "epr:x".into(),
            kind: "item".into(),
            closure_rule_json: None,
            priority: 1,
        }).unwrap();
        set_pin_status(&mut conn, created.id, "removed").unwrap();
        assert!(list_active_pins(&mut conn).unwrap().is_empty());
        // re-pin revives
        upsert_pin(&mut conn, NewAcquisitionPin {
            agent_pub_key: "local-device".into(),
            head_ref: "epr:x".into(),
            kind: "item".into(),
            closure_rule_json: None,
            priority: 1,
        }).unwrap();
        assert_eq!(list_active_pins(&mut conn).unwrap().len(), 1);
    }
}
```

If `crate::db::test_support::test_conn` does not exist, locate the in-memory-connection +
run-migrations pattern the existing `db/` tests use (grep `embed_migrations\|run_pending_migrations`
in `src/db/`) and use that exact helper — do NOT invent a parallel one.

- [ ] **Step 4: Run**

```bash
cargo test --lib acquisition_pins 2>&1 | tail -5
```
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-06-07-000000_acquisition_pins elohim/elohim-storage/src/db/
git commit -m "feat(storage): acquisition_pins DevicePin store — Category B local, airplane-mode creatable (spec §1.1)"
```

---

### Task 4: `AcquisitionState` + `PullStatusInfo` (the .pull counts)

**Files:**
- Create: `elohim/elohim-storage/src/p2p/acquisition.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (`pub mod acquisition;`)

- [ ] **Step 1: Write failing tests + the module**

```rust
// elohim/elohim-storage/src/p2p/acquisition.rs
//! Acquisition reconcile stream state (spec §4) — the sibling of the
//! replication stream. Per-pin GapTrackers over the DECLARED desired set;
//! all state Category C (in-memory, recomputed on restart from active pins
//! × local inventory). Wire vocabulary is the unified set (spec §4.3):
//! {total, fetched, pending, failed, caughtUp}.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;

use super::reconcile_rails::GapTracker;

const MAX_RETRIES: u32 = 3;
/// Device-paced: deliberately below replication's 50 (R-E — the acquisition
/// stream serves a person's wants, not node policy; it must not starve the
/// node-level stream).
pub const MAX_ACQUISITION_INFLIGHT: usize = 25;

/// Pull-queue rollup. None on the wire means "cannot compute" = keep waiting
/// (the wait-for-drain tri-state contract, spec §4.3) — never caught-up.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PullStatusInfo {
    /// Size of the resolved desired set across active pins. Slice 1: item
    /// pins resolve trivially (total = pin count); cluster pins are 501.
    pub total: i32,
    /// Items with sha256-verified bytes durably stored (R-A byte-arrival).
    pub fetched: i32,
    /// Resolved-but-not-yet-fetched items.
    pub pending: i32,
    /// Items whose retries are exhausted this cycle window (visible, never
    /// silently dropped — spec §10).
    pub failed: i32,
    /// total concrete ∧ pending == 0 (the expectedMin guard lives in the
    /// CONSUMER per spec §4.3 — wait-for-pull asserts total >= expectedMin).
    pub caught_up: bool,
}

/// Per-pin progress, served on GET /api/v1/pins (own node only).
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PinPullStatus {
    pub pin_id: i32,
    pub total: i32,
    pub fetched: i32,
    pub pending: i32,
    pub failed: i32,
    pub caught_up: bool,
}

#[derive(Debug, Default)]
struct AcquisitionInner {
    /// pin row id → tracker over that pin's resolved item set
    trackers: HashMap<i32, GapTracker>,
    /// pin row id → resolved desired-set size (Slice 1: 1 for item pins)
    totals: HashMap<i32, usize>,
    /// content id → pin ids wanting it (completion fan-out)
    wanted_by: HashMap<String, Vec<i32>>,
}

#[derive(Debug, Clone, Default)]
pub struct AcquisitionState {
    inner: Arc<RwLock<AcquisitionInner>>,
}

impl AcquisitionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconcile the declared wants (spec §4.2 step 1–2). `pin_wants` maps
    /// each active pin to its resolved item ids; `local_has` is the set of
    /// ids already present in the local projection. Returns newly-pending
    /// content ids in pin-priority order (highest priority first — the
    /// caller passes pins pre-sorted; lanes beyond explicit pins arrive in
    /// Slice 3).
    pub async fn reconcile(
        &self,
        pin_wants: Vec<(i32, Vec<String>)>,
        local_has: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        let mut inner = self.inner.write().await;
        // Drop trackers for pins no longer active (un-pinned → queue drains, spec §10)
        let live: std::collections::HashSet<i32> = pin_wants.iter().map(|(id, _)| *id).collect();
        inner.trackers.retain(|id, _| live.contains(id));
        inner.totals.retain(|id, _| live.contains(id));
        inner.wanted_by.retain(|_, pins| {
            pins.retain(|p| live.contains(p));
            !pins.is_empty()
        });

        let mut to_dispatch = Vec::new();
        for (pin_id, want_ids) in pin_wants {
            inner.totals.insert(pin_id, want_ids.len());
            for id in &want_ids {
                let entry = inner.wanted_by.entry(id.clone()).or_default();
                if !entry.contains(&pin_id) {
                    entry.push(pin_id);
                }
            }
            let tracker = inner
                .trackers
                .entry(pin_id)
                .or_insert_with(|| GapTracker::new(MAX_RETRIES));
            tracker.set_local_ids(local_has.clone());
            let gaps = tracker.reconcile_desired(want_ids);
            to_dispatch.extend(gaps);
        }
        to_dispatch
    }

    /// Byte-arrival done-signal (R-A): called from the ContentData completion
    /// path AFTER bulk_create_content succeeds — never on inventory receipt.
    pub async fn mark_completed(&self, content_id: &str) {
        let mut inner = self.inner.write().await;
        if let Some(pin_ids) = inner.wanted_by.get(content_id).cloned() {
            for pin_id in pin_ids {
                if let Some(t) = inner.trackers.get_mut(&pin_id) {
                    t.mark_completed(content_id);
                    t.update_caught_up();
                }
            }
        }
    }

    pub async fn mark_failed(&self, content_id: &str) {
        let mut inner = self.inner.write().await;
        if let Some(pin_ids) = inner.wanted_by.get(content_id).cloned() {
            for pin_id in pin_ids {
                if let Some(t) = inner.trackers.get_mut(&pin_id) {
                    t.mark_failed(content_id);
                }
            }
        }
    }

    /// True if any tracker still wants this id (dispatch filter).
    pub async fn wants(&self, content_id: &str) -> bool {
        let inner = self.inner.read().await;
        inner
            .wanted_by
            .get(content_id)
            .map(|pins| {
                pins.iter().any(|p| {
                    inner.trackers.get(p).map(|t| t.wants(content_id)).unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    pub async fn rollup(&self) -> PullStatusInfo {
        let inner = self.inner.read().await;
        let mut s = PullStatusInfo::default();
        for (pin_id, t) in &inner.trackers {
            let c = t.counts();
            s.total += *inner.totals.get(pin_id).unwrap_or(&0) as i32;
            s.fetched += c.completed as i32;
            s.pending += c.pending as i32;
            s.failed += c.failed as i32;
        }
        s.caught_up = s.pending == 0;
        s
    }

    pub async fn per_pin(&self) -> Vec<PinPullStatus> {
        let inner = self.inner.read().await;
        let mut out: Vec<PinPullStatus> = inner
            .trackers
            .iter()
            .map(|(pin_id, t)| {
                let c = t.counts();
                PinPullStatus {
                    pin_id: *pin_id,
                    total: *inner.totals.get(pin_id).unwrap_or(&0) as i32,
                    fetched: c.completed as i32,
                    pending: c.pending as i32,
                    failed: c.failed as i32,
                    caught_up: c.pending == 0,
                }
            })
            .collect();
        out.sort_by_key(|p| p.pin_id);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[tokio::test]
    async fn reconcile_diffs_wants_and_rolls_up() {
        let acq = AcquisitionState::new();
        let local: HashSet<String> = ["have-1".to_string()].into_iter().collect();
        let dispatch = acq
            .reconcile(
                vec![(1, vec!["have-1".into(), "need-1".into()]), (2, vec!["need-2".into()])],
                &local,
            )
            .await;
        assert_eq!(dispatch.len(), 2);
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched, r.pending), (3, 0, 2));
        assert!(!r.caught_up);
    }

    #[tokio::test]
    async fn byte_arrival_completes_every_wanting_pin() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(
            vec![(1, vec!["shared".into()]), (2, vec!["shared".into()])],
            &local,
        )
        .await;
        acq.mark_completed("shared").await;
        let pins = acq.per_pin().await;
        assert!(pins.iter().all(|p| p.caught_up && p.fetched == 1));
    }

    #[tokio::test]
    async fn unpinned_pin_drains_out_of_state() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec!["a".into()]), (2, vec!["b".into()])], &local).await;
        // next reconcile: pin 2 removed (un-pinned)
        acq.reconcile(vec![(1, vec!["a".into()])], &local).await;
        let pins = acq.per_pin().await;
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].pin_id, 1);
        assert!(!acq.wants("b").await);
    }
}
```

- [ ] **Step 2: Run**

```bash
cargo test --lib acquisition 2>&1 | tail -5
```
Expected: 3 passed (plus the acquisition_pins db tests from Task 3 in the filter).

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/p2p/acquisition.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(storage): AcquisitionState — per-pin GapTrackers, unified-vocab PullStatusInfo (spec §4.3)"
```

---

### Task 5: wire the stream into the p2p event loop + `P2PStatusInfo.pull`

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` —
  (a) node field + constructor init, (b) intervals in `run()` (~line 2070), (c) dispatch fn,
  (d) completion hooks in the `ContentData` handler (~line 3704–3760) and the failure paths,
  (e) `refresh_status` (~line 6547) + `P2PStatusInfo` struct (~line 682) + BOTH `initial_status`
  literals (lines ~1086, ~1608),
- Modify: `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json`

- [ ] **Step 1: Update the schema FIRST (View Schema Contract workflow)**

Add to `properties` in `p2p-status-view.schema.json` (NOT to `required` — tri-state nullable like `drain`):

```json
"pull": {
  "type": ["object", "null"],
  "description": "Acquisition pull-queue rollup (spec 2026-06-07-epr-acquisition-pull-queue-design §4.3-4.4). null = cannot compute = keep waiting, never caught-up. Unified vocab shared with the future wait-for-* contract.",
  "required": ["total", "fetched", "pending", "failed", "caughtUp"],
  "properties": {
    "total":   { "type": "integer", "description": "Resolved desired-set size across active pins" },
    "fetched": { "type": "integer", "description": "Items with sha256-verified bytes stored (byte-arrival, never inventory-arrival)" },
    "pending": { "type": "integer" },
    "failed":  { "type": "integer", "description": "Retry-exhausted items — visible, never silently dropped" },
    "caughtUp": { "type": "boolean" }
  }
}
```

- [ ] **Step 2: Node struct + status struct**

In the node struct (near `replication_state` / `gap_queue` ~line 505):

```rust
    /// Acquisition stream state (spec §4) — sibling of replication_state.
    acquisition: acquisition::AcquisitionState,
    /// content ids dispatched for acquisition, keyed by request id
    pending_acquisition_fetches: Arc<tokio::sync::Mutex<HashMap<OutboundRequestId, String>>>,
    /// acquisition gap queue (priority-ordered at enqueue time)
    acquisition_queue: Arc<tokio::sync::Mutex<std::collections::VecDeque<String>>>,
```

(Initialize all three in the constructor next to `gap_queue`'s init ~line 1678:
`acquisition: acquisition::AcquisitionState::new()`, empty Mutex'd map + VecDeque.)

In `P2PStatusInfo` (after the `drain` field, ~line 704):

```rust
    /// Acquisition pull-queue rollup — None when state cannot be computed.
    /// Consumers treat None as "keep waiting", NEVER as caught up (spec §4.3).
    pub pull: Option<acquisition::PullStatusInfo>,
```

Add `pull: None,` to BOTH `initial_status` literals (~lines 1086 and 1608 — grep
`let initial_status = P2PStatusInfo` to catch every site).

- [ ] **Step 3: Reconcile + dispatch in `run()`**

In `run()` next to the existing intervals (~line 2076):

```rust
        // Acquisition stream (spec §4.2): reconcile wants every 60s,
        // dispatch from the acquisition queue every 5s (sibling cadence).
        let mut acquisition_reconcile_interval = tokio::time::interval(Duration::from_secs(60));
        acquisition_reconcile_interval
            .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut acquisition_dispatch_interval = tokio::time::interval(Duration::from_secs(5));
        acquisition_dispatch_interval
            .set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
```

In the `tokio::select!` (after the `gap_dispatch_interval` arm, ~line 2206):

```rust
                _ = acquisition_reconcile_interval.tick() => {
                    drop(swarm);
                    if !self.sync_paused.load(Ordering::Acquire) {
                        self.run_acquisition_reconcile().await;
                    }
                }
                _ = acquisition_dispatch_interval.tick() => {
                    drop(swarm);
                    if !self.sync_paused.load(Ordering::Acquire) {
                        self.drain_acquisition_queue().await;
                    }
                }
```

- [ ] **Step 4: The reconcile + dispatch methods** (next to `run_replication_cycle` ~line 6440)

```rust
    /// Acquisition reconcile (spec §4.2 steps 1–2): load active pins, resolve
    /// wants (Slice 1: item pins only — head_ref IS the item id; cluster pins
    /// are rejected at POST time), diff against local presence, enqueue gaps
    /// priority-ordered. Pure local computation — no network here (R-H).
    async fn run_acquisition_reconcile(&self) {
        let Some(ref pool) = self.db_pool else { return };
        let Ok(mut conn) = pool.get() else { return };

        let pins = match crate::db::acquisition_pins::list_active_pins(&mut conn) {
            Ok(p) => p,
            Err(e) => {
                debug!(error = %e, "acquisition reconcile: pin load failed");
                return;
            }
        };
        // list_active_pins orders priority DESC — enqueue order IS lane order
        // (Slice 1 has only the explicit-pin lane; spec §4.2).
        let pin_wants: Vec<(i32, Vec<String>)> = pins
            .iter()
            .filter(|p| p.kind == "item")
            .map(|p| (p.id, vec![p.head_ref.clone()]))
            .collect();

        // Local presence = the content projection (byte-arrival's durable home).
        let want_ids: Vec<String> = pin_wants
            .iter()
            .flat_map(|(_, ids)| ids.iter().cloned())
            .collect();
        let app_ctx = crate::db::AppContext::default_lamad();
        let local_has = match crate::db::content_diesel::content_ids_present(
            &mut conn, &app_ctx, &want_ids,
        ) {
            Ok(set) => set,
            Err(e) => {
                debug!(error = %e, "acquisition reconcile: presence query failed");
                return;
            }
        };

        let to_dispatch = self.acquisition.reconcile(pin_wants, &local_has).await;
        if !to_dispatch.is_empty() {
            let mut q = self.acquisition_queue.lock().await;
            for id in to_dispatch {
                if !q.contains(&id) {
                    q.push_back(id);
                }
            }
        }
    }

    /// Acquisition dispatch (spec §4.2 step 3): shared-rails budget, round-robin
    /// GetContent across connected peers — the same wire request the replication
    /// stream uses; the streams differ in WHAT they want, not HOW they fetch.
    async fn drain_acquisition_queue(&self) {
        use super::p2p::reconcile_rails::DispatchBudget; // adjust path to crate layout
        let budget = DispatchBudget::new(acquisition::MAX_ACQUISITION_INFLIGHT);

        let peers: Vec<PeerId> = {
            let swarm = self.swarm.read().await;
            swarm.connected_peers().cloned().collect()
        };
        if peers.is_empty() {
            return; // peer-gated: no peers ⇒ no progress, no phantom counts (R-E)
        }
        let in_flight = self.pending_acquisition_fetches.lock().await.len();
        let available = budget.available(in_flight);
        if available == 0 {
            return;
        }
        let to_dispatch: Vec<String> = {
            let mut queue = self.acquisition_queue.lock().await;
            if queue.is_empty() {
                return;
            }
            let len = queue.len();
            queue.drain(..available.min(len)).collect()
        };
        for (i, id) in to_dispatch.iter().enumerate() {
            // Skip items no longer wanted (un-pinned between enqueue and dispatch)
            if !self.acquisition.wants(id).await {
                continue;
            }
            let peer = peers[i % peers.len()];
            let request = ShardRequest::GetContent { id: id.clone() };
            let mut swarm = self.swarm.write().await;
            let request_id = swarm
                .behaviour_mut()
                .shard_protocol
                .send_request(&peer, request);
            drop(swarm);
            self.pending_acquisition_fetches
                .lock()
                .await
                .insert(request_id, id.clone());
        }
    }
```

`content_ids_present` is a new small helper in `content_diesel.rs`:

```rust
/// Which of `ids` exist in the local content projection (acquisition presence diff).
pub fn content_ids_present(
    conn: &mut SqliteConnection,
    app_ctx: &AppContext,
    ids: &[String],
) -> QueryResult<std::collections::HashSet<String>> {
    use crate::db::diesel_schema::content::dsl as c;
    if ids.is_empty() {
        return Ok(Default::default());
    }
    let found: Vec<String> = c::content
        .filter(c::app_id.eq(&app_ctx.app_id))
        .filter(c::id.eq_any(ids))
        .select(c::id)
        .load(conn)?;
    Ok(found.into_iter().collect())
}
```

(Match the existing column/filter names in `content_diesel.rs` — if the app-context filter there
uses a different column or helper, mirror THAT, not this sketch.)

- [ ] **Step 5: Completion hooks (byte-arrival)**

In the `ContentData` response handler, the request-id is looked up to find which stream asked.
Where `pending_replication_fetches` is consulted, ALSO consult `pending_acquisition_fetches`; after
the existing `bulk_create_content` success branch (~line 3727) add:

```rust
                                                // Acquisition byte-arrival (R-A):
                                                // verified row landed — complete
                                                // every pin wanting it. Cheap no-op
                                                // when no pin wants this id.
                                                self.acquisition.mark_completed(&content_id).await;
```

And in EVERY failure path of that handler where `replication_state.mark_failed(&content_id)` is
called (pool-get failure ~line 3697, deserialize failure ~line 3690, insert error branch), add the
sibling call:

```rust
                                                self.acquisition.mark_failed(&content_id).await;
```

Also: wherever the outbound-failure handler removes from `pending_replication_fetches` (grep
`pending_replication_fetches` for the `OutboundFailure` site), mirror removal +
`acquisition.mark_failed` for `pending_acquisition_fetches`.

- [ ] **Step 6: `refresh_status` rollup**

In `refresh_status` (~line 6557, after `let replication = ...`):

```rust
        let pull = Some(self.acquisition.rollup().await);
```

and add `pull,` to the `P2PStatusInfo` construction at the bottom of `refresh_status`.
(`Some(...)` is correct here: the rollup is computable whenever the node runs; the `Option` exists
for consumers + non-p2p surfaces. A DB-failure during reconcile leaves stale-but-honest counts.)

- [ ] **Step 7: Build + full lib tests + schema contract**

```bash
cargo build 2>&1 | tail -3
cargo test --lib 2>&1 | tail -3
cargo test --test schema_contract 2>&1 | tail -3
```
Expected: build clean; lib tests pass; `p2p_status_view_matches_schema` passes (the new optional
`pull` is schema-matched).

- [ ] **Step 8: Regenerate TS bindings**

```bash
cargo test export_bindings 2>&1 | tail -3
git status --short elohim/sdk/storage-client-ts/src/generated/ | head
```
Expected: `PullStatusInfo.ts`, `PinPullStatus.ts` created; `P2PStatusInfo.ts` gains `pull`.

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-storage/src/p2p/ elohim/elohim-storage/src/db/content_diesel.rs \
  elohim/sdk/schemas/v1/views/p2p-status-view.schema.json elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): acquisition reconcile stream wired — intervals, dispatch budget, byte-arrival hooks, P2PStatusInfo.pull (spec §4)"
```

---

### Task 6: `/api/v1/pins` routes + views (HTTP designed last)

**Files:**
- Create: `elohim/elohim-views/src/acquisition.rs` (+ `pub mod acquisition;` in `elohim/elohim-views/src/lib.rs`)
- Modify: `elohim/elohim-storage/src/http.rs` (match arms near the p2p block ~line 826 + handlers)
- Create: `elohim/elohim-storage/tests/acquisition_pins_http.rs`

> These routes are **own-node only** (Category B): they are deliberately NOT added to
> `build_manifest()` (the doorway route manifest, http.rs ~8863) — a doorway must never serve
> another agent's pins. Tauri-direct (`:8090`) and local dev reach them directly.

- [ ] **Step 1: View types**

```rust
// elohim/elohim-views/src/acquisition.rs
//! DevicePin wire shapes (spec §1.1, §4.4). camelCase out, parsed JSON —
//! snake_case never leaves the Rust boundary.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct PinView {
    pub id: i32,
    pub agent_pub_key: String,
    pub head_ref: String,
    pub kind: String,
    pub closure_rule: Option<Value>,
    pub priority: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct CreatePinInputView {
    pub head_ref: String,
    /// 'item' (default) | 'cluster' (501 until Slice 3)
    pub kind: Option<String>,
    pub closure_rule: Option<Value>,
    pub priority: Option<i32>,
}
```

(Check the exact `export_to` relative path against a neighboring elohim-views module — ts-rs paths
are source-crate-relative, the cross-crate trap. Mirror a sibling file byte-for-byte.)

- [ ] **Step 2: Failing integration test (the airplane-mode property)**

```rust
// elohim/elohim-storage/tests/acquisition_pins_http.rs
//! Airplane-mode property (spec §1.1, a2o scenario 1): pin CRUD works with
//! NO p2p, NO conductor, NO peers — pure local HTTP + SQLite.
//! Mirror the harness setup of an existing http-route test in this dir
//! (e.g. rea_commitments_http_route.rs) for server bootstrap WITHOUT p2p.

// Pseudocode shape — bind to the actual harness helpers found in
// tests/rea_commitments_http_route.rs / tests/harness:
//   let server = spawn_http_only_storage().await;        // p2p disabled
//   POST /api/v1/pins {"headRef":"epr:x"}                → 201, kind=item, status=active
//   POST /api/v1/pins {"headRef":"epr:x"}                → 200 idempotent (same id)
//   POST /api/v1/pins {"headRef":"epr:c","kind":"cluster"} → 501 body contains "slice-3"
//   GET  /api/v1/pins                                     → 200, one active pin, pull:null (p2p off)
//   DELETE /api/v1/pins/{id}                              → 200; GET shows status=removed
```

Write it as a real test against the same in-process server harness the sibling test uses — copy its
bootstrap verbatim, then the five assertions above (each a separate `#[tokio::test]` or one
sequential test matching the sibling's style).

- [ ] **Step 3: Match arms + handlers in http.rs**

Next to the p2p arms (~line 826) — NOT inside `#[cfg(feature = "p2p")]` (airplane-mode: pins are
DB-only; only the `pull` enrichment is p2p-gated):

```rust
            // Acquisition DevicePins (spec §1.1) — OWN NODE ONLY: deliberately
            // absent from build_manifest(); a doorway never serves pins.
            (Method::GET, "/api/v1/pins") => self.handle_list_pins().await,
            (Method::POST, "/api/v1/pins") => self.handle_create_pin(req).await,
            (Method::DELETE, p) if p.starts_with("/api/v1/pins/") => {
                let id = p.trim_start_matches("/api/v1/pins/").to_string();
                self.handle_remove_pin(&id).await
            }
```

Handlers (mirror the JSON/error envelope style of neighboring handlers — e.g. the commitments
handlers; the sketch below shows logic, bind the response helpers to whatever `http.rs` actually
uses, e.g. `json_response` / `error_response` equivalents found in the file):

```rust
    async fn handle_create_pin(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = req.collect().await?.to_bytes();
        let input: elohim_views::acquisition::CreatePinInputView =
            serde_json::from_slice(&body).map_err(StorageError::bad_request)?;

        let kind = input.kind.unwrap_or_else(|| "item".to_string());
        if kind == "cluster" {
            // Slice-1 honesty (spec §13): closure resolver is Slice 3.
            return Ok(json_error(
                StatusCode::NOT_IMPLEMENTED,
                "cluster pins land in slice-3 (closure resolver); see spec §5",
            ));
        }
        if kind != "item" {
            return Ok(json_error(StatusCode::BAD_REQUEST, "kind must be 'item' or 'cluster'"));
        }

        let pool = self.require_db_pool()?;
        let mut conn = pool.get().map_err(StorageError::from)?;
        let created = crate::db::acquisition_pins::upsert_pin(
            &mut conn,
            crate::db::models::NewAcquisitionPin {
                agent_pub_key: "local-device".to_string(), // single-agent device context, spec §3
                head_ref: input.head_ref,
                kind,
                closure_rule_json: input.closure_rule.map(|v| v.to_string()),
                priority: input.priority.unwrap_or(1),
            },
        )
        .map_err(StorageError::from)?;
        Ok(json_response(StatusCode::CREATED, &pin_to_view(created)))
    }

    async fn handle_list_pins(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self.require_db_pool()?;
        let mut conn = pool.get().map_err(StorageError::from)?;
        let rows = crate::db::acquisition_pins::list_all_pins(&mut conn)
            .map_err(StorageError::from)?;
        // Per-pin pull counts when the p2p node is running (Option = tri-state)
        #[cfg(feature = "p2p")]
        let pull = match &self.p2p_node {
            Some(node) => Some(node.acquisition_per_pin().await), // small accessor added on the node
            None => None,
        };
        #[cfg(not(feature = "p2p"))]
        let pull: Option<Vec<()>> = None;
        Ok(json_response(StatusCode::OK, &serde_json::json!({
            "pins": rows.into_iter().map(pin_to_view).collect::<Vec<_>>(),
            "pull": pull,
        })))
    }

    async fn handle_remove_pin(&self, id: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        let pin_id: i32 = id.parse().map_err(|_| StorageError::bad_request("invalid pin id"))?;
        let pool = self.require_db_pool()?;
        let mut conn = pool.get().map_err(StorageError::from)?;
        let n = crate::db::acquisition_pins::set_pin_status(&mut conn, pin_id, "removed")
            .map_err(StorageError::from)?;
        if n == 0 {
            return Ok(json_error(StatusCode::NOT_FOUND, "pin not found"));
        }
        Ok(json_response(StatusCode::OK, &serde_json::json!({ "removed": pin_id })))
    }
```

with the view mapper (in http.rs or views.rs shim, matching where neighboring `From` impls live):

```rust
fn pin_to_view(p: crate::db::models::AcquisitionPin) -> elohim_views::acquisition::PinView {
    elohim_views::acquisition::PinView {
        id: p.id,
        agent_pub_key: p.agent_pub_key,
        head_ref: p.head_ref,
        kind: p.kind,
        closure_rule: p.closure_rule_json.as_deref().and_then(|s| serde_json::from_str(s).ok()),
        priority: p.priority,
        status: p.status,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}
```

Add the tiny accessor on the p2p node type: `pub async fn acquisition_per_pin(&self) -> Vec<acquisition::PinPullStatus> { self.acquisition.per_pin().await }`.

- [ ] **Step 4: Run the integration test + regenerate bindings**

```bash
cargo test --test acquisition_pins_http 2>&1 | tail -5
cargo test export_bindings 2>&1 | tail -2   # run in elohim-views too: cd ../elohim-views && cargo test export_bindings
```
Expected: all assertions pass; `PinView.ts` / `CreatePinInputView.ts` generated.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-views/src/ elohim/elohim-storage/src/http.rs \
  elohim/elohim-storage/tests/acquisition_pins_http.rs elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): /api/v1/pins routes — own-node only, airplane-mode property tested (spec §4.4)"
```

---

### Task 7: `wait-for-pull.ts` (seeder tooling)

**Files:**
- Create: `genesis/seeder/src/wait-for-pull.ts`
- Test: `genesis/seeder/src/wait-for-pull.spec.ts` (mirror the existing wait-for-drain test if one exists — `ls genesis/seeder/src/*.spec.ts`; if drain has no spec, add a minimal one for pull only)

- [ ] **Step 1: Implementation (mirror wait-for-drain.ts byte-for-byte in structure)**

```typescript
/**
 * waitForPull — poll elohim-storage /p2p/status until the acquisition pull
 * queue is caught up (spec 2026-06-07-epr-acquisition-pull-queue-design §4.3).
 *
 * Tri-state contract (shared with waitForDrain):
 *   pull === null|undefined  → "cannot compute" → KEEP WAITING, never success.
 *   Termination: pull !== null
 *     && pull.total >= expectedMinTotal   (concrete-count guard — a zero-link
 *                                          desired set must not false-complete)
 *     && pull.pending === 0
 *   All conditions on the SAME poll response.
 */

import type { P2PStatusInfo, PullStatusInfo } from '@elohim/storage-client';

export interface WaitForPullOptions {
  /** Total wait budget before throwing. Default: 5 minutes. */
  timeoutMs?: number;
  /** Poll interval. Default: 2 seconds. */
  pollIntervalMs?: number;
  /** Expected minimum desired-set size. Default: 1. */
  expectedMinTotal?: number;
}

export async function waitForPull(
  baseUrl: string,
  options: WaitForPullOptions = {},
): Promise<PullStatusInfo> {
  const timeoutMs = options.timeoutMs ?? 5 * 60_000;
  const pollIntervalMs = options.pollIntervalMs ?? 2_000;
  const expectedMinTotal = options.expectedMinTotal ?? 1;

  const url = `${baseUrl.replace(/\/$/, '')}/p2p/status`;
  const deadline = Date.now() + timeoutMs;
  let lastLoggedPending = -1;
  let consecutiveFetchErrors = 0;
  let lastSeen: PullStatusInfo | null = null;

  console.log(
    `waitForPull: polling ${url} every ${pollIntervalMs}ms (timeout ${timeoutMs}ms, expectedMinTotal=${expectedMinTotal})`,
  );

  while (Date.now() < deadline) {
    try {
      const resp = await fetch(url);
      if (!resp.ok) {
        consecutiveFetchErrors++;
        if (consecutiveFetchErrors >= 10) {
          throw new Error(`waitForPull: /p2p/status returned ${resp.status} after 10 attempts`);
        }
      } else {
        consecutiveFetchErrors = 0;
        const status = (await resp.json()) as P2PStatusInfo;
        const pull = status.pull;
        if (pull === null || pull === undefined) {
          console.log('waitForPull: pull=null (state unavailable), waiting...');
        } else {
          lastSeen = pull;
          if (pull.pending !== lastLoggedPending) {
            console.log(
              `waitForPull: ${pull.fetched}/${pull.total} fetched, ${pull.pending} pending, ${pull.failed} failed`,
            );
            lastLoggedPending = pull.pending;
          }
          if (pull.pending === 0 && pull.total >= expectedMinTotal) {
            console.log(`waitForPull: complete — ${pull.fetched}/${pull.total} fetched`);
            return pull;
          }
        }
      }
    } catch (err) {
      consecutiveFetchErrors++;
      if (consecutiveFetchErrors >= 10) {
        throw new Error(`waitForPull: fetch failed 10 times in a row: ${err}`);
      }
    }
    await new Promise(resolve => setTimeout(resolve, pollIntervalMs));
  }

  const lastState = lastSeen
    ? `last seen pull: total=${lastSeen.total}, fetched=${lastSeen.fetched}, pending=${lastSeen.pending}, failed=${lastSeen.failed}`
    : 'pull was null throughout';
  throw new Error(`waitForPull: did not complete within ${timeoutMs}ms — ${lastState}`);
}
```

- [ ] **Step 2: Typecheck + seeder tests**

```bash
cd /projects/elohim/genesis/seeder && pnpm exec tsc --noEmit 2>&1 | tail -3 && pnpm test 2>&1 | tail -3
```
Expected: clean compile (the generated `PullStatusInfo` type from Task 5 resolves); tests pass.

- [ ] **Step 3: Commit**

```bash
git add genesis/seeder/src/wait-for-pull.ts genesis/seeder/src/wait-for-pull.spec.ts
git commit -m "feat(seeder): wait-for-pull — tri-state poller sharing the wait-for-drain contract (spec §4.4)"
```

---

### Task 8: ladder rungs 2–3 on the link surface (Angular host)

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/acquisition.service.ts`
- Modify: `app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts`
- Test: `app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.spec.ts` (extend),
  `app/elohim-app/src/app/elohim/services/acquisition.service.spec.ts` (new)

> Elements gospel: `<elohim-epr-link>` is NOT modified. Menu composition + handling are host-side.
> `ContextMenuItem` stays `{id, label, disabled?}` — no progress bolted on (spec §8).

- [ ] **Step 1: AcquisitionService (capability detection + thin calls)**

```typescript
// app/elohim-app/src/app/elohim/services/acquisition.service.ts
/**
 * Acquisition affordances (spec 2026-06-07-epr-acquisition-pull-queue-design §8).
 * Thin service: capability detection + the rung-3 download disposition.
 *  - Peer-capable (Tauri-direct :8090): POST a DevicePin to the OWN-NODE pins
 *    API — the acquisition stream pulls + verifies (byte-arrival).
 *  - Browser: warm the SW cache lane by fetching through the normal content
 *    path (no DevicePin object exists in the browser — spec §8).
 * Rung 4 (pin-as-peer/provide) is Slice 2 — NOT here.
 */
import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';

import { StorageClientService } from './storage-client.service';

export type AcquisitionCapability = 'peer' | 'browser';

@Injectable({ providedIn: 'root' })
export class AcquisitionService {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);

  /** Tauri-direct = the storage sidecar answers locally (deployment context 4). */
  capability(): AcquisitionCapability {
    return this.storage.isTauriDirect() ? 'peer' : 'browser';
  }

  /** Rung 3: Download for offline. Returns the disposition taken. */
  async download(eprRef: string): Promise<AcquisitionCapability> {
    if (this.capability() === 'peer') {
      const base = this.storage.getStorageBaseUrl();
      await firstValueFrom(
        this.http.post(`${base}/api/v1/pins`, { headRef: eprRef, kind: 'item' }),
      );
      return 'peer';
    }
    // Browser SW lane: fetch-through warms the cache (apps-sw caches verified
    // responses); no pin object exists in the browser by design.
    const id = eprRef.replace(/^epr:/, '');
    const base = this.storage.getStorageBaseUrl();
    await fetch(`${base}/db/content/${encodeURIComponent(id)}`);
    return 'browser';
  }
}
```

Bind `isTauriDirect()` / `getStorageBaseUrl()` to the REAL methods on `StorageClientService` /
`doorway-connection-strategy.ts` (grep `getStorageBaseUrl` — CLAUDE.md names it; if Tauri detection
has a different accessor, e.g. a deployment-context enum on the connection strategy, use that and
adjust the service accordingly — do not invent a parallel detector).

- [ ] **Step 2: Component spec — failing tests for the new items + dispositions**

Extend `epr-link.component.spec.ts` (mirror the existing `epr-menu-select` dispatch pattern at
lines ~157-200):

```typescript
  it('includes the download action in the menu (rung 3)', () => {
    const litEl = host.querySelector('elohim-epr-link') as ElohimEprLink & {
      contextMenuItems: { id: string; label: string }[];
    };
    expect(litEl.contextMenuItems.some(i => i.id === 'download')).toBe(true);
  });

  it('routes download selection to AcquisitionService', () => {
    const acquisition = TestBed.inject(AcquisitionService);
    const spy = vi.spyOn(acquisition, 'download').mockResolvedValue('browser');
    host.dispatchEvent(
      new CustomEvent('epr-menu-select', {
        detail: { id: 'download', epr: 'epr:test-1' },
        bubbles: true,
      }),
    );
    expect(spy).toHaveBeenCalledWith('epr:test-1');
  });

  it('emits open-in navigation through EprNavService for cross-bundle targets', () => {
    // arrange a resolution whose href is cross-bundle, select 'open-in'
    // assert eprNav.navigate called with the universal href (mirror the
    // existing navigateResolved spec pattern in this file)
  });
```

- [ ] **Step 3: Component changes**

In `epr-link.component.ts`:

```typescript
// imports
import { AcquisitionService } from '../../services/acquisition.service';

// inject (next to eprNav)
private readonly acquisition = inject(AcquisitionService);

// fullActionList: insert after 'copy' (keep the three built-ins FIRST —
// the de-@wip'd context-menu scenario asserts their presence):
        { id: 'open-in', label: 'Open in app' },
        { id: 'download', label: 'Download for offline' },

// handleMenuSelect: new cases before the governance block
      case 'open-in':
        // Rung 2: route via the universal address — in-bundle when claimed,
        // cross-bundle full-load otherwise (claims table IS the router map,
        // conformance spec §7.5; navigateResolved already implements both arms).
        this.navigateToResource(epr);
        break;
      case 'download':
        // Rung 3 (spec §8): disposition by capability; CID-verify happens at
        // the substrate (peer) or Loader/SW (browser) — never here.
        void this.acquisition.download(epr).catch(() => {
          // download failure is non-fatal UI-side; the pins API / queue owns
          // retry semantics (R-E). Surface via console only in slice 1.
          console.warn('[EprLink] download failed for', epr);
        });
        break;
```

(Slice-1 honest cut for rung 2: `open-in` resolves through the SAME claims-driven
`navigateResolved` path the `open` action uses — the §7.5 *per-pillar enumerated* menu
(`Open in Lamad` / `Open in Shefa` …) needs the grant-claims table exposed to the client and lands
with the Slice-3 `contentToSync` consumer work. One generic `open-in` that honors claims-routing is
the rung-2 floor; note this in the commit message.)

- [ ] **Step 4: Run the component + service specs**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts src/app/elohim/components/epr-link src/app/elohim/services/acquisition.service.spec.ts 2>&1 | tail -5
```
Expected: all pass, including the three pre-existing menu assertions (open/about/copy first).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-link/ app/elohim-app/src/app/elohim/services/acquisition.service.*
git commit -m "feat(app): ladder rungs 2-3 — open-in + download menu actions, AcquisitionService capability disposition (spec §8)"
```

---

### Task 9: a2o scenarios (story-first) + two-node pull e2e

**Files:**
- Create: `genesis/a2o/features/delivery/acquisition-pins.feature`
- Create: `elohim/elohim-storage/tests/acquisition_pull_e2e.rs` (two-node in-process; mirror the harness style of `tests/epr_atom_federation_integration.rs` / `tests/harness`)

- [ ] **Step 1: Feature file (scenarios 1–3 of spec §11 in slice-1 form)**

```gherkin
Feature: Acquisition pins — the device pin and the pull queue (slice 1)
  The device pin is the airplane-mode floor (spec §1.1): declarable with no
  hub, no conductor, no peers. The pull queue satisfies pins by byte-arrival,
  never inventory-arrival (R-A).

  # API-level — runs against a storage node with p2p disabled (airplane mode)
  Scenario: A pin is creatable and durable with no network at all
    Given a storage node running without p2p
    When I POST a pin for "epr:strawberry-guide" to /api/v1/pins
    Then the response status is 201
    And GET /api/v1/pins lists one active pin for "epr:strawberry-guide"
    And the pin survives a node restart

  Scenario: Cluster pins are honestly refused until the closure resolver lands
    Given a storage node running without p2p
    When I POST a pin with kind "cluster" to /api/v1/pins
    Then the response status is 501
    And the response names the slice-3 closure resolver

  # Two-peer — byte-arrival parity (requires the second household node fixture)
  @requires:household-nodes
  Scenario: A pin completes only when verified bytes land on disk
    Given two connected storage peers where only peer A holds "epr:strawberry-guide"
    When peer B pins "epr:strawberry-guide"
    And the pull queue drains
    Then peer B's /p2p/status pull shows fetched 1 of total 1
    And the content row exists in peer B's local projection
    And a zero-item desired set never reports caught up with total 0 satisfied
```

Step definitions: extend `genesis/a2o/steps/` following the existing storage-API step pattern
(grep `steps/stewardship.steps.ts` for the API-call style). The two-peer scenario MAY stay `@wip`
in this slice if the cucumber fixture lacks a second node — the Rust e2e below is the binding
regression; note which in the commit.

- [ ] **Step 2: Two-node Rust e2e (the binding byte-arrival test)**

`tests/acquisition_pull_e2e.rs` — copy the two-node bootstrap from the closest existing
integration (`epr_atom_federation_integration.rs` or `back_prop_record_predecessor_announce_e2e.rs`),
then:

1. Seed node A's content table with one row (`bulk_create_content`, id `pull-e2e-1`).
2. On node B: insert an active pin for `pull-e2e-1` via `db::acquisition_pins::upsert_pin`.
3. Drive node B's loop (call `run_acquisition_reconcile()` + `drain_acquisition_queue()` directly,
   or wait on the intervals with a generous timeout, matching the sibling test's driving style).
4. Assert: B's content projection gains `pull-e2e-1` (byte-arrival), and
   `acquisition.rollup()` reads `{total:1, fetched:1, pending:0, caught_up:true}`.
5. Negative guard: a pin for a nonexistent id ends `pending:0/fetched:0` ONLY via retry exhaustion
   (`failed:1`), never `caught_up` with phantom fetched.

```bash
cargo test --test acquisition_pull_e2e 2>&1 | tail -5
```
Expected: pass (allow ~60s budget; mirror sibling timeouts).

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/features/delivery/acquisition-pins.feature genesis/a2o/steps/ \
  elohim/elohim-storage/tests/acquisition_pull_e2e.rs
git commit -m "test(acquisition): a2o pin scenarios + two-node byte-arrival e2e (spec §11 #1-#3)"
```

---

### Task 10: full gates + ledger close-out

- [ ] **Step 1: Rust gates (storage)**

```bash
cd /projects/elohim/elohim/elohim-storage
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev
cargo fmt --check
cargo clippy -- -D warnings 2>&1 | tail -3
cargo test --lib 2>&1 | tail -3
cargo test --test schema_contract --test acquisition_pins_http --test acquisition_pull_e2e 2>&1 | tail -3
cd ../elohim-views && cargo fmt --check && cargo clippy -- -D warnings 2>&1 | tail -3
```
Expected: all clean. (Plain `cargo test` — no nextest in this container.)

- [ ] **Step 2: TS gates**

```bash
cd /projects/elohim/app/elohim-app && pnpm run lint 2>&1 | tail -3 && pnpm exec vitest run --config vite.config.ts src/app/elohim 2>&1 | tail -3
cd /projects/elohim/genesis/seeder && pnpm exec tsc --noEmit && pnpm test 2>&1 | tail -3
```
Expected: clean. NOTE the full elohim-app eslint baseline is 603 errors latent (backlog) — `pnpm run lint`
scope per the project config; do not attempt a baseline drive-down here.

- [ ] **Step 3: Schema codegen freshness**

```bash
cd /projects/elohim && pnpm run schema:codegen:ts 2>&1 | tail -3 && git status --short elohim/sdk | head
```
Expected: no unexpected diffs beyond the committed generated files (Prettier oscillation on
Reach/ContentFormat is known-cosmetic — leave untouched if it appears).

- [ ] **Step 4: Flip the gap-items**

Edit `.claude/memory-kit/gap-items/specs__2026-06-07-epr-acquisition-pull-queue-design.json`:
items #1–#6 `state: OPEN → CLAIMED` (a checked box is a claim — verification happens via the
delivery loop, never self-asserted as done). Then:

```bash
python3 .claude/scripts/memory-kit/placement-audit.py --ledger | head -12
```

- [ ] **Step 5: Final commit (work rides the next dispatch; integrator owns push)**

```bash
git add -A -- elohim/elohim-storage elohim/elohim-views elohim/sdk genesis/seeder genesis/a2o app/elohim-app
git commit -m "feat(acquisition): slice 1 complete — pull queue + DevicePin + .pull wire + rungs 2-3 (spec §13 slice-1; gaps #1-#6 claimed)"
```

---

## Self-review checklist (run before handing off)

1. **Spec coverage**: §1.1 DevicePin (T3/T6), §4.1 rails (T1/T2), §4.2 stream (T4/T5), §4.3
   tri-state + unified vocab (T4/T5/T7), §4.4 wire (T5/T6/T7), §8 rungs 2–3 (T8), §10 honest
   failures (T4/T9), §11 scenarios 1–3 (T9). NOT in slice 1 (by design): §1.2/§6 (slice 2),
   §1.3/§7 (follow-on), §5 (slice 3), §9 (follow-on spec).
2. **Type consistency**: `PullStatusInfo{total,fetched,pending,failed,caught_up}` identical in
   acquisition.rs / schema json / wait-for-pull.ts. `GapTracker` API used by both streams matches
   Task 1's definition. `NewAcquisitionPin` fields match the migration columns.
3. **Adjust-to-reality clauses**: harness bootstrap (T6/T9), `test_conn` helper (T3),
   `isTauriDirect` accessor (T8), `json_response/json_error` helpers (T6) — each says "mirror the
   sibling, don't invent." These are deliberate: the implementer binds to the real codebase idiom
   at those four seams.
