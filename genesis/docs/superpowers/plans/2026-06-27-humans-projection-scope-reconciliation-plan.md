---
title: Humans-projection scope reconciliation — light the household-diversity dataplane (imagodei-write / lamad-read split)
id: humans-projection-scope-reconciliation-plan
status: Draft
class: substrate
domain: D-substrate-distribution
sprint: substrate-coherence enabler for the household-diversity dataplane (composes 2026-06-19-resilience-card-lighting-plan; lights blob-custody P3-8 1a/1b + the ingest selector + the doorway humans cache). household-nodes-testable; live cross-peer proof deferred to the mesh.
cites:
  - genesis/data/timeline/backlog/resilience-card-membership-humans-projection-gap-2026-06-19.md
  - genesis/data/timeline/backlog/qahal-collective-cid-formation-projection-gap.md
  - resilience-card-lighting-plan | the sibling card-lighting arc this composes — same dark-card root, captured the /db/humans scope artifact and the decide-ONE-scope directive | sha256:be6dfb65e5e8a433 | path: genesis/docs/superpowers/plans/2026-06-19-resilience-card-lighting-plan.md
  - blob-custody-p3-8-1b-salvage-household-plumbing-handoff | 1b landed the salvage humans-join under lamad; this plan reconciles the scope that made 1b dormant + retires its threaded scope param | sha256:c56746839ec07679 | path: genesis/docs/superpowers/plans/2026-06-26-blob-custody-p3-8-1b-salvage-household-plumbing-handoff.md
  - blob-custody-phase3-xor-salvage-placement-design | the P3-8 anchor spec — diversity placement is the dataplane consumer this scope-fix finally feeds real households | sha256:f4c139eee8478b9a | path: genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md
  - elohim/elohim-storage/src/db/context.rs
  - elohim/elohim-storage/src/services/peer_selection.rs
  - elohim/elohim-storage/src/services/salvage_commitment_author.rs
  - elohim/elohim-storage/src/db/cache_queries.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/api/identity.rs
  - elohim/elohim-storage/src/services/genesis_self_heal.rs
requires_env: [household-nodes]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md
  - elohim/elohim-storage/CLAUDE.md
---

# Humans-projection scope reconciliation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.
> superpowers:test-driven-development per task. Story-first: land the a2o scenario (Task 7) with the
> implementation. The **p2p-design-gate is pre-answered below** (Category C operational read-scope fix, no new
> entity) — re-confirm it still holds before coding; do NOT widen this into a new identity entity/route.

**Goal:** make the household-diversity dataplane (the ingest peer-selector, blob-custody salvage placement, the
doorway public-humans cache, and `GET /db/humans`) actually read the `household_id` that production already
writes — by reconciling the one scope split that silently empties every household join: humans rows are WRITTEN
under `h_app_id="imagodei"` but those four readers FILTER humans under the operating scope (`"lamad"`).

