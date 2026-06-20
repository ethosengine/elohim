---
title: "Operational-Weave Lens — Implementation Plan (Wave A of the Weave Epic)"
id: operational-weave-lens-plan
status: Draft
class: protocol-canonical
domain: D5
sprint: weave-epic-wave-a
cites:
  - operational-weave-facing-lens-design | the charter this plan implements end-to-end (its 4-slice sequence) | sha256:fc432fea065dca00 | path: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md
  - weave-epic-arc-design | the epic this plan is Wave A / #0 of (the lens that seeds the arc) | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - light-up-topology-operational-visibility-arc | operational-visibility precedent (landed & evolved) this dual-projection composes with | sha256:638498c0762573c3 | path: genesis/docs/content/elohim-protocol/history/2026-06-02-light-up-topology-operational-visibility-arc.md
# Mixed-env (gap-granular): NO doc-level requires_env so Slice 1 (DB-free proof gate) stays fair-game.
# Slices 2-4 loaders need household-nodes; the gauge half needs observability — both AVAILABLE now.
# The in-repo green test is each slice's deliverable; live-alpha lighting is operator-owned.
---

# Operational-Weave Lens Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Light the per-cluster operational-weave *eyes* — a read-only facing that projects placement-gap, RS-coverage, capacity, and tier/region occupancy as both a typed `WeaveView` (`GET /api/v1/weave`) and Prometheus gauges, from one set of pure folds.

**Architecture:** A new `operational_weave` fold module in the pure `elohim-facings` crate (deps = `elohim-views` + std only, no diesel) holds DB-free folds over DB-free mirror rows; a new `elohim-storage/src/services/operational_weave_facing.rs` holds the impure loaders + two thin adapters that call the *same* folds and emit two wire shapes (the JSON view and `GAUGE.set()`). This is the §11 add-a-lens recipe (`elohim/elohim-facings/CLAUDE.md`), exactly as the `epr_content` and `resiliency` facings already do it.

**Tech Stack:** Rust (native build), `elohim-facings` pure crate, `prometheus` (`IntGauge`/`IntGaugeVec` via `lazy_static`), ts-rs codegen, axum-style HTTP in `elohim-storage/src/http.rs`.

## Global Constraints

- **Build env (native; the ambient WASM getrandom flag breaks linking):** every cargo invocation in this plan runs as `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv cargo …` (the `/tmp` target dodges the pool-slot fingerprint-ENOENT; `RUSTC_WRAPPER=""` dodges the sccache null-byte; disk is at 85%).
- **`elohim-facings` purity is the dependency graph, not a lint:** the crate has NO `diesel`, NO `elohim-storage`. A fold that writes `use diesel;` fails to compile. Define every DB-free mirror `Row` struct INSIDE `folds/operational_weave.rs` (not in `relation.rs`).
- **Not-selected-field contract:** every per-lens optional field on a View type carries `#[serde(default, skip_serializing_if = "Option::is_none")]` + `#[ts(optional)]` (missing ≡ not-selected, never a present `null`).
- **Combinators return `BTreeMap`** (`crate::fold::bucket_by`/`distinct_count_by`) — deterministic; never hand-roll iteration a combinator composes.
- **Never `set()` a gauge inside a fold.** Folds return numbers; the storage adapter calls the fold, then `GAUGE.set()`s. One fold → two projections (JSON + gauge).
- **P2P-gate verdict is settled (do not re-open):** Operational-C, **zero new DHT entry types**, `agent_cid` is the sole join key. ⚠ `shard_locations.peer_id` *holds* `agent_cid` (misnamed column — same trap as `household_resilience.rs:74`).
- **Route-shadow guard:** a new `GET` route MUST be added to BOTH the match arm AND `is_service_path` in `http.rs`, or the EPR router shadows it into the SPA bundle (the `/auth/portal` incident). Add a routing unit test.
- **Branch `feat/frontend-eyes-sprint`, commit-only** (the integrator pushes; never `git push`).
- **Per-slice env:** Slice 1 is DB-free (no env). Slices 2–4's loaders need `household-nodes` data; the gauge half needs `observability`. Live-alpha lighting is operator-owned — each slice's *in-repo green test* is the deliverable here, not a live metric.

