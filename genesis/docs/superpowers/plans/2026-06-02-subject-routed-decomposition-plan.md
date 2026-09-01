---
title: "Subject-Routed Decomposition — Bootstrap Implementation Plan"
id: subject-routed-decomposition-plan
status: Draft
created: 2026-06-02
class: process-meta
process_subdomain: doc-lifecycle
sprint: bootstrap
cites:
  - "subject-routed-decomposition-design | the spec this plan implements — its load-bearing phase order builds the readers before the manifest | sha256:0d910143a8498b64 | path: genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md"
  - .claude/scripts/_lib/frontmatter.py
  - .claude/scripts/memory-kit/decompose.py
  - .claude/commands/brainstorm.md
  - "placement | the contract this plan proposes the subject-class axis section for | sha256:f84d7cb16bea9379 | status: stale — target content moved on; re-verify | path: genesis/docs/PLACEMENT.md"
  - "map | the product-domain lattice this plan adds Axis 0 subject class above | sha256:4d707bfda967a21a | path: genesis/docs/content/elohim-protocol/architecture/MAP.md"
derived_from:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md  # dogfood breadcrumb (vocabulary only)
requires_env: []   # pure dev-tooling (python/yaml/markdown) — testable on household-nodes, no shem/harbor/alpha
---

# Subject-Routed Decomposition — Bootstrap Implementation Plan

Implements `2026-06-02-subject-routed-decomposition-design.md`. **The order is load-bearing** (spec §6): the *readers* (Phases 2–4) must exist before the manifest is relied on, and the relocations (Phase 6) must run only after the BACK reader can route them. Verification is process-meta-shaped: **"verified" = the test passes / the script exits 0 / the gate fires** — there is no pillar a2o scenario or CI-green for this work. Each phase names its files, its gap-item, and a concrete check.

This plan is itself `class: process-meta` — the dogfood: the plan that builds the class gate is classed by the taxonomy the gate will enforce, with no `domain: D#`.

## Phase 1 — The parent constitution

- [ ] Hand-author **`.claude/subject-routing.yaml`** (tracked, NOT gitignored — confirm it joins `context-coverage.yaml` on the tracked side; it lives at `.claude/` root, so the `memory-kit/` "one hand-edited file" rule is untouched). Encode: `version`, `default_class: protocol-canonical`, the `discriminator` (classify by deliverable-TARGET, never vocabulary), and the two classes + `provisional`:
  - `protocol-canonical` → `{has_domain: true, write_location, decomposition_flow{canonical, verified(a2o+code), history, story, research, backlog, body}, status_modes:[vision]}`
  - `process-meta` → `{has_domain: false, write_location, gospel_homes{root, memory-tools, memory-data, memory-prims, placement, doc-state, ci, a2o, timeline}, tool_homes{skill, agent, command, hook, workflow, script, schema, prompt}, decomposition_flow{gospel_diff, tool, history(LIVE/judgment), reports(comet), backlog, body}, NULL_legs:[canonical, a2o]}`
  - `provisional` → reconciled at back-fire
  - modifiers: `status: vision` (archetype mode of protocol-canonical), `derived_from:` (dogfood breadcrumb; replaces method-bridge)
  - `artifact_kind` axis: `spec|plan|brainstorm|kickoff|handoff|run-output` with retention (run-output → comet/git, never a substrate seed)
  - `gate_signals`: `vocab-vs-target-mismatch`, `forced-a2o-leg` (only on `a2o/features/<pillar>/`, NOT `a2o/scripts|steps|src/framework/`), `forced-architecture-seed` (scoped off when the deliverable IS an architecture rewrite), and the **dropped** `archetype-scheduled-a-sprint` (advisory, not hard) — covers spec gap-items §8.1, §8.7 (artifact_kind), §8.9 (tracked).
- [ ] **Check:** `python3 -c "import yaml; yaml.safe_load(open('.claude/subject-routing.yaml'))"` parses; `git check-ignore .claude/subject-routing.yaml` returns nothing (tracked).

## Phase 2 — The shared resolver (`_lib`, ≥2 callers)

- [ ] Write **`.claude/scripts/_lib/subject_routing.py`** (pure-stdlib; imports `_lib.frontmatter`). Functions: `load_routing(start_path)` (walk UP from `start_path` merging every `.claude/subject-routing.yaml` + CLAUDE.md routing-block on the path — union classes, nearest-wins home-remaps; the mono-repo cascade); `classify(frontmatter, targets)` (apply the deliverable-TARGET discriminator → class | `provisional`); `reconcile(provisional, residue_targets)` (BACK-fire resolution); `gate_check(frontmatter, targets) -> [Signal]` (the fail-loud detectors) — covers spec §8.2.
- [ ] Write **`.claude/scripts/_lib/__tests__/subject_routing.test.py`** (or alongside the existing `_lib` test convention): assert a known process-meta spec (only `.claude/` targets) → `process-meta`; a product spec (`app/`+`architecture/`) → `protocol-canonical`; a `domain: D4` spec whose targets are all `.claude/` → `gate_check` fires `vocab-vs-target-mismatch`; a spec touching only `a2o/scripts/` does NOT fire `forced-a2o-leg`; the cascade merges a sub-tree routing block over root.
- [ ] **Check:** the test file runs green (`python3 .claude/scripts/_lib/__tests__/subject_routing.test.py` exits 0).

## Phase 3 — The BACK reader (HARD PREREQUISITE)