**Architecture:** establish ONE canonical source of truth for the humans-projection scope (a `HUMANS_HAPP_ID`
constant = `"imagodei"`, the identity pillar humans belong to), route the four affected readers' *humans-table
filter* through it (leaving their other-table filters on the operating scope untouched), and converge the two
production writers onto the same symbol so the scope can never drift again ("decide ONE scope, flip both
together"). This is **reader-side behavior change only** — writers already write `imagodei`; the readers were
filtering humans by the wrong pillar's scope. The resilience card already demonstrates the correct pattern (it
joins humans on `agent_pub_key` with NO h_app_id filter, which is why it lit with matthew's imagodei row).

**Tech Stack:** Rust, Diesel (SQLite), the elohim-storage crate. Plain `cargo test --lib` (no nextest in this
container).

## Honest ceiling (read FIRST — this calibrates every "done")

This plan removes ONE of THREE independent reasons the household join is dark in production. It is necessary,
not sufficient. After it lands, a candidate's `household_id` populates **iff** its humans row has BOTH a
populated `agent_pub_key` AND a `household_id`, under `imagodei`, AND the candidate id is that same `agent_cid`.

| Gate to full production efficacy | Owned by (NOT this plan) |
|---|---|
| **Scope split** (imagodei-write / lamad-read) | **THIS PLAN** |
| **NULL `agent_pub_key` population** — the DHT humans-replayer is a stub; only `genesis_self_heal` fills the self pod; other pods need per-pod registration | `2026-06-19-resilience-card-lighting-plan` Sprint 2 (per-pod self-heal self-insert); the humans-replayer arc |
| **Transport-id vs `agent_cid` namespace** — `self_cid` / `salvage_capacity.agent_cid` may be a libp2p (`12D3Koo…`) / iroh id unless `SELF_CID` pins the agent key; then the join misses even under the right scope | the **blocked** `2026-06-15-coherent-transport-identity-resolver-design`; or `SELF_CID` per deployment |

**What this plan DOES light, concretely:** on alpha, matthew's humans row is healed
(`agent_pub_key=uhCAk…`, `household_id=household-dowell`, `h_app_id=imagodei` — proven by the 2026-06-19 operator
probe U2). So for any node whose humans are populated-and-imagodei AND whose candidate ids are agent_cids,
selection/placement/cache/`/db/humans` flip from empty → correct. Nobody may re-assert "diversity works in
production" until the two gates above also clear — state that ceiling in front so the walk-back does not recur.

## p2p-design-gate — pre-answered (re-confirm before coding)

- **No new DHT entry type / table / route / sync message.** This changes the `h_app_id` *value* four existing
  reads filter by. The humans projection, its columns, and all routes are unchanged. Category **C
  (operational)** — a read-scope correction.
- **Identity-ontology framing:** the fix REINFORCES that `humans` are the **imagodei** pillar's projection (the
  identity home), not `lamad` (content). It does not introduce a sovereignty tier or a new agency claim. It is
  coherent with the imago-dei identity-home framing (see `feedback-identity-sovereignty-ontology-guard`).
- **Join key unchanged:** `humans.agent_pub_key` (uhCAk…), the canonical `agent_cid` namespace — no
  cross-namespace string compare introduced (the all-zeros-card rule, `elohim-storage/CLAUDE.md`).

## Global Constraints (verbatim — every task inherits)

- elohim-storage builds KEEP the ambient `RUSTFLAGS=--cfg getrandom_backend="custom"` (Holochain WASM). Set
  `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/elohim__elohim-storage/dev`; on a fingerprint
  ENOENT fall back to a `/tmp` target dir + `export RUSTC_WRAPPER=""`. Plain `cargo test --lib` (NO nextest).
  Disk at the 85% soft watermark — prefer `--lib` targeted tests over full builds.
