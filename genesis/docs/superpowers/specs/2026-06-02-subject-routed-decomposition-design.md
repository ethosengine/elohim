---
title: "Subject-Routed Decomposition — A Brainstorm-Time Class Gate That Binds Write-Location and Decompose-Flow"
id: subject-routed-decomposition-design
status: Draft
created: 2026-06-02
tier: design-spec
class: process-meta
process_subdomain: doc-lifecycle
topic: [decomposition, routing, subject-class, manifest, brainstorm-gate, placement, meta-vs-substrate, gospel-diff, decompose-flow, MAP-axis, process-meta, method-bridge, run-output, lossless-discard]
cites:
  - placement | the contract this proposes a subject-class axis section for, mirroring its cluster-state env pointer | sha256:f84d7cb16bea9379 | status: stale — target content moved on; re-verify | path: genesis/docs/PLACEMENT.md
  - genesis/docs/claude.md
  - map | the product-domain lattice this proposes Axis 0 above — subject class before D#+pillar | sha256:de878342b28843e8 | status: stale — target content moved on; re-verify | path: genesis/docs/content/elohim-protocol/architecture/MAP.md
  - .claude/scripts/memory-kit/decompose.py
  - .claude/commands/brainstorm.md
  - .claude/memory-kit/CLAUDE.md
derived_from:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md     # the shared-vocabulary PRODUCT seed every process spec dogfoods — a CITATION breadcrumb, never a domain claim
refines:
  - genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md  # turns its single-flow §5.2 three-fates table into a per-class multi-flow keyed on class:
proposed_amendments:
  - genesis/docs/PLACEMENT.md                                  # PROPOSED §3 — the subject-class axis section pointing at .claude/subject-routing.yaml (mirrors how it points at cluster-state.yaml for env)
  - genesis/docs/content/elohim-protocol/architecture/MAP.md   # PROPOSED §4 — Axis 0 (subject class) above D#+pillar; map_has_meta_axis→true
  - .claude/scripts/memory-kit/decompose.py                    # PROPOSED §5 — frontmatter parser + cascade-resolver + class-stamp + fail-loud gate_signals
  - .claude/commands/brainstorm.md                             # PROPOSED §5 — Step 1c.0 CLASSIFY-SUBJECT (reads cascaded routing); Step 4 class-conditional frontmatter
requires_env: []
---

# Subject-Routed Decomposition

> Grounded by the `subject-routed-decomposition` workflow (corpus survey + single-flow map + mis-placement audit + gospel inventory + adversarial verify). Where this spec asserts a corpus fact, the workflow verified it on disk.

## 1. The gap

A spec's *subject* determines how it should decompose, but nothing classifies subject. The decompose flow is **single-flow** — it routes every spec, regardless of subject, through one substrate-shaped fate table (`compaction-loop §5.2`: durable truth → `architecture/`, verified behavior → `a2o/<pillar>` + pillar code, lesson → `history/`). That flow **assumes the spec is protocol/product-shaped**. But ~30 of ~53 sampled specs are **process-meta** (they build the dev machinery — memory-kit, skills, agents, CI, the loops), and ~9 are **method-bridge** (process work that dogfoods product primitives). For those, the substrate legs are wrong: there is no `<pillar>` scenario and no `architecture/` seed; the durable residue is a **CLAUDE.md gospel-diff + a `.claude/` tool**, and the prose is scaffolding to discard.

**Root cause (verified):** `MAP.md §1` has exactly two axes — the D1–D10 concern lattice and the pillar code-location axis — **both substrate-scoped** (`map_has_meta_axis: false`). A process spec has no honest D# and no pillar, so its author reaches for the nearest-sounding product domain, and it is **always D4** ("Memory Lifecycle — the comet"), because the dev tooling *dogfoods the protocol's own vocabulary* (compact / merge / forget / the comet / cites-as-edges). The collision is irresistible: a doc-citation audit borrows "the comet — links survive dissolution," files under D4, and cites the product seed `memory-lifecycle-design.md` as canonical parent. `PLACEMENT.md §12` already admits the consequence: *"Dev-doc / functional-doc home undefined… a fourth home risks re-opening a dumping ground."* The fourth home is not a new directory — it is the `.claude/`+CLAUDE.md gospel-diff target the process classes already decompose into; it just has no *classifier* routing residue there.

