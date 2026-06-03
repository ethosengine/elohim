---
decided: 2026-05-13
purpose: orient contributors to the project's memory system — storage, hygiene tools, hooks, subagents, and the three-perspective frame they serve
---

# Memory System

This is the navigational map for the project's memory tooling. The system is structured around **three temporal perspectives** on a single substrate (the epic-graph over time), each served by a dedicated subagent and toolkit.

## ⚖ Pair-off boundary — native memory is Claude's; this discipline governs the ARTIFACTS (2026-06-03)

`.claude/memory/` is Claude's **native** memory store (symlinked to the harness memory dir). It is now
**Claude-self-managed and LIGHT** — saved per Claude's own native spec (only non-obvious facts useful across
sessions), NOT a curation target for the heavy discipline below, and it must not accumulate durable knowledge.

This kit's discipline (cleanup · dedupe · comet · cites · coherence · the `MEMORY.md` budget · the four-perspective
team) governs the **MANAGED ARTIFACTS** — `genesis/docs/` · specs · plans · `genesis/data/stories/` · the CLAUDE.md
gospel surfaces — routed by `.claude/subject-routing.yaml`. That is where durable architecture/vision/wisdom and
stable how-to-work rules live, content-addressed and scope-aware.

**Durable knowledge GRADUATES to the managed artifacts; it does not pile up in native memory.** The 2026-06-03
pair-off drained 260 dumped entries → ~19 light ones (architecture/vision → the protocol corpus; stable rules →
specific agent/skill/CLAUDE.md homes; duplicates/stale dropped). Going forward, route a durable fact to its managed
home (per subject-routing), not to a new memory entry. Where the table below says "tend `MEMORY.md`," read it as
"tend the artifact layer and keep native memory light."

| Perspective | Subagent | Toolkit | Purpose |
|---|---|---|---|
| **Past** | [`historian`](../../agents/historian.md) | MemPalace MCP (wired) + archive walks + git log | Surface precedent/risk patterns from mined corpus (shifts/memory/plans/elohim-protocol wings) + archive + epic git history into present work |
| **Present** | [`librarian`](../../agents/librarian.md) | this directory's scripts + MemPalace MCP curate-grade | Tend MEMORY.md, CLAUDE.md surfaces, skill catalog. `mempalace_sync` complements `cleanup-scan`; `mempalace_check_duplicate` replaces TF-IDF dedupe approximation. |
| **Future** | [`cartographer`](../../agents/cartographer.md) | `/converge` skill + scripts at `.claude/scripts/converge/` | Synthesize memkit reports into a ranked next-actions menu, hand off to `/shift` |

All three are Opus-tier subagents. The mechanical work (running scripts, parsing reports) is cheap; the orchestration + judgment is what they encode.

### Meaning axis (orthogonal to time)

A fourth subagent — the **storyteller** ([`.claude/agents/storyteller.md`](../../agents/storyteller.md)) — operates orthogonally to the three temporal perspectives. They don't tend a slice of time; they decide which memory artifacts graduate to canonical story, which are memorialized in deep archive (Isildur's-diary tier), and which are held for later. They own [`genesis/data/stories/`](../../../genesis/data/stories/) as the catalog of canonical human stories that compose with humans, devices, epics, and Gherkin scenarios. See [`project_forgetting_as_design.md`](../../memory/project_forgetting_as_design.md) for the principle.

During disposition triage (the librarian's hygiene-sweep hands archive candidates to the storyteller — see the storyteller agent's "Disposition triage" section), the storyteller produces a graduate/memorialize/hold decision that the cartographer can fold into the next-actions menu.

## State & Coherence Tooling — the deterministic memory-stasis kit (2026-06)

A second toolkit sits alongside the hygiene scripts: a **deterministic (no-LLM) state machine** over the doc +
memory surfaces, governed by the placement contract at **`genesis/docs/PLACEMENT.md`**. It exists so every file
is instantly auditable for *position + state*, and so the surface trends toward **stasis** — equilibrium:
pressure dirs empty, no dumps, every artifact has a next state.

| Tool | What it gives you |
|---|---|
| `placement-audit.py` | scoreboard + `--ledger` (the BUDGET: every file → position + state + next-action) + `--focus` (currently-testable surface from cluster-state) + `--headline` (the SessionStart budget line) + structural anti-dump check |
| `spec-coherence-index.py [--query "<topic>"]` | deterministic prior-art index (token overlap) → "have we spec'd this?" so you compose from canonical instead of re-speccing |
| `decompose.py <doc>` | spec/plan → bounded, cited gap-items (OPEN = implement, CLAIMED = verify) — the gap budget |
| `state-machine-gen.py` | regenerates the `genesis/docs/_state/<state>/` pressure-dir CLAUDE.md gates (blockers / regression / unverified / needs-triage) |
| `prep-brainstorm.py "<topic>"` | the `/brainstorm` pre-step preload (prior-art + focus + budget) |
| `genesis/manifests/cluster-state.yaml` | the env-reality input — flip a node/cluster/registry → `--focus` scope cascades |

The lifecycle, the four verification states (VERIFIED-STABLE / CLAIMED-ONLY / REGRESSED / BLOCKED-BY-ENV), the
feedback graph (regression *warms* a doc; env-unavailable *holds* it), and the home taxonomy (CANONICAL /
HISTORY / ACTIVE + pressure dirs) all live in `genesis/docs/PLACEMENT.md`. Read it once.

### Gotchas (hard-won — do not relearn the expensive way)

