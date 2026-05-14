---
decided: 2026-05-13
purpose: orient contributors to the project's memory system — storage, hygiene tools, hooks, subagents, and the three-perspective frame they serve
---

# Memory System

This is the navigational map for the project's memory tooling. The system is structured around **three temporal perspectives** on a single substrate (the epic-graph over time), each served by a dedicated subagent and toolkit.

| Perspective | Subagent | Toolkit | Purpose |
|---|---|---|---|
| **Past** | [`historian`](../../agents/historian.md) | MemPalace MCP (wired) + archive walks + git log | Surface precedent/risk patterns from mined corpus (shifts/memory/plans/elohim-protocol wings) + archive + epic git history into present work |
| **Present** | [`librarian`](../../agents/librarian.md) | this directory's scripts + MemPalace MCP curate-grade | Tend MEMORY.md, CLAUDE.md surfaces, skill catalog. `mempalace_sync` complements `cleanup-scan`; `mempalace_check_duplicate` replaces TF-IDF dedupe approximation. |
| **Future** | [`cartographer`](../../agents/cartographer.md) | `/converge` skill + scripts at `.claude/scripts/converge/` | Synthesize memkit reports into a ranked next-actions menu, hand off to `/shift` |

All three are Opus-tier subagents. The mechanical work (running scripts, parsing reports) is cheap; the orchestration + judgment is what they encode.

### Meaning axis (orthogonal to time)

A fourth subagent — the **storyteller** ([`.claude/agents/storyteller.md`](../../agents/storyteller.md)) — operates orthogonally to the three temporal perspectives. They don't tend a slice of time; they decide which memory artifacts graduate to canonical story, which are memorialized in deep archive (Isildur's-diary tier), and which are held for later. They own [`genesis/data/stories/`](../../../genesis/data/stories/) as the catalog of canonical human stories that compose with humans, devices, epics, and Gherkin scenarios. See [`project_forgetting_as_design.md`](../../memory/project_forgetting_as_design.md) for the principle.

In memory sprints, the storyteller joins as a third parallel voice alongside historian and librarian, producing a graduate/memorialize/hold disposition triage that the cartographer can fold into the next-actions menu.

## Architecture

```
storage tier        .claude/memory/                      ← primary (in repo, git-tracked, PVC-recoverable)
                    .claude-config/.../memory  →  symlink to primary

scripts (this dir)  cleanup-{scan,apply}.py              ← archive stale specs/plans/memory
                    path-update-{scan,apply}.py          ← propagate renames into stale citations
                    dedupe-memory-scan.py                ← surface merge candidates (TF-IDF)
                    memory-review.py                     ← MEMORY.md size/drift/growth/types
                    skill-audit.py                       ← always-loaded skill descriptions
                    claude-md-audit.py                   ← CLAUDE.md drift + fit + missing + opted-out
                    _lib/                                ← shared helpers (paths, store, frontmatter, drift_score)

hooks               .claude/hooks/pre-tool-memory.py     ← PreToolUse * — injects MEMORY.md across subagents
                    .claude/hooks/claude-md-drift-signal.py     ← PostToolUse Edit/Write — counters
                    .claude/hooks/claude-md-structural-signal.py ← PostToolUse Bash — mv/cp/rm signal

skills              .claude/skills/memory-kit/SKILL.md   ← user-facing toolkit doc
                    .claude/skills/converge/SKILL.md     ← future-projection synthesis

subagents           .claude/agents/librarian.md          ← present-tending (operates this dir)
                    .claude/agents/historian.md          ← past-surface (operates archive + epic git)
                    .claude/agents/cartographer.md       ← future-projection (operates /converge)

reports / state     .claude/memory-kit/<YYYY-MM-DD>/     ← dated reports (operator review surface)
                    .claude/memory-kit/claude-md-drift.json ← signal accumulator state
                    .claude/archive/<YYYY-MM-DD>/        ← cleanup destinations (preserves trajectory)
```

## Operating principles

**Memory in repo is team-shareable and PVC-recoverable.** Primary lives at `.claude/memory/` (git-tracked). Personal observations could optionally stay at `.claude-config/`, but the corpus is overwhelmingly project knowledge. Recovery from a fresh PVC: `git clone` + recreate the symlink.

**Signal-driven ceremonies, not fixed cadence.** Hooks accumulate cheap counters; the audit ceremony runs only when `drift_score ≥ threshold` for a given CLAUDE.md (or when the operator invokes it). This mirrors the EPR feedback pattern (`signal_kind` vocabulary → threshold → mandatory review).