## Non-goals (verbatim from the charter)

Pantry/storage-temperature occupancy (no source table); PVC/disk-pressure (stays CI/hook-side, `pool-policy.json`); fabricating `shard_locations` rows; the boot self-session; deploy/reseed; a `Lens` trait / Cozo / untyped envelope. Slice 1 is the in-repo green proof, distinct from live-alpha lighting.

## File Structure

- **Create** `elohim/elohim-facings/src/folds/operational_weave.rs` — the pure folds + their DB-free mirror rows (`PlacementGapRow`/`CustodianRow` mirrors) + unit tests.
- **Modify** `elohim/elohim-facings/src/folds/mod.rs` — add `pub mod operational_weave;`.
- **Create** `elohim/elohim-storage/src/services/operational_weave_facing.rs` — impure loaders (`load_*_relation`, `&mut conn`) + the two adapters (JSON view builder + gauge emitter).
- **Modify** `elohim/elohim-storage/src/services/mod.rs` — register the new service module.
- **Modify** `elohim/elohim-storage/src/metrics.rs` — add the `elohim_*` gauges + register them in `register_all()`.
- **Modify** `elohim/elohim-views/src/infrastructure.rs` — add the cluster-scoped `WeaveView` (ts-rs).
- **Modify** `elohim/elohim-storage/src/http.rs` — `GET /api/v1/weave` match arm + `is_service_path` arm + routing unit test; wire the dead `api/peer_capacity.rs` handler.
- **Modify** `elohim/sdk/schemas/scripts/codegen-ts.mjs` `INTERFACE_FILES` + run `pnpm run schema:codegen:ts` (Slice 2, when `WeaveView` first ships).

---

### Task 1: Slice 1 — `placement_gap_count` pure fold (DB-free proof gate)

**Files:**
- Create: `elohim/elohim-facings/src/folds/operational_weave.rs`
- Modify: `elohim/elohim-facings/src/folds/mod.rs`
- Test: inline `#[cfg(test)] mod tests` in `operational_weave.rs`

**Interfaces:**
- Consumes: `elohim_views::PlacementGapView` (fields: `id, content_id, shard_hash, requested_steward_count: i32, achieved_steward_count: i32, contract_coverage: f32, gap_kind: String, first_seen_at, last_seen_at`), `crate::fold::bucket_by`.
- Produces: `pub fn placement_gap_count(gaps: &[PlacementGapView]) -> usize`; `pub fn gaps_by_kind(gaps: &[PlacementGapView]) -> std::collections::BTreeMap<String, usize>`.

> Note: `PlacementGapView` is already a plain serde/TS view (no diesel) in `elohim-views`, so the fold consumes it directly — no mirror row needed for this fold (the mirror-row pattern from `epr_content.rs` is needed in Task 4 where the source is a diesel row).

- [ ] **Step 1: Write the failing test**

