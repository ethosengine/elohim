---
name: memory-kit
description: "Librarian-solo hygiene-sweep toolkit and cadence: deterministic hygiene/audit tools (cleanup, path-update, dedupe, memory-review, claude-md/agent/skill audits, memory-coherence) plus a stasis/coverage tier (placement-audit ledger, substrate-currency, story-coverage, decompose, retention, scope reconcilers) and an edit-time layer of PreToolUse/PostToolUse hooks keeping new agentic-agent text born-governed at emission (prevention), complementing the after-the-fact sweep (remediation): byte-budget enforcement, archive-ratio tracking, dead-citation hygiene, in-flight memory-to-code coherence. Read-only by default; mutations are operator-gated. Sibling to /memory-ceremony (the four-lens substrate-currency rewrite this kit never attempts) and to the memory-stasis-loop workflow. Read `placement-audit.py --ledger` first so the sweep drains a measured budget. Use for byte-budget enforcement, an audit-numbers pass, or chaining hygiene tools ad-hoc — not for the rewrite ceremony or the unified memory+dev stasis loop."
metadata:
  runtime: antigravity
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/memory-kit.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/memory-kit"
---

# Memory Kit — Hygiene-Sweep Cadence (Librarian-Solo)

Project memory lives at **`/projects/elohim/.claude/memory/`** — git-tracked, team-shareable, recoverable from a fresh clone. The harness-conventional path at `/projects/.claude-config/projects/-projects-elohim/memory/` is a symlink to that primary, so native auto-memory writes land in the repo rather than in a per-machine store. This kit tends the `MEMORY.md` index and its typed topic files, fixes drift, surfaces archive candidates, and audits the always-loaded surfaces. It does not replace native auto-memory — native memory stays Claude-self-managed and LIGHT (non-obvious cross-session facts only); durable architecture, vision, and wisdom *graduate* to the managed artifacts this kit tends rather than piling up in native memory (the managed-surface edit discipline, `.claude/memory/feedback_managed_surface_edit_discipline.md`).

## Pollution regulation for agentic-developer context

The kit's governance has a left-shifted half. The *sweep* (this kit's tools) is **remediation** — it cleans drift after the fact. The edit-time *hooks* are **prevention** — they keep new text born-governed at the point of emission, so the emitting directory carries the cost of its own output instead of every downstream reader paying the noise. Two ends of one pipe: the gate narrows what the sweep must clean.

The edit-time layer (the *why* — full inventory in **The Hooks** below):

- **`.epr-meta` compose-gates** — cascading per-directory manifests; the `epr-meta-resolver` PreToolUse hook gates new docs against the directory's declared contract. The *canon* step of `flag → agent → canon → stasis`, co-located and executable: an agent that hits a drift **authors the rule that flags it for the next agent**, right where it bit. Author with the **`elohim-epr-metafile`** skill — whose whole subject is choosing the *lightest* signal that converges instead of nagging. `[[project_epr_meta_compose_gate]]`
- **`managed-surface-context.py`** (PreToolUse) injects a surface's discipline before the edit; the PostToolUse drift/cite signals nudge new surfaces toward born-linked. `[[feedback_managed_surface_edit_discipline]]`

Caught **before a byte compiles** — left of the compile/CI loop, the cheapest point on the gradient that ends in a RED coherence verdict. And **distributed**: the rule lives in the directory it governs, composed nearest-wins up the tree — no central registry.

## Relationship to /memory-ceremony

The memory team has two distinct cadences. Confusing them is what made the old 6-wave ceremony bloat:

| Cadence | Lead | Deliverable | Frequency |
|---|---|---|---|
| **`/hygiene-sweep` (this kit)** | librarian solo | byte budgets clean, dead citations fixed, archive cascades applied, audit numbers moved | weekly or signal-driven (the SessionStart `cleanup:` gate / drift-score accumulator) |
| **`/memory-ceremony`** | four-agent team (librarian + historian + cartographer + storyteller) | 1-2 gospel-tier surfaces rewritten with substrate-grounded, citation-linked, paste-ready content | when substrate-currency-audit flags drift, or on substrate landing |

Audit numbers (`CLAUDE.md OVER-BUDGET count`, `cleanup-scan flags`, `Surface:Archive ratio`, `MEMORY.md byte size`) move here, NOT in the ceremony. The ceremony's deliverable is a *rewrite*; this kit's deliverable is *hygiene*. They are sibling rhythms — one can run without the other. A third sibling, the **`memory-stasis-loop`** workflow, is the unified auto-trigger that drains every memory discipline back to stasis once a heavy-dev week has accumulated drift (see */hygiene-sweep cadence* below).

The frame, after Pawel Huryn's article ("How I Finally Sorted My Claude Code Memory"):

| Phase | What | Where in this kit |
|---|---|---|
| **Setup** | One-time storage layout | Already done — `MEMORY.md` + typed topic files exist |
| **Maintenance** | Periodic dedupe / archive / rename fixup | `cleanup` + `path-update` + `dedupe-memory` |
| **Hooks** | Inject memory + govern edits across subagents/compaction | the edit-time source-regulation layer (PreToolUse injectors + PostToolUse signals) |
| **Review** | Audit how well memory is working | `memory-review` + `skill-audit` |

**Related skills** (extracted from this kit on 2026-05-13):

- `/converge` — synthesis layer (vision×readiness scoring + next-actions menu). Reads memory-kit reports; produces the session-start handoff.
- Dev-dashboard scripts at `.claude/scripts/dev-dashboard/` — `plan-status.py` and `sprint-distill.py`. Useful, not memory-hygiene.

**Spec reference**: `genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md` — the comet-shaped memory model and lifecycle primitives. Also see `genesis/docs/superpowers/specs/2026-05-13-historian-and-epic-timeline.md` for the three-perspective frame (history/roadmap/development) this kit sits inside. The deterministic stasis/coverage tier is governed by the placement contract at `genesis/docs/PLACEMENT.md`.

**When to invoke**: weekly hygiene pass, before kicking off a major `/shift`, when "memory says X exists but X is gone" surfaces in agent output, or whenever the corpus feels noisy.

## Architecture