- [ ] Patch **`.claude/scripts/memory-kit/decompose.py`** to `import _lib.subject_routing` + `_lib.frontmatter`: parse the source spec's frontmatter, resolve its cascaded `class:`, **stamp `it['class']` onto every gap-item record**, and run `gate_check` fail-loud (a `domain:` + all-`.claude/` targets aborts with the mis-class message rather than silently defaulting). Default to `protocol-canonical` ONLY when frontmatter has no `class:` AND no mismatch signal — covers spec §8.3.
- [ ] **Check:** re-`decompose.py` this plan's own spec → every gap-item JSON record now carries `"class": "process-meta"`; decompose a product spec (e.g. an `app/` spec) → `"class": "protocol-canonical"`; decompose a deliberately-mis-tagged fixture (`domain: D4` + `.claude/` targets) → fail-loud, non-zero exit.

## Phase 4 — The FRONT reader

- [ ] Edit **`.claude/commands/brainstorm.md`**: insert **Step 1c.0 — CLASSIFY-SUBJECT** *before* the existing (1) MAP-PATH (call `_lib.subject_routing` for the cascaded class menu; answer "whose experience / where does it land"; branch — substrate classes → proceed into MAP-PATH; process classes → skip the D#+pillar lookup, name the process home + `process_subdomain`). Make **Step 4** frontmatter class-conditional: `class:` always; `domain:`+`informed-by:<architecture seed>` only on a resolved substrate class; `process_subdomain:`+`informed-by:<process gospel>` for process; `derived_from:<product seed>` as the dogfood breadcrumb; `class: provisional` permitted for spikes — covers spec §8.4.
- [ ] **Check:** dry-run the gate logic on two known topics (one product, one process) — confirm the process topic classifies `process-meta` and the Step-4 block omits `domain:`. (Prose/skill edit — review + a manual classification dry-run is the test.)

## Phase 5 — The prose surfaces (the class axis becomes authoritative)

- [ ] **`MAP.md` §1** — add **Axis 0 · Subject class** above the D#+pillar axes (the two classes + the rule that D#/pillar apply only to the substrate branch; process work routes via `subject-routing.yaml`); flip `map_has_meta_axis`→true; extend the Q1-canonical-organization citation from two axes to three — covers spec §8.4 (MAP).
- [ ] **`PLACEMENT.md`** — add the subject-class-axis section pointing at `.claude/subject-routing.yaml` (mirror the `cluster-state.yaml` env-axis pointer); resolve the §12 dev-doc-home open issue as "the gospel-diff target, not a new dir" — covers spec §8.5.
- [ ] **`compaction-loop §5.2b`** — add the per-class fate-routing section (the existing §5.2 table becomes the `protocol-canonical` branch; `process-meta` gets `architecture`/`a2o` NULL + gospel-diff/tool/history-LIVE legs); state the unifying keep/discard-by-reasoning-value principle + the process-meta history-leg-LIVE + stub-then-grade default + tool-design-rationale residue — covers spec §8.6, §8.8.
- [ ] **Check:** each surface references `.claude/subject-routing.yaml`; `grep -l subject-routing.yaml` finds MAP.md + PLACEMENT.md; the compaction-loop §5.2b table has a process-meta column with NULL architecture/a2o legs.

## Phase 6 — Relocations + the CURATE lesson (only after Phases 2–4 exist)

- [ ] Hand-stamp `class:` + retag the **four SEMANTIC relocations** (frontmatter only — no file moves; all stay in `superpowers/specs/`): `scope-tree-reconciler-design` (`class: process-meta`, `process_subdomain: doc-lifecycle`, drop `domain: D4`, `informed-by:→derived_from:`); `semantic-computable-links-design` (`process-meta`/`memory`); `spec-plan-compaction-loop-design` (`process-meta`/`doc-lifecycle`, `canonical_seed:` → `PLACEMENT.md`); `unified-memory-loop-design` (`process-meta`/`memory`, re-anchor parent to `.claude/` cadence docs) — covers spec §8.10.
- [ ] Write the mandatory CURATE lesson **`genesis/docs/content/elohim-protocol/history/2026-06-02-d4-name-collision.md`** (`type: history-gotcha`): distill the relocation forensics (the 0-product-code-refs proof per spec; why the D4 seed name was magnetic; "classify by deliverable-TARGET, not vocabulary"); cite it from `gate_signals.vocab-vs-target-mismatch` — covers spec §8.10 (CURATE) + §4 (lossless: keep the forensic before the body retires).
- [ ] **Check:** re-`decompose.py` each relocated spec → gap-items now carry `class: process-meta`; `memory-coherence-audit` / `spec-coherence-index` no longer flag them as D4-product; the history lesson exists and is bidirectionally linked.

## Phase 7 — Close the loop (end-to-end verification)

- [ ] Run a fresh `/brainstorm` on a NEW process topic end-to-end → it classifies `process-meta`, skips MAP-PATH, lands `class:` frontmatter; `decompose.py` stamps the gap-items `process-meta`; the gate_signals stay silent (correct classification). Then run it on a product topic → `protocol-canonical`, MAP-PATH runs, substrate legs live.
- [ ] **Check (loop closed):** the FRONT classification and the BACK stamp agree for both a product and a process topic, with no `gate_signals` false-fire — the single `class:` field flows front→back, making the decompose multi-flow instead of single-flow.

## Sequencing & rollback

- **Strict order:** 1 → 2 → 3 → 4 → 5 → 6 → 7. Phase 6 (relocations) MUST follow Phase 3 (the BACK reader), or the relocated specs decompose under the un-patched router and mis-route to substrate legs.
- **Each phase is independently revertable** (a new file or a localized patch); nothing here touches product code, CI, or the cluster, so blast radius is the dev-tooling surface only.
- **Lazy back-fill** (spec §7): the existing ~119-doc corpus is *not* swept; `class:` back-fills as each doc is next touched. Only the four known mis-placements get hand-stamped (Phase 6).