- **Commit-only; the integrator pushes/merges/deploys.** Autonomous mode never `git push`, never `kubectl`.
- **Do NOT relax any assertion to force green.** An empty read is a real condition to fix at the source.
- **Shared worktree — selective staging; never bulk-revert ambient mods.** `api/identity.rs` is currently
  ambient-dirty (someone else's uncommitted changes) — when Task 6 touches it, `git add -p` ONLY the
  `HUMANS_HAPP_ID` hunk; never stage the ambient changes.
- `feat/*` / `sprint/*` are NOT orchestrator-indexed — none of these stages are CI-verifiable from a dev
  session; CI is the operator dev-merge + redeploy. State that honestly per task.
- **The reader change is monotonic-safe:** every affected read returns EMPTY today (humans are imagodei, the
  filter is lamad). Flipping to `imagodei` can only turn empty → correct rows, never correct → wrong. There is
  no production data that the new scope mis-selects.

---

## Task 1: Canonical `HUMANS_HAPP_ID` constant (the "decide ONE scope" seam)

**Files:**
- Modify: `elohim/elohim-storage/src/db/context.rs` (add the const + doc; near `AppContext`)
- Test: `elohim/elohim-storage/src/db/context.rs` (`#[cfg(test)] mod tests` already exists at the file end)

**Interfaces:**
- Produces: `pub const HUMANS_HAPP_ID: &str` (value `"imagodei"`) — the single source of truth every later task
  imports as `crate::db::context::HUMANS_HAPP_ID` (re-exported via `crate::db::HUMANS_HAPP_ID` if `db/mod.rs`
  re-exports context symbols — confirm in Step 1 and add the re-export if the module pattern uses one).

- [ ] **Step 1: Read the module + confirm the re-export path.** Read `db/context.rs` (the `AppContext` doc says
  "All database operations are scoped by h_app_id") and `db/mod.rs` to see whether `AppContext` is re-exported
  as `crate::db::AppContext` (it is — used widely). Mirror that re-export for the new const if one exists.

- [ ] **Step 2: Write the failing test.** Add to the existing `#[cfg(test)] mod tests` in `db/context.rs`:

```rust
#[test]
fn humans_happ_id_is_imagodei() {
    // Humans are the imagodei pillar's projection. Production writers
    // (api/identity.rs register_human; services/genesis_self_heal.rs) write this
    // scope; every household-join reader MUST filter humans by it (not the
    // operating content scope "lamad"), or the join silently empties.
    assert_eq!(super::HUMANS_HAPP_ID, "imagodei");
}
```

- [ ] **Step 3: Run, verify FAIL.** `cargo test --lib db::context::tests::humans_happ_id_is_imagodei` →
  FAIL ("cannot find value `HUMANS_HAPP_ID`").

- [ ] **Step 4: Add the const.** In `db/context.rs`, above `pub struct AppContext`:

```rust
/// Canonical `h_app_id` scope for the `humans` projection.
///
/// Humans are the **imagodei** (identity) pillar's projection — production
/// writers (`api/identity.rs::register_human`, `services/genesis_self_heal.rs`)
/// write them under this scope, and the membership projection
/// (`reconcile/controller.rs`) UPDATEs `household_id` onto those rows in place.
///
/// A reader that joins/filters `humans` for IDENTITY or HOUSEHOLD data MUST scope
/// by this constant — NOT by the operating app scope (`"lamad"` for content
/// distribution). Filtering humans by `"lamad"` silently empties the join (the
/// 2026-06-19 dark-card probe artifact; see
/// `backlog/resilience-card-membership-humans-projection-gap-2026-06-19.md`).
/// This is the single source of truth so writers and readers cannot drift
/// ("decide ONE scope, flip both together").
pub const HUMANS_HAPP_ID: &str = "imagodei";
```

  If `db/mod.rs` re-exports `AppContext`, add `pub use context::HUMANS_HAPP_ID;` beside it.

- [ ] **Step 5: Run, verify PASS.** `cargo test --lib db::context::tests::humans_happ_id_is_imagodei` → PASS.

- [ ] **Step 6: Commit.**

```bash
git add elohim/elohim-storage/src/db/context.rs
git commit -m "feat(storage): canonical HUMANS_HAPP_ID scope const (humans-projection reconciliation seam)"
```

---

## Task 2: Ingest selector reads humans under the canonical scope

**Why:** `peer_selection.rs` is the household-first ingest selector. It filters the humans household-enrichment
join (`:193`) AND the node/archetype enrichment join (`:222`) by `input.h_app_id` (`"lamad"`), so it enriches
ZERO households in production. Its other filters (`rea_commitments.h_app_id`, `:123`) correctly stay on the
operating scope — only the humans filters move.

**Files:**
- Modify: `elohim/elohim-storage/src/services/peer_selection.rs:193` and `:222`
- Test: `elohim/elohim-storage/src/services/peer_selection.rs` (its `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `crate::db::context::HUMANS_HAPP_ID` (Task 1).

- [ ] **Step 1: Write the failing test.** In the peer_selection test module, seed: a `peer_statuses` row
  (`online`, `general_pool_member=1`) for `agent="uhCAk-a"`; a `rea_commitments` row scoping that agent into
  the content (h_app_id `"lamad"`, the content commitment the selector uses); and a `humans` row
  `agent_pub_key="uhCAk-a"`, `household_id="hh-1"`, **`h_app_id="imagodei"`**. Call the selector and assert the
  chosen peer carries `household_id = Some("hh-1")`. (Reuse the module's existing fixture helpers; mirror an
  existing `select` test for the commitment/peer_status seeding shape.) RED today — the `"lamad"` humans filter
  misses the imagodei row, so household enrichment is `None`.

- [ ] **Step 2: Run, verify FAIL.** `cargo test --lib peer_selection -- --nocapture` → the new test FAILs
  (household is `None`, expected `Some("hh-1")`).

- [ ] **Step 3: Change both humans filters to the canonical scope.** Add `use crate::db::context::HUMANS_HAPP_ID;`
  to the module imports. At `:193`:

```rust
        let human_rows: Vec<HumanRow> = humans::table
            .filter(humans::h_app_id.eq(HUMANS_HAPP_ID))   // was: input.h_app_id
            .filter(humans::agent_pub_key.eq_any(&accepting))
```

  and the identical change at the node/archetype join (`:222`):

```rust
        let human_id_rows: Vec<HumanIdRow> = humans::table
            .filter(humans::h_app_id.eq(HUMANS_HAPP_ID))   // was: input.h_app_id
            .filter(humans::agent_pub_key.eq_any(&accepting))
```

  Leave the `rea_commitments::h_app_id.eq(input.h_app_id)` filter (`:123`) UNCHANGED — content commitments are
  correctly operating-scoped.

- [ ] **Step 4: Run, verify PASS.** `cargo test --lib peer_selection -- --nocapture` → all PASS (the new test
  green; any pre-existing test that seeded humans under `"lamad"` must be updated to seed under `imagodei` —
  fix those in this step, they were asserting the buggy scope).

- [ ] **Step 5: Commit.**

```bash
git add elohim/elohim-storage/src/services/peer_selection.rs
git commit -m "fix(storage): ingest selector reads humans under HUMANS_HAPP_ID (household enrichment was empty in prod)"
```

---

## Task 3: Salvage placement reads humans under the canonical scope (+ retire the threaded scope param)

**Why:** blob-custody P3-8 1b (commit `6cca8927b`) threaded an `h_app_id` param into `run_salvage_pass` /
`build_salvage_candidates` and passed `"lamad"`, which the 1b adversarial review already flagged as dormant.
Now that the scope is canonical, the param is redundant: `build_salvage_candidates` filters humans by
`HUMANS_HAPP_ID` directly, and the param (its ONLY use) is removed from both functions and the call site.

**Files:**
- Modify: `elohim/elohim-storage/src/services/salvage_commitment_author.rs` (`build_salvage_candidates`,
  `run_salvage_pass`, and the `#[cfg(test)] mod tests`)
- Modify: `elohim/elohim-storage/src/main.rs` (the salvage tick call — drop the `"lamad"` argument)

**Interfaces:**
- Consumes: `crate::db::context::HUMANS_HAPP_ID` (Task 1).
- Produces: `run_salvage_pass(conn, self_cid, author, enabled, target_replicas, inventory_freshness_seconds,
  diversity_placement, now)` — the `h_app_id: &str` parameter is REMOVED (8 args, was 9).
  `build_salvage_candidates(conn, self_cid, fresh_after)` — the `h_app_id: &str` parameter is REMOVED.

- [ ] **Step 1: Update the join to the const + drop the param (production code).** In `build_salvage_candidates`,
  remove the `h_app_id: &str` parameter and change the filter:

```rust
    let human_rows: Vec<HumanRow> = humans::table
        .filter(humans::h_app_id.eq(crate::db::context::HUMANS_HAPP_ID))  // was: .eq(h_app_id)
        .filter(humans::agent_pub_key.eq_any(&cids))
        .order_by(humans::id.asc())
        .select((humans::agent_pub_key, humans::household_id))
        .load::<HumanRow>(conn)
        .map_err(|e| StorageError::Internal(e.to_string()))?;
```

  In `run_salvage_pass`, remove the `h_app_id: &str` parameter and its forwarding to `build_salvage_candidates`
  (`build_salvage_candidates(conn, self_cid, &fresh_after)?`). Update the rustdoc: the join scope is now the
  canonical `HUMANS_HAPP_ID` (imagodei), keep the honest dormancy paragraph (the `imagodei`-write/`lamad`-read
  split is RESOLVED by this plan; the NULL-`agent_pub_key` + transport-id dormancies remain — keep those named).

- [ ] **Step 2: Update the call site.** In `main.rs`, the salvage tick call to `run_salvage_pass` — remove the
  `"lamad"` argument (and its scope-explainer comment block, now obsolete). The call becomes:

```rust
        match elohim_storage::services::salvage_commitment_author::run_salvage_pass(
            &mut conn,
            &salvage_self_cid,
            salvage_author.as_ref(),
            true, // enabled (gated by the match arm above)
            salvage_target_replicas,
            salvage_freshness,
            salvage_diversity,
            chrono::Utc::now(),
        ) {
```

- [ ] **Step 3: Migrate the 1b tests to the canonical scope.** In the salvage `#[cfg(test)] mod tests`: change
  `const APP: &str = "lamad";` → seed humans via `seed_human(.., crate::db::context::HUMANS_HAPP_ID)` everywhere
  (the join now only matches imagodei rows). Drop the `h_app_id` argument from all `build_salvage_candidates(..)`
  test calls. Replace `h_app_id_scope_filter_is_load_bearing` with a test that pins the const is load-bearing:

```rust
#[test]
fn humans_join_uses_canonical_imagodei_scope_only() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    seed_capacity(&mut conn, "uhCAk-a");
    // A human under the WRONG (operating/content) scope must NOT be seen...
    seed_human(&mut conn, "h-lamad", Some("uhCAk-a"), Some("hh-wrong"), "lamad");
    let cands = build_salvage_candidates(&mut conn, "uhCAk-self", &fresh_cutoff()).unwrap();
    assert!(hh_of(&cands, "uhCAk-a").is_none(), "lamad-scoped human must not match");
    // ...only the imagodei (canonical) row is.
    seed_human(&mut conn, "h-imagodei", Some("uhCAk-a"), Some("hh-1"),
               crate::db::context::HUMANS_HAPP_ID);
    let cands = build_salvage_candidates(&mut conn, "uhCAk-self", &fresh_cutoff()).unwrap();
    assert_eq!(hh_of(&cands, "uhCAk-a"), Some("hh-1"), "imagodei-scoped human matches");
}
```

  Update `run_salvage_pass_authors_when_self_selected_and_gates_on_enabled` to drop the `APP` argument from its
  two `run_salvage_pass(..)` calls.

- [ ] **Step 4: Run, verify PASS.** `cargo test --lib salvage_commitment_author -- --nocapture` → all PASS.

- [ ] **Step 5: Build the binary (the call-site arg change is binary-only).**
  `cargo build --bin elohim-storage` → Finished. (`--lib` does not compile `main.rs`.)

- [ ] **Step 6: Commit.**

```bash
git add elohim/elohim-storage/src/services/salvage_commitment_author.rs elohim/elohim-storage/src/main.rs
git commit -m "refactor(storage): salvage humans-join uses canonical HUMANS_HAPP_ID; retire threaded scope param"
```

---

## Task 4: Doorway cacheable-humans projection reads under the canonical scope

**Why:** `cache_queries.rs::list_cacheable_humans` (`:39`) builds the doorway projection cache of public humans,
filtering by `ctx.h_app_id`. Under the operating scope it returns ZERO public humans (they are imagodei),
leaving the doorway `/api/v1/cache/*` humans projection empty.

**Files:**
- Modify: `elohim/elohim-storage/src/db/cache_queries.rs:39`
- Test: `elohim/elohim-storage/src/db/cache_queries.rs` (add a `#[cfg(test)]` module if absent; use `test_pool`)

- [ ] **Step 1: Write the failing test.**

```rust
#[test]
fn list_cacheable_humans_finds_imagodei_public_humans() {
    use crate::db::models::NewHuman;
    use crate::db::diesel_schema::humans;
    use diesel::prelude::*;
    let pool = crate::test_util::test_pool();
    let mut conn = pool.get().unwrap();
    diesel::insert_into(humans::table)
        .values(&NewHuman {
            id: "h-1".into(), agent_pub_key: Some("uhCAk-a".into()),
            display_name: "A".into(), bio: None, affinities: "[]".into(),
            profile_reach: "public".into(), location: None, profile_photo_url: None,
            h_app_id: crate::db::context::HUMANS_HAPP_ID.into(), household_id: None,
        })
        .execute(&mut conn).unwrap();
    // The doorway cache operates in the content (lamad) context.
    let ctx = crate::db::AppContext::new("lamad");
    let rows = super::list_cacheable_humans(&mut conn, &ctx, 100, 0).unwrap();
    assert_eq!(rows.len(), 1, "public imagodei humans must be cacheable from the lamad context");
}
```

- [ ] **Step 2: Run, verify FAIL.** `cargo test --lib cache_queries::` → FAIL (`rows.len()==0`).

- [ ] **Step 3: Change the humans filter.** Add `use crate::db::context::HUMANS_HAPP_ID;` to the file imports.
  At `:39`:

```rust
    humans::table
        .filter(humans::h_app_id.eq(HUMANS_HAPP_ID))   // was: &ctx.h_app_id
        .filter(humans::profile_reach.eq("public"))
```

  Leave `list_cacheable_content` / `list_cacheable_relationships` UNCHANGED (content + relationships are
  correctly operating-scoped; only humans belong to imagodei).

- [ ] **Step 4: Run, verify PASS.** `cargo test --lib cache_queries::` → PASS.

- [ ] **Step 5: Commit.**

```bash
git add elohim/elohim-storage/src/db/cache_queries.rs
git commit -m "fix(storage): doorway cacheable-humans reads under HUMANS_HAPP_ID (public humans cache was empty)"
```

---

## Task 5: `GET /db/humans` legibility — the read-scope artifact that misled the 2026-06-19 probe

**Why:** `handle_list_humans` (`http.rs:8405`) calls `humans::list_humans(&mut conn, &ctx.h_app_id)`; for
`/db/humans` the operating ctx is the content scope, so it returns an EMPTY list despite a populated table —
the exact artifact that misled the dark-card probe ("matthew's row exists; it was filtered out", backlog U2).

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:8412` (the `humans::list_humans` call in `handle_list_humans`)
- Test: a handler/integration test if the file has a `#[cfg(test)]` http harness; otherwise a `db::humans`
  unit test asserting `list_humans(conn, HUMANS_HAPP_ID)` returns imagodei rows (the handler delegates to it).

- [ ] **Step 1: Locate + read.** Confirm `handle_list_humans` at `http.rs:8405-8420` and that `db/humans.rs`
  `list_humans(conn, h_app_id)` filters by `h_app_id` (`db/humans.rs:153`/`:165`).

- [ ] **Step 2: Write the failing test** (in `db/humans.rs` `#[cfg(test)]`, the unit the handler delegates to):

```rust
#[test]
fn list_humans_under_canonical_scope_finds_imagodei_rows() {
    let pool = crate::test_util::test_pool();
    let mut conn = pool.get().unwrap();
    create_human(&mut conn, CreateHumanInput {
        id: "h-1".into(), agent_pub_key: Some("uhCAk-a".into()), display_name: "A".into(),
        bio: None, affinities: "[]".into(), profile_reach: "commons".into(), location: None,
        profile_photo_url: None, h_app_id: crate::db::context::HUMANS_HAPP_ID.into(),
        household_id: Some("hh-1".into()),
    }).unwrap();
    let rows = list_humans(&mut conn, crate::db::context::HUMANS_HAPP_ID).unwrap();
    assert_eq!(rows.len(), 1, "list_humans under the canonical scope returns the imagodei row");
}
```

- [ ] **Step 3: Run, verify PASS** (this test passes immediately — it documents the contract). Then write the
  RED handler-side change test/assertion: confirm `handle_list_humans` passes the operating ctx today by
  reading the call, and that the fix routes it to the const.

- [ ] **Step 4: Change the handler call.** At `http.rs:8412`:

```rust
        // /db/humans lists the imagodei identity projection, NOT the operating
        // content scope — else a populated table reads empty (the 2026-06-19
        // dark-card probe artifact). See db::context::HUMANS_HAPP_ID.
        match humans::list_humans(&mut conn, crate::db::context::HUMANS_HAPP_ID) {
```

- [ ] **Step 5: Run + build.** `cargo test --lib db::humans::` → PASS; `cargo build --bin elohim-storage` →
  Finished.

- [ ] **Step 6: Commit.**

```bash
git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/db/humans.rs
git commit -m "fix(storage): GET /db/humans lists the imagodei scope (was empty under the content scope — probe artifact)"
```

---

## Task 6: Converge the production writers onto the const (flip-both-together; drift guard)

**Why:** the writers already write `"imagodei"` correctly, but as string literals. Route them through
`HUMANS_HAPP_ID` so the scope has exactly ONE definition — a future scope change then propagates to readers and
writers together by construction. Behaviorally a no-op (`HUMANS_HAPP_ID == "imagodei"`).

**Files:**
- Modify: `elohim/elohim-storage/src/api/identity.rs:112` (`register_human`)
- Modify: `elohim/elohim-storage/src/services/genesis_self_heal.rs:109`

- [ ] **Step 1: Route `register_human`.** At `api/identity.rs:112`, replace `h_app_id: "imagodei".to_string()`
  with `h_app_id: crate::db::context::HUMANS_HAPP_ID.to_string()`. **STAGING CAUTION:** `api/identity.rs` is
  ambient-dirty — `git add -p` ONLY this one-line hunk.

- [ ] **Step 2: Route `genesis_self_heal`.** At `services/genesis_self_heal.rs:109`, replace
  `h_app_id: "imagodei".to_string()` with `h_app_id: crate::db::context::HUMANS_HAPP_ID.to_string()`.

- [ ] **Step 3: Build + targeted tests.** `cargo build --bin elohim-storage` → Finished;
  `cargo test --lib genesis_self_heal` → PASS (the self-heal tests already assert the row is written; they now
  assert it via the const — value-identical, so they stay green). If `genesis_self_heal`'s tests assert the
  literal `"imagodei"`, leave the assertion as the literal (it documents the value) — only the production write
  uses the const.

- [ ] **Step 4: Commit** (selective-stage; do NOT include ambient identity.rs hunks).

```bash
git add -p elohim/elohim-storage/src/api/identity.rs        # ONLY the HUMANS_HAPP_ID hunk
git add elohim/elohim-storage/src/services/genesis_self_heal.rs
git commit -m "refactor(storage): humans writers reference HUMANS_HAPP_ID (one scope definition, no drift)"
```

---

## Task 7: a2o scenario + honest dormancy documentation

**Files:**
- Create/Extend: `genesis/a2o/features/resilience/` (or the blob-custody durability coverage) — a `.feature`
- Modify: `genesis/data/timeline/backlog/resilience-card-membership-humans-projection-gap-2026-06-19.md`
  (record the scope leg as resolved-by-this-plan; the other two gates remain)

- [ ] **Step 1: Write the a2o scenario** (household floor — M/J/J; NO `@requires:shem`). Assert the
  household-diversity path now reads household data: a seeded household with imagodei humans (populated
  `agent_pub_key` + `household_id`) and an under-replicated blob → the salvage/ingest selection enriches
  `household_id` (the property is "the candidate set carries real households", the unit-fixture-proven
  behavior; the live cross-peer "replica count rises" remains the held mesh leg). Tag `@requires:household-nodes`.

```gherkin
Feature: Household-diversity dataplane reads the household projection
  Scenario: Salvage candidates carry real households once humans are imagodei-populated
    Given a household "dowell" with members whose humans rows have populated agent keys
    And a content blob that is under-replicated
    When the node builds its salvage candidate pool
    Then each candidate that maps to a known household carries that household_id
    And the diversity placement strategy can span distinct households
```

- [ ] **Step 2: Update the backlog item.** Append a section to
  `resilience-card-membership-humans-projection-gap-2026-06-19.md`: the imagodei/lamad **scope leg is resolved**
  by this plan (commit refs), the `HUMANS_HAPP_ID` const is the single source of truth, and the TWO remaining
  gates (NULL `agent_pub_key` population; transport-id namespace) stay open with their owners. Keep
  `status: open` until population + namespace also clear and a live read proves non-empty.

- [ ] **Step 3: Capture the sibling, do NOT absorb.** Add a one-line backlog item (home:
  `qahal-collective-cid-formation-projection-gap.md`'s "Riding coherence cleanup") noting the **collectives**
  `lamad`/`qahal` scope-split is the same class as this humans fix and is the next instance to reconcile — but
  it needs writer convergence too (collectives ARE written under both), so it is NOT folded here.

- [ ] **Step 4: Commit.**

```bash
git add genesis/a2o/features/ genesis/data/timeline/backlog/resilience-card-membership-humans-projection-gap-2026-06-19.md genesis/data/timeline/backlog/qahal-collective-cid-formation-projection-gap.md
git commit -m "test(a2o): household-diversity dataplane reads the imagodei humans projection + honest dormancy ceiling"
```

---

## Operator step (keystone, not code)

Redeploy alpha storage from dev (image rebuild + rolling restart — **NOT** `ALLOW_DNA_REINSTALL`; no DNA
change). **No reseed needed for matthew** (his imagodei humans row is already healed). Verify on a node with a
populated imagodei humans row:
- `GET /db/humans` returns the household members (was `{items:[],count:0}`).
- The ingest selector / salvage tick logs show household enrichment is non-empty (no longer all-`None`).

If reads stay empty AFTER deploy, the blocker is one of the two OTHER gates (NULL `agent_pub_key` → the
per-pod registration arc; or transport-id candidate cids → `SELF_CID` / the resolver) — NOT this scope fix.
Do not re-chase the scope.

---

## Done (stability-gated, not single-green)

- `HUMANS_HAPP_ID` is the single scope definition; the four affected readers (ingest selector, salvage, doorway
  humans cache, `/db/humans`) filter humans by it; the two production writers reference it.
- On a node with a populated-and-imagodei humans row whose `agent_cid` matches the candidate id, household data
  flows: `/db/humans` non-empty, ingest/salvage enrich households, the doorway humans cache populates.
- The honest ceiling held in front: the NULL-`agent_pub_key` population gate and the transport-id namespace gate
  remain, named with their owners; nobody re-asserts "diversity works in production" without them.
- The collectives `lamad`/`qahal` sibling scope-split is captured (not absorbed) as the next instance.

## Self-review

- **Spec coverage:** scope split → Tasks 1–6 (the four readers + the writer convergence + the const seam); the
  NULL-key and transport-id dormancies → named in the Honest Ceiling + Task 7 backlog (explicitly out of scope,
  owned elsewhere); the legibility artifact → Task 5; the sibling collectives split → Task 7 Step 3 capture.
- **No placeholders:** every reader change shows the exact before/after filter line; every test shows real seed
  + assertion code.
- **Type consistency:** `HUMANS_HAPP_ID: &str` is consumed identically (`humans::h_app_id.eq(HUMANS_HAPP_ID)`)
  in Tasks 2–5 and `.to_string()`-ed in Task 6 (the writer `NewHuman.h_app_id: String` field). The salvage
  signature change (Task 3) drops `h_app_id` from both `run_salvage_pass` and `build_salvage_candidates` and the
  `main.rs` call site consistently (8 args).
- **Gate honesty:** the p2p-design-gate is pre-answered (Category C, no new entity); the honest ceiling names
  the two un-fixed gates so the deploy can't be over-claimed.
- **Monotonic safety:** every reader change is empty→correct only; no production data is mis-selected by the
  new scope (the writers already wrote imagodei).
