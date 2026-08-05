---
title: "CI Change-Detection Convergence + `.ci-ignore`→`.epr-meta` Fold (P6)"
id: ci-detection-convergence-epr-meta-fold
status: Draft
class: process-meta
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
created: 2026-06-25
maintainers: Matthew Dowell + Opus 4.8
refines:
  - genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md
cites:
  - doc-lifecycle-as-epr-development-substrate | Doc-Lifecycle as EPR (framing spec, §12 P6) | path: genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md
  - epr-meta-compose-gate | The .epr-meta compose-gate (P1) | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
---

# CI Change-Detection Convergence + `.ci-ignore`→`.epr-meta` Fold (P6)

> **Scope.** This is **P6** of the framing spec
> (`2026-06-25-doc-lifecycle-as-epr-development-substrate-design`). The chosen cut is **"sweep + fold"**:
> (A) finish the never-completed convergence on the graph-walker by sweeping its leftovers, and (B) fold
> the `.ci-ignore` concern into the `.epr-meta` cascade as a generated projection. The **deeper pre-push
> convergence** (giving the grep-only gates manifest homes, then slimming the grep fallback) is specified
> as the explicit follow-on **P6.2 (§7)** — sequenced, not built here, because it has a real precondition
> and a fail-open risk.

## 1. The reframe: the graph-walker is already authoritative — it was never swept

The premise that we should "use the graph-walker" is **already true in the runtime**; what's missing is
the cleanup after the migration. Verified facts (file:line):

- **Auto-mode CI dispatch is graph-walker-only.** `build-graph.groovy:walkBuildGraph` (entry :670)
  composes the manifest graph, detects staleness by `inputs.sources` glob match (`checkSourceChanges`
  :216) + `inputs.buildProcess` function-hash (`checkBuildProcessChanges` :268), and emits
  `pipelineSteps` — the **sole** authority for auto dispatch (`applyBuildGraphRouting`, Jenkinsfile :861).
  The legacy `analyzeChangeset` already returns `pipelines: []` for *selection* (:788); its sibling
  `analysis` map is display-only (:765-767).
- **The cross-project cascade is the graph-walker's, and it's live.** Step-level `depends` edges +
  `propagateStaleness` (build-graph.groovy:341) — cross-pipeline-qualified at compose time (:91) — make
  a change to `elohim-sophia:build-sophia-umd` propagate into `elohim:build-angular`
  (`app/elohim-app/build-manifest.json:31`). This is "cascades our builds."
- **The local detector is the graph-walker too.** `.husky/pre-push:252` pipes the changeset through
  `graph-walker.mjs --shell-lines` (`walkGraph`, graph-walker.mjs:65), mapping stale steps to
  `gate.projects` — authoritative whenever node is present and a manifest matches (`USE_MANIFEST=true`).
- **`.ci-ignore` is a *local/pre-push* optimization — CI never honors it.** The Groovy
  `loadCiIgnore`/`matchesCiIgnore` (Jenkinsfile 266/295) are **dead** (grep-proven: definitions only, no
  call sites); `build-graph.groovy` references ci-ignore nowhere. Server dispatch relies purely on
  manifest source-globs (a path matching no glob triggers nothing). So the `.ci-ignore` header's claim
  that the Jenkinsfile consumes it — "drift caught by tests" — is **false on both clauses**.

The migration happened; the corpses remain. P6(A) buries them; P6(B) does the fold on the clean surface.

## 2. Two halves of one concern

| Half | What | Risk |
|---|---|---|
| **A — Sweep** | retire dead change-detection code, fix two *functional* bugs, correct one inverted label, fix stale references | low (mostly removal + 2 small fixes) |
| **B — Fold** | a root `ci-trigger:`-only `.epr-meta` → a projector codegens the flat `.ci-ignore` (byte-identical) behind a freshness gate | moderate, reversible, fail-open-guarded |

The order is **A then B**: the sweep reclaims CPS headroom and removes phantom references that B would
otherwise copy into its own comments.

## 3. Half A — the sweep (exact dispositions)