- **Scripts** live at `.claude/scripts/memory-kit/<tool>.py`. Pure Python stdlib; no external deps.
- **Outputs** land at `.claude/memory-kit/<YYYY-MM-DD>/<tool>-<artifact>.md`. Shared dated directory across all tools (legacy name retained for `/converge` and dev-dashboard compatibility). This process-artifact tier is **comet-retained** by `memkit-retention.py`: the HEAD stays full, the TAIL compacts to `_digest.md`, and aged entries drop to a one-line `TRAJECTORY.md` row. `TRAJECTORY.md` is the durable spine — older dated bulk is recoverable from git history, not a permanent walk-back surface.
- **Archive destination** (cleanup only): `.claude/archive/<YYYY-MM-DD>/<original-relative-path>`. Mirrors repo structure so trajectory stays walkable backward.
- **Skill entry point**: this single skill, deferred-loaded. **Not always-active** — pull out periodically.

## The Eight Tools

### 1. `cleanup` — archive stale specs/plans/memory

```bash
python3 .claude/scripts/memory-kit/cleanup-scan.py     # Phase 1: deterministic scan
# (judgment subagent dispatched as Phase 2 — see below)
python3 .claude/scripts/memory-kit/cleanup-apply.py    # Phase 3: operator-approved archive
```

Three-phase workflow:

1. **Scan** (deterministic) — surfaces archive candidates (`status: superseded`, completed plans, stale proposals) and dangling-reference flags. Outputs `cleanup-proposals.md`.
2. **Judge** (LLM + semantic search) — dispatch a `general-purpose` subagent: read `cleanup-proposals.md`, investigate each candidate (semantic search for successor specs, recent git activity, dev-intent mentions), classify as ARCHIVE / BACKLOG / KEEP-FRESH / OPERATOR-CALL. Writes `cleanup-proposals-judged.md` (refined proposals with Accept checkboxes only on ARCHIVE) and `cleanup-backlog-refresh.md` (still-relevant unfinished work). The judge auto-classifies the unambiguous archives and reserves OPERATOR-CALL for genuine forks — the apply gate stays operator-approved regardless. This is the decide-clear-calls discipline instantiated (`[[feedback-decide-clear-calls-not-over-ask]]`): the judge settles the obvious archives itself and parks only the real trade-offs the operator owns, rather than routing every candidate up.
3. **Apply** — operator marks `- [x] Accept`; apply moves entries to dated archive. Trajectory-preserving, not delete.

**Boundary**: only modifies via moves, never deletes. The archived items remain queryable for future historian work (see related spec). The scan's **stray-doc census** is the remedial counterpart to the `.epr-meta` compose-gate — the gate keeps new docs born-governed, the census catches residue already on disk.

### 2. `path-update` — propagate renames into stale citations

```bash
python3 .claude/scripts/memory-kit/path-update-scan.py    # detect renames + scan
python3 .claude/scripts/memory-kit/path-update-apply.py   # apply approved replacements
```

`git log --diff-filter=R --name-status` for the last year captures rename pairs. For each, ripgrep finds documents still citing the OLD path. Plus inferred renames via suffix-drop heuristics. Outputs `path-update-proposals.md` with `- [ ] Accept` checkboxes per pair.

**Caveats**: `text.replace` is unscoped — operator should spot-check before bulk-accepting. Suffix-drop heuristic is hand-curated.

**Boundary**: only modifies path strings; never archives, never changes content meaning. (Doc-to-doc cites are content-addressed envelopes that survive moves on their own — see tool 8 — so path-update is for raw-path citations, not for the cite-graph.)

### 3. `dedupe-memory` — surface merge candidates

```bash
python3 .claude/scripts/memory-kit/dedupe-memory-scan.py [--threshold 0.30]
```

TF-IDF cosine similarity across MEMORY topic files. Clusters pairs above threshold. Outputs `dedupe-clusters.md` with similarity scores and shared key terms.

**Calibration**: default threshold is 0.55 per spec, but the corpus is diverse — practical threshold is closer to 0.30. The TF-IDF cosine here is a pure-stdlib *approximation* of semantic dedup over the flat topic files; the embedding-backed equivalent for the palace store is the MemPalace substrate's `mempalace_check_duplicate` (`[[reference_mempalace]]`), which this stdlib method deliberately stands in for where no embedding index is present.

**Boundary**: read-only surface. Does NOT perform merges. Merge is a future tool that consumes these clusters; see the lifecycle spec on the `c ≈ 0.6-0.95` lossiness coefficient.

### 4. `memory-review` — health diagnostic on the auto-memory complement

```bash
python3 .claude/scripts/memory-kit/memory-review.py
```

Read-only diagnostic. Reports:
- MEMORY.md line count vs 200-budget (entries past line 200 are invisible to Claude)
- Topic file count grouped by type prefix (`feedback_/project_/reference_/user_`)
- Growth — entries touched in last 7d / 30d
- Stale candidates — files untouched > 90 days
- Index-vs-files drift — entries in MEMORY.md with no backing file; backing files missing from MEMORY.md

**Boundary**: read-only. Does not propose mutations; surfaces "is the memory layer healthy?" rather than "what should change?". Operator acts on findings via direct MEMORY.md edits or by running `cleanup-scan.py`.

### 5. `claude-md-audit` — signal-triggered CLAUDE.md re-alignment pass

```bash
python3 .claude/scripts/memory-kit/claude-md-audit.py [--threshold 3.0] [--no-reset]
```

CLAUDE.md files are gospel (always-loaded, treated as authoritative) — until they drift. This tool is the **re-alignment pass** for bringing them back in line with project reality. It is invoked when drift signals have accumulated, not on a fixed cadence. ("Re-alignment pass," not "ceremony" — the four-lens substrate-currency rite is the only thing this kit calls a *ceremony*; see *Relationship to /memory-ceremony*.)

**Two-part architecture (signal accumulator + re-alignment pass):**

1. **PostToolUse drift-signal hook** (`.claude/hooks/claude-md-drift-signal.py`) runs after every Edit/Write. It walks up from the edited file to find every enclosing CLAUDE.md scope (a file edit inside `doorway/` counts against `CLAUDE.md` AND `doorway/CLAUDE.md`). It increments `direct_edits` and `scope_edits` counters in `.claude/memory-kit/claude-md-drift.json` and re-computes a `drift_score` every `RESCORE_EVERY_N_EDITS` calls. Cheap path: single-digit ms.

