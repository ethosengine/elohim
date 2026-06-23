---
title: "EPR / Content-Perspective Facing — the content atom's first-person fold over its leg-neighborhood"
id: epr-content-perspective-facing-lens-design
status: Draft
class: protocol-canonical
domain: D5
topic: [epr, content-perspective, facings, lens, coupling, legs, neighborhood, dataplane]
refines:
  - genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
informed-by:
  - genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
cites:
  - resilience-facings-select-fold-aggregate-design | the select→fold→aggregate lens framework (§11) this facing is a child of — its materialized-relation + pure-fold + typed-view substrate | sha256:93279fd25a0600d1 | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - lens-complete-epr-resolution-four-leg-coupling-design | the four-leg coupling law + §2 projection law this lens facing ultimately descends from | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
requires_env: [household-nodes]
---

# EPR / Content-Perspective Facing — the content's own view of its dataplane place

> **One-line:** the EPR facing is the content atom's first-person fold — `(epr_cid, leg, target_cid)` rows bucketed into its knowledge·value·governance neighborhood — a named fold over a flat materialized leg-relation, the least-complete sibling of the proven resiliency facing.

## 0. Provenance

Surfaced 2026-06-19 from the same operator framing as the parent facings spec: the atom layer (`/api/v1/epr/{cid}/envelope`) *carries* the legs while the operator resolver (doorway 302) *drops* to one — the EPR home needs a home of its own. This **refines** `2026-06-19-resilience-facings-select-fold-aggregate-design.md` (the select→fold→aggregate framework) and `2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md` (the four-leg coupling-law); the resiliency facing (`household_resilience::snapshot`) is the proven reference.

## 1. The materialized relation this lens SELECTS

The leg-neighborhood is a **flat 3-column relation**, not a closure-walk. SELECT materializes the framework's `LegRow` from `epr_coupling(epr_cid, leg, target_cid)` (`db/epr_atoms.rs`), read 1-hop forward via `fetch_coupling_for_atom` and reverse via `fetch_reverse_coupling` — the only diesel touch, storage-side loaders. The grounding is honest here: the depth-capped, reach-bounded `ClusterClosure` (acquisition-pull-queue §5.1) is **design-only and unwired to legs** — it is named as the future deepening, not the v1 relation. The rival graph-side `epr_edge`/`NEIGHBORHOOD` Datalog walk (`graph/primitives.rs`) feeds other views and is **not** joined here.

```rust
// elohim-facings/src/relation.rs  (reuses the framework primitive)
pub struct LegRow { pub epr_cid: String, pub leg: String, pub target_cid: String, pub reverse: bool }
```

## 2. The FOLDS it computes (pure fns over the relation)

- **`fold_legs(rows: &[LegRow]) -> EprCouplingView`** — `bucket_by(rows, |r| Some(r.leg.clone()))`, one target per known leg. Metric: leg-coverage (how many of the canonical legs are coupled). Lifts `to_envelope_view`'s `match row.leg.as_str()` loop (`api/epr.rs`) into the pure crate, DB-free testable.
- **`neighborhood_degree(rows: &[LegRow]) -> usize`** — `distinct_count_by(rows, |r| r.target_cid.clone())`. Metric: out+in coupling fan (the content's immediate dataplane reach).
- **`reverse_part_of(rows: &[LegRow]) -> Vec<String>`** — the `reverse` rows, who couples *to* this atom. Metric: inbound-citation count.

The first two are the composability proof — both are the framework's generic combinators, no loop.

## 3. The typed VIEW output + HTTP surface

`fold_legs` returns the existing `EprCouplingView { knowledge, value, governance }` (`elohim-views/src/epr.rs`, ts-rs camelCase) embedded in `EprEnvelopeView`. **No new POST.** Extend the existing GET `/api/v1/epr/{cid}/envelope` (`api/epr.rs`); add the §6 Slice-2 inspector GET `/api/v1/epr/{cid}/raw` returning legs + degree + reverse. Each new route needs **two gates**: the match arm AND a doorway `is_service_path` arm (the EPR-router shadow trap), plus an `INTERFACE_FILES` entry for any new view.

## 4. Aggregation level(s)

Per-atom (the legs of one content CID) is the only v1 level. Per-neighborhood rollup (closure-join across coupled atoms) is the unbuilt deepening; there is **no dashboard verdict-rollup** for this facing in v1 — honest absence, not omission.

## 5. P2P Design Gate output

**Operational-C, zero new DHT types.** This facing *projects* already-notarized EPR atoms (`epr_codec.rs` hashes `content_format` into the CIDv1 — self-certifying identity already exists); it mints nothing. The one thing to watch: adding the missing **4th `process` leg** extends an existing entry's `coupling` field — it does **not** mint a new entry type. No exception to flag.

## 6. Slices (sequence)

**Blocked-until (lens, not fold):** unlike the other facings, v1 leg-bucketing reads *existing* `epr_coupling` rows (the 3 current legs), so the proof fold is **not data-blocked** — it is the cheapest of the four to light. What IS blocked: 4-of-4 leg completeness (the `process` leg has **no data** and `epr_compose.rs` does not enforce it — the `_ => {}` leg drop), and the per-neighborhood deepening (the `ClusterClosure` closure-walk is unbuilt). So v1 is honestly *leg-bucketing over the 3 existing legs*, not the architecture's aspirational "fold over all four legs."

- **Slice 0 — proof-gate (test-first, one metric, existing seed):** lift `fold_legs` into `elohim-facings` as a pure fn; unit-test with a hand-built `Vec<LegRow>` (no DB), asserting a seeded atom's `epr_coupling` rows render a non-empty `EprCouplingView`. Mirrors `build_felt_status`. Gate cutover on byte-identical `/envelope` JSON before/after.
- **Slice 1:** add the `process` field to `EprCouplingView` + the loader; the `_ => {}` drop becomes a populated leg once seed exists.
- **Slice 2:** `/api/v1/epr/{cid}/raw` inspector (legs + `neighborhood_degree` + `reverse_part_of`).
- **Slice 3:** join the flat leg-relation to the bounded `ClusterClosure` — per-neighborhood aggregation.

## 7. Non-goals / operator-owned

Out of scope (each is loader-and-view or resolver work, not fold work): building the `ClusterClosure` closure-walk; **enforcing the compose-law** (`compose_epr` does only `ReachVerdict`, legs are all `Option<String>` — atoms can be put leg-less; `epr_compose.rs`); the lens-complete resolver that dispatches focal render on `content_format`; reconciling the reach vocabularies (backlog item 13). The framework gives this lens a home; it does not author the data behind it.