```rust
// elohim/elohim-facings/src/folds/operational_weave.rs
//! The operational facing's folds — per-shard → per-node → per-cluster "weave"
//! health, folded from shard-placement + custodian-capacity relations. Pure
//! (DB-free); the diesel rows live in elohim-storage and are mirrored before they
//! reach these folds (the §11 add-a-lens recipe, elohim/elohim-facings/CLAUDE.md).
//! Charter: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md

use std::collections::BTreeMap;

use elohim_views::PlacementGapView;

/// Count the open placement gaps (one row = one under-replicated shard).
pub fn placement_gap_count(gaps: &[PlacementGapView]) -> usize {
    gaps.len()
}

/// Group the gaps by `gap_kind`, counting per kind. `BTreeMap` → deterministic
/// wire order if a caller serializes it.
pub fn gaps_by_kind(gaps: &[PlacementGapView]) -> BTreeMap<String, usize> {
    crate::fold::bucket_by(gaps, |g| Some(g.gap_kind.clone()))
        .into_iter()
        .map(|(kind, rows)| (kind, rows.len()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(kind: &str) -> PlacementGapView {
        PlacementGapView {
            id: format!("gap-{kind}"),
            content_id: "c".into(),
            shard_hash: "s".into(),
            requested_steward_count: 3,
            achieved_steward_count: 1,
            contract_coverage: 0.33,
            gap_kind: kind.into(),
            first_seen_at: "t0".into(),
            last_seen_at: "t1".into(),
        }
    }

    #[test]
    fn placement_gap_count_counts_rows() {
        let gaps = vec![gap("under_replicated"), gap("unplaced"), gap("under_replicated")];
        assert_eq!(placement_gap_count(&gaps), 3);
        assert_eq!(placement_gap_count(&[]), 0);
    }

    #[test]
    fn gaps_by_kind_buckets_deterministically() {
        let gaps = vec![gap("under_replicated"), gap("unplaced"), gap("under_replicated")];
        let by_kind = gaps_by_kind(&gaps);
        assert_eq!(by_kind.get("under_replicated"), Some(&2));
        assert_eq!(by_kind.get("unplaced"), Some(&1));
        // BTreeMap iteration is sorted → first key is "under_replicated" < "unplaced"? no: 'un' tie,
        // 'd' < 'p' so "under_replicated" sorts first — deterministic regardless.
        let keys: Vec<&String> = by_kind.keys().collect();
        assert_eq!(keys, vec!["under_replicated", "unplaced"]);
    }
}
```

- [ ] **Step 2: Register the module — modify `folds/mod.rs`**

Add after the existing `pub mod epr_content;`:

```rust
pub mod operational_weave;
```

- [ ] **Step 3: Run the tests to verify they pass**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv cargo test --manifest-path elohim/elohim-facings/Cargo.toml operational_weave`
Expected: PASS (2 tests). If `PlacementGapView` is not yet re-exported from the `elohim_views` crate root, import it from its module path (`elohim_views::infrastructure::PlacementGapView`) — confirm with `grep "pub use.*PlacementGapView\|pub struct PlacementGapView" elohim/elohim-views/src/lib.rs elohim/elohim-views/src/infrastructure.rs`.

- [ ] **Step 4: Clippy gate**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv cargo clippy --manifest-path elohim/elohim-facings/Cargo.toml --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-facings/src/folds/operational_weave.rs elohim/elohim-facings/src/folds/mod.rs
git commit -m "feat(facings): operational-weave placement_gap_count fold (Slice 1, DB-free)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Slice 1 — the `elohim_placement_gap_count` gauge + adapter (second projection)

**Files:**
- Modify: `elohim/elohim-storage/src/metrics.rs` (add gauge + register)
- Create: `elohim/elohim-storage/src/services/operational_weave_facing.rs` (the adapter)
- Modify: `elohim/elohim-storage/src/services/mod.rs` (register module)
- Test: inline test in `operational_weave_facing.rs`

**Interfaces:**
- Consumes: `elohim_facings::folds::operational_weave::placement_gap_count`, `crate::metrics::ELOHIM_PLACEMENT_GAP_COUNT`.
- Produces: `pub fn emit_placement_gap_gauge(gaps: &[PlacementGapView])` — calls the fold, then `.set()`s the gauge. (The adapter, NOT the fold, touches the gauge.)

- [ ] **Step 1: Add the gauge to `metrics.rs`**

Inside the existing `lazy_static! { … }` block (mirror `NODE_DB_MAX_READERS: IntGauge` at `metrics.rs:66`):

```rust
    pub static ref ELOHIM_PLACEMENT_GAP_COUNT: IntGauge = IntGauge::new(
        "elohim_placement_gap_count",
        "Open shard-placement gaps (under-replicated shards) across the cluster weave",
    )
    .expect("valid gauge");
