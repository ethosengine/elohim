---
title: "Closure Posture — what silence means, declared per axis"
id: closure-posture-axis-card-plan
status: Landed
class: protocol-canonical
created: 2026-07-24
domain: D2
topic: [ontology, closure, axis-registry, verdict, open-world, agentic-negotiation]
cites:
  - genesis/research/owl2-graduation-floor-ceiling-ontology-2026-07-23.md
  - ontology-keel-slice1-verdict-spine-plan | Ontology Keel Slice 1 | sha256:059f604e7ebc7821 | path: genesis/docs/superpowers/plans/2026-07-23-ontology-keel-slice1-verdict-spine-plan.md
  - reach-ontology-vocabulary-split-spec | Reach Ontology/Vocabulary Split | sha256:2a1ef52c1ced3c48 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - elohim/epr/src/verdict.rs
  - elohim/epr-rea/src/epistemic.rs
  - elohim/sdk/schemas/v1/registries/axes/epistemic.axis.json
---

# Closure Posture — what silence means, declared per axis

> **The founding decision.** Every gradient axis has two layers with **opposite** postures toward
> absence, and until now neither was declared. On the **fact** layer absence means *unknown* — a
> peer's non-observation is not a negative fact, and treating it as one manufactures evidence.
> On the **verdict** layer absence means *refuse* — the absence of a permit is a denial, and
> treating it as "unknown, therefore possibly allowed" is the fail-open inversion. The bridge
> between them is a **classifier that must be total**: `permit | refuse | refer`, with `refer`
> reachable from every path. This slice makes that declaration structural instead of remembered.

## Why now, and why this shape

The axis-card format landed 2026-07-23 with **exactly one instance**
(`registries/axes/epistemic.axis.json`) and **no schema** — `schema:test` globs `enums/` and does
not look at `registries/` at all. A required field costs one file today and a retrofit-plus-argument
across N cards later. This is the Cedar ethic already in the graduation ledger's binding laws:
cheap now, expensive to retrofit.

The distinction is not imposed — it is already load-bearing and undeclared in shipped code.
`epistemic.rs` has an **open** fact layer (no review ≠ dismissal; absence yields `emergent`, and
`standing_ratio()` returns `None` rather than 0.0 on no evidence) and a **closed** verdict layer
(`classify()` is total; `cite_gate()` routes contest to `refer`). One axis, both postures. Reach is
the same shape: observations open, `ReachVerdict` closed.

**What it buys beyond tidiness.** Under an assumption of ubiquitous embedded agentic negotiation,
the scarce resource is *mutual verifiability without shared code*. A counterparty that reads an axis
card must be able to tell, mechanically, whether silence on that axis is an admission or a denial —
because getting it backwards is the whole attack. `closure.facts == "open"` also marks exactly the
vocabulary that is safe to project to RDF/Turtle for foreign agents (the open-world posture is
correct there, and the projection changes zero verdicts by construction). `closure.verdicts ==
"closed"` marks the vocabulary that must never leave as RDF, because open-world semantics at the
verdict layer is the ex-falso fail-open the OWL2 graduation refused. The expand/limit decision stops
being one global architecture bet and becomes a per-axis declaration with a gate behind it.

## P2P Design Gate: Closure Posture Declaration

### Entity: `closure` vocabulary (`open` | `closed`)
- **Classification**: not an entity — a **closed protocol vocabulary** (`elohim/sdk/schemas/v1/enums/`),
  app-tier. **No `_dna` block**: adding a DNA constant moves integrity code; graduation is a later
  deliberate DNA-lineage event, declared here, not executed.
- **Content Address Strategy**: n/a (a vocabulary, not an instance).
- **Source of Truth**: the schema file; codegen projects to TS (glob-based auto-discovery over `enums/`).
- **Anti-Pattern Check**: drift check performed.
  - `plane` is **rejected as the term** — 427 occurrences repo-wide (dataplane / control plane) and
    `EdgePlane{Doc,Sidecar}` (`elohim/eprfs/epr-cli/src/flow/edges.rs:29`) already owns it with a
    different meaning (which projection an edge came from). Naming this `plane` would be the
    five-way reach drift, re-run deliberately.
  - `closure_rule` (`v1/commitments/replicates-content.schema.json:44`) is a free-text replication
    scope field — unrelated domain, recorded as `relatedVocabularies`, no collision.
  - No route, no table, no UUID, no new DHT entry type.

