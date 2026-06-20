---
title: "Storage serve-routing — score consumer at the race_fetch sites (Wave-3 of the Doorway Membrane arc)"
id: storage-serve-routing-plan
status: Draft
class: protocol-canonical
domain: D1
sprint: doorway-membrane-wave-c
cites:
  - doorway-membrane-prosocial-routing-design | the arc spec this plan implements (Wave-3: capability-aware serve-routing, the score consumer, D1 byte axis) | sha256:10ba2875185c52b0 | path: genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md
  - elohim-peer-fabric-spine-plan | the Wave-1 plan that built the score module this serve-routing consumes | sha256:dd4fe05da91829de | path: genesis/docs/superpowers/plans/2026-06-20-elohim-peer-fabric-spine-plan.md
refines: genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md
# Mixed-env. Household-now: the selection LOGIC + the agent_cid-native serve fold + DbPool-fixture
# integration. @requires:shem (inline, T7 only): live cross-WAN RTT-based ORDERING (household RTT ≈ 0
# ties every peer, so a latency-ordering assertion is vacuous on household). NO doc-level requires_env.
---

# Storage Serve-Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `elohim-storage` choose a *capable* peer (capability × headroom × attested-RTT × bonded × household-diversity × delivery) when it must fetch content bytes it doesn't hold — by consuming the built `elohim-peer-fabric::score` module at the serve-time `race_fetch` candidate sites. This is the D1 byte axis: **storage chooses, never doorway**.

**Architecture:** A new `services/serve_routing.rs::select_serve_peers(pool, blob_hash, n)` invoked at the three `race_fetch` candidate-construction sites. It splits impure `load_serve_rows` (diesel) from a pure `fold_candidates` (no diesel, unit-testable — the `elohim-facings` pattern), folds projection rows into `Vec<score::Candidate>` joined on **`agent_cid`**, calls `score::select_diverse`, and returns the chosen peers. `score` returning empty (all saturated) ⇒ the caller sheds, never fans out (the single-target D1 boundary).

**Tech Stack:** Rust (native, `RUSTFLAGS='--cfg getrandom_backend="custom"'`), diesel/SQLite, `elohim-peer-fabric` (path dep, `serve-routing` feature).

## ⚠ Two headline reversals the recon surfaced (read first)

1. **`distribution_view.rs` is the WRONG home** (the task named it). That file is a Category-C count/ratio view-composer (`replica_target_for`, `compute_projection_tier`) with **no peer-picker and no byte-fetch**. The serve-time "which peer do I ask" decision lives at the **`race_fetch` candidate sites** (`http.rs:2439` primary; `p2p/mod.rs:2754`; `reconcile/custody_sweep.rs:146`), which today build `Vec<String>` from `lookup_hosts(...).map(|r| r.peer_id)` with zero capability/headroom/RTT/diversity input.
2. **The live `libp2p→agent_cid` join is BLOCKED-BY-CODE (not env).** `peer_blob_inventory.peer_id` is libp2p-keyed (`12D3Koo…`); `score::Candidate` + every projection table are `agent_cid`-keyed (`uhCAk…`); the only bridge (`peer_transport_manifest.libp2p_peer_id`) has **only `#[cfg(test)]` writers — empty in prod**. So serve-routing splits: ship the **selection logic + agent_cid-native serve fold (household-now)**; stage the libp2p-path wiring behind a named upstream manifest-population gap.

## Locked design decisions (confirm at review — recon-recommended)

1. **Candidate source = `shard_locations` (agent_cid-native), NOT `peer_blob_inventory` (libp2p).** `shard_locations` (PK `(shard_hash, peer_id)`) is **agent_cid-keyed despite the column name** (storage identity-table convention), so the `agent_cid` join is DIRECT and the live path unblocks NOW — sidestepping the unfed libp2p→agent_cid resolver. The `peer_blob_inventory`+resolver path (Option-a) is the inventory-coverage fallback, staged behind the manifest-population gap. *(This is the one genuine fork — flag for override.)*
2. **Neutral defaults for absent signals** (the score crate already neutralizes): `capability_level` from `stewarded_nodes.capability_level`, NULL → a `MIN_CAP` floor (never panic); `attested_rtt_ms` absent → `None` (crate → 0.5 neutral); `current_load` absent column → `0.0` (full headroom); `delivery_score` absent column → `1.0`. Neutral-everything collapses score toward capability+bond only — honest degradation, documented.
3. **`MIN_CAP` floor** = a named const (start `0`; tune later) so serve-routing never excludes all peers on a NULL capability column.

## Global Constraints