### 3.1 Genuine dead code → RETIRE (grep-proven, zero behavior change)
- `Jenkinsfile:266` `loadCiIgnore()` + `Jenkinsfile:295` `matchesCiIgnore()` — the dead Groovy `.ci-ignore`
  port. No call sites. *(CPS note: these are top-level helper methods, so removal is byte-negative but does
  NOT shrink the constrained `Determine Build Plan` dispatch method — it's still net-positive hygiene.)*
- `Jenkinsfile:471` `propagateDependencies()` — the orphaned pipeline-level cascade, superseded by the
  live step-level `propagateStaleness`. No call sites.
- `pipeline-registry.mjs` exports `pipelinesThatTriggerGenesis`, `pipelineDependencyMap`,
  `dispatchablePipelines` — no in-repo callers (the Jenkinsfile rebuilds its registry inline). *Plan must
  re-confirm no external/Jenkins consumer before deleting; if uncertain, leave with a deprecation note.*

### 3.2 Two FUNCTIONAL bugs (not comment hygiene) → FIX
Both stem from the deleted `orchestrator-strategy.mjs` (the prior system-of-record, now absent):
- **Unreachable freshness trigger** — `.husky/pre-push:264` keys the `pipeline-list-fresh` gate on a diff
  to `genesis/orchestrator/orchestrator-strategy.mjs`, a file that no longer exists → the trigger can
  **never fire**. `pipeline-list.json` is now generated from `build-manifest.json` via
  `pipeline-registry.mjs` → `generate-pipeline-list.mjs`. **Fix:** re-key the trigger to changes in
  `**/build-manifest.json` / `genesis/orchestrator/pipeline-registry.mjs` (or drop the arm — the gate body
  at :640-650 is otherwise covered by `orchestrator-integration.test.mjs:126-143`).
- **Broken fallback gate** — `.husky/pre-push:758` runs `node --test orchestrator-strategy.test.mjs …`
  against a **missing** test file → errors when reached (mitigated only because the preferred `just gate`
  path doesn't hit it). **Fix:** point at surviving tests:
  `graph-walker.test.mjs orchestrator-integration.test.mjs jenkinsfile-cps-scope.test.mjs`.

### 3.3 Inverted label → FIX (keep the function)
- `Jenkinsfile:326` tags `analyzeChangeset` `// DEPRECATED: advisory only, will be removed`. It is
  **live and load-bearing** — the sole source of the changeset feeding `walkBuildGraph` (`:746` →
  `applyBuildGraphRouting :1620` → `runBuildGraph :1270` → `walkBuildGraph :1272`). Only the sibling
  `analysis` *map* is advisory. **Fix:** remove the mislabel; the function stays.

### 3.4 Stale references → FIX (comment/doc hygiene)
Dangling references to the deleted `orchestrator-strategy.mjs` (+ its gone `orchestrator-strategy.test.mjs`):
`ci-ignore.mjs:6,12` (also names a nonexistent Groovy `isCiIgnored` and a gone parity test);
`Jenkinsfile:254,293,703`; `justfile:31,42`; `count-pipeline-failures.sh:18`; `pipeline-trajectory.mjs:17`;
`README.md:112-128` (heavily stale — describes the deleted strategy module as live). And correct the
`.ci-ignore` header (§3.5). *Keep* the historically-correct "replaces/extracted-from/survived-deletion"
notes in `pipeline-registry.mjs`, `commit-tag-parser.mjs`, `orchestrator-integration.test.mjs`.

### 3.5 The `.ci-ignore` header → restate honestly
Rewrite the "Consumed by" block to the truth: consumed by `ci-ignore.mjs` (CLI), the `graph-walker.mjs`
CLI boundary (`filterChanged`, graph-walker.mjs:158 — CLI only, **not** the exported `walkGraph`), and
`.husky/pre-push`; **CI does not apply it** (it's a local/pre-push optimization). Drop the false
"Jenkinsfile mirrors + tested for drift" claim.

## 4. Half B — the fold (`.ci-ignore` as a projection of the `.epr-meta` cascade)

### 4.1 The signal — a top-level `ci-trigger:` leg, orthogonal to the enforcement classes
`ci-trigger:` is a **build-time** signal, *not* an author-time rule. It is **not** a member of the
`deny`/`ask`/`inject`/`measure`/`dispatch` class ladder, and it must **not** live in the `rules:` array
(the engine would flag an unknown rule key). It is a **top-level key** (sibling to `purpose:`/`root:`),
part of the app-manifest leg. The existing Python resolver is correctly blind to it: `merge_rules`
(epr_meta.py:98) merges only `rules`/`validators`, and `validate_meta` only inspects keys *inside* rules
— so a top-level `ci-trigger:` is invisible to the PreToolUse dev-gate and read only by the new projector.

**Inline-now shape** (the decided sequencing): the `ci-trigger:` declaration carries the ignore patterns
directly. It is shaped to *optionally* carry an `epr:` reference later (the content-addressed `ci-ignore`
EPR, which needs P1b) — a forward hook that changes no consumer when it lands. Indicative:

```yaml
ci-trigger:
  ignore:                       # "changes matching these never trigger source pipelines"
    - .claude/                  # subtree-prefix
    - .github/
    - .husky/
    - CLAUDE.md                 # basename-anywhere
    - AGENTS.md
    - GEMINI.md
    - .no-claude.md
    - genesis/orchestrator/Jenkinsfile        # exact-path (orchestrator self-host)
    - genesis/orchestrator/build-graph.groovy
  # defines: epr:ci-ignore@1   # FORWARD HOOK (P1b): point at the content-addressed ignore EPR
```

### 4.2 Where each pattern lives (verified directionality)
- **Subtree-prefixes** (`.claude/`·`.github/`·`.husky/`) *can* decentralize into those dirs' own
  `.epr-meta` — but inline-now keeps them in the root for the first cut; per-subtree relocation is a later
  refinement (and requires planting `.epr-meta` in currently-ungoverned dirs).
- **basename-anywhere** (`CLAUDE.md` et al.) and **repo-root exact-paths cannot
  decentralize** — no anchor directory — so they live in the **root `.epr-meta`** permanently.
- **Directory exact-paths** (orchestrator `Jenkinsfile`/`build-graph.groovy`) are *file-scoped* ignores in
  the root (or, later, `genesis/orchestrator/.epr-meta`) — the directory has real source, so it is **not**
  a subtree ignore.

### 4.3 The repo-root constitutional `.epr-meta` (the prerequisite)
There is **no repo-root `root: true` `.epr-meta` today** (the only `root: true` is at
`genesis/docs/superpowers/`). The fold needs one — to host the cross-cutting ignores and to terminate the
cascade. Per the decided scope it is **`ci-trigger:`-only**: it carries `root: true` + `ci-trigger:` and
**no author-time `rules:`**, so the resolver merges no rules → every repo-wide Write/Edit still resolves to
a silent allow → **authoring behavior is unchanged repo-wide**. It also retires the "no `root: true`
constitutional base" advisory the resolver emits for governed subtrees.

### 4.4 The projector + the single-source-of-truth contract
A small projector (Python alongside `epr_meta.py`, or a tiny `projector.mjs`) walks the cascade
(reusing `collect_cascade`; it must add its **own** nearest-wins merge for `ci-trigger`, since
`merge_rules` discards top-level keys) and emits the flat `.ci-ignore` with a `# GENERATED — DO NOT EDIT`
header. Contract:
- **Byte-identical bring-up.** The generator's first output must be **sha256-identical** to the current
  `.ci-ignore` before the source of truth flips (the same discipline as the ts-rs byte-identical rule).
- **Freshness gate.** A `--verify` mode + `git diff --quiet -- .ci-ignore`, wired into the pre-push gate,
  triggered on any `.epr-meta` **or** `.ci-ignore` change — mirroring `schema:codegen:ts --verify` and the
  `pipeline-list-fresh` pattern. This is what makes "source of truth shifted root-file→cascade" real
  rather than two files that drift.
- **`.ci-ignore` stays a committed artifact.** It is NOT replaced by feeding `ci-trigger` to the
  graph-walker directly, for two hard reasons: (1) `ci-ignore.mjs` computes `CI_IGNORE_PATTERNS` at
  import time via `readFileSync(.ci-ignore)` with **no fallback** (line 68), and `graph-walker.mjs` imports
  it — a missing file crashes both; (2) the Groovy side can read a flat file but cannot import `.mjs` or
  parse YAML. The flat file is the language-agnostic interchange format.
- **Zero new Jenkinsfile CPS bytecode** — all new logic is Python/JS outside the Jenkinsfile.

### 4.5 What `.ci-ignore` must still do after the fold (it is not redundant)
The opt-in glob model makes "ignore" the default, but three jobs are structurally not subsumable and must
survive in the generated `.ci-ignore`:
1. **Subtractive overrides** of broad manifest globs (orchestrator self-host; docs/storybook globs sweeping
   a co-located `CLAUDE.md`) — globs are additive and cannot say "match subtree EXCEPT these".
2. **The pre-push "all-ignored → exit 0" fast-path** (pre-push:182-185) — it short-circuits *before* the
   humans/presences/devices schema-validation greps, which an empty `gate.projects` does not.
3. **basename-anywhere** semantics — manifests cannot express `!**/CLAUDE.md`.

## 5. Invariants (every task upholds these)
- **Fail-open.** A missing/malformed `.epr-meta`, a missing/stale `.ci-ignore`, or absent node must let the
  push/build proceed — never block. (Matches the resolver's `{}`-on-failure and pre-push's node guard.)
- **Fail toward over-triggering.** Any ambiguity resolves to running *more* gates / triggering *more*
  pipelines, never fewer. A projector bug that drops a pattern over-triggers (noisy, safe); one that *adds*
  a pattern could skip a real build (dangerous) — the byte-identical check guards that direction.
- **Zero Jenkinsfile CPS growth.** No new logic in the CPS-bound Jenkinsfile; prefer *removing* Groovy.
- **Byte-identical bring-up** of the generated `.ci-ignore` before the source-of-truth flip.

## 6. Acceptance criteria (the DoD this spec must earn)
1. Dead code retired (`loadCiIgnore`, `matchesCiIgnore`, `propagateDependencies`, the three uncalled
   `pipeline-registry.mjs` exports) with grep showing no remaining references.
2. The two functional bugs fixed (re-keyed `pipeline-list-fresh` trigger; repointed pre-push test target),
   each demonstrated to now fire / pass.
3. The `analyzeChangeset` `DEPRECATED` mislabel removed; function unchanged.
4. Stale `orchestrator-strategy.mjs` references corrected across the named files; `.ci-ignore` header
   restated honestly; `README.md` change-detection section made current.
5. A repo-root `root: true` `ci-trigger:`-only `.epr-meta` exists; authoring behavior repo-wide is
   provably unchanged (a frontmatter-less write outside governed trees still silently allows).
6. The projector emits a `.ci-ignore` **sha256-identical** to the pre-fold file; `.ci-ignore` carries the
   GENERATED header; the `--verify` freshness gate is wired into pre-push and fails on a hand-edit.
7. All existing change-detection tests pass (`graph-walker.test.mjs`, `orchestrator-integration.test.mjs`,
   `jenkinsfile-cps-scope.test.mjs`); `jenkinsfile-cps-scope.test.mjs` confirms no CPS regression.

## 7. Explicitly deferred — P6.2 (the deeper pre-push convergence) + notes
Not built here; specified so it isn't lost:
- **The 8 grep-only gates have no manifest home** — `epr-storage`, `reach-drift`, `rakia-codegen`,
  `rakia-validate`, `cargo-coverage`, `elements-codegen`, `domain-types`, `sophia` are emitted **only** by
  the pre-push grep fallback, which runs only in degraded/no-node mode → **they already do not run locally
  on the normal path.** P6.2 must give each a `build-manifest.json` `gate.projects` home (source globs +
  matching `run_gate` case) and prove PROJECTS parity against a historical-diff corpus **before** the grep
  fallback is touched. *(This is a pre-existing latent gap, surfaced here.)*
- **Converge pre-push onto the graph-walker as sole *positive* detector**, slimming the grep fallback to a
  **minimal no-node safety net** — never deleting it (fail-open). Gated behind the gate-parity work above.
- **A JS↔Groovy parity test** for the graph-walker twins (the old `orchestrator-strategy.test.mjs` parity
  check was deleted). The twins have three real divergences (JS omits the `depends` cascade; JS does
  membership-only `buildProcess`; different output namespace) — a parity harness closes the recurring
  twin-drift class. Optional hardening.
- **Per-subtree `ci-trigger:` decentralization** (move `.claude/`·`.github/`·`.husky/` ignores into those
  dirs' own `.epr-meta`) — a refinement once those dirs are governed.
- **The content-addressed `ci-ignore` EPR** (`ci-trigger.defines: epr:…`) — needs P1b (the projector that
  mints CIDs from source). The inline `ci-trigger:` is the forward-compatible hook.

## 8. Risks / what the plan must verify against code
- Re-confirm the three `pipeline-registry.mjs` exports have **no** consumer (incl. any Jenkins-side
  `node` invocation) before deleting.
- Confirm the `pipeline-list-fresh` re-key target is the true generation input (`build-manifest.json` →
  `pipeline-registry.mjs` → `generate-pipeline-list.mjs`), and that the gate body still validates correctly.
- Verify the projector's `ci-trigger` merge matches the three `.ci-ignore` pattern kinds
  (`prefix`/`exact`/`basename`) exactly, and that whitespace/order produce a byte-identical file.
- Confirm the freshness-gate trigger covers both `.epr-meta` and `.ci-ignore` edits and fails open if node
  is absent.
- Confirm no edit inflates the Jenkinsfile's constrained CPS method (run `jenkinsfile-cps-scope.test.mjs`).

## Related
- Framing spec §12 P6: `genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md`
- `.epr-meta` compose-gate (P1): `genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md`
- `.epr-meta` authoring skill: `.claude/skills/elohim-epr-metafile/SKILL.md`
- Change-detection: `genesis/orchestrator/{graph-walker.mjs,build-graph.groovy,ci-ignore.mjs,pipeline-registry.mjs}` · `.husky/pre-push`