2. **`claude-md-audit.py` (this re-alignment pass)** walks every CLAUDE.md in the repo, computes each file's *direct scope* (files in its dir-tree minus descendant CLAUDE.md subscopes), and classifies by drift signal + content patterns + **fit**:
   - `MAYBE-UNNECESSARY` — tiny scope (≤3 files, no sub-CLAUDE.md descendants) with a non-trivial doc; consider deleting
   - `UNDER-WROTE` — claude_md_lines / expected_lines < 0.4; the doc isn't covering the directory's complexity
   - `OVER-WROTE` — claude_md_lines / expected_lines > 2.5; the doc overspecifies for the scope
   - `DRIFTED-FACTUAL` — cited paths/commands no longer exist
   - `DRIFTED-NORMATIVE` — drift_score ≥ threshold (signal accumulated)
   - `OVER-BUDGET` — line count exceeds 200
   - `OVER-IMPERATIVE` — contains "always X / never Y / must Z" rules without a `because…` / rationale anchor within ±3 lines
   - `FRESH` — none of the above

   **Fit calculation**: `expected_lines = base(20) + distinct_extensions × 8 + scope_files × 0.5` (capped at 200). Re-tunable via `FIT_TUNABLES`. Fit classifications take priority over drift-only signals — a directory that may not need a CLAUDE.md at all is the highest-value finding.

   **MISSING-CLAUDE-MD detection**: a separate pass walks the tree looking for directories with substantial content (≥15 files OR ≥4 subdirs, ≥2 distinct complexity extensions) whose nearest ancestor CLAUDE.md is ≥2 levels up. These are candidates for *writing* a new CLAUDE.md. Submodules (read from `.gitmodules`) are excluded.

   **Opt-out via `.no-claude.md` markers**: when a flagged directory genuinely doesn't need a CLAUDE.md (static assets, parent-doc-is-close-enough, etc.), drop a `.no-claude.md` file in the directory with the rationale. The audit surfaces these in their own "Opted out" section so the decision chain stays auditable; the directory is excluded from candidacy until the marker is removed. Marker format: optional YAML frontmatter (`decided`, `revisit-if`) + markdown body explaining the reasoning. The audit uses `_lib.frontmatter` to parse markers and renders a one-line rationale excerpt.

   After producing the report, resets each file's counters so future signals are net-new (use `--no-reset` to preview without resetting).

**Output**: `.claude/memory-kit/<YYYY-MM-DD>/claude-md-audit.md`

**Trust-compute gradient**: the accumulator's compute is layered — by default it does the cheapest work (counter increments). It can be tuned to do more (full git-diff in scope, change-velocity tracking) when signal is noisy or time-since-audit is long. Weights in `SCORE_WEIGHTS` are re-tunable without changing the protocol.

**Boundary**: read-only audit. Operator decides whether to revise any flagged CLAUDE.md. Never auto-writes.

### 6. `agent-audit` — agent catalog quality

```bash
python3 .claude/scripts/memory-kit/agent-audit.py
```

Parallel to `skill-audit` but for `.claude/agents/*.md`. Agent metadata (name + description) is always-loaded into context; bodies encode role + tool grants + model assignments and drift like any doc surface. Checks:

- **VAGUE-DESCRIPTION**: too-short, generic phrases ("use this agent for…"), no trigger/when language
- **TRIGGER-OVERLAP**: distinctive token overlap with sibling agents (may indicate competing trigger surfaces)
- **STALE-MTIME**: untouched > 90 days
- **DRIFTED-FACTUAL**: dead multi-component path citations (bare filenames excluded to limit false positives)
- **OVER-IMPERATIVE**: "always/never/must" without rationale (same regex as claude-md-audit; excludes code fences)
- **TOOLS-MISMATCH**: tools declared in frontmatter but not referenced in body — a known *durable* false-positive class, not a refinement-in-progress. An agent prompt legitimately describes role and judgment, not tool mechanics, so a granted-but-unmentioned tool is expected rather than a defect to tune away. The librarian treats it as low-signal by design.
- **MISSING-MODEL**: no `model:` field (inheritance from parent is usually unintended)

**Boundary**: read-only diagnostic. Operator decides which agents to revise.

### 7. `skill-audit` — skill catalog quality

```bash
python3 .claude/scripts/memory-kit/skill-audit.py
```

Scans `.claude/skills/*/SKILL.md`. Three issue classes: vague descriptions (too short / generic / no `Use when` framing), trigger-overlap pairs, stale-by-mtime (>90 days).

**Why this is in memory-kit**: skill metadata is *always-loaded into Claude's context*. Vague descriptions cost tokens and clutter trigger-matching. Auditing skill descriptions is auditing the always-on context budget — the same surface MEMORY.md occupies.

**Boundary**: read-only diagnostic. Doesn't rewrite descriptions or merge skills.

### 8. `memory-coherence-audit` — keep memory current with the code it cites

```bash
python3 .claude/scripts/memory-kit/memory-coherence-audit.py             # audit + rebuild cites-index
git diff --name-only origin/dev | \
  python3 .claude/scripts/memory-kit/memory-coherence-audit.py --changed -   # what changed re-opens which memory
```

