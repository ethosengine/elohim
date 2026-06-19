---
title: "elohim-facings crate extraction — behavior-preserving migration of the resiliency one-off into the pure lens crate"
id: elohim-facings-crate-extraction-plan
status: Landed
domain: D5
sprint: facings-framework-migration
cites:
  - resilience-facings-select-fold-aggregate-design | the framework spec whose §11 migration slice + gap-item #12 this plan executes | sha256:60f173daec6a0e0c | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - qahal-epr-household-lattice-design | the canonical household/hub lattice the resiliency facing (being migrated) folds over | sha256:ed5c1d3d2698b567 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
requires_env: [household-nodes]
---

# elohim-facings crate extraction (the §11 migration slice)

> **✅ LANDED 2026-06-19** on `feat/frontend-eyes-sprint`. Two commits: the resiliency-vertical
> **foundation** (`c274d844c` — one materialized holder-relation + the `intra_hub_peers` field) and
> the **migration** (this plan — the pure `elohim-facings` crate + `household_resilience.rs` thin-adapter
> rewire). All gates green: **34/34** `household_resilience` (incl. the byte-identical golden — JSON
> reproduced exactly, behavior preserved), **17** `elohim-facings`, **212** `schema_contract`, **374**
> ts-rs `export_bindings`; `elohim-facings` clippy `-D warnings` clean; fmt clean. The boundary is the
> dependency graph (verified: `use diesel;` in a fold → `error[E0433]`). The four child lens charters
> can now proceed in **parallel sessions** — each adds its own `folds/<facing>.rs` + storage loader.