### Entity: `axis-card.schema.json` (meta-schema)
- **Classification**: not an entity — a validator for registry declarations. No persistence, no wire form.
- **Source of Truth**: the schema file; enforced by a new `registries/` leg in `schema:test`.
- **Anti-Pattern Check**: none apply — codegen/validation surface only.

### Entity: `reach.axis.json` (backfill)
- **Classification**: not an entity — a registry declaration. **Owed already**:
  `epistemic.axis.json:9` names reach in `orthogonalTo` as *"not yet carded."*
- **Anti-Pattern Check**: the card **describes** `reach_earning.rs`; it must not restate the reach
  vocabulary (that lives in `epr:schema:enum:reach` and is `$ref`'d, never copied).

### Design Constraints Discovered
- Axis cards are convention-only today: one instance, zero validation. Formalizing costs one backfill.
- `schema:test` builds its AJV `idMap` from `enums/` only; the registries leg must load enum schemas
  first so `vocabulary.$ref` resolves.
- `ReachVerdict` (`Allowed`/`Blocked`/`Pending`) predates the `Decision` spine and does not speak it
  yet; ontology-keel Task 4 owns that conversion. This slice **declares** the classifier, it does not
  rewrite it.
- `policy_ref: None` at three sites in `reach_earning.rs` — a closed-verdict axis that does not cite
  the policy producing its verdict is unauditable by a counterparty. Recorded as a declared gap on
  the card, not fixed here (it needs the manifest revision→contentHash pin, which is its own slice).

## Global Constraints

1. **No DNA changes, no new tables, no new routes, no migrations.** Vocabulary + meta-schema +
   declarations + gates only.
2. **The two closure laws are protocol-owned** and stated once, in the `closure` enum description:
   absence of an assertion on an **open** layer is *unknown*, never a negative fact; absence of a
   permit on a **closed** layer is *refuse*, never "possibly allowed."
3. **Totality law**: any axis declaring `closure.verdicts == "closed"` MUST declare a `classifier`,
   and that classifier's return type must be able to express `refer`. The schema enforces the
   declaration; Rust's exhaustiveness enforces the totality.
4. **`refer` is never a fallthrough.** Restated on every card, because it is the single most
   re-drifted concept in the corpus (`ask` / `FlagForHuman` / `Pending`).
5. **Cards describe, never duplicate.** `vocabulary` is a `$ref` to the enum's `$id`. A card that
   inlines its value list has forked the vocabulary.
6. **Path-limited commits only.** The worktree carries in-flight foreign diffs; never `git add -A`.
7. **Born-linked**: this plan and any doc edits seal via `cite-gen.py --seal` — never a hand-written
   slug, fingerprint, or path.

## Tasks

- [x] **Task 1 — `closure` vocabulary.** `v1/enums/closure.schema.json`: `["open","closed"]`, house
      `$id` form (`epr:schema:enum:closure`), description carrying both laws + the source-of-truth
      pointer. No `_dna`, no `_ordinal` (the two values are not ordered — they are opposite postures,
      not a gradient). Verify codegen auto-discovery via `pnpm run schema:codegen:ts`.
- [x] **Task 2 — axis-card meta-schema.** `v1/registries/axis-card.schema.json` formalizing the
      format `epistemic.axis.json` already implies: required `$id`, `axis`, `version`, `kind`,
      `semantics`, `vocabulary`, `closure`, `sourceOfRecord`. `kind` ∈ `declared | derived |
      two-layer`. `closure` = `{facts, verdicts}`, both required, both `$ref` the closure enum.
      Conditional: `closure.verdicts == "closed"` ⇒ `classifier` required
      (`{crate, module, function, returns}`).
- [x] **Task 3 — registries validation leg** in `scripts/test-schema.mjs`: load every
      `registries/axes/*.axis.json`, validate against the meta-schema, assert the conditional fires
      (a closed-verdict card without a classifier must FAIL — assert the negative, not just the
      positive), and assert `vocabulary.$ref` resolves to a real enum `$id`.
- [x] **Task 4 — backfill `epistemic.axis.json`.** Add `closure: {facts: open, verdicts: closed}`
      with the per-layer rationale drawn from the shipped code (`standing_ratio() -> Option<f64>`
      is the open-layer evidence: no reviews yields `None`, not zero), and the `classifier` block
      naming `elohim-epr-rea::src/epistemic.rs::classify` / `cite_gate` returning
      `elohim_epr::verdict::Decision`.
- [x] **Task 5 — card `reach`.** `registries/axes/reach.axis.json`, `kind: two-layer` (declared
      floor + derived verdict, per the vocabulary-split spec §2), `closure: {facts: open,
      verdicts: closed}`, `vocabulary` → `epr:schema:enum:reach`, classifier →
      `elohim-storage::src/services/reach_earning.rs::evaluate`. Record two honest declared gaps:
      `ReachVerdict` does not yet speak `Decision` (ontology-keel Task 4), and `policy_ref` is
      unpopulated. Update `epistemic.axis.json`'s `orthogonalTo` to drop "not yet carded."
- [x] **Task 6 — a2o capture.** `genesis/a2o/features/content/closure-posture.feature`: absence on an
      open layer is not a negative fact; absence of a permit on a closed layer is a refusal; a
      closed-verdict axis without a declared classifier fails the gate.
- [x] **Task 7 — gates + commits.** `pnpm run schema:test`, `schema:validate`, `schema:codegen:ts`
      freshness; `cargo fmt --check` + clippy + nextest on `elohim/epr` and `elohim/epr-rea` at their
      pool slots (`RUSTFLAGS=""`, `CARGO_TARGET_DIR` per `cargo-pool key`). Path-limited commits, one
      per concern. Branch left un-pushed — the integrator owns push.

## Landing note (2026-07-24)

Landed at `0313c8b5f..111553118`. **Two deviations from the plan as written, both deliberate:**

- **Task 0 (branch) was not taken.** The worktree is shared with concurrent sessions and already
  carried a same-session commit on `integ/resilience-card-governance`; switching branches under a
  shared worktree would have moved another session's ground. Committed path-limited on the current
  branch instead. Un-pushed — the integrator owns push.
- **Task 7's cargo legs were not run, because no Rust changed.** The slice is JSON + one `.mjs` +
  one `.feature`. `elohim/epr` and `elohim/epr-rea` are untouched; the totality law is enforced at
  the schema layer here, and Rust exhaustiveness already enforces it in `Decision`.

**Gate evidence.** `pnpm run schema:test` — 19 new assertions, all green, including all four
negatives (the totality-law conditional is proven to bite, not merely present). The two `ContentView`
failures at HEAD are **pre-existing on this branch**, measured against a baseline with this slice's
changes removed: baseline 51 passed / 2 failed → 70 passed / 2 failed. Zero introduced.
`pnpm run schema:validate` 3431 valid / 0 errors. `codegen-ts --verify` reports up to date — `closure`
carries no `_dna` block, so no constants generate and the generated tree stays byte-identical
(the known Prettier line-wrap oscillation is not triggered).

## What this slice deliberately does not do

Emit any RDF/Turtle projection (a later slice reads `closure.facts` to decide what is projectable) ·
touch any DNA · convert `ReachVerdict` to the `Decision` spine (ontology-keel Task 4 owns it) ·
populate `policy_ref` or content-pin policy revisions (needs the manifest pinning slice) · card the
remaining axes (witness-ladder, compute/trust variance, retention) · change `.epr-meta`'s fail-open
authoring default — that boundary gets **marked** on the cards, never moved here · build any rule
engine, reasoner, or inference layer.