**Trust-compute gradient.** Cheap accumulators in hot paths (PostToolUse hooks: single-digit ms). Expensive ceremony only at operator invocation. Heavier-impact signals (structural ops like mv/cp/rm) weight ~6× direct edits. Re-tunable in `_lib/drift_score.py` without changing the protocol.

**Counters are source of truth.** `drift_score` is derived. Hooks update it lazily; the audit recomputes live from counters. No risk of stored-score drift.

**Opt-out markers preserve decision chains.** `.no-claude.md` in a directory excludes it from MISSING-CLAUDE-MD candidacy. Frontmatter (`decided`, `revisit-if`) + markdown body capture rationale. Audit surfaces these in their own section so surrounding-doc updates can reference what's been considered.

**Three-perspective separation.** Librarian, historian, cartographer are peers, not nested. Each owns its temporal slice. The operator (or a higher-level orchestrator) decides which to invoke when.

## When to invoke what

| Operator question | Invoke |
|---|---|
| "Run a memory hygiene pass" | `librarian` (or `/memory-kit`) |
| "Is memory healthy?" | `librarian` for a quick `memory-review.py` summary |
| "What's next?" | `cartographer` (or `/converge`) — assumes recent memkit reports exist |
| "Pre-shift readiness check" | `librarian` for hygiene, then `cartographer` for objective selection |
| "I'm about to do X; anything from history?" | `historian` |
| "This caching bug feels familiar" | `historian` |
| "Are CLAUDE.md files drifting?" | `librarian` runs `claude-md-audit.py` |
| "Audit found false positives" | `librarian` triages, places `.no-claude.md` markers |

## Workflow — weekly hygiene + synthesis (~25 min)

1. `librarian` invoked → runs `memory-review.py` first (baseline)
2. `librarian` checks drift store, decides scope (light vs full)
3. Full pass: `cleanup-scan.py` → judgment subagent → `cleanup-apply.py`
4. Then: `path-update-scan/apply.py`
5. Then (monthly): `dedupe-memory-scan.py`, `skill-audit.py`, `claude-md-audit.py`
6. Reports land in `.claude/memory-kit/<today>/`
7. `cartographer` invoked → reads memkit reports → runs `converge-scan.py`
8. Cartographer's synthesis subagent produces per-theme proposals + `next-actions.md`
9. Operator reads `next-actions.md`, picks recommendation, invokes `/shift` or `/deliver`

## Shared helpers (`_lib/`)

Pure-stdlib modules used by scripts AND hooks. Bootstrap pattern: walk up from `__file__` looking for `.claude/scripts/_lib`. See `_lib/__init__.py` for the snippet to copy. Discipline: extract only when 3+ callers share a pattern.

| Module | Use |
|---|---|
| `_lib.paths` | `repo_root_from_file`, `reports_root`, `reports_dir_for_today`, `memory_dir` |
| `_lib.store` | Best-effort JSON load/save with safe defaults — for accumulator state |
| `_lib.frontmatter` | Minimal YAML-frontmatter parser for memory entries + opt-out markers |
| `_lib.drift_score` | Canonical drift-score formula (counters → score) |

## Related memory entries

These are the architectural insights the subagents internalize:

- `project_three_temporal_perspectives.md` — history/development/roadmap as views on the epic-graph
- `project_memory_in_repo_two_tier.md` — primary at `.claude/memory/`, personal slot via symlink
- `.claude/memory/project_signal_driven_audit_ceremonies.md` — accumulator + ceremony pattern (mirrors EPR feedback)
- `.claude/memory/project_shared_lib_pattern.md` — `_lib/` extraction discipline
- `.claude/memory/project_no_claude_md_opt_out_pattern.md` — operator-rationale markers
- `.claude/memory/project_historian_pattern_surface_agent.md` — past-surface role
- `.claude/memory/project_wisdom_resolves_into_epics.md` — memory's destination is story-compaction
- `.claude/memory/reference_mempalace.md` — proposed substrate for historian

## Specs

- `genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md` — lifecycle primitives (`promote`, `compact`, `merge`, `submerge`/`surface`, `close-interval`, `memorialize`, `forget`, `quarantine`)
- `genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md` — converge design rationale + end-state vision
- `genesis/docs/superpowers/specs/2026-05-13-historian-and-epic-timeline.md` — historian role + epic-graph-as-timeline + MemPalace substrate proposal

## What this system is NOT

- Not a general memory consolidator (no `merge`, `promote`, `compact` primitives implemented — those need their own design)
- Not autonomous — operator approval is structural for any file modification
- Not always-active — skills are deferred-loaded; subagents dispatched on demand
- Not the historian's substrate (MemPalace pilot is future work)
- Not the auto-memory replacement — it complements Claude's native chat-side memory
