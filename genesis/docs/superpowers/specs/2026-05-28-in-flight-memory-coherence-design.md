# In-Flight Memory Coherence — Design

**Date:** 2026-05-28
**Status:** approved (operator) — implementation in progress
**Author:** memory-coherence review (operator-directed)

## Problem

The memory team is excellent at *periodic, batch* maintenance (librarian hygiene-sweeps, the four-agent substrate-currency ceremony, `/converge`) but thin on *in-flight, continuous* coherence. Proof: the memory team's own gospel surfaces drifted for ~2 weeks (the cartographer carried a deleted "Wave 3 stasis template," every agent spoke in "Waves" the ceremony skill had renamed to "Phases") and nothing surfaced the contradiction until a manual review. The only mechanism that catches such drift is a ceremony nobody had pointed at the memory team itself.

The deeper gap: **memory is the one substrate in this repo that lacks edge-invalidation on source change.** The project already applies the reconciliation-controller pattern everywhere else —

- the build-graph (`genesis/orchestrator/graph-walker.mjs` walks `build-manifest.json` `inputs.sources` globs and marks steps stale when a changed file matches),
- code→scenario coherence (`.claude/hooks/sync-check.py` + `.claude/file-relationships.json`),
- story→scenario/epic/memory links (`genesis/data/stories/CONVENTIONS.md` frontmatter, read by `story-coverage-audit.py`),
- memory-slug→gospel-surface and gospel-surface→path (`substrate-currency-audit.py`),

— but a memory entry that says *"see `path_service.rs`"* has **no machine-walkable edge**. When `path_service.rs` moves or its contract changes, nothing re-opens the lesson. Memory→code/spec/scenario links exist today only as unstructured prose backticks (62 of 233 entries contain a code path; zero declare it structurally).

## Goal

Close the memory→code/spec/scenario edge and give it a cheap in-flight walker, so memory drift is surfaced continuously (signal-driven, into the hygiene-sweep) instead of only when a periodic ceremony runs. This is the **capture-time discipline**: writing a memory entry that leans on code means declaring the dependency, exactly as a canonical story declares `feature:` and `anchors_epics:`.

Non-goals (v1): typed/semantic edges (paths and globs only); scoped MEMORY.md injection (a future affordance the field unlocks, not built here); auto-rewriting memory; immediate hot-path reminders (chosen design accumulates into the sweep).

## Design

Three small components, each reusing an existing primitive. Pure stdlib; consistent with the trust-compute gradient (cheap accumulator in the hot path, judgment deferred to the sweep).

### C1 — the `cites:` edge (memory frontmatter convention)

Memory entries gain an optional top-level `cites:` list of repo-relative paths or globs whose change should re-open the entry for re-verification:

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

- Mirrors stories' `feature:`/`anchors_epics:` exactly — same "declare the dependency" philosophy.
- The `_lib.frontmatter` parser already handles YAML lists; no parser change needed.
- **Rollout: seed + convention, organic growth.** Seed the ~10 entries that already lead with a `**Canon (in-tree, authoritative):** <path>` pointer. Document the convention; let it grow as entries are written or touched. No big-bang backfill.

### C2 — `memory-coherence-audit.py` (the walker)

Sibling to `substrate-currency-audit.py` in `.claude/scripts/memory-kit/`. Pure stdlib, uses `_lib` (paths, frontmatter, store). Two modes:

- **audit (default):** parse every memory entry's `cites:`; check each path/glob resolves on disk (reuse the path-existence approach from `substrate-currency-audit.py`). Emit:
  - `DEAD-CITE` — a `cites:` path no longer exists (the lesson cites code that moved/was deleted).
  - `CITE-CANDIDATE` — an entry with inline code-path backticks but no `cites:` field (suggests adding the edge). Advisory; supports organic rollout.
  Writes a derived projection to `.claude/memory-kit/<date>/memory-coherence-audit.{json,md}` (the established single-writer report pattern).
  Also **rebuilds the cites-index** at `.claude/memory-kit/cites-index.json` (`{glob_or_path: [slug, ...]}`) consumed by the hook.