```

In `register_all()` (after the existing `REGISTRY.register(...)` lines, ~`metrics.rs:166`):

```rust
        let _ = REGISTRY.register(Box::new(ELOHIM_PLACEMENT_GAP_COUNT.clone()));
```

- [ ] **Step 2: Write the failing adapter test**

```rust
// elohim/elohim-storage/src/services/operational_weave_facing.rs
//! Operational-weave facing — impure loaders + the two adapters that project the
//! pure folds (elohim_facings::folds::operational_weave) as a typed WeaveView AND
//! as Prometheus gauges. The fold returns numbers; the adapter emits both wire
//! shapes. Charter: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md

use elohim_facings::folds::operational_weave::placement_gap_count;
use elohim_views::PlacementGapView;

/// Adapter: fold the gaps, then publish the count to the /metrics gauge.
/// NEVER call `.set()` inside the fold — the fold is pure; this adapter is the
/// only place a gauge is touched.
pub fn emit_placement_gap_gauge(gaps: &[PlacementGapView]) {
    let count = placement_gap_count(gaps);
    crate::metrics::ELOHIM_PLACEMENT_GAP_COUNT.set(count as i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap() -> PlacementGapView {
        PlacementGapView {
            id: "g".into(), content_id: "c".into(), shard_hash: "s".into(),
            requested_steward_count: 3, achieved_steward_count: 1, contract_coverage: 0.33,
            gap_kind: "under_replicated".into(), first_seen_at: "t0".into(), last_seen_at: "t1".into(),
        }
    }

    #[test]
    fn emit_sets_the_gauge_to_the_fold_count() {
        crate::metrics::register_all(); // idempotent (Once-guarded)
        emit_placement_gap_gauge(&[gap(), gap()]);
        assert_eq!(crate::metrics::ELOHIM_PLACEMENT_GAP_COUNT.get(), 2);
        emit_placement_gap_gauge(&[]);
        assert_eq!(crate::metrics::ELOHIM_PLACEMENT_GAP_COUNT.get(), 0);
    }
}
```

- [ ] **Step 3: Register the module — modify `services/mod.rs`**

Add (match the existing `pub mod household_resilience;` style):

```rust
pub mod operational_weave_facing;
```

- [ ] **Step 4: Run the test (native env — this is the storage crate, NOT the WASM workspace)**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv cargo test --manifest-path elohim/elohim-storage/Cargo.toml operational_weave_facing`
Expected: PASS. (If the storage crate's default test build is heavy, scope with `--lib`.)

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/metrics.rs elohim/elohim-storage/src/services/operational_weave_facing.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): elohim_placement_gap_count gauge + weave adapter (Slice 1)

One fold, two projections — the adapter sets the gauge, never the fold.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Slice 2 — `rs_coverage` fold + `WeaveView` + ts-rs codegen + gauge

**Files:**
- Modify: `elohim/elohim-facings/src/folds/operational_weave.rs` (add `rs_coverage`)
- Modify: `elohim/elohim-views/src/infrastructure.rs` (add `WeaveView`)
- Modify: `elohim/elohim-storage/src/metrics.rs` (add `elohim_rs_coverage` gauge, scaled ×1000 — gauges are integers)
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (`INTERFACE_FILES`) + run codegen
- Test: inline fold test + a ts-rs export assertion

**Interfaces:**
- Consumes: `PlacementGapView.contract_coverage: f32`, `achieved_steward_count`, `requested_steward_count`.
- Produces: `pub fn rs_coverage(gaps: &[PlacementGapView]) -> f32` (mean `contract_coverage`, `1.0` when empty = fully covered); `WeaveView { placement_gap_count: u32, rs_coverage: Option<f32>, … }`.

- [ ] **Step 1: Write the failing fold test (append to `operational_weave.rs` tests)**

```rust
    #[test]
    fn rs_coverage_is_mean_contract_coverage_and_empty_is_full() {
        let mut a = gap("under_replicated"); a.contract_coverage = 0.5;
        let mut b = gap("under_replicated"); b.contract_coverage = 1.0;
        assert!((super::rs_coverage(&[a, b]) - 0.75).abs() < 1e-6);
        assert_eq!(super::rs_coverage(&[]), 1.0, "no gaps ⇒ fully covered");
    }
```

- [ ] **Step 2: Implement `rs_coverage` (append to `operational_weave.rs`)**

```rust
/// Mean RS contract-coverage across the open gaps (`PlacementGapView.contract_coverage`
/// already = achieved/requested). Empty ⇒ 1.0 (no gaps means nothing under-covered).
pub fn rs_coverage(gaps: &[PlacementGapView]) -> f32 {
    if gaps.is_empty() {
        return 1.0;
    }
    gaps.iter().map(|g| g.contract_coverage).sum::<f32>() / gaps.len() as f32
}
```

- [ ] **Step 3: Add `WeaveView` to `elohim-views/src/infrastructure.rs`** (mirror the `#[derive(TS)]` + `export_to` of the sibling views at `:1433-1436`; not-selected-field contract on every fold-optional field)

```rust
/// Cluster-scoped operational weave projection: placement/coverage/capacity/occupancy
/// folded per-shard → per-node → per-cluster. Each lens field is OPTIONAL — a facing
/// carries only the lenses it selected (the not-selected-field contract).
///
/// Wire format: `weave-view.schema.json` (added in Slice 4).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct WeaveView {
    pub placement_gap_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rs_coverage: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cluster_capacity: Option<ComputeTriptych>, // populated in Slice 3
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tier_occupancy: Option<RiskTierDistribution>, // Slice 4
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub region_occupancy: Option<RegionalDistributionView>, // Slice 4
    pub measured_at: String,
}
```

- [ ] **Step 4: Add the gauge (`metrics.rs`, scaled — Prometheus IntGauge is integer)**

```rust
    pub static ref ELOHIM_RS_COVERAGE_MILLI: IntGauge = IntGauge::new(
        "elohim_rs_coverage_milli",
        "Mean RS contract-coverage across open gaps, ×1000 (1000 = fully covered)",
    )
    .expect("valid gauge");
```
Register it in `register_all()`; in the adapter, `ELOHIM_RS_COVERAGE_MILLI.set((rs_coverage(&gaps) * 1000.0) as i64)`.

- [ ] **Step 5: Regenerate ts-rs bindings + run the contract**

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings` (the ts-rs harness uses the WASM-flagged workspace — this is the one place the custom backend flag is correct, per root CLAUDE.md). Then add `WeaveView` to `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs` and run `pnpm run schema:codegen:ts`.
Expected: a new `WeaveView.ts` under `elohim/sdk/storage-client-ts/src/generated/`, with `rsCoverage?: number` (NOT `number | null`). Verify: `grep "rsCoverage" elohim/sdk/storage-client-ts/src/generated/WeaveView.ts`.

- [ ] **Step 6: Run fold tests + clippy + commit** (commands as Task 1 Steps 3–4; commit the fold, view, metrics, and generated TS **in one commit** — codegen output goes with its View type).

---

### Task 4: Slice 3 — `node_capacity` + cluster `aggregate` + wire the dead `peer_capacity` handler

**Files:**
- Modify: `elohim/elohim-facings/src/folds/operational_weave.rs` (mirror `CustodianRow` + `node_capacity` + `aggregate_capacity`)
- Modify: `elohim/elohim-storage/src/services/operational_weave_facing.rs` (`load_custodian_relation` loader + adapter populates `cluster_capacity`)
- Modify: `elohim/elohim-storage/src/http.rs` (route the dead `api/peer_capacity.rs` handler)
- Test: fold tests + a loader smoke test

**Interfaces:**
- Consumes: `custodian_metrics` + `system_metrics` diesel rows (mirror to `CustodianRow { agent_cid: String, free: Option<u64>, used: Option<u64>, stewarded: Option<u64> }` in the fold file).
- Produces: `pub fn node_capacity(c: &CustodianRow) -> ComputeTriptych`; `pub fn aggregate_capacity(rows: &[CustodianRow]) -> ComputeTriptych` (per-field `Option`-summing rollup — `None`s skipped; all-`None` ⇒ `None`).

- [ ] **Step 1: Write the failing aggregate test (append to `operational_weave.rs` tests)**

```rust
    fn cust(free: Option<u64>, used: Option<u64>, stewarded: Option<u64>) -> super::CustodianRow {
        super::CustodianRow { agent_cid: "a".into(), free, used, stewarded }
    }

    #[test]
    fn aggregate_capacity_sums_per_field_skipping_none() {
        let rows = vec![cust(Some(10), Some(5), None), cust(Some(20), None, Some(3))];
        let t = super::aggregate_capacity(&rows);
        assert_eq!(t.free, Some(30));
        assert_eq!(t.used, Some(5));       // second row's used is None → skipped
        assert_eq!(t.stewarded, Some(3));
    }

    #[test]
    fn aggregate_capacity_all_none_is_none() {
        let t = super::aggregate_capacity(&[cust(None, None, None)]);
        assert!(t.free.is_none() && t.used.is_none() && t.stewarded.is_none());
    }
```

- [ ] **Step 2: Implement the mirror row + folds**

```rust
use elohim_views::ComputeTriptych;

/// DB-free mirror of a custodian's capacity (custodian_metrics ⨝ system_metrics),
/// keyed by `agent_cid` (NOT a transport id). The diesel rows are mapped onto this
/// by the storage loader so the folds stay DB-free.
#[derive(Debug, Clone)]
pub struct CustodianRow {
    pub agent_cid: String,
    pub free: Option<u64>,
    pub used: Option<u64>,
    pub stewarded: Option<u64>,
}

pub fn node_capacity(c: &CustodianRow) -> ComputeTriptych {
    ComputeTriptych { free: c.free, used: c.used, stewarded: c.stewarded }
}

/// Per-field Option-sum rollup across nodes. A field is `Some` iff at least one node
/// reported it; `None`s are skipped (an unsampled node doesn't zero the cluster).
pub fn aggregate_capacity(rows: &[CustodianRow]) -> ComputeTriptych {
    fn sum(vals: impl Iterator<Item = Option<u64>>) -> Option<u64> {
        let mut acc: Option<u64> = None;
        for v in vals.flatten() {
            acc = Some(acc.unwrap_or(0) + v);
        }
        acc
    }
    ComputeTriptych {
        free: sum(rows.iter().map(|r| r.free)),
        used: sum(rows.iter().map(|r| r.used)),
        stewarded: sum(rows.iter().map(|r| r.stewarded)),
    }
}
```

- [ ] **Step 3: Loader + wire the dead handler.** In `operational_weave_facing.rs` add `load_custodian_relation(conn: &mut SqliteConnection) -> Vec<CustodianRow>` (join `custodian_metrics`/`system_metrics`, map to the mirror row, key on `agent_cid` — the existing `ComputeTriptych` builder in `household_resilience.rs` is the reference join). In `http.rs`, route `api/peer_capacity::handle` (currently returns `METHOD_NOT_ALLOWED` at `:19/:36` because it is unrouted) — add its match arm; confirm it now returns the capacity payload.

- [ ] **Step 4: Run fold tests + clippy + commit** (as Task 1 Steps 3–5; commit message `feat(storage): operational-weave node_capacity rollup + wire peer_capacity (Slice 3)`).

---

### Task 5: Slice 4 — `tier_occupancy` + `region_occupancy` + full `WeaveView` + `GET /api/v1/weave`

**Files:**
- Modify: `elohim/elohim-facings/src/folds/operational_weave.rs` (`tier_occupancy`, `region_occupancy` over a `HolderRow` mirror — reuse `crate::fold::bucket_by`)
- Modify: `elohim/elohim-storage/src/services/operational_weave_facing.rs` (full `WeaveView` builder)
- Modify: `elohim/elohim-storage/src/http.rs` (`GET /api/v1/weave` match arm + `is_service_path` arm + routing unit test)
- Create: `elohim/sdk/schemas/v1/views/weave-view.schema.json` + add to `tests/schema_contract.rs`

**Interfaces:**
- Consumes: the framework's `HolderRow { hub_id, agent_id, region }` (already in `elohim-facings/src/relation.rs`), `crate::fold::bucket_by`.
- Produces: `pub fn tier_occupancy(holders: &[HolderRow]) -> RiskTierDistribution`; `pub fn region_occupancy(holders: &[HolderRow]) -> RegionalDistributionView`; `GET /api/v1/weave -> WeaveView`.

- [ ] **Step 1: Write the failing routing test FIRST (the shadow guard).** In `http.rs` tests, assert `is_service_path("/api/v1/weave")` is `true` and that the dispatch routes to the weave handler, not the SPA/EPR fallthrough (mirror the `is_auth_owned_path` unit test added in `37c822d1c`).

```rust
    #[test]
    fn weave_path_is_service_owned_not_spa_shadowed() {
        assert!(is_service_path("/api/v1/weave"), "weave must be service-owned or the EPR router shadows it");
    }
```

- [ ] **Step 2: Run it — Expected FAIL** (`is_service_path` returns false until the arm is added).

- [ ] **Step 3: Add the `is_service_path` arm + the match arm + the handler** in `http.rs` (the handler calls `operational_weave_facing::build_weave_view(&mut conn)`); add `region_occupancy`/`tier_occupancy` folds (bucket `HolderRow` by `region`/tier, count into the existing `RegionalDistributionView`/`RiskTierDistribution`). Define the `weave-view.schema.json` + add to `INTERFACE_FILES` and `schema_contract.rs` (the 6-step "adding a new view" recipe in root CLAUDE.md).

- [ ] **Step 4: Run routing test (PASS) + fold tests + the schema contract + ts-rs codegen freshness; commit** (`feat(storage): full WeaveView + GET /api/v1/weave (Slice 4)`; commit the schema + generated TS together).

---

## Self-Review

- **Spec coverage:** Slice 1 → Tasks 1–2 (`placement_gap_count` + gauge); Slice 2 → Task 3 (`rs_coverage` + `WeaveView` + codegen); Slice 3 → Task 4 (`node_capacity`/`aggregate` + wire `peer_capacity`); Slice 4 → Task 5 (`tier`/`region` occupancy + route + `is_service_path` + schema). Dual projection (view + gauges) covered in Tasks 2–4. ✓
- **Non-placeholders:** all fold steps carry real test + impl code against verified view shapes; integration glue (gauge register, route arm, codegen) cites the exact reference line/recipe. The `load_*_relation` loaders (Task 4 Step 3) and the full `build_weave_view` (Task 5 Step 3) are described against the `household_resilience.rs` reference rather than reproduced byte-for-byte — they are diesel-join glue the implementer writes against live schema, the one place a verbatim block would go stale.
- **Type consistency:** `placement_gap_count -> usize` (fold) cast to `i64` at the gauge and `u32` on `WeaveView`; `rs_coverage -> f32` (`Option<f32>` on the view); `aggregate_capacity -> ComputeTriptych` matches `WeaveView.cluster_capacity`. `agent_cid` is the join key in `CustodianRow` and the loader. ✓