- **A "landed" / checkbox / ✅ claim is NOT done.** Checkboxes lie (the iroh delivery-master showed every gate
  ✅ while CI held landing commits for *unrun soaks*). "Done" is EARNED by the verification gate
  (ci-investigator / CI / soak), never self-asserted. Treat `status: landed`-without-evidence as CLAIMED.
- **Env-unavailable ≠ regression.** A node/cluster/registry down is `BLOCKED-BY-ENV` — *held, not broken* (a
  partial cluster is the steady state). It must NOT fire a regression cascade. Only *red on an available env*
  is a regression. Update `cluster-state.yaml`; scope cascades; no false alarm.
- **No dumping grounds, anywhere.** Runaway docs, runaway directory trees (no `blocked-by-env/shem/…` nesting —
  reason goes inline), and growing archives are all debt. `_retired/` holds ONLY verified-stable work. The
  pressure dirs are MEANT to be empty.
- **Don't archive trash.** mem-palace ingestion is a deliberate graduation act (landed-canonical +
  distilled-history only); raw/abandoned/superseded is never embedded. Coherence comes from the cheap
  deterministic floor, not from mining everything.
- **Compose, don't fork.** Run `spec-coherence-index.py --query` before proposing — extend canonical; revive
  nothing superseded (read its history gotcha first).
- **Fix deployed ≠ fix converged.** When a ceremony deploys a substrate fix, the immediate post-deploy counter
  shows the cascade unmasked — not convergence. Record it as the *starting baseline*. The next ceremony's
  same-counter reading is the convergence evidence. One-cycle observation is insufficient; two cycles = minimum
  proof. Applies to any drift counter (claude-md, cleanup-scan, dedupe clusters).
- **Re-indexing orphans grows MEMORY.md.** After tightening entries AND re-indexing orphans, expect the file to
  grow — orphans are load-bearing, just invisible. Tightening is bounded by entry-shape floor (~100–150 chars).
  Real compression comes from: umbrella entries (fold related items under one link), graduation to stories
  (one story carries 6+ entries), or archive-with-pointer. Per-entry tightening is a tiny correction, not a
  budget path.

### Ceremony discipline — balance sheet

Run `genesis/scripts/memory-balance.sh` at **Wave 0** (baseline) and **Wave 6** (close) of every memory ceremony.
Snapshots persist to `balance-sheets/<ts>.{json,txt}` and auto-diff against the prior run. The **Surface:Archive
ratio** is the smoking-gun metric — healthy: trending <100:1; runaway: flat archive across multiple ceremonies.
Paste the delta into the chronicle entry as the ceremony's evidence. Healthy ceremony targets: MEMORY.md ≤ 24KB;
≥1 canonical story; 0 memorialize-archive orphans missing `story_pointer`.

### Your mandate (broad — you decide the how)

You own a slice of this surface (past / present / future / meaning). **Use these tools to drive your domain
toward memory stasis** — fewer no-status orphans, fewer unlinked memory entries, claims verified or moved to
regression, dead paths distilled to history, pressure dirs empty. The budget (`placement-audit.py --ledger`)
is your scoreboard; the debt numbers must fall. *How* you get there is your judgment — these are instruments,
not a script.

## Architecture

```
storage tier        .claude/memory/                      ← primary (in repo, git-tracked, PVC-recoverable)
                    .claude-config/.../memory  →  symlink to primary

scripts (this dir)  cleanup-{scan,apply}.py              ← archive stale specs/plans/memory
                    path-update-scan.py                  ← propagate renames into stale citations
                    path-update-apply.py                 ← apply approved replacements
                    dedupe-memory-scan.py                ← surface merge candidates (TF-IDF)
                    memory-review.py                     ← MEMORY.md size/drift/growth/types
                    skill-audit.py                       ← always-loaded skill descriptions
                    claude-md-audit.py                   ← CLAUDE.md drift + fit + missing + opted-out
                    memory-coherence-audit.py            ← memory↔code cites: edge; DEAD-CITE/CITE-CANDIDATE; builds cites-index
                    _lib/                                ← shared helpers (paths, store, frontmatter, drift_score)

hooks               .claude/hooks/pre-tool-memory.py     ← PreToolUse * — injects MEMORY.md across subagents
                    .claude/hooks/claude-md-drift-signal.py     ← PostToolUse Edit/Write — counters
                    .claude/hooks/claude-md-structural-signal.py ← PostToolUse Bash — mv/cp/rm signal
                    .claude/hooks/memory-coherence-signal.py    ← PostToolUse Edit/Write — bumps a memory entry when edited code matches its cites:

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
4. Then: `path-update-scan.py` → `path-update-apply.py`
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

- `genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md` — lifecycle primitives (`promote`, `compact`, `merge`, `submerge`/`surface`, `close-interval`, `memorialize`, `forget`, `quarantine`)
- `genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md` — converge design rationale + end-state vision
- `genesis/docs/superpowers/specs/2026-05-13-historian-and-epic-timeline.md` — historian role + epic-graph-as-timeline + MemPalace substrate proposal

## What this system is NOT

- Not a general memory consolidator (no `merge`, `promote`, `compact` primitives implemented — those need their own design)
- Not autonomous — operator approval is structural for any file modification
- Not always-active — skills are deferred-loaded; subagents dispatched on demand
- Not the historian's substrate (MemPalace pilot is future work)
- Not the auto-memory replacement — it complements Claude's native chat-side memory