Memory used to be the one substrate lacking edge-invalidation on source change: a memory entry that said "see `path_service.rs`" had no machine-walkable link, so when the code moved the lesson silently went stale. This tool closes that edge — it is the reconciliation-controller pattern (the build-graph's `graph-walker.mjs`, `sync-check.py`) applied to memory. The capability is the **in-flight memory↔code coherence** plane (design `2026-05-28-in-flight-memory-coherence-design.md`).

**Two classes of `cites:` — and they behave differently:**

- **Code/spec/scenario subscriptions** (the in-flight edge): a memory entry may declare an optional top-level `cites:` list of repo-relative **paths or globs** whose *change* should re-open it — the memory-side mirror of a story's `feature:`/`anchors_epics:`. These stay raw paths on purpose: they are a husky-consumable changed-file matcher, not a doc pointer.

  ```yaml
  ---
  name: project_rea_compute_commitment_primitive
  description: ...
  metadata:
    type: project
  cites:
    - genesis/docs/architecture/rea-compute-commitment-primitive.md
    - elohim/holochain/dna/mishpat/**
  ---
  ```

- **Doc-to-doc cites** (cross-references between managed `.md` surfaces): these are **content-addressed envelopes** (`<slug> | desc | fingerprint`) generated by `cite-gen` — *never hand-written* — that **survive file moves** (a rename never breaks an inbound cite). The `cite-{gen,describe,propagate}.py` + `cites-migrate.py` family maintains them; the coherence audit and the `semantic-links` skill share the verdict vocabulary. See `.claude/skills/semantic-links/SKILL.md`.

When you write or revise a memory entry that leans on specific code, specs, or `.feature` scenarios, add the subscription paths to `cites:`. Rollout is organic — seed entries already carry it; the audit's `CITE-CANDIDATE` finding nominates entries that have code paths in their body but no `cites:` yet.

**Modes**:
- *audit* (default): verifies every `cites:` path/envelope still resolves and rebuilds `.claude/memory-kit/cites-index.json` (the index the in-flight hook reads). Report at `.claude/memory-kit/<date>/memory-coherence-audit.{json,md}`.
- *changed-files* (`--changed -` from stdin, or `--since <git-ref>`): glob-matches a changed-file list against every entry's `cites:` and emits `STALE-CANDIDATE` — "entry X cites Y which just changed; re-verify." `graph-walker.mjs`'s walk shape applied to memory nodes; usable from a husky pre-push consumer.

**Verdict vocabulary** (shared with `semantic-links`):
- `DEAD-CITE` — the target no longer resolves; re-point or remove.
- `HELD-CITE` — the target is a `held/` doc (scope-narrowed off the plate); it **still resolves** — do NOT treat as dead.
- `STALE-CANDIDATE` — fingerprint drift on a doc cite, or a subscribed path just changed; re-verify the lesson.
- `CITE-FORMAT-CANDIDATE` — a legacy raw-path doc cite; run `cites-migrate.py` to drain it into a content-addressed envelope (the `cites_legacy` drain).
- `CITE-CANDIDATE` — a body that names code paths but carries no `cites:` yet.

**Boundary**: read-only over memory; the only writes are the derived report + cites-index (single-writer projection). Never edits a memory entry's lesson — it surfaces which lessons to re-verify; the librarian/operator decides.

Design: `genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md`.

## The Stasis & Coverage Tier (deterministic, no-LLM)

A second toolset sits alongside the eight hygiene tools: a deterministic state machine over the doc + memory surfaces, governed by the placement contract at **`genesis/docs/PLACEMENT.md`**. Where the hygiene tier asks "is this surface clean?", the stasis tier asks "is every artifact at a known *position + state*, and is the surface trending toward **stasis** (pressure dirs empty, no dumps, every file has a next state)?" Read it once; then read the ledger first on every sweep.

| Tool | What it gives you |
|---|---|
| `placement-audit.py` | the **budget/ledger AND the stasis/coverage meter** — `--ledger` (every file → position + state + next-action), `--focus` (currently-testable surface from `cluster-state.yaml`), `--headline` (the SessionStart MEMORY-BUDGET line, including the `cleanup:` and `scope:` gates), `--stasis` (the per-dimension context-coverage composite score — the named coverage dimensions + hard gates; this is the report `context-ratchet.py` reads via `--stasis --json`), `--coverage` (a *different* metric: the spec/plan decompose/capture workload — active / captured / needs-agent / undecomposed / uncaptured), plus the `--epr-meta` cut (the sibling `--subject` and `--report` cuts belong to `focus-baseline.py` and `scope-reconcile.py`, which `placement-audit` shells out to — not flags of `placement-audit` itself). The deterministic instrument that *directs* the sweep so hygiene drains a measured budget rather than running blind. The cartographer/`/converge` primary instrument. |
| `context-ratchet.py` | the **context-coverage GATE** — the code-coverage ratchet applied to context. Compares each coverage dimension against `.claude/memory-kit/context-coverage-baseline.json`, FAILS (exit 1) on any drop below `baseline − EPSILON` (a regression), and ratchets the baseline UP as the loop improves it. This is the *gate* half of the coverage-instrument-vs-coverage-gate split: `placement-audit --stasis` **measures** (the per-dimension coverage score), `context-ratchet` **enforces**. |
| `spec-coherence-index.py` | the prior-art **topic → ranked-specs index** (`--query <topic>`) — the "have we spec'd this?" lookup so a new design *composes from canon* instead of re-specifying ground already covered. |
| `prep-brainstorm.py` | the deterministic **/brainstorm FRONT-seam preload** — composes `spec-coherence-index --query` (prior art), `placement-audit --focus` (testable / blocked-by-env surface), and `placement-audit --ledger --json` (outstanding pressure) into one brief. Its `DRIFT_THRESHOLD` (120) matches the documented `cleanup:` gate, so an over-pressure corpus is flagged for a structuring pass before the brainstorm opens. |
| `state-machine-gen.py` | the physical realizer of "**directory IS state**" — (re)creates the pressure state-dirs under a single root and writes each dir's `CLAUDE.md` GATE (EMPTY = EQUILIBRIUM). The named generator behind the stasis *position* vocabulary the ledger reads. |
| `substrate-currency-audit.py` | the **gospel-tier substrate-drift detector** — flags an always-loaded surface (agent / skill / CLAUDE.md) whose claims have drifted from substrate. Its output is the named hand-off into `/memory-ceremony`; this kit never attempts the rewrite. |
| `story-coverage-audit.py` | writes `.claude/memory-kit/story-coverage-audit.json` — orphan a2o features + per-orphan `leverage_score` + sourcing-completeness flags that `/converge` and the storyteller consume to rank canonical-story authoring. |
| `decompose.py` | spec/plan → bounded, cited **gap-items** (OPEN = implement, CLAIMED = verify) — the Spec/Plan Compaction Loop's drain. Three-zone stasis: the ACTIVE pile shrinks, the `history/` museum grows, working memory stays budgeted. |
| `memkit-retention.py` | comet-retention over the dated report tier: HEAD full / TAIL → `_digest.md` / aged → a one-line `TRAJECTORY.md` row. Keeps the process-artifact tier bounded; `TRAJECTORY.md` is the durable spine. |
| `mempalace-currency.py` | the MemPalace semantic index as an audited store on the same measured + enforced footing as the others — a staleness tripwire comparing last-mine vs newest mined-surface mtime. `--status` / `--remine`. Only the **clean** surface is mined, never the pile/junk — the MCP-scoped palace substrate whose per-subagent ownership keeps mining to the curated surface (`[[reference_mempalace]]`). |
| `scope-reconcile.py` · `focus-baseline.py` · `locus-drift.py` | the scope/focus instruments — reader-twins of the ledger. The held-tree reconciler (`git mv`s docs live↔held as a capability is lost/returns, inbound cites flip `HELD-CITE ↔ healthy` automatically), the per-subject focus baseline, and the per-locus drift roll-up. Trust the structured cluster-state flag over a prose note (`[[scope-flag-beats-prose-note]]`). |

**Measurement vs gate.** The tier separates *reading* the coverage from *enforcing* it: `placement-audit.py --stasis` is the instrument (the per-dimension coverage score you read), and `context-ratchet.py` is the gate (a baseline that fails on regression and ratchets up). The `spec-coherence-index → prep-brainstorm` pair is the same tier facing FRONT — the deterministic compose-from-canon preload that runs before a `/brainstorm` opens, so a new design starts from prior art and the current pressure budget rather than a blank page.

**Boundary**: all deterministic (no-LLM) and read-only by default; the writes are derived reports, the ledger, the cites-index, the coverage baseline, and operator-gated `git mv`s in `scope-reconcile`. The lifecycle, the four verification states (VERIFIED-STABLE / CLAIMED-ONLY / REGRESSED / BLOCKED-BY-ENV), and the home taxonomy all live in `genesis/docs/PLACEMENT.md`. (`delivery-status-distribution.py` physically lives in this kit's script dir but is delivery-stasis domain — driven by `/delivery-stasis`, not this kit; it is listed here only so a reader scanning the directory isn't surprised.)

## The Hooks

The hooks split by job. The **always-on `MEMORY.md` injector** primes context, and the **pickup-time semantic surfacing** hook complements it with a once-per-session palace recall. The rest are the **edit-time source-regulation layer** — they fire for the emitting agent at the moment of a `Write`/`Edit` so the pollution is internalized at the source (see *Pollution Regulation for Agentic-Developer Context* above): PreToolUse injectors that gate or inject discipline before the edit lands, and PostToolUse hooks that project derived surfaces and accumulate drift toward the audits.

### PreToolUse Memory Hook

`.claude/hooks/pre-tool-memory.py` is registered with matcher `"*"` in `.claude/settings.json`. It injects the first 200 lines of `MEMORY.md` into context before the first tool call of each session / subagent process tree. The `/tmp/claude-memory-loaded-<ppid>` flag file gates it to fire once per process tree.

**Why this matters**: Claude Code's `SessionStart` hook fires once per session and does not reach subagents or survive context compaction. The PreToolUse hook covers both gaps. Cost is single-digit ms per tool call after the flag is set.

**If overhead becomes a concern**: drop back to SessionStart-only by removing the `*` matcher entry from `PreToolUse` in settings.json.

### Pickup-Time Semantic Surfacing Hook

`.claude/hooks/pickup-semantic-surfacing.py` (registered in settings.json on both `UserPromptSubmit` and `PreToolUse`) is the **PICKUP-time complement** to the always-on `MEMORY.md` injector: where `pre-tool-memory.py` injects the static index, this fires *once per session* on the pickup vocabulary — eligible on prompts 1-3, or on the first search-shaped tool call — and pulls a semantic recall from the MemPalace store. The engine is the MemPalace **CLI**, not the MCP (the MCP is absent in the main session), gated first by a millisecond `mempalace-currency.py --status` staleness check and then a ~3s `mempalace search`. Injection contract: below the cosine floor (`COSINE_FLOOR = 0.35`) it is a silent no-op; a stale index degrades to a banner rather than blocking. Read-only; never mutates.

### PreToolUse `.epr-meta` Compose-Gate Hook

`.claude/hooks/epr-meta-resolver.py` (matcher: `Edit|Write`; resolver lib `_lib/epr_meta.py`) resolves the cascade of directory-local `.epr-meta` manifests above the edited path and returns a compose-gate verdict. Authoring a NEW `.md` under a governed root must satisfy that directory's declared frontmatter contract or the write is DENIED; editing an existing file is not gated (`new: true`). A malformed manifest is strict-but-recoverable — the subtree downgrades `deny→ask` and the `.epr-meta` itself stays editable, because an unvalidated manifest is *proposed* governance (Private-reach intent), never *binding* deny. This is the born-governed leg of edit-time source regulation; author manifests with the `elohim-epr-metafile` skill. See `[[project_epr_meta_compose_gate]]`.

### PreToolUse Managed-Surface Context Hook

`.claude/hooks/managed-surface-context.py` (matcher: `Edit|Write`, timeout 3s) injects a managed-memory
surface's discipline + exact tooling BEFORE an edit lands on it — gospel CLAUDE.mds, specs/plans, doc-roots,
memory entries, a2o features, stories, skills/agents, the axis manifests. Scope comes from
`_lib/managed_surfaces.py`, the single edit-time registry (surface class → discipline → tooling → cite-graph
membership) that `cite-seal-signal.py` and the cite-gen/migrate sweeps also consult — **scope is defined once,
never re-hardcoded per tool**, and a stale cite is routed to the re-verify queue (`--refresh`), never left
silently dead (the managed-surface edit discipline, `[[feedback_managed_surface_edit_discipline]]`). Fires
once per (file, process tree); fail-open. Design:
`genesis/docs/superpowers/specs/2026-06-05-managed-surface-edit-discipline-design.md`.

### PostToolUse Drift-Signal Hook

`.claude/hooks/claude-md-drift-signal.py` (matcher: `Edit|Write`, timeout 2s) accumulates drift signal for `claude-md-audit` (section 5). Best-effort: never blocks the tool call. See section 5 for the layered-compute design.

### PostToolUse Memory-Coherence Signal Hook

`.claude/hooks/memory-coherence-signal.py` (matcher: `Edit|Write`, timeout 2s) is the in-flight accumulator for `memory-coherence-audit` (section 8). When an edited file matches a memory entry's `cites:` glob (via the cached `.claude/memory-kit/cites-index.json`), it bumps that entry's counter in `.claude/memory-kit/memory-coherence-drift.json`. The librarian surfaces accumulated counts during `/hygiene-sweep` ("N entries cite code that changed since last verified") and resets them. Best-effort; never blocks. If the index doesn't exist yet, the hook is dormant until the first `memory-coherence-audit.py` run builds it.

### PostToolUse Memory-Index Projector Hook

`.claude/scripts/memory-kit/memory-index-projector.py --hook` (matcher: `Edit|Write`, registered directly in settings.json — the hook IS the script's thin transport). **`MEMORY.md` is a GENERATED projection** of each topic file's `title:` + `description:` frontmatter (decided 2026-07-02, after a hand-maintained index bloated past the harness load cap and loaded truncated — the context-leak class). On any write under `.claude/memory/`, the hook re-projects the index; per-entry violations (missing/overlong description) and over-soft-budget land as a snapshot in `.claude/memory-kit/memory-index-drift.json`, which `cleanup-pressure.py` counts toward the `cleanup:` gate — budget pressure escalates to the memory-stasis-loop on the same flag→canon→stasis rails as every other accumulator. Budgets: soft 20,000B (start consolidating) / hard 24,000B (the harness cap with headroom); entry discipline: `title:` ≤ 80 chars, `description:` one line ≤ 200 (target ≤ 160). The co-located `.claude/memory/.epr-meta` carries the compose-gate half: new memory files are DENIED at birth without `name`/`title`/`description`, and hand-writes to `MEMORY.md` get an `ask` pointing back to the projector. Budget relief is population work — umbrella entries (`index: false` on folded members) and graduation — never index hand-editing.

### PostToolUse Placement-Drift & Map-Currency Signal Hooks

`.claude/hooks/placement-drift-signal.py` and `.claude/hooks/map-drift-signal.py` (matcher: `Edit|Write`, timeout 2s) are the two remaining activity accumulators the `cleanup:` gate reads. `placement-drift-signal.py` queues a `decompose-due` entry in `placement-drift.json` when an edit lands a terminal status (`landed`/`superseded`/`abandoned`) into a doc still living plan-shaped in an ACTIVE home (`genesis/docs/superpowers/{specs,plans}`, `genesis/docs/plans`); it self-heals when the status is edited back to non-terminal. `map-drift-signal.py` bumps `map-currency-drift.json` when an architecture seed under `genesis/docs/content/elohim-protocol/architecture/*.md` changes while `MAP.md` itself is untouched, and resets to zero when `MAP.md` is edited (the walk just got refreshed). Both surface at SessionStart through `placement-audit.py --headline` (the decompose-due line and the `path:` currency line). Best-effort; never block.

### Born-Linked Signal Hooks

`.claude/hooks/cite-seal-signal.py` (PostToolUse `Edit|Write`) and `.claude/hooks/claude-md-structural-signal.py` (**PreToolUse `Bash`** — it inspects the command string for `mv`/`git mv`/`cp -r`/`rm -rf`/`mkdir` structural ops) are the **born-linked** nudges of the edit-time layer. cite-seal accumulates signal when a managed surface is edited without a sealed cite; claude-md-structural bumps a `structural_edits` counter on the affected CLAUDE.md scopes when a directory moves/copies/removes — weighted heavier than a plain edit, since a moved directory can strand the CLAUDE.md describing its old shape. Both are best-effort and never block; their counters feed the audits in this kit on the same `flag → agent → canon → stasis` shape as the other accumulators.

A related PostToolUse `Edit|Write` accumulator, `.claude/hooks/sovereignty-guard-signal.py`, lives in the edit-time layer but feeds a **separate** loop: it accumulates governance-lexicon signal (the crypto self-sovereignty apex drift, `[[feedback-identity-sovereignty-ontology-guard]]`) into its own sovereignty ontology guard, and is deliberately **not** one of the five activity stores the `cleanup:` gate sums (below). It is named here so the five-store enumeration and this sixth governance signal don't read as a contradiction — different gate, same born-governed shape.

## Shared Helpers (`_lib/`)

Pure-stdlib helpers at `.claude/scripts/_lib/` for use by scripts AND hooks:

| Module | Purpose |
|---|---|
| `_lib.paths` | `repo_root_from_file(__file__)` (robust replacement for `parents[N]`), `reports_root()`, `reports_dir_for_today()`, `memory_dir()` |
| `_lib.frontmatter` | Minimal YAML-frontmatter parser for memory entries (handles scalars + simple lists; not PyYAML-dependent) |
| `_lib.store` | Best-effort JSON load/save with safe defaults — used by accumulator hooks where filesystem errors must not crash the tool call |
| `_lib.managed_surfaces` | The single edit-time registry (surface class → discipline → tooling → cite-graph membership) read by the managed-surface hook, the cite-seal signal, and the cite sweeps |
| `_lib.epr_meta` | The `.epr-meta` cascade resolver (directory-local manifest load + nearest-wins override + verdict) read by the `epr-meta-resolver` PreToolUse hook |

**Import pattern** (works from any script or hook):

```python
from pathlib import Path
import sys
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import paths, store  # noqa: E402
```

**Discipline**: only extract when 3+ callers share the same pattern. Resist scope creep. New scripts use these from the start; older scripts migrate when touched. Scope lives in the registry only — never re-hardcode a surface class per hook.

## /hygiene-sweep cadence

This kit's primary periodic role. Librarian-solo (no four-agent ceremony, no operator gates beyond the apply-step approvals on cleanup/path-update). Designed to keep audit numbers from regressing so the substrate-currency ceremony can focus on rewrites, not byte-budget triage.

**Weekly sweep (~15 minutes)** — the standard /hygiene-sweep rhythm:

1. `python3 .claude/scripts/memory-kit/placement-audit.py --ledger` — read the budget FIRST: per-file position + state + next-action, so the sweep drains a measured budget rather than running blind (the SessionStart MEMORY-BUDGET headline derives from `--headline`)
2. `python3 .claude/scripts/memory-kit/memory-review.py` — baseline + byte-budget check
3. `python3 .claude/scripts/memory-kit/cleanup-scan.py` — surface archive candidates
4. Dispatch the cleanup-judge subagent over `cleanup-proposals.md`
5. `python3 .claude/scripts/memory-kit/path-update-scan.py`
6. Review the dated `.claude/memory-kit/<today>/` directory
7. Mark `- [x] Accept` on cleanup and path-update entries you approve
8. Run apply scripts: `cleanup-apply.py` → `path-update-apply.py`
9. `genesis/scripts/memory-balance.sh` — log Surface:Archive ratio + budget deltas
10. `python3 .claude/scripts/memory-kit/memory-coherence-audit.py` — rebuild the cites-index and surface entries whose cited code changed (the in-flight hook accumulates these into `memory-coherence-drift.json` between sweeps; surface and reset)

**Monthly deeper sweep (add to the weekly)**:
- `python3 .claude/scripts/memory-kit/dedupe-memory-scan.py --threshold 0.30`
- `python3 .claude/scripts/memory-kit/claude-md-audit.py` — byte-budget + drift across all CLAUDE.md
- `python3 .claude/scripts/memory-kit/agent-audit.py`
- `python3 .claude/scripts/memory-kit/skill-audit.py`
- `python3 .claude/scripts/memory-kit/substrate-currency-audit.py` — flag gospel-tier drift for the next `/memory-ceremony`
- `python3 .claude/scripts/memory-kit/story-coverage-audit.py` — orphan features + leverage scores for `/converge` and the storyteller
- `python3 .claude/scripts/memory-kit/mempalace-currency.py --status` — semantic-index staleness; `--remine` if the clean surface drifted past the tripwire
- `python3 .claude/scripts/memory-kit/context-ratchet.py` — the context-coverage gate: fail on any dimension regressing below baseline, else ratchet the baseline up

**Signal-driven trigger (canonical, drift-accurate).** The SessionStart MEMORY-BUDGET headline carries a `cleanup:` gate. When it reads `cleanup: ⚠ … due` — roughly a week of heavy development has accumulated drift past threshold — run the **`memory-stasis-loop`** workflow, which drains every memory discipline (compaction · dumps · decompose · MAP/path · roadmap · memkit · MemPalace) back to stasis and self-resets the drift baseline. It fires at most ~once per heavy-dev week; on a quiet week the gate reads `held ✅` and you skip. Under the hood the gate (`cleanup-pressure.py`) sums the **distinct drifted items** accumulated across this kit's five activity stores — `placement-drift.json`, `map-currency-drift.json`, `claude-md-drift.json`, `memory-coherence-drift.json`, and `memory-index-drift.json` — and reads `due` when that count crosses `THRESHOLD` (120, calibrated to ~a week of heavy development churning ~100–150 distinct docs/entries). (The governance-lexicon accumulator `sovereignty-guard-signal.py` is a *sixth* signal but feeds its own guard loop, not this gate — see *Born-Linked Signal Hooks*.) Running the loop's `--reset` zeroes those accumulators so the next week starts fresh. The `cleanup:` gate is the unified front door; the per-accumulator item counts are its inputs.

**Before a major `/shift`**: at minimum run `placement-audit.py --ledger` + cleanup + path-update + memory-review. If MEMORY.md is near budget, tighten by *graduating* (one canonical story carries 6+ entries) or *umbrella-folding* related items (`index: false` on folded members; the umbrella file links them) — never by hand-editing the index, which is GENERATED (`memory-index-projector.py --apply`). A correct re-index of orphaned-but-load-bearing entries *grows* the index; budget relief comes from graduation / umbrella / archive-with-pointer, never from deletion (the lifecycle spec's `close-interval` ≠ `forget`; the same gotcha is recorded under "Re-indexing orphans grows MEMORY.md" in `.claude/scripts/memory-kit/CLAUDE.md`).

**Before a `/brainstorm`**: run `prep-brainstorm.py` — the deterministic FRONT-seam preload composes prior-art (`spec-coherence-index --query`), the testable/blocked surface (`placement-audit --focus`), and the outstanding pressure budget (`placement-audit --ledger`) so a new design starts from canon, not a blank page.

**Handoff to `/converge`**: after a sweep, if the operator asks "what's next?", invoke `/converge`. It reads this kit's reports from `.claude/memory-kit/<today>/` (including `story-coverage-audit.json` and the placement ledger) and produces the ranked next-actions menu.

**Handoff to `/memory-ceremony`**: if `substrate-currency-audit.py` (run as part of the sweep or separately) flags a gospel-tier surface as high-drift, the next ceremony picks it up. The hygiene sweep does NOT attempt rewrites — that's the ceremony's deliverable.

## Design Principles

- **Prevention over remediation (source regulation).** The kit governs the context commons at two points: edit-time, where per-directory `.epr-meta` compose-gates and managed-surface injection keep new agentic-agent text born-governed at the emitter (*Pollution Regulation for Agentic-Developer Context*); and after-the-fact, where the hygiene sweep remediates the drift that slipped past. Internalizing the externality at the source is cheaper than the sweep — and cheaper than the substrate-currency ceremony's RED coherence verdict, which is the same pollution measured downstream. See `[[project_epr_meta_compose_gate]]` and `[[feedback_managed_surface_edit_discipline]]`.
- **Complement, don't replace.** Native auto-memory is Claude-self-managed and kept LIGHT (non-obvious cross-session facts only); durable architecture / vision / wisdom *graduates* to the managed artifacts this kit tends rather than piling up in native memory. The boundary is firm (the 2026-06-03 pair-off; see `.claude/scripts/memory-kit/CLAUDE.md`).
- **Deterministic-first.** Seven of the eight hygiene tools are pure rule-based at the surfacing layer; only `cleanup` uses judgment (the Phase-2 subagent), and even that is operator-gated at apply. The entire stasis/coverage tier is deterministic (no-LLM).
- **Measure, then gate.** Coverage separates instrument from gate: `placement-audit --stasis` reads the per-dimension coverage score; `context-ratchet.py` enforces it (fail-on-regression, ratchet-baseline-up). Keep the reader and the gate separate — the score informs the sweep; the gate defends the floor.
- **Operator-approved.** No tool modifies files without an explicit `- [x] Accept` (or, for `scope-reconcile`, an operator scope decision). Even cleanup's apply step skips unchecked entries. The judge decides the clear archives itself and reserves OPERATOR-CALL for genuine forks (`[[feedback-decide-clear-calls-not-over-ask]]`) — operator-approved is the *apply* gate, not a decision-parking habit.
- **Archive, never delete.** Matches the lifecycle spec — `close-interval` is structurally distinct from `forget`. Archived items remain queryable; future historian work (see related spec) depends on this.
- **Read-only by default.** The most common interaction is "scan → review → close" without any modification. Modification is the exception. (The edit-time gates are the deliberate exception in the other direction — they act *before* a write, fail-open, and never mutate content; they gate or signal, they do not rewrite.)
- **Trajectory-preserving.** Archived items keep their original relative path under a dated directory. The process-artifact report tier is comet-retained (`memkit-retention.py`), with `TRAJECTORY.md` as the durable spine. Walking history backward stays possible.
- **Signal-accumulator architecture.** The accumulator → ceremony → reset shape (a cheap deterministic hook writes a ledger flag → a *new* flag dispatches the audit/ceremony → a re-encounter cites canon instead of re-firing → the stasis sweep resets the counter to baseline) is the project's canonical event-automation pattern (`[[feedback_deterministic_flag_agent_canon_stasis_pattern]]`). Extend an audit tool along this shape — never re-derive an ad-hoc accumulator, which re-invites the agent-storm-on-known-blocked-items and the tombstone-instead-of-reset failures the pattern prevents.
- **Describe durable structure, not status.** Every audit class and every line in this kit names stable architecture, never transient sprint/tool state ("currently noisy", "refinement candidate", "in-flight" as a status word) — the `[[feedback_agent_prompts_no_process_status]]` discipline, applied to the kit's own surfaces. The catalog audits (`claude-md-audit`, `agent-audit`, `skill-audit`) do not carry a process-status check class of their own; this discipline is enforced by hand and by the substrate-currency ceremony.
- **Bounded but widened scope.** The hygiene tier tends `MEMORY.md` + topic files + the always-loaded skill/agent/CLAUDE.md metadata; the stasis/coverage tier extends to the managed artifacts (specs, plans, `genesis/docs/`, `genesis/data/stories/`, gospel CLAUDE.mds) governed by `genesis/docs/PLACEMENT.md` and routed by `.claude/subject-routing.yaml`; the edit-time layer extends governance to any directory that carries an `.epr-meta`. It does not touch arbitrary application code, test files, or CI artifacts.
- **Single skill entry point.** This skill. Eight hygiene tools plus the stasis/coverage tier. The hooks — one always-on context injector plus the pickup-time semantic surfacing and the edit-time source-regulation layer (the PreToolUse gates and the PostToolUse drift/cite/projector accumulators inventoried in *The Hooks*). Deferred-loaded. Periodic invocation.

## What This Kit Is NOT

- Not a general auto-merge / auto-promote consolidator (those remain future design; see the lifecycle spec). **Compaction is covered**: `decompose.py` drains specs/plans into bounded cited gap-items and the Spec/Plan Compaction Loop graduates their lessons to `history/` and to canonical stories. The three-zone stasis verdict holds — the ACTIVE pile is the only shrink-target; the `history/` museum is a grow-target, never force-shrunk. (No "now" / "ships" status word here on purpose — `[[feedback_agent_prompts_no_process_status]]`.)
- Not autonomous (operator approval is structural for any file modification; the edit-time gates act before a write and never rewrite content)
- Not always-active (single skill, deferred-loaded, periodic — the hooks are the only always-resident surface, and they gate/signal rather than run the kit)
- Not the historian (pattern-aware un-archive + precedent surfacing is the historian's role; see related spec)
- Not the converge synthesizer (extracted to `/converge` skill 2026-05-13)
- Not the dev-dashboard (plan-status + sprint-distill moved to `.claude/scripts/dev-dashboard/`)

## Related

- `.claude/skills/converge/SKILL.md` — synthesis layer; consumes memory-kit reports
- `.claude/skills/semantic-links/SKILL.md` — content-addressed cite envelopes (`cite-gen`); the doc-cite verdict vocabulary this kit shares
- `.claude/skills/elohim-epr-metafile/SKILL.md` — authoring guide for the `.epr-meta` directory-local compose-gates (the born-governed edit-time leg)
- `.claude/hooks/epr-meta-resolver.py` — the PreToolUse `.epr-meta` compose-gate resolver
- `.claude/hooks/pickup-semantic-surfacing.py` — the pickup-time MemPalace semantic recall hook (`[[reference_mempalace]]`)
- `.claude/memory/project_epr_meta_compose_gate.md` — the compose-gate lesson (`[[project_epr_meta_compose_gate]]`)
- `.claude/memory/feedback_managed_surface_edit_discipline.md` — the managed-surface edit discipline (`[[feedback_managed_surface_edit_discipline]]`)
- `genesis/docs/PLACEMENT.md` — the placement contract governing the stasis/coverage tier (positions, the four verification states, the gap-item budget)
- `genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md` — lifecycle primitives (the design language)
- `genesis/docs/superpowers/specs/2026-05-13-historian-and-epic-timeline.md` — the three-perspective frame this kit sits inside; the historian role
- `genesis/docs/content/elohim-protocol/living_memory/epic.md` — narrative on what living memory means in the protocol
- `.claude/hooks/pre-tool-memory.py` — the PreToolUse memory injector
- `.claude/hooks/load-project-context.py` — the SessionStart context loader (complementary, broader scope)
- `.claude/memory-kit/<YYYY-MM-DD>/` — dated outputs (operator review surface; comet-retained via `memkit-retention.py`, `TRAJECTORY.md` is the durable spine)
- `.claude/archive/<YYYY-MM-DD>/` — cleanup destination (preserves trajectory; future historian indexes this)
