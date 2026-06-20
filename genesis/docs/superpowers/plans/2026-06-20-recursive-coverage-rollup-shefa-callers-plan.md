---
title: "Recursive weave aggregation — consume CoverageRollup in the shefa builders (Weave Epic #1, actionable core)"
id: recursive-coverage-rollup-shefa-callers-plan
status: Draft
class: protocol-canonical
domain: D5
sprint: weave-epic-wave-b
cites:
  - recursive-architecture-design | the canon this drains — §3.1 names CoverageRollup + re-express the two shefa builders as its first callers; §1.3 the descent-erasure identity | sha256:053f260af9989d4b | path: genesis/docs/superpowers/specs/2026-06-14-recursive-architecture-design.md
  - weave-epic-arc-design | the epic this is subsystem #1 of (recursive aggregation); its #1 fork (CoverageDomain capacity mapping) is the deferred lens-wiring | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - operational-weave-facing-lens-design | the just-landed lens whose cluster aggregate is the DEFERRED recursive-rollup target (needs the CoverageDomain fork) | sha256:fc432fea065dca00 | path: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md
  - resilience-facings-select-fold-aggregate-design | the select->fold->aggregate framework the shefa builders + the operational lens share | sha256:8f2136ecd8678e6c | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
# Mixed-env: NO doc-level requires_env. Slice 0 is pure/DB-free. Slices 1-2 re-express graph-view
# builders — testable on household-nodes via the Cozo engine; the in-repo proof is the rollup logic.
# The council N-level recursion, the trait-Governor lift, and the operational-lens cluster->council
# wiring (which needs the CoverageDomain capacity-mapping fork) are the DEFERRED remainder — see Non-goals.
---

# Recursive CoverageRollup — shefa first-callers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consume the already-built-but-unconsumed `CoverageRollup` keystone (`elohim/elohim-storage/src/recursion.rs`) in the shefa graph-view builders — replacing `rows.len()` *descent-erasure* with descent-preserving aggregation (the recursive-architecture §3.1 "re-express the two shefa builders as its first callers" work), and prove the operator composes transitively at N=2.

**Architecture:** `CoverageRollup` (recursion.rs) is a content-addressed aggregate-with-descent: `covered`/`required`/`deficit: CoverageSet` (keyspace intervals) + `constituents: Vec<Cid>` (the down-pointer) + BLAKE3 `rollup_hash`. The shefa builders (`graph_views/shefa/resilience_snapshot.rs`, `distribution.rs`) currently aggregate diversity by `rows.len()` — a bare count that erases *which* stewards/collectives covered it, so you cannot descend to "which steward is the gap." Re-express the diversity aggregate as a `CoverageRollup`: `required = CoverageSet::full(target_diversity)`, `covered` = the achieved slots, `constituents` = the steward/collective CIDs, `deficit` = the shortfall (the descent target). The recursion's core identity — *aggregation preserves descent* — applied at two real production sites.

**Tech Stack:** Rust, `elohim/elohim-storage/src/recursion.rs` (CoverageRollup/CoverageSet/ChildCoverage/CoverageDomain), the Cozo graph engine (shefa builders), ts-rs + schema codegen (if a descent field is exposed on a view).

## Global Constraints

