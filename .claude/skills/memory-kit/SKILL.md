---
name: memory-kit
description: Librarian-solo hygiene-sweep toolkit and cadence. Provides eight deterministic tools (cleanup, path-update, dedupe-memory, memory-review, claude-md-audit, skill-audit, agent-audit, memory-coherence-audit) plus a PreToolUse injector and PostToolUse drift accumulators for byte-budget enforcement, archive-ratio tracking, dead-citation hygiene, and in-flight memory↔code coherence. Read-only by default; mutations are operator-gated. Sibling to /memory-ceremony (which is the four-lens substrate-currency rewrite ceremony). Use this when you want byte-budget enforcement, an audit-numbers pass, or to chain a few hygiene tools ad-hoc.
---

# Memory Kit — Hygiene-Sweep Cadence (Librarian-Solo)

Project memory lives at **`/projects/elohim/.claude/memory/`** — git-tracked, team-shareable, recoverable from clone. The harness-conventional path at `/projects/.claude-config/projects/-projects-elohim/memory/` is a symlink to the primary (so auto-memory writes land in the repo). See `.claude/memory/project_memory_in_repo_two_tier.md` for the rationale. This kit tends that `MEMORY.md` index and its typed topic files, fixes drift, surfaces archive candidates, and audits the always-loaded surfaces. It does not replace native auto-memory; it complements it.

## Relationship to /memory-ceremony

The memory team has two distinct cadences. Confusing them is what made the old 6-wave ceremony bloat:

| Cadence | Lead | Deliverable | Frequency |
|---|---|---|---|
| **`/hygiene-sweep` (this kit)** | librarian solo | byte budgets clean, dead citations fixed, archive cascades applied, audit numbers moved | weekly or signal-driven (drift-score accumulator) |
| **`/memory-ceremony`** | four-agent team (librarian + historian + cartographer + storyteller) | 1-2 gospel-tier surfaces rewritten with substrate-grounded, citation-linked, paste-ready content | when substrate-currency-audit flags drift, or on substrate landing |

Audit numbers (`CLAUDE.md OVER-BUDGET count`, `cleanup-scan flags`, `Surface:Archive ratio`, `MEMORY.md byte size`) move here, NOT in the ceremony. The ceremony's deliverable is a *rewrite*; this kit's deliverable is *hygiene*. They are sibling rhythms — one can run without the other.

The frame, after Pawel Huryn's article ("How I Finally Sorted My Claude Code Memory"):

| Phase | What | Where in this kit |
|---|---|---|
| **Setup** | One-time storage layout | Already done — `MEMORY.md` + typed topic files exist |
| **Maintenance** | Periodic dedupe / archive / rename fixup | `cleanup` + `path-update` + `dedupe-memory` |
| **Hooks** | Inject memory into context across subagents/compaction | `.claude/hooks/pre-tool-memory.py` (PreToolUse `*`) |
| **Review** | Audit how well memory is working | `memory-review` + `skill-audit` |

**Related skills** (extracted from this kit on 2026-05-13):

- `/converge` — synthesis layer (vision×readiness scoring + next-actions menu). Reads memory-kit reports; produces the session-start handoff.
- Dev-dashboard scripts at `.claude/scripts/dev-dashboard/` — `plan-status.py` and `sprint-distill.py`. Useful, not memory-hygiene.

**Spec reference**: `genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md` — the comet-shaped memory model and lifecycle primitives. Also see `genesis/docs/superpowers/specs/2026-05-13-historian-and-epic-timeline.md` for the three-perspective frame (history/roadmap/development) this kit sits inside.

**When to invoke**: weekly hygiene pass, before kicking off a major `/shift`, when "memory says X exists but X is gone" surfaces in agent output, or whenever the corpus feels noisy.

## Architecture

- **Scripts** live at `.claude/scripts/memory-kit/<tool>.py`. Pure Python stdlib; no external deps.
- **Outputs** land at `.claude/memory-kit/<YYYY-MM-DD>/<tool>-<artifact>.md`. Shared dated directory across all tools (legacy name retained for compatibility with `/converge` and dev-dashboard scripts).
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
2. **Judge** (LLM + semantic search) — dispatch a `general-purpose` subagent: read `cleanup-proposals.md`, investigate each candidate (semantic search for successor specs, recent git activity, dev-intent mentions), classify as ARCHIVE / BACKLOG / KEEP-FRESH / OPERATOR-CALL. Writes `cleanup-proposals-judged.md` (refined proposals with Accept checkboxes only on ARCHIVE) and `cleanup-backlog-refresh.md` (still-relevant unfinished work).
3. **Apply** — operator marks `- [x] Accept`; apply moves entries to dated archive. Trajectory-preserving, not delete.