## 2. Two axes, not one

Routing needs **two orthogonal axes** (the adversary proved the single class axis under-covers):

- **`class`** — *whose experience does the deliverable change, and where does the landed change physically live?* **Two classes** (collapsed from the survey's four — §7): `protocol-canonical` (learner/peer · `app/`+`architecture/`) and `process-meta` (developer/agent · `.claude/`+CLAUDE.md). Plus the unresolved state `provisional` (reconciled at the BACK-fire, not assigned at the front), and two modifiers — `status: vision` (the subsumption-**archetype** mode of protocol-canonical) and `derived_from: <product-seed>` (the dogfood-lineage breadcrumb on a process-meta spec that borrows product vocabulary — *replaces the former method-bridge class*, watched by the `vocab-vs-target-mismatch` gate signal).
- **`artifact_kind`** — *what shape of doc is this?* `spec` · `plan` · `brainstorm` · `kickoff`/`handoff` · `run-output` (shift-result, horizon-scan, ceremony-rewrite, story-harvest, audit report). **Run-outputs are loop OUTPUTS, not design→plan→land docs** — they route by retention (comet/`TRAJECTORY.md` or git), never into a fake D# + pillar a2o leg. The class axis routes *design/plan* docs only; `artifact_kind` catches everything the class axis would silently default into the substrate flow.

**The discriminator (binding rule):** classify by **deliverable-TARGET**, never by **vocabulary**. A spec that says "comet / EPR / cites" but amends only `.claude/scripts/memory-kit/*.py` + `PLACEMENT.md` (0 product-code refs) is process-meta, full stop. The product-vocabulary `informed-by:`/`derived_from:` is a **lineage breadcrumb**, not a routing key.

## 3. The classes and their decompose-flows

### protocol-canonical — the rich flow (unchanged)
The existing `compaction-loop §5.2` single flow *is* this class's flow: durable truth → `architecture/` (compact); verified behavior → green `a2o/features/<pillar>/*.feature` + code under `app/|elohim/|doorway/|steward/` (verify); dead-path lesson → `history/` (curate); human-narrative → `genesis/data/stories/` (graduate); research → `genesis/docs/research/`; residual → `backlog/`; body → git (forget). **Dual-target fix:** a protocol-canonical spec that also lands a CI/build change sheds a **process-residue leg** → `genesis/orchestrator/README.md` + root `CLAUDE.md` CI section.

**`status: vision` mode (subsumption archetypes).** A protocol-canonical spec in `vision` mode is a subsumption study ("how would Google Drive / a bank compose from substrate primitives?"), born canonical in `architecture/applications/` (or `horizons/`). Its decompose-flow differs only in that the *study mapping STAYS* (it is the durable artifact, not scaffolding) and a surfaced primitive-gap sheds to `MAP.md §3` Gap-Ledger + backlog. Its next-state (preserving the every-doc-has-a-next-state invariant): **stay-as-reference** or **graduate-to-impl** when an archetype's primitives get scheduled (the corpus shows this happens — there is no over-commit alarm).

### process-meta — the light flow
- **Durable operating-discipline (gospel-diff)** → the matching CLAUDE.md by subdomain: cross-cutting build/test/workflow → root `CLAUDE.md`; memory-process → `.claude/scripts/memory-kit/CLAUDE.md` (tooling) + `.claude/memory-kit/CLAUDE.md` (data) + `LIFECYCLE.md` (primitives); doc-placement → `PLACEMENT.md` + `_state/README.md`; CI → `genesis/orchestrator/README.md`; BDD → `genesis/a2o/CLAUDE.md`; chronicle/roadmap/backlog → `genesis/data/timeline/CONVENTIONS.md`.
- **Reusable executable capability (tool)** → a `.claude/` home: `skills/<n>/SKILL.md` · `agents/<n>.md` · `commands/<n>.md` · `hooks/<n>.py` (deterministic enforcement) · `workflows/<n>.js` · `scripts/{memory-kit,converge,_lib}/` · `schemas/<n>.json` · `prompts/<n>.md`.
- **Lesson / tool-design rationale (history leg — LIVE, not NULL)** → `history/` as a `type:history-gotcha` record when it carries reusable reasoning (a tried-and-failed mechanism, a threshold/rejected-alternative behind a tool's shape), bidirectionally linked to its process-canonical (the governing CLAUDE.md / `.claude/` tool). *Corpus-proven:* `history/` already holds narrow process lessons (the CI-orchestrator anti-patterns museum; `2026-06-02-archetype-primary-a2o-taxonomy-not-executed`). The `history/` leg is **judgment-gated**, not auto-skipped — only the `architecture/` and `a2o/<pillar>` legs are NULL for this class.
- **Generated reports** → comet / `.claude/memory-kit/TRAJECTORY.md` (separate retention discipline; does not decompose-to-zero).
- **Residual** → `backlog/` (tagged `class:process-meta`). **Body** → git (forget) — *after* the gospel-diff + tool + (judgment) curated lesson exist.

### (former method-bridge) — collapsed into a `derived_from:` breadcrumb
The survey's fourth class is **not a home** — it routes identically to process-meta, so it is collapsed (§7 decision). A process-meta spec that *borrows product vocabulary* (dogfoods EPR/comet/cites/REA) carries a `derived_from: <product-seed>` breadcrumb — a **citation, never a domain claim**. The `vocab-vs-target-mismatch` gate signal watches for the failure mode (product `domain:` set while all targets are under `.claude/`). The **one seam** where such a spec legitimately touches the product tree: a verified substrate-gap the dogfood exposed sheds to `MAP.md §3` Gap-Ledger + a protocol-canonical backlog item. (This very spec is the proof: hand-labeled `process-meta`, `derived_from:` the memory-lifecycle seed — no fourth class needed.)

## 4. The unifying keep/discard principle (lossless by construction)

> **Discard the FORM; keep any RESIDUE that carries reusable reasoning — regardless of class.**

The *form* (dispatch choreography, plan checkboxes, prose ordering, the raw body) retires to git. A *why-we-turned*, a *tried-and-failed*, a *tool-design rationale*, a *mis-classification forensic* is **kept** as a curated record. **Class decides the HOME** (`architecture/` vs a CLAUDE.md vs `history/`); **reasoning-value decides whether a curated record is written at all.** A one-off process finding that records a failure or a non-obvious constraint defaults to a tiny `history-gotcha` **stub** (reusing the compaction-loop §5.3 AUTO stub op), promotable on recurrence, demotable to a CLAUDE.md one-liner only after an agent confirms it carries no reusable why. **Stub-then-grade is the default; git-forget is the explicit exception.** This replaces the per-class enumeration of what's lossy with one test, and it is what makes "throw the rest away" actually lossless.

## 5. The mechanism

### 5.0 The three-tier cascade (parent sets options; children specialize)

Routing config cascades like Claude Code's own `user → project → local`, and like the protocol's manifest/SDK IoC (the contract sets the vocabulary; implementations comply):

| Tier | Scope | Artifact | Role |
|---|---|---|---|
| **Method pattern** | user (`~/.claude/`) or a skill | the *generic* "classify subject → bind location + flow" gate logic | reusable across repos; knows *that* classes route, not *which* |
| **Class constitution (PARENT)** | **repo** — `.claude/subject-routing.yaml` | THIS repo's `class` values + the decomposition-home menu + the `artifact_kind` axis | the OPTIONS; the FRONT/BACK gates read it |
| **Operational manifests (CHILDREN)** | repo — `.claude/memory-kit/*.yaml`, `genesis/manifests/cluster-state.yaml` | stasis budgets, env-availability — specialize *within* the parent's options | the bindings |

The **class values are repo-scoped** (`protocol-canonical` only means something in *this* repo; another repo names different classes), so the constitution is committed at `.claude/` root alongside the existing repo-constitution tier (`settings.json`, `file-relationships.json`, `horizon-scan-sources.md`) — **not** in `memory-kit/`, which is only *one* of its consumers. Placing it at root also preserves `memory-kit/`'s "only `context-coverage.yaml` is hand-edited" rule (the parent is a new hand-edited config at a *different* dir). The generic *pattern* (the gate logic) is the part that could live user-scope and read whatever repo-parent it finds.

**Nested cascade — one repo → mono-repo (optional, by construction).** The three tiers are really an **N-tier nested cascade that mirrors the CLAUDE.md gospel tree**. The same way `genesis/a2o/CLAUDE.md` extends `genesis/`'s extends root, a sub-project may drop its own `.claude/subject-routing.yaml` (or a routing block in its CLAUDE.md) that cascades *on top of* the root constitution. The FRONT gate resolves the **effective** class menu by walking *up* from the spec's target location and merging — root provides the base classes/homes; each sub-tree optionally **adds** classes (union) or **remaps** a class's decomposition home for its subtree (nearest-wins), exactly as CLAUDE.md cascade loads every gospel on the path and settings cascade lets project override user. A single-purpose repo needs only the root parent; a mono-repo **layers** sub-project parents with no redesign — the cascade *is* the scaling mechanism. Elohim is already this case: a `doorway/` spec can route its residue to `doorway/CLAUDE.md` via a doorway-tier remap, and **sophia (a submodule — its own repo + `CLAUDE.md` + plans-travel-with-code)** resolves its routing on top of Elohim's root — "one repo → mono-repo → sub-repo" with no new machinery. This is the **protocol's own nested-scope federation** (household → collective → commons; the hub abstraction) applied to dev-process routing — the routing cascade is the dev-tooling instance of the nested-scope pattern the product ships.

### 5.1 The parent — `.claude/subject-routing.yaml`

Hand-tuned config (tracked, not gitignored): declares the `class` registry → `{write_location, decomposition_flow, discard_rule}`, the `artifact_kind` axis, and the `gate_signals` (deterministic mis-class detectors). **It is the single answer to "where are all the brainstorming classes."** `PLACEMENT.md` and `MAP.md` each gain a one-line pointer to it as the class axis they lack (mirroring how `PLACEMENT.md` already points at `cluster-state.yaml` for the env axis). One parent, two prose surfaces cite it, two gates read it.

**The FRONT gate — `brainstorm.md` Step 1c.0 (CLASSIFY-SUBJECT, before MAP-PATH):** read the manifest; answer "whose experience does the deliverable change, and where does it land?"; branch — `protocol-canonical`/`application-archetype` → proceed into MAP-PATH (D# + architecture seed + pillar) unchanged; `process-meta`/`method-bridge` → **skip** MAP-PATH's D#+pillar lookup (it cannot honestly answer; forcing it is the root cause), name the process home + `process_subdomain` instead. **Step 4** frontmatter becomes class-conditional: `class:` always; `domain:`+`informed-by:<architecture seed>` only for substrate classes; `process_subdomain:`+`informed-by:<process gospel>` for process classes; `derived_from:<product seed>` as the method-bridge breadcrumb.

**The BACK gate — `decompose.py`:** **hard prerequisite (adversary-verified gap):** `decompose.py` has *no frontmatter parser today* — it must gain `_lib.frontmatter` wiring to read `class:` **before** any class-routed decomposition runs, else it silently defaults to `protocol-canonical` and mis-routes every process residue into substrate legs. Then: stamp `class:` onto each gap-item, route per-class via the manifest, and **re-run `gate_signals` fail-loud** (don't trust the stamped field). `gate_signals`: `vocab-vs-target-mismatch` (D# set but all targets under `.claude/`/`PLACEMENT.md`) · `forced-a2o-leg` (process chunk → `a2o/features/<pillar>/` — but **NOT** `a2o/scripts|steps|src/framework/`, which is process-owned harness) · `forced-architecture-seed` (process chunk → `architecture/` as non-cite — **scoped off** when the spec's *deliverable* is itself an architecture rewrite, e.g. memory-ceremony/historian) · `archetype-scheduled-a-sprint` (**soft advisory only** — the corpus shows archetypes legitimately spawn primitive-realizing plans).

## 6. Bootstrap (trusted-issuer; adversary-verified sound)

This spec is itself `class: process-meta` (it dogfoods a *concept*, not an executable primitive — so by its own discriminator it is not method-bridge; that honesty surfaces §7's open question). The chicken-egg resolves via the **trusted-issuer pattern** (`project_reach_earned_genesis_seeder_grades_homework`): the first classification is **hand-issued** by the operator/architect at authoring time; the installed gate grades every classification after. It does not need the gate to have run on it. **Strict sequence (order is load-bearing — the readers must exist before the manifest is relied on):**

1. Hand-author `.claude/subject-routing.yaml` (the repo-root PARENT constitution; tracked).
2. Land the `decompose.py` frontmatter-parser + cascade-resolver + `gate_signals` patch (the BACK reader).
3. Land `brainstorm.md` Step 1c.0 + Step-4 class-conditional frontmatter (the FRONT reader). (No `memory-kit/CLAUDE.md` amendment — the parent lives at `.claude/` root, so the "one hand-edited file" rule there is untouched.)
4. Hand-stamp `class:` on this spec + the four relocations (below).
5. **Only then** decompose.

**Mandatory CURATE step (lossless §4):** this spec's own bootstrap writes one `history/` lesson — *"The D4 name-collision: why four process specs mis-filed as substrate"* — distilling the relocation forensics (the 0-product-code-refs proof per spec, why the D4 seed name was magnetic), cited by `gate_signals.vocab-vs-target-mismatch`. The bodies retire to git only after that lesson exists.

**The four relocations are SEMANTIC, not physical** (all four stay in `superpowers/specs/`; only frontmatter changes — no file moves, so no cites break regardless of slug-resolution status):

| spec | from | to |
|---|---|---|
| `scope-tree-reconciler-design` | `domain: D4` + product-seed `informed-by:` | drop `domain:`; `class: process-meta`; `process_subdomain: doc-lifecycle`; `informed-by:→derived_from:` (dogfood breadcrumb) |
| `semantic-computable-links-design` | `domain: D4` + product-seed `informed-by:` | drop `domain:`; `class: process-meta`; `process_subdomain: memory`; `informed-by:→derived_from:` |
| `spec-plan-compaction-loop-design` | `canonical_seed:` = product seed | `class: process-meta`; `canonical_seed:` → `PLACEMENT.md` (its true tooling-lifecycle parent); keep `derived_from:` breadcrumb |
| `unified-memory-loop-design` | product seed as canonical parent | `class: process-meta`; re-anchor parent to the `.claude/` cadence docs it orchestrates; demote seed to `derived_from:` |

## 7. Resolved taxonomy decisions (+ remaining opens)

**RESOLVED:**

- **`method-bridge` collapses.** It has no unique home (routes as process-meta), and even this defining spec is plain process-meta by the strict rule. It becomes a **`derived_from: <product-seed>` breadcrumb** on a process-meta spec, watched by the `vocab-vs-target-mismatch` gate signal — *not* a class. (§2, §3.)
- **`application-archetype` → a `status: vision` mode of protocol-canonical**, not a class. It shares protocol-canonical's home; the corpus falsifies "never schedules code"; so its next-state is *stay-as-reference* or *graduate-to-impl*, restoring the every-doc-has-a-next-state invariant. The hard `archetype-scheduled-a-sprint` gate signal is dropped (a normal transition, not an over-commit). (§3.)
- **`class: provisional` adopted** as a real state, reconciled at the BACK-fire from the actual residue-landing site (fail-loud via `gate_signals` if a stamped class contradicts the residue) — mirroring `PLACEMENT.md`'s "scope is reconciled, not assigned." So `domain:`/`informed-by:` are **conditional on a resolved class**, never REQUIRED at the front; a spike isn't blocked by a `domain:` it cannot yet answer. (§2, §5.)

**Net taxonomy: two classes** (`protocol-canonical`, `process-meta`) + `provisional` + the `status: vision` and `derived_from:` modifiers + the orthogonal `artifact_kind` axis.

**REMAINING OPEN (defer to implementation):**

- **Dual-deliverable** (historian, memory-ceremony land in *both* `architecture/` and `.claude/`): primary+secondary `class:`, or a `dual` flag? The `forced-architecture-seed` scope-off (§5) already kills the false-positive; only the residue-routing for genuine dual-landing still needs a rule. *Lean:* a `secondary_class:` field, resolved at BACK-fire like `provisional`.
- **Existing-corpus back-fill:** lazy (back-fill `class:` as each doc is next touched) vs one bulk sweep. *Lean:* **lazy** — a bulk sweep risks the "classify a dumping ground" anti-pattern; the budget under-count self-corrects as docs are touched, and the four known mis-placements (§6) are back-filled by hand in the bootstrap.

## 8. Decomposition (gap-items)

- [ ] `.claude/subject-routing.yaml` PARENT constitution at repo root — `class → {write_location, decomposition_flow, discard_rule}` + `artifact_kind` axis + `gate_signals`; tracked, hand-tuned; the repo-scope class registry (§5.0/§5.1).
- [ ] `.claude/scripts/_lib/subject_routing.py` — shared cascade-resolver + class-reconciler (both gates import it; `_lib` ≥2-caller discipline): walk up the tree merging routing parents (root + sub-tree `.claude/subject-routing.yaml` / CLAUDE.md blocks; union classes, nearest-wins home-remaps), reconcile `provisional`, run `gate_signals`. The one-repo→mono-repo→submodule scaling mechanism, optional by construction (§5.0).
- [ ] `decompose.py` — import `_lib.subject_routing` + `_lib.frontmatter` to read the cascaded `class:`, stamp it on gap-items, route per-class, fail-loud `gate_signals` — the hard prerequisite (§5).
- [ ] `brainstorm.md` Step 1c.0 CLASSIFY-SUBJECT gate (before MAP-PATH; calls `_lib.subject_routing` for the cascaded class menu) + Step-4 class-conditional frontmatter (`class:` always; `domain:`/`informed-by:` only on a resolved substrate class) (§5).
- [ ] `MAP.md` Axis 0 (subject class) above D#+pillar; `map_has_meta_axis`→true; extend the Q1 citation to three axes (§5, §1).
- [ ] `PLACEMENT.md` subject-class-axis section pointing at `.claude/subject-routing.yaml` (mirror the `cluster-state.yaml` env-axis pointer); resolve the §12 dev-doc-home open issue as "the gospel-diff target, not a new dir" (§5, §1).
- [ ] `compaction-loop §5.2b` — per-class fate routing (the §5.2 table becomes the protocol-canonical branch); the unifying keep/discard-by-reasoning-value principle (§3, §4).
- [ ] process-meta `history/` leg LIVE + tool-design-rationale residue row + stub-then-grade default (§3, §4).
- [ ] `artifact_kind` axis rows for run-output / brainstorm / kickoff / handoff with retention rules (comet/git, never a substrate seed) (§2).
- [ ] `.claude/subject-routing.yaml` tracked-not-gitignored at repo root; no `memory-kit/` amendment needed (root placement preserves its "one hand-edited file" rule — the adversary's bootstrap collision dissolves) (§5.0/§6).
- [ ] The four SEMANTIC relocations + the mandatory `history/` CURATE lesson "The D4 name-collision" (§6).
- [ ] Dual-target process-residue leg on protocol-canonical (CI/build → `orchestrator/README.md`) + dual-deliverable primary/secondary class (§3, §7).