- **Build env (native):** every cargo run is `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv cargo …` (the `/tmp` target + empty RUSTC_WRAPPER avoid the pool-fingerprint/sccache traps; disk pressure).
- **The CoverageRollup API is FIXED — do not modify recursion.rs's types:** `CoverageRollup::rollup(scope_cid: impl Into<String>, domain: CoverageDomain, required: CoverageSet, children: &[ChildCoverage]) -> CoverageRollup`. Build children with `ChildCoverage::readable(cid, CoverageSet)`. `CoverageSet::{empty(), interval(s,e), full(span), union(&), difference(&), is_empty(), measure(), intervals()}`. `CoverageDomain::{CorpusBytes, ArcKeyspace}`. Read `recursion.rs:30-281` for the exact surface; the existing tests (`rollup_aggregates_coverage_and_exposes_the_deficit`, `rollup_is_content_addressed_and_order_independent`) are the call-pattern reference.
- **The diversity→keyspace mapping (the v1 model):** a content's required diversity of N distinct collectives = `CoverageSet::full(N)`; each achieved collective covers one slot `interval(i, i+1)`; `constituents` = the collective/steward CIDs. `deficit.measure()` = how many slots short; `deficit` intervals = which slots are the gap. (This is the unambiguous mapping for diversity coverage — the §2.4 `CoverageDomain` capacity fork does NOT bite here; it only bites the operational-lens byte-capacity wiring, which is DEFERRED.)
- **Descent is CIDs:** `constituents` hold collective/steward CIDs (content-derived). Never put a count there; never raw-string-compare identity namespaces (the all-zeros-card trap — see elohim-storage/CLAUDE.md Identity section). The steward set here is graph `epr_edge` CIDs, already content-addressed.
- **Not-selected-field contract** (if a descent field is added to a View): `#[serde(default, skip_serializing_if = "Option::is_none")]` + `#[ts(optional)]`; update the schema + run `cargo test --test schema_contract` + `pnpm run schema:codegen:ts` (the lens's Slice 4 is the reference).
- **Branch `feat/frontend-eyes-sprint`, commit-only** (integrator pushes; never `git push`); a concurrent session commits on this branch — stage ONLY your files.

## Non-goals (the DEFERRED remainder of Weave Epic #1)

- **The operational-weave lens's cluster→council recursive rollup.** It needs the §2.4 `CoverageDomain` capacity-mapping fork resolved (does byte-capacity map onto a deficit keyspace, or is it a scalar?) + a seeded `{"kind":"council"}` Collective + MEMBER_OF edges. Deferred — this plan re-expresses the shefa diversity builders only, where the mapping is unambiguous.
- **The `trait Governor` lift** from `arc_actuator.rs:383` into a shared location (recursive-architecture §3.1 prerequisite for the donut floors/ceilings). The CoverageRollup re-expression does NOT need it. Deferred.
- **The council N-level (councils-of-councils) recursion** + the `CoverageRollupAttestation` DHT notarization (§2.4 GATED fork — do NOT take). Deferred.
- **`topology_overview.rs`** (a third `rows.len()` site) if its count is not a *diversity/coverage* aggregate — assess and defer if it doesn't fit the model.

## File Structure

- **Modify** `elohim/elohim-storage/src/recursion.rs` — add the N=2 transitive proof test (no production-code change to recursion.rs).
- **Modify** `elohim/elohim-storage/src/graph_views/shefa/resilience_snapshot.rs` — re-express the two `rows.len()` aggregates (`:29` stewarding, `:38` commitment-backed) via `CoverageRollup`.
- **Modify** `elohim/elohim-storage/src/graph_views/shefa/distribution.rs` — re-express its `rows.len()` aggregate via `CoverageRollup` (the second first-caller).
- **(If descent is exposed)** `elohim/elohim-views/src/infrastructure.rs` + the relevant `*.schema.json` + generated TS — add an optional descent/deficit field.

---

### Task 1: Slice 0 — N=2 transitive CoverageRollup proof (DB-free, unblocked)

**Files:**
- Modify: `elohim/elohim-storage/src/recursion.rs` (append to `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `CoverageRollup::rollup`, `ChildCoverage::readable`, `CoverageSet::{interval,full,union}`, `CoverageDomain::CorpusBytes`, `.covered`, `.deficit`, `.descend()`.
- Produces: a passing proof that rollup composes transitively (no new public API).

- [ ] **Step 1: Write the failing transitive test**

```rust
    // N=2: a region rolls up two council rollups; descent + coverage compose.
    #[test]
    fn rollup_composes_transitively_at_two_levels_preserving_descent() {
        // Two councils, each a rollup of its households (level 1).
        let council_a = CoverageRollup::rollup(
            "council:a", CoverageDomain::CorpusBytes, CoverageSet::full(50),
            &[child("h1", 0, 30), child("h2", 30, 50)],   // covered 0..50 — fully covered
        );
        let council_b = CoverageRollup::rollup(
            "council:b", CoverageDomain::CorpusBytes, CoverageSet::full(50),
            &[child("h3", 0, 20)],                          // covered 0..20 — deficit 20..50
        );
        // The region rolls up the two councils (level 2): each council is a child whose
        // covered-set is its own covered, lifted into the region's 0..100 keyspace.
        let region = CoverageRollup::rollup(
            "region:x", CoverageDomain::CorpusBytes, CoverageSet::full(100),
            &[
                ChildCoverage::readable("council:a", CoverageSet::interval(0, 50)),
                ChildCoverage::readable("council:b", CoverageSet::interval(50, 70)),
            ],
        );
        // (1) coverage composes: region.covered == union of the councils' lifted coverage
        assert_eq!(region.covered, CoverageSet::interval(0, 70));
        // (2) the externality is the region's descent target
        assert_eq!(region.deficit, CoverageSet::interval(70, 100));
        // (3) descent preserved at the top: both councils reachable (sorted CIDs)
        assert_eq!(region.descend(), &["council:a".to_string(), "council:b".to_string()]);
        // (4) two-level descent: a household trapped in council_b's deficit is reachable by
        //     walking constituents region -> council_b -> its households.
        let by_cid = [(&council_a.scope_cid, &council_a), (&council_b.scope_cid, &council_b)];
        let cb = by_cid.iter().find(|(c, _)| *c == "council:b").map(|(_, r)| *r).unwrap();
        assert!(!cb.is_covered());                          // council_b has a real gap
        assert_eq!(cb.descend(), &["h3".to_string()]);      // and you can descend to its leaf
    }
```

- [ ] **Step 2: Run it — Expected PASS** (recursion.rs already implements `rollup`; this proves composition, no production change).

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib recursion::tests::rollup_composes_transitively`
Expected: PASS. (If `ChildCoverage::readable`'s exact name/signature differs, read `recursion.rs:30-188` and match it — adapt the test, not recursion.rs.)

- [ ] **Step 3: Clippy + commit**

```bash
RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv cargo clippy --manifest-path elohim/elohim-storage/Cargo.toml --lib -- -D warnings
git add elohim/elohim-storage/src/recursion.rs
git commit -m "test(recursion): prove CoverageRollup composes transitively at N=2 with descent preserved

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Slice 1 — re-express `resilience_snapshot.rs` as a CoverageRollup caller

**Files:**
- Modify: `elohim/elohim-storage/src/graph_views/shefa/resilience_snapshot.rs`
- (If exposing descent) Modify: `elohim/elohim-views/src/infrastructure.rs` + `resilience-snapshot-view.schema.json` + generated TS

**Interfaces:**
- Consumes: the existing `steward_result.rows` (each row has `steward_cid`) and `committed_result.rows` (the commitment-backed stewards); `CoverageRollup`, `CoverageSet`, `ChildCoverage`.
- Produces: the same `stewarding_count`/`commitment_backed_count` values (regression-preserving) NOW derived from a `CoverageRollup`, with the steward CIDs preserved as `constituents` and the diversity `deficit` available for descent.

- [ ] **Step 1: Characterize the current behavior (regression anchor).** Read `resilience_snapshot.rs:22-45`. The two `rows.len() as i32` sites (`stewarding_count` at `:29`, `commitment_backed_count` at `:38`) are the descent-erasure. Note the exact view fields they feed (`ResilienceSnapshotView.stewardingCollectives` etc.) — these byte-for-byte outputs MUST be preserved (write a golden assertion of the current numbers for a known fixture first).

- [ ] **Step 2: Write the failing test — counts preserved AND descent available**

```rust
    // In resilience_snapshot.rs tests (or a sibling test module with a graph fixture):
    #[test]
    fn stewarding_aggregate_preserves_count_and_exposes_descent() {
        // Given a content with 3 distinct stewards (target diversity), 2 achieved:
        let stewards = vec!["steward-a".to_string(), "steward-c".to_string()]; // 2 of 3
        let target = 3u64;
        let rollup = build_stewarding_rollup("content-x", target, &stewards); // the new helper
        // (1) regression: the count the view exposes is unchanged
        assert_eq!(rollup.constituents.len() as i32, 2);
        // (2) descent NEW: which stewards (not just how many)
        assert_eq!(rollup.descend(), &["steward-a".to_string(), "steward-c".to_string()]);
        // (3) the externality: 1 slot short — the descent target a consumer can act on
        assert_eq!(rollup.deficit.measure(), 1);
    }
```

- [ ] **Step 3: Implement the `build_stewarding_rollup` helper + re-wire the builder.** Add a small pure helper that maps the steward CIDs onto the diversity keyspace and rolls up:

```rust
/// Diversity coverage as a CoverageRollup: required = full(target distinct collectives);
/// each achieved steward covers one slot; constituents = the steward CIDs (descent preserved).
fn build_stewarding_rollup(content_cid: &str, target: u64, steward_cids: &[String]) -> CoverageRollup {
    let children: Vec<ChildCoverage> = steward_cids.iter().enumerate()
        .map(|(i, cid)| ChildCoverage::readable(cid.clone(), CoverageSet::interval(i as u64, i as u64 + 1)))
        .collect();
    CoverageRollup::rollup(content_cid, CoverageDomain::CorpusBytes, CoverageSet::full(target.max(steward_cids.len() as u64)), &children)
}
```
Then in `build(...)`, derive `stewarding_count` from `rollup.constituents.len() as i32` (regression-identical), keeping the rollup so its `deficit`/`descend()` are available. Do the same for the commitment-backed set at `:38`. The `target` diversity floor comes from the existing `floor_for_tier`/resilience floor (reuse it; default "standard" floor if undeclared).

- [ ] **Step 4: (Optional, if descent is exposed) add the view field.** If the snapshot should surface the descent, add an optional `coverageDeficit`/`shortfallSlots` field (not-selected-field contract) + schema + codegen. If keeping this slice minimal (count-preserving, internal rollup only), DEFER the view exposure and note it — the regression win (counts route through the descent-preserving primitive) lands either way.

- [ ] **Step 5: Run the golden + the new test + clippy; commit** (`feat(recursion): resilience_snapshot aggregates via CoverageRollup — descent preserved (first caller)`). Counts byte-identical; descent newly available.

---

### Task 3: Slice 2 — re-express `distribution.rs` (the second shefa first-caller)

**Files:**
- Modify: `elohim/elohim-storage/src/graph_views/shefa/distribution.rs`

**Interfaces:**
- Consumes: `distribution.rs`'s `rows.len()` diversity aggregate + the `build_stewarding_rollup`-shaped helper (lift it to a shared `graph_views/shefa/` helper if both builders use the identical mapping — DRY).
- Produces: the same distribution counts, derived from a `CoverageRollup`, descent preserved.

- [ ] **Step 1: Read `distribution.rs`, find its `rows.len()` site(s), and confirm the count is a DIVERSITY/coverage aggregate** (not an unrelated tally). If it is not coverage-shaped, STOP and report — defer it like `topology_overview.rs` (note in the report).

- [ ] **Step 2: Write the failing count-preserving + descent test** (mirror Task 2 Step 2 against distribution.rs's fixture).

- [ ] **Step 3: Re-express via the shared rollup helper** (extract the Task-2 helper into a `graph_views/shefa/coverage.rs` shared fn if both callers use it — one mapping, two callers, per §3.1 "first callers").

- [ ] **Step 4: Run tests + clippy; commit** (`feat(recursion): distribution view aggregates via CoverageRollup — second first-caller`).

---

## Self-Review

- **Spec coverage:** recursive-architecture §3.1 item 1's remaining half ("re-express the two shefa builders as its first callers") → Tasks 2-3; the §1.3 transitive-descent identity → Task 1's proof. The operator built `CoverageRollup` (§3.1 item 1's first half); this plan wires the callers. ✓
- **Placeholders:** Task 1 carries exact, runnable code against the verified API; Tasks 2-3 carry the helper + the test shape, and reference the builder sites for the exact `rows.len()` lines + the `floor_for_tier` reuse (the diesel/graph glue the implementer reads in-file — the one place a verbatim block would go stale, like the lens loader).
- **Type consistency:** `build_stewarding_rollup -> CoverageRollup`; counts derived as `constituents.len() as i32` (regression-identical to the old `rows.len() as i32`); `deficit.measure()` is the shortfall. `CoverageDomain::CorpusBytes` used uniformly. ✓
- **Scope:** strictly the two shefa first-callers + the N=2 proof. Council N-level, Governor lift, the lens cluster→council wiring (CoverageDomain fork), and `topology_overview.rs` are explicit non-goals/deferred.