**Boundary**: only modifies via moves, never deletes. The archived items remain queryable for future historian work (see related spec).

### 2. `path-update` — propagate renames into stale citations

```bash
python3 .claude/scripts/memory-kit/path-update-scan.py    # detect renames + scan
python3 .claude/scripts/memory-kit/path-update-apply.py   # apply approved replacements
```

`git log --diff-filter=R --name-status` for the last year captures rename pairs. For each, ripgrep finds documents still citing the OLD path. Plus inferred renames via suffix-drop heuristics. Outputs `path-update-proposals.md` with `- [ ] Accept` checkboxes per pair.

**Caveats**: `text.replace` is unscoped — operator should spot-check before bulk-accepting. Suffix-drop heuristic is hand-curated.

**Boundary**: only modifies path strings; never archives, never changes content meaning.

### 3. `dedupe-memory` — surface merge candidates

```bash
python3 .claude/scripts/memory-kit/dedupe-memory-scan.py [--threshold 0.30]
```

TF-IDF cosine similarity across MEMORY topic files. Clusters pairs above threshold. Outputs `dedupe-clusters.md` with similarity scores and shared key terms.

**Calibration**: default threshold is 0.55 per spec, but the corpus is diverse — practical threshold is closer to 0.30.

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

### 5. `claude-md-audit` — signal-triggered CLAUDE.md ceremony

```bash
python3 .claude/scripts/memory-kit/claude-md-audit.py [--threshold 3.0] [--no-reset]
```

CLAUDE.md files are gospel (always-loaded, treated as authoritative) — until they drift. This tool is the **ceremony** for re-aligning them with project reality. It is invoked when drift signals have accumulated, not on a fixed cadence.

**Two-part architecture (signal accumulator + ceremony):**

1. **PostToolUse drift-signal hook** (`.claude/hooks/claude-md-drift-signal.py`) runs after every Edit/Write. It walks up from the edited file to find every enclosing CLAUDE.md scope (a file edit inside `doorway/` counts against `CLAUDE.md` AND `doorway/CLAUDE.md`). It increments `direct_edits` and `scope_edits` counters in `.claude/memory-kit/claude-md-drift.json` and re-computes a `drift_score` every `RESCORE_EVERY_N_EDITS` calls. Cheap path: single-digit ms.

2. **`claude-md-audit.py` (this ceremony)** walks every CLAUDE.md in the repo, computes each file's *direct scope* (files in its dir-tree minus descendant CLAUDE.md subscopes), and classifies by drift signal + content patterns + **fit**:
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

**Boundary**: read-only ceremony. Operator decides whether to revise any flagged CLAUDE.md. Never auto-writes.

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
- **TOOLS-MISMATCH**: tools declared in frontmatter but not referenced in body (currently noisy — agent prompts often describe role, not tool mechanics; refinement candidate)
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