**For a fresh implementation session.** This drains gap-item #12 of
`2026-06-19-resilience-facings-select-fold-aggregate-design.md` (§11 "Lens Framework" → "Migration of
household_resilience.rs"). It is a **behavior-preserving refactor**: it creates the pure `elohim-facings`
crate and moves the already-pure folds into it, with the resiliency facing's behavior unchanged. Once this
lands, the four lens charters (`reach-projection`, `rea-economic`, `operational-weave`,
`epr-content-perspective` `…-facing-lens-design.md`) can be implemented in **parallel sessions**, each adding
its `folds/<facing>.rs` + a storage-side loader against this substrate.

## Why this is safe (verified facts — read before doubting the approach)

- The folds are **already pure** (`build_felt_status`, `floor_for_tier`, `intra_hub_peers`, the regional
  bucket loop, the stewarding collect) — `build_felt_status` + `floor_for_tier` + `intra_hub_peers` are
  `pub(crate)` no-DB fns, tested DB-free in `mod felt_status_tests` (10 tests) in
  `elohim/elohim-storage/src/services/household_resilience.rs`.
- The **views already live in `elohim-views`** (`infrastructure.rs`), which has **no diesel** — so the
  ts-rs cross-crate trap (CLAUDE.md's `../../../` breakage) **cannot bite**: no `#[derive(TS)]` type moves,
  no codegen change, no schema change.
- `snapshot()` + `compute()` take `&DbPool` (impure) → they **cannot move** into the pure crate; they stay
  in `household_resilience` as thin adapters with **unchanged signatures**, which alone keeps all 33
  integration tests green (the tests are a separate compilation unit; they call only the `pub` adapters).
- `HolderRow` and the folds are `pub(crate)` — invisible to the 33 integration tests. So no re-exports are
  needed; only the adapters' public signatures must hold.

## PRE-FLIGHT (do this first — the environment will block you otherwise)

Heavy cargo is **DENIED at the 85% PVC ceiling** by `.claude/hooks/cargo-disk-guard.py`, and
`FORCE_HEAVY_GATES` does **not** bypass it (memory `project_cargo_disk_guard_override`). Before any build:

1. Check `df -B1G /projects` and `cargo-pool estimate`. A full `elohim-storage` test build is ~6–15G; ensure
   free space ≫ that against the **911G PVC cap** (not just the %).
2. If `pct ≥ 85`: either the operator frees non-pool space, or bump `volume_hard_pct` in
   `genesis/agentic/pool-policy.json` (e.g. 85→90 — leaves ≥90G PVC margin), **and revert it when done.**
3. Every cargo invocation in this plan uses: `RUSTFLAGS=""` (native; the ambient WASM getrandom flag breaks
   linking), `RUSTC_WRAPPER=""` (sccache returns null bytes on the clippy-driver `--print` probe), and
   `CARGO_TARGET_DIR=/tmp/<slot>` (the `/projects` pool slot throws fingerprint ENOENT on cold builds).
   Run heavy cargo with `run_in_background` (>10min) so a Bash timeout can't orphan the `.cargo-lock`.

## Phase 0 — Capture the byte-identical-JSON baseline (BEFORE any code change)

The cutover gate is **byte-identical serialized JSON before/after** (mirror the sha256-the-generated-TS
discipline). Establish the golden BEFORE touching anything.

1. Add a throwaway golden test (or a `#[test]` you keep) in `tests/household_resilience.rs` that builds 3
   representative snapshots via the existing seed helpers and records their serialized JSON:
   - the lit-card case (`resilience_card_lights_with_coherent_agent_keyed_substrate`'s seed → `snapshot()`),
   - an `unmeasured` case (`content-never-seeded`),
   - the intra-hub case (`intra_hub_peers_counts_distinct_agents_per_hub`'s seed).
   `serde_json::to_string(&snapshot).` Print + record the **sha256 of each** (and the raw JSON).
2. Run it (PRE-FLIGHT env) and **commit the 3 hashes + raw JSON into the test as golden constants.**
   - ✅ Gate: test passes and records 3 stable hashes.
   - ⛔ If the snapshot JSON is non-deterministic (e.g. HashSet ordering leaks into a Vec), FIX that
     determinism first (sort before serialize) — a non-deterministic baseline cannot gate a refactor.

## Phase 1 — Scaffold the pure crate (no logic yet)

1. Create `elohim/elohim-facings/Cargo.toml`: `[dependencies]` = **`elohim-views` (path) + std ONLY.** Do
   **NOT** add `diesel`, `elohim-storage`, `chrono`, or `serde` unless a *moved* type actually needs it — v1
   does not (the folds return view types defined in `elohim-views`; `HolderRow` is a plain struct deriving
   only `Debug`/`Clone`). Minimal deps is the entire point of the crate. (Add `serde`/`chrono` later, per-lens,
   only when a fold's own type needs them.)
2. Add `elohim/elohim-facings` to the workspace `members` (root `Cargo.toml`).
3. `elohim/elohim-facings/src/lib.rs`: `pub mod relation; pub mod fold; pub mod folds;` (+ `folds/mod.rs`
   with `pub mod resiliency;`).
4. **The boundary is enforced by the dependency graph, NOT by deny.toml.** Because `diesel` is absent from
   `elohim-facings/Cargo.toml`, a `&mut SqliteConnection` / `use diesel;` in a fold **won't resolve — the
   compile fails.** That compile-failure IS the boundary (the certain mechanism).
   - ✅ Gate (the real enforcement): `cargo check -p elohim-facings` (PRE-FLIGHT env) compiles the empty crate;
     then deliberately add `use diesel::prelude::*;` to a facings file and confirm it **fails to compile**
     ("unresolved import: diesel"), then remove it.
   - Optional defense-in-depth: a deny.toml rule — **but first verify cargo-deny can even express "crate A may
     not depend on crate B."** Its `[bans.deny]` denies a crate *workspace-globally*, which would break
     `elohim-storage`'s legitimate diesel; if it can't express per-crate scoping, **SKIP it** — do not add a
     rule that breaks the storage build. The Cargo.toml omission above is the enforcement that matters.

## Phase 2 — Move the pure primitives + folds into elohim-facings

Move (don't rewrite) the already-pure code; rewrite only `intra_hub_peers` as a combinator composition.

1. `relation.rs`: move the **v1 `HolderRow` as-is** — `{ hub_id: Option<String>, agent_id: String, region:
   Option<String> }` (do NOT add `content_cid`/`online` here; that is per-lens work, not this migration).
   Flip `pub(crate)` → `pub`.
2. `fold.rs`: add the generic combinators (NEW): `bucket_by<R,K>(rows, key: Fn(&R)->Option<K>) ->
   HashMap<K,Vec<&R>>` (None key drops the row — preserves "exclude null-hub") and
   `distinct_count_by<R,K>(rows, key) -> usize`. Unit-test both DB-free.
3. `folds/resiliency.rs`:
   - Move `build_felt_status`, `floor_for_tier`, `intra_hub_peers` verbatim; flip to `pub`. **Rewrite
     `intra_hub_peers`** as `bucket_by(rel, |r| r.hub_id.clone())` then per-bucket `distinct_count_by(_, |r|
     r.agent_id.clone())` (the composability proof — same output, no hand loop).
   - **Extract the two not-yet-pure folds** as named `pub fn`s over `&[HolderRow]`: `stewarding_hubs(rel) ->
     HashSet<String>` (the `compute()` collect, `relation.iter().filter_map(hub_id)`); and
     `regional_distribution(rel, viewer_region: Option<&str>) -> RegionalDistributionView` (the
     `compute_regional_distribution` dedupe-by-hub bucket loop — **preserve the null-hub→unknown behavior**;
     the standalone-rustc check on 2026-06-19 confirmed this logic).
   - Move `mod felt_status_tests` (the 10 tests) into this file.
   - ✅ Gate: `cargo test -p elohim-facings` — the 10 felt_status_tests + the new combinator/intra tests pass.

## Phase 3 — Rewire household_resilience.rs as a thin adapter

1. Add `elohim-facings` to `elohim-storage/Cargo.toml` deps.
2. `load_holder_relation` + `count_online_peers_in_households` **stay in `household_resilience.rs`** (they
   take `&mut conn` — impure, storage-side). (Optional follow-on, not required for the boundary: relocate
   `load_holder_relation` to `db/loaders.rs`.)
3. Rewrite `compute()` + `snapshot()` to call `elohim_facings::folds::resiliency::*`:
   - `compute()`: `let rel = load_holder_relation(...)?;` then `let steward = facings::stewarding_hubs(&rel);`
     → unchanged downstream (online count, status, health). **Signature unchanged.**
   - `snapshot()`: load the relation, call `facings::intra_hub_peers(&rel)`, `facings::regional_distribution`,
     `facings::build_felt_status`, assemble the same `ResilienceSnapshotView`. **Signature unchanged.**
   - Delete the now-moved fn bodies + the `mod felt_status_tests` from `household_resilience.rs` (they live in
     the crate now). Add `use elohim_facings::...` imports.
   - ✅ Gate: `cargo build -p elohim-storage` compiles.

## Phase 4 — Verify (the cutover gate — all must pass)

Run all under PRE-FLIGHT env (`/tmp` target, `RUSTC_WRAPPER=""`, `RUSTFLAGS=""`):

1. `cargo test --test household_resilience` → **33/33 green** (behavior at the field level preserved).
2. `cargo test -p elohim-facings` → **10 felt_status_tests + combinator tests green**.
3. `cargo test --test schema_contract` → **green** (no view/schema change, so this should be untouched —
   confirms it).
4. **Byte-identical JSON**: the Phase-0 golden test reproduces the **same 3 sha256 hashes**. ⛔ If any hash
   differs, the refactor changed serialization — diff the raw JSON, fix, do not proceed.
5. `cargo fmt --check` (both crates) + `cargo clippy -p elohim-facings --all-targets -- -D warnings` (the new
   crate must be clippy-clean; the pre-existing `elohim-storage` clippy debt — 13 lints in 8 unrelated files,
   clippy-1.96 drift — is NOT this plan's to fix, but do not ADD to it).
6. `cargo deny check` → the boundary holds.

## Phase 5 — Land + handoff

1. Remove the throwaway scaffolding if any; keep the golden test (it's a permanent regression guard).
2. Update `elohim/elohim-storage/CLAUDE.md` (and add a short `elohim/elohim-facings/CLAUDE.md`) noting the
   pure-crate boundary + the add-a-lens recipe pointer to §11.
3. Commit on the branch (commit-only; integrator pushes). Flip gap-item #12 → CLAIMED (locally verified).
4. **Handoff to parallel lens sessions:** each of the 4 child charters can now proceed independently — each
   adds `elohim-facings/src/folds/<facing>.rs` + a storage-side `load_<facing>_relation` + a route, following
   its spec's proof-fold-first Slice. They do not conflict (separate `folds/` files + separate loaders).

## Risks / watch-outs

- **Non-deterministic JSON** (HashSet→Vec ordering) would fail the byte-identical gate falsely — Phase 0
  forces a sort if needed. (`steward_households_sorted` already sorts; confirm regional/intra outputs are
  order-stable too.)
- **deny.toml graph**: if `elohim-facings` accidentally gains `elohim-storage` (transitively via a shared
  util), the boundary collapses — Phase 1's deliberate `use diesel;` failure test catches it.
- **Disk**: the whole plan is heavy-cargo; if the ceiling re-blocks mid-plan, stop and reclaim — do not
  fight the hook.
- **Scope discipline**: this is the migration ONLY. Do **not** add `content_cid`/`online` to `HolderRow`, do
  **not** build any new lens, do **not** move loaders to `db/loaders.rs` (optional follow-on) — those are
  separate slices. Keeping this PR a pure move is what makes the byte-identical gate meaningful.