- **Build env:** `RUSTFLAGS='--cfg getrandom_backend="custom"'` (elohim-storage is a WASM-adjacent crate — NOT `""`, that's doorway/steward). `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/elohim__elohim-storage/dev` (frontend-family slot). Build `--lib` to skip iroh/ssr/bench. `run_in_background` for >10min cargo; one profile per gate phase.
- **🚨 DISK GATE (ELEVATED):** `/projects` is at **87% — past the 85% hard ceiling**; the PreToolUse hook DENIES heavy cargo regardless of `CARGO_TARGET_DIR`, and `FORCE_HEAVY_GATES` does NOT bypass it. `elohim-storage` is a near-cold ~925MB compile. **Clear the ceiling before any T3+ build** (operator frees non-pool space, OR the `/tmp` target-dir + `RUSTC_WRAPPER=""` escape for fingerprint/sccache traps — but `/tmp` is the same overlay, so a near-cold storage build there still adds GB). This is `feat/*` (orchestrator doesn't index it) — **the host build IS the gate**.
- **`elohim-storage` is NOT in the `elohim/` workspace** (it's standalone with plain path deps). Add the dep as a plain path.
- **Commit discipline (shared worktree):** path-limited (`git add <paths>` then `git commit -m "…" -- <paths>`), NEVER `-A`/`--amend`-without-HEAD-recheck/`push`.
- **`agent_cid` is the canonical join key** — never raw-compare libp2p/iroh ids; the `shard_locations` source is agent_cid-native so no cross-namespace compare is needed (decision #1).
- **No new DHT entry types / no new schema columns** in this wave — pure consumer over existing projections (the absent `current_load`/`delivery_score` columns degrade to neutral; building those projections is a follow-on).

---

### Task 0: Verify the `custodian_id` namespace (read-only — no build)

**Files:** none (investigation).

- [ ] **Step 1:** grep the POST `custodian_metrics` ingest handler that sets `input.custodian_id` (`api/custodians.rs`, the upsert path near `get_metrics_by_id`) and the writers, to confirm whether `custodian_metrics.custodian_id` holds an `agent_cid` (`uhCAk…`) or a separate custodian namespace.
- [ ] **Step 2:** Record the verdict in the plan/ledger:
  - If `agent_cid` → the `current_load`/`attested_rtt_ms` joins (when those projections are built) key on `agent_cid` directly.
  - If a separate namespace → those two fields stay neutral-defaulted in this wave (no join), and a future task adds the namespace resolution. **Either way this wave ships with neutral load/RTT** (the columns don't exist yet), so T0 only decides a FUTURE mapping — it does not block T1–T6.

---

### Task 1: Pin the `score` contract this wave depends on (pure — light, no storage build)

**Files:** Test in `elohim/elohim-peer-fabric/src/score.rs` (the crate's own test module) OR a doc-test — these assert the serve-routing-relevant contract.

**Interfaces:** Consumes `score::{rank, select_diverse, Candidate, ScoredPeer}` (already built).

- [ ] **Step 1: Add contract tests** (if not already covered by the spine's tests) asserting the serve-routing-relevant behavior over hand-built `Candidate` vecs: (a) `select_diverse` returns ≤ n and spreads households; (b) all-saturated (`current_load >= 1.0`) → empty (caller sheds); (c) capability floor filters. (Most exist from Wave 1 — add only what serve-routing specifically relies on, e.g. an explicit "n=2 from 1 household backfills" case if missing.)
- [ ] **Step 2: Run** (light — score crate only, std-only): `cd elohim/elohim-peer-fabric && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo test` → all pass.
- [ ] **Step 3: Commit** path-limited (if any test added).

---

### Task 2: Pure `fold_candidates` (no diesel — unit-testable without compiling storage)

**Files:**
- Create: `elohim/elohim-storage/src/services/serve_routing.rs` (the `ServeRow` struct + pure `fold_candidates` + `MIN_CAP`)
- Modify: `elohim/elohim-storage/src/services/mod.rs` (`pub mod serve_routing;`)

**Interfaces:**
- Produces: `pub struct ServeRow { pub agent_cid: String, pub household_id: Option<String>, pub capability_level: Option<i32>, pub bonded: bool, pub current_load: Option<f64>, pub attested_rtt_ms: Option<u32>, pub delivery_score: Option<f64> }` and `pub fn fold_candidates(rows: &[ServeRow]) -> Vec<elohim_peer_fabric::score::Candidate>` and `pub const MIN_CAP: u8 = 0;`

- [ ] **Step 1: Write the failing test** (in `serve_routing.rs`, `#[cfg(test)]`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn row(cid: &str, hh: Option<&str>, cap: Option<i32>, bonded: bool) -> ServeRow {
        ServeRow { agent_cid: cid.into(), household_id: hh.map(Into::into), capability_level: cap,
                   bonded, current_load: None, attested_rtt_ms: None, delivery_score: None }
    }
    #[test]
    fn fold_maps_columns_and_neutralizes_absent_signals() {
        let rows = vec![row("uhCAk-a", Some("h1"), Some(5), true), row("uhCAk-b", None, None, false)];
        let cands = fold_candidates(&rows);
        assert_eq!(cands.len(), 2);
        // capability NULL → MIN_CAP floor; absent load → 0.0 (full headroom); absent rtt → None; absent delivery → 1.0
        assert_eq!(cands[1].capability_level, MIN_CAP);
        assert_eq!(cands[1].current_load, 0.0);
        assert_eq!(cands[1].attested_rtt_ms, None);
        assert_eq!(cands[1].delivery_score, 1.0);
        assert_eq!(cands[1].household_id, ""); // None → "" (no false fault-domain grouping)
        assert!(!cands[1].bonded);
        assert_eq!(cands[0].capability_level, 5);
        assert!(cands[0].bonded);
    }
}
```
- [ ] **Step 2: Run** (placed in a leaf module so it compiles light, but elohim-storage is one crate — this needs the storage build; if disk-blocked, defer the run to the T3 batch): `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=<slot> cargo test --lib serve_routing` → FAIL (undefined).
- [ ] **Step 3: Implement** `ServeRow`, `MIN_CAP`, and `fold_candidates` (pure map; `capability_level.unwrap_or(MIN_CAP as i32) as u8` clamped to `u8`; `household_id.unwrap_or_default()`; `current_load.unwrap_or(0.0)`; `delivery_score.unwrap_or(1.0)`; `attested_rtt_ms` passthrough; `bonded` passthrough). NO diesel import in this fn.
- [ ] **Step 4: Run** → PASS. **Step 5: Commit** path-limited.

---

### Task 3: Add the crate dep + feature (first heavy build — clear disk first)

**Files:** Modify `elohim/elohim-storage/Cargo.toml`

- [ ] **Step 1:** Add under `[dependencies]`: `elohim-peer-fabric = { path = "../elohim-peer-fabric", default-features = false, features = ["serve-routing"] }` (`default-features = false` matters — the crate default enables all three role features; storage wants only `serve-routing`, which gates the `score` module).
- [ ] **Step 2 (DISK-GATED):** ensure `/projects` < 85% (operator reclaim), then `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=<slot> cargo build --lib` (`run_in_background`). Expected: resolves + compiles; `use elohim_peer_fabric::score::{Candidate, select_diverse};` available.
- [ ] **Step 3: Commit** `Cargo.toml` (+ `Cargo.lock` if changed) path-limited.

---

### Task 4: Impure `load_serve_rows` (the agent_cid-native multi-table join) — heavy

**Files:** Modify `elohim/elohim-storage/src/services/serve_routing.rs`

**Interfaces:** Produces `pub fn load_serve_rows(conn: &mut SqliteConnection, blob_hash: &str) -> Vec<ServeRow>`.

- [ ] **Step 1: Write the DbPool-fixture test** (template `tests/peer_selection.rs:23-282` — `test_util::test_pool`, seed real column names): seed `shard_locations` (agent_cid-keyed) holding `blob_hash` on 3 nodes across 2 households + `stewarded_nodes.capability_level` + a `humans` household join + an active `rea_commitments` provide for one node; assert `load_serve_rows(blob_hash)` returns 3 rows with the right `agent_cid`/`household_id`/`capability_level`/`bonded`, and `current_load`/`attested_rtt_ms`/`delivery_score` as `None` (no columns yet).
- [ ] **Step 2: Run** (heavy) → FAIL. **Step 3: Implement** the join: `shard_locations` (candidate set via `shard_manifests.content_id→blob_hash` per recon §2, agent_cid-keyed) → `stewarded_nodes`/`node_stewardship`→`humans` (household_id) → `rea_commitments` (bonded = active provide: `provider=agent_cid AND action ∈ {provide,replicates-content,replicates-commons,custody-blob} AND state="active" AND finished=0`). `current_load`/`attested_rtt_ms`/`delivery_score` → `None` (no source column this wave; T0 decided the future key).
- [ ] **Step 4: Run** → PASS (in the T6 batch if disk-staged). **Step 5: Commit** path-limited.

---

### Task 5: Adapter `select_serve_peers` (load → fold → select_diverse) — heavy

**Files:** Modify `elohim/elohim-storage/src/services/serve_routing.rs`

**Interfaces:** Consumes `load_serve_rows`, `fold_candidates`, `score::select_diverse`. Produces `pub fn select_serve_peers(conn: &mut SqliteConnection, blob_hash: &str, n: usize) -> Vec<String>` (returns chosen `agent_cid`s — agent_cid-native, no libp2p remap needed for the `shard_locations` path).

- [ ] **Step 1: Write the DbPool integration test**: seed the same fixture; assert `select_serve_peers(blob_hash, 2)` returns 2 agent_cids spread across the 2 households (diversity), the bonded/higher-capability peer ranks in; and an all-saturated fixture (`current_load` forced 1.0 once that's wired — for now assert the empty-set path via a no-eligible-peer fixture) → empty Vec (caller sheds). Use **synthetic/neutral RTT** (household = ~0, non-discriminating — do NOT assert latency ordering here; that's T7).
- [ ] **Step 2: Run** (heavy) → FAIL. **Step 3: Implement** the thin adapter: `let rows = load_serve_rows(conn, blob_hash); let cands = fold_candidates(&rows); score::select_diverse(&cands, MIN_CAP, n).into_iter().map(|s| s.agent_cid).collect()`.
- [ ] **Step 4: Run** → PASS. **Step 5: Commit** path-limited.

---

### Task 6: Wire into the `race_fetch` candidate sites — heavy

**Files:** Modify `elohim/elohim-storage/src/server/http.rs` (~:2439), `src/p2p/mod.rs` (~:2754), `src/reconcile/custody_sweep.rs` (~:146)

- [ ] **Step 1: Write/extend the integration test** at the primary site (`http.rs`): the existing serve path now consults `select_serve_peers` for the candidate ordering; preserve the existing **8-peer connected-fallback** as the degradation path when `select_serve_peers` returns empty (no eligible/known peers) — never regress availability.
- [ ] **Step 2: Run** (heavy) → FAIL/compile. **Step 3: Implement:** replace the raw `lookup_hosts(...).map(|r| r.peer_id)` candidate construction with `select_serve_peers(conn, blob_hash, N)`; for the `shard_locations` agent_cid-native path, the chosen `agent_cid`s feed `race_fetch` after resolving to the transport id race_fetch expects (if `race_fetch` needs libp2p ids and the agent_cid→libp2p direction is also unfed, keep the existing libp2p candidate list as the fallback ordering and apply `score` ordering only where the resolution is available — document the partial-coverage honestly). Keep the connected-fallback.
- [ ] **Step 4: Run** the storage suite (heavy, batched) → green. **Step 5: Commit** path-limited per site.

---

### Task 7: `@requires:shem` a2o scenario for live cross-WAN RTT ordering (no local build)

**Files:** Create/extend a scenario in `genesis/a2o/features/federation/` (compose with `epr-cross-peer-resolution.feature` / `peer-loss-failover.feature`).

- [ ] **Step 1:** Author a `@wip @requires:shem` scenario asserting that, cross-WAN, serve-routing prefers the lower-measured-RTT capable peer (the latency signal only discriminates cross-WAN; household RTT ≈ 0 ties every peer, so this assertion is vacuous on household and MUST be env-gated). Mark held; do NOT run household.
- [ ] **Step 2: Commit** path-limited. (No cargo.)

### Final gate (batch once after T3–T6 — disk-gated)
- [ ] (DISK-GATED, `run_in_background`): `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=<slot> sh -c 'cargo build --lib && cargo test --lib serve_routing && cargo clippy --lib -- -D warnings && cargo fmt --check'` — real exit codes, no pipe-to-tail.

## Out of scope (named gaps / later work — do NOT block T1–T6)

- **Production writer for `peer_transport_manifest.libp2p_peer_id`** — the upstream gap that unblocks the Option-a `peer_blob_inventory`+resolver path. Only needed if inventory coverage exceeds `shard_locations`. Capture as a backlog item, not a sprint task.
- **`current_load` + `delivery_score` projections** (column-blocked): a JSON-parse projection (load, from `custodian_metrics.health_json`) and a decay projection (delivery, from `peer_blob_inventory` fetch-success). Build-tasks gated only by disk; this wave neutral-defaults them.
- **Live cross-WAN RTT ordering** (T7) — `@requires:shem`.

## Self-Review

- **Spec coverage:** implements the spec's "capability-aware serve-routing (D1)" — §3 meta-cap split (storage chooses, doorway never), §4 entity 2 (selection logic, Operational-C — realized as the pure `fold_candidates`), the reach-earned/`score`-consumer intent. The two reversals (wrong file; blocked-by-code join) are recon-surfaced and stated in the plan, not hidden.
- **Placeholder scan:** T1/T2 carry complete code; T4–T6 give exact anchors + the join spec + the pattern to mirror (`tests/peer_selection.rs`) for editing the large storage files; T6's partial-coverage caveat is explicit, not a TBD.
- **Type consistency:** `ServeRow`/`fold_candidates`/`MIN_CAP` (T2) feed `select_serve_peers` (T5) feed the race_fetch sites (T6); `score::{Candidate,select_diverse}` signatures match the built crate.
- **Scope:** one consumer (storage/score) at the serve sites; the manifest-population gap, the load/delivery projections, and live RTT ordering are explicitly out-of-scope/staged.