Memory used to be the one substrate lacking edge-invalidation on source change: a memory entry that said "see `path_service.rs`" had no machine-walkable link, so when the code moved the lesson silently went stale. This tool closes that edge — it is the reconciliation-controller pattern (the build-graph's `graph-walker.mjs`, `sync-check.py`) applied to memory.

**The `cites:` convention (capture-time discipline)**: a memory entry may declare an optional top-level `cites:` frontmatter list of repo-relative paths or globs whose change should re-open it — the memory-side mirror of a story's `feature:`/`anchors_epics:`:

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

When you write or revise a memory entry that leans on specific code, specs, or `.feature` scenarios, add them to `cites:`. Rollout is organic — seed entries already carry it; the audit's `CITE-CANDIDATE` finding nominates entries that have code paths in their body but no `cites:` yet. See `.claude/memory/project_memory_cites_edge.md`.

**Modes**:
- *audit* (default): verifies every `cites:` path still resolves (`DEAD-CITE`), nominates `CITE-CANDIDATE`s, and rebuilds `.claude/memory-kit/cites-index.json` (the index the in-flight hook reads). Report at `.claude/memory-kit/<date>/memory-coherence-audit.{json,md}`.
- *changed-files* (`--changed -` from stdin, or `--since <git-ref>`): glob-matches a changed-file list against every entry's `cites:` and emits `STALE-CANDIDATE` — "entry X cites Y which just changed; re-verify." `graph-walker.mjs`'s walk shape applied to memory nodes; usable from a husky pre-push consumer.

**Boundary**: read-only over memory; the only writes are the derived report + cites-index (single-writer projection). Never edits a memory entry's lesson — it surfaces which lessons to re-verify; the librarian/operator decides.

Design: `genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md`.

## The Hooks

### PreToolUse Memory Hook

`.claude/hooks/pre-tool-memory.py` is registered with matcher `"*"` in `.claude/settings.json`. It injects the first 200 lines of `MEMORY.md` into context before the first tool call of each session / subagent process tree. The `/tmp/claude-memory-loaded-<ppid>` flag file gates it to fire once per process tree.

**Why this matters**: Claude Code's `SessionStart` hook fires once per session and does not reach subagents or survive context compaction. The PreToolUse hook covers both gaps. Cost is single-digit ms per tool call after the flag is set.

**If overhead becomes a concern**: drop back to SessionStart-only by removing the `*` matcher entry from `PreToolUse` in settings.json.

### PostToolUse Drift-Signal Hook

`.claude/hooks/claude-md-drift-signal.py` (matcher: `Edit|Write`, timeout 2s) accumulates drift signal for `claude-md-audit` (section 5). Best-effort: never blocks the tool call. See section 5 for the layered-compute design.

### PreToolUse Managed-Surface Context Hook

`.claude/hooks/managed-surface-context.py` (matcher: `Edit|Write`, timeout 3s) injects a managed-memory
surface's discipline + exact tooling BEFORE an edit lands on it — gospel CLAUDE.mds, specs/plans, doc-roots,
memory entries, a2o features, stories, skills/agents, the axis manifests. Scope comes from
`_lib/managed_surfaces.py`, the single edit-time registry (surface class → discipline → tooling → cite-graph
membership) that `cite-seal-signal.py` and the cite-gen/migrate sweeps also consult — scope is defined once,
never re-hardcoded per tool. Fires once per (file, process tree); fail-open. Design:
`genesis/docs/superpowers/specs/2026-06-05-managed-surface-edit-discipline-design.md`.

### PostToolUse Memory-Coherence Signal Hook

`.claude/hooks/memory-coherence-signal.py` (matcher: `Edit|Write`, timeout 2s) is the in-flight accumulator for `memory-coherence-audit` (section 8). When an edited file matches a memory entry's `cites:` glob (via the cached `.claude/memory-kit/cites-index.json`), it bumps that entry's counter in `.claude/memory-kit/memory-coherence-drift.json`. The librarian surfaces accumulated counts during `/hygiene-sweep` ("N entries cite code that changed since last verified") and resets them. Best-effort; never blocks. If the index doesn't exist yet, the hook is dormant until the first `memory-coherence-audit.py` run builds it.

## Shared Helpers (`_lib/`)

Pure-stdlib helpers at `.claude/scripts/_lib/` for use by scripts AND hooks:

| Module | Purpose |
|---|---|
| `_lib.paths` | `repo_root_from_file(__file__)` (robust replacement for `parents[N]`), `reports_root()`, `reports_dir_for_today()`, `memory_dir()` |
| `_lib.frontmatter` | Minimal YAML-frontmatter parser for memory entries (handles scalars + simple lists; not PyYAML-dependent) |
| `_lib.store` | Best-effort JSON load/save with safe defaults — used by accumulator hooks where filesystem errors must not crash the tool call |

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

**Discipline**: only extract when 3+ callers share the same pattern. Resist scope creep. New scripts use these from the start; older scripts migrate when touched.

## /hygiene-sweep cadence

This kit's primary periodic role. Librarian-solo (no four-agent ceremony, no operator gates beyond the apply-step approvals on cleanup/path-update). Designed to keep audit numbers from regressing so the substrate-currency ceremony can focus on rewrites, not byte-budget triage.

**Weekly sweep (~15 minutes)** — the standard /hygiene-sweep rhythm:

1. `python3 .claude/scripts/memory-kit/memory-review.py` — baseline + budget check
2. `python3 .claude/scripts/memory-kit/cleanup-scan.py` — surface archive candidates
3. Dispatch the cleanup-judge subagent over `cleanup-proposals.md`
4. `python3 .claude/scripts/memory-kit/path-update-scan.py`
5. Review the dated `.claude/memory-kit/<today>/` directory
6. Mark `- [x] Accept` on cleanup and path-update entries you approve
7. Run apply scripts: `cleanup-apply.py` → `path-update-apply.py`
8. `genesis/scripts/memory-balance.sh` — log Surface:Archive ratio + budget deltas
9. `python3 .claude/scripts/memory-kit/memory-coherence-audit.py` — rebuild the cites-index and surface entries whose cited code changed (the in-flight hook accumulates these into `memory-coherence-drift.json` between sweeps; surface and reset)

**Monthly deeper sweep (add to the weekly)**:
- `python3 .claude/scripts/memory-kit/dedupe-memory-scan.py --threshold 0.30`
- `python3 .claude/scripts/memory-kit/claude-md-audit.py` — byte-budget + drift across all CLAUDE.md
- `python3 .claude/scripts/memory-kit/agent-audit.py`
- `python3 .claude/scripts/memory-kit/skill-audit.py`

**Signal-driven trigger** (preferred over fixed cadence): when `.claude/memory-kit/claude-md-drift.json` accumulates `drift_score ≥ 3.0` on any file, or `memory-review.py` reports MEMORY.md at ≥90% of byte budget, the sweep is overdue.

**Before a major `/shift`**: at minimum run cleanup + path-update + memory-review. If MEMORY.md is near budget, also tighten the index (graduate or umbrella per `[[feedback_correct_reindex_grows_index]]`).

**Handoff to `/converge`**: after a sweep, if the operator asks "what's next?", invoke `/converge`. It reads this kit's reports from `.claude/memory-kit/<today>/` and produces the ranked next-actions menu.

**Handoff to `/memory-ceremony`**: if `substrate-currency-audit.py` (run as part of the sweep or separately) flags a gospel-tier surface as high-drift, the next ceremony picks it up. The hygiene sweep does NOT attempt rewrites — that's the ceremony's deliverable.

## Design Principles

- **Complement, don't replace.** Native auto-memory handles chat-side; this kit handles the project-referenced complement. The boundary is firm.
- **Deterministic-first.** Four of five tools are pure rule-based at the surfacing layer. Only cleanup uses judgment (Phase 2 subagent), and even that is operator-gated at apply.
- **Operator-approved.** No tool modifies files without an explicit `- [x] Accept`. Even cleanup's apply step skips unchecked entries.
- **Archive, never delete.** Matches the lifecycle spec — `close-interval` is structurally distinct from `forget`. Archived items remain queryable; future historian work (see related spec) depends on this.
- **Read-only by default.** The most common interaction is "scan → review → close" without any modification. Modification is the exception.
- **Trajectory-preserving.** Archived items keep their original relative path under a dated directory. Walking history backward stays possible.
- **Bounded scope.** Covers MEMORY.md + topic files + skill metadata. Not code, not test files, not CI artifacts. Plans/sprint-results moved to dev-dashboard. Converge moved to its own skill.
- **Single skill entry point.** This skill. Eight tools. Three hooks. Deferred-loaded. Periodic invocation.

## What This Kit Is NOT

- Not a general memory consolidator (no merge, no promote, no compact — those need their own design; see the lifecycle spec)
- Not autonomous (operator approval is structural for any file modification)
- Not always-active (single skill, deferred-loaded, periodic)
- Not the historian (pattern-aware un-archive is a future agent; see related spec)
- Not the converge synthesizer (extracted to `/converge` skill 2026-05-13)
- Not the dev-dashboard (plan-status + sprint-distill moved to `.claude/scripts/dev-dashboard/`)

## Related

- `.claude/skills/converge/SKILL.md` — synthesis layer; consumes memory-kit reports
- `genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md` — lifecycle primitives (the design language)
- `genesis/docs/superpowers/specs/2026-05-13-historian-and-epic-timeline.md` — the three-perspective frame this kit sits inside; future historian role
- `genesis/docs/content/elohim-protocol/living_memory/epic.md` — narrative on what living memory means in the protocol
- `.claude/hooks/pre-tool-memory.py` — the PreToolUse memory injector
- `.claude/hooks/load-project-context.py` — the SessionStart context loader (complementary, broader scope)
- `.claude/memory-kit/<YYYY-MM-DD>/` — dated outputs (operator review surface)
- `.claude/archive/<YYYY-MM-DD>/` — cleanup destination (preserves trajectory; future historian indexes this)