- **changed-files (`--changed -` reads paths from stdin, or `--since <git-ref>`):** glob-match the changed-file list against every entry's `cites:` and emit `STALE-CANDIDATE` — *"memory entry X cites Y which just changed — re-verify the lesson."* Mirrors `graph-walker.mjs`'s walk shape, applied to memory nodes. Usable from a husky pre-push consumer later.

### C3 — `memory-coherence-signal.py` (in-flight accumulator hook)

PostToolUse `Edit|Write` hook, sibling to `claude-md-drift-signal.py`. Cheap path only:

1. Read the edited `file_path` from stdin (same shape as the drift-signal hook).
2. Load the cached `cites-index.json` (small; absent → no-op, graceful degradation until the first audit builds it).
3. `fnmatch` the edited path against the index globs; for each matched entry, bump a per-entry counter in `.claude/memory-kit/memory-coherence-drift.json`.
4. Best-effort, never blocks. **Accumulates only — no reminder injection** (per the approved "accumulate into the sweep" choice).

Cost: load one small JSON + fnmatch one path against N globs ≈ low single-digit ms. It joins the existing 8 Edit|Write hooks; the index keeps it O(index size), not O(233 files).

### C4 — hygiene-sweep integration

The librarian surfaces *"N memory entries cite code that changed since last verified"* during `/hygiene-sweep`, reading `memory-coherence-drift.json` + the audit. Counters reset after surfacing (mirrors `claude-md-drift.json`). Registered as memory-kit tool #8 + a `/hygiene-sweep` cadence step; added to the librarian's tool table.

## Components & files

| File | Kind | Action |
|---|---|---|
| `.claude/scripts/memory-kit/memory-coherence-audit.py` | new script | C2 walker (audit + changed-files modes; builds cites-index) |
| `.claude/hooks/memory-coherence-signal.py` | new hook | C3 accumulator |
| `.claude/settings.json` | edit | register C3 under PostToolUse `Edit|Write` |
| `.claude/skills/memory-kit/SKILL.md` | edit | document tool #8 + `cites:` convention + hygiene-sweep step |
| `.claude/agents/librarian.md` | edit | add tool to table + surfacing step |
| `.claude/scripts/memory-kit/CLAUDE.md` | edit | add script to the architecture map |
| `.claude/memory/project_memory_cites_edge.md` | new memory | the convention as a discoverable project memory |
| ~10 existing memory entries | edit | seed `cites:` on Canon-pointer entries |

## Testing / acceptance

- `memory-coherence-audit.py` runs clean stdlib, produces the JSON+MD report and `cites-index.json`; `DEAD-CITE` correctly flags a deliberately-broken `cites:` path; `CITE-CANDIDATE` surfaces an un-migrated entry.
- `--changed` mode: piping a path that matches a seeded entry's `cites:` emits the `STALE-CANDIDATE` for that entry; a non-matching path emits nothing.
- `memory-coherence-signal.py`: with the index present, editing a cited path bumps the entry's counter; with the index absent, it no-ops without error; never blocks the tool call.
- Coherence: `substrate-currency-audit.py` shows 0 new path/process-status findings on the edited surfaces.

## Future affordances (not v1)

- **Scoped injection:** `cites-index.json` is exactly the index that could later let the PreToolUse hook inject *only* the memory entries whose `cites:` match the files in play, replacing the flat 40 KB MEMORY.md injection as the corpus grows.
- **Husky pre-push:** `memory-coherence-audit.py --since origin/dev` as a pre-push consumer, mirroring the orchestrator's `graph-walker.mjs --shell-lines` usage.
- **Bidirectional:** stories already declare `graduates_memory:` (story→memory); `cites:` adds memory→code. Together they make memory a fully first-class node in the repo coherence graph.
