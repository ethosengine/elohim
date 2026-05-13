---
title: Historian, Epic-Graph Timeline, and the Three Temporal Perspectives
date: 2026-05-13
status: draft
related:
  - genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md
  - genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md
  - .claude/skills/memory-kit/SKILL.md
  - .claude/skills/converge/SKILL.md
  - https://github.com/mempalace/mempalace (proposed substrate for historian)
---

# Historian, Epic-Graph Timeline, and the Three Temporal Perspectives

## Context

While realigning `memory-kit` against the Pawel Huryn article ("How I Finally Sorted My Claude Code Memory"), three architectural insights surfaced that don't fit inside the kit but reshape its purpose. The kit serves the present. The present is one of three temporal perspectives, all looking at the same underlying timeline. Naming that frame — and the historian role that animates it — completes the lifecycle spec's `surface` primitive and tells us where memory ultimately resolves.

This spec captures the frame so future work can build against it. It does not propose immediate implementation — it follows the memory-kit realignment landing first.

## The Three Temporal Perspectives

> *History is perspective on the timeline of the past. Roadmap is the perspective of the timeline of epics planned / wip for the future. Development is the cycle on which we work to change the present to achieve the vision of a future of human thriving.*

Each perspective is a view on a single substrate: **the epic-graph over time**.

| Perspective | Direction | Substrate slice | Primary actor |
|---|---|---|---|
| **History** | past-facing | git history of `genesis/docs/content/elohim-protocol/` + `.claude/archive/` | Historian (new role) |
| **Development** | present-tense cycle | active plans, sprint-results, memory-kit reports | memory-kit, converge, `/shift`, `/deliver` |
| **Roadmap** | future-facing | planned epics, work-in-progress epics, the next-actions menu | converge (today), roadmap-walker (future) |

The triad is not three systems. It is three viewpoints on one substrate. Development is the cycle that converts roadmap-perspective items into history-perspective entries. Memory-kit, converge, and the proposed historian all serve different perspectives but read from the same source-of-truth: the epic-graph and its accumulated commits.

## Epic-Graph as Substrate

Each epic at `genesis/docs/content/elohim-protocol/<epic>/epic.md` is a **snapshot** of the protocol's becoming at a point in time. The git history of those files — diffs, dates, commit messages — is the actual narrative. Today we treat epics as static reference; under this frame they are commits in a meta-narrative repository.

Implications for implementation:

- `git log --follow genesis/docs/content/elohim-protocol/<epic>/epic.md` is a first-class data source for any tool that walks the timeline
- A read-only `epic-timeline` tool can produce a chronological reading of the protocol's evolution by walking those histories
- Linking between epics (cross-references in epic body text) becomes an actual graph that historian + roadmap-walker can traverse
- The "next epic" is not authored fresh — it is the natural continuation of the trajectory, computable from where the existing graph is heading

## Historian — The Surface Agent

Cleanup archives stale memory. **Historian is the inverse motion: pattern-aware un-archive.** It performs the `surface` primitive named in the memory lifecycle spec but underdeveloped there.

### Operational shape

1. **Archive indexing.** Cleanup writes to `.claude/archive/<date>/`. Historian periodically extracts pattern signatures from archived entries — failure shapes, success shapes, decision frames, recurring stack traces, environmental triggers.

2. **Pattern signature schema.** Each archived entry produces a signature with:
   - Trigger conditions (what state of the system preceded the entry)
   - Outcome class (success / failure / abandonment / pivot)
   - Decision frame (the choice that was at stake)
   - Adjacent themes (vocabulary co-occurring in the entry)

3. **Continuous comparison.** During planning (pre-`/shift`) and execution (sense-and-respond), historian compares the current trajectory's state against indexed signatures.

4. **Surface annotation.** On match, historian emits an inline annotation into the active plan or sprint context: *"This is shaped like ARCHIVE entry X, which went Y."* Risk if Y was failure; precedent if Y was success.

5. **Annotation is context, not notification.** Historian's output enters live planning as background context that pivots decisions — it is not an interrupt or a popup. The user sees it as part of the plan's rendered shape.

### Distinction from cleanup

| Cleanup | Historian |
|---|---|
| Lateral motion (archive stale) | Vertical motion (re-surface relevant) |
| Operator-gated, weekly | Continuous, sensor-pattern |
| Modifies file locations | Read-only on archive; writes annotations into active context |
| Reduces present-tense noise | Increases present-tense relevance |

### When to build

After memory-kit realignment lands. Historian needs:
- Stable archive directory structure (cleanup produces it today)
- A pattern signature schema (new design — needs spec extension)
- Integration points in `/shift` pre-flight and (eventually) the sense-and-respond execution layer

## Wisdom-Compaction Direction (Vertical Lifecycle)

The user's observation: *memory should resolve into stories — not as imagination but as compaction of detail into meaning.* Lord-of-the-Rings doesn't expose subtext; the richness becomes the meaning-shape that carries forward.

This adds a **vertical** direction to the lifecycle spec. The existing primitives (`promote`, `compact`, `merge`, `submerge`/`surface`, `close-interval`, `memorialize`, `forget`, `quarantine`) describe *lateral* flow between tiers. The vertical direction is:

```
ephemeral chat
    ↓ (auto-memory promotes)
project memory entries
    ↓ (converge clusters; future: promote-to-epic compacts)
epic paragraphs
    ↓ (epic graduation; future spec)
manifesto principles
```

Wisdom is the asymptote each tier reaches toward. Class #7 of `[Memory classes]` is not a class alongside the others — it is the receiving stratum that the others compact into.

### Future primitive: `promote-to-epic`

Distinct from cleanup's archive motion. Inputs: a cluster of related memory entries (or sprint-results, or sprint open-questions). Output: a paragraph drafted into the relevant epic's `epic.md`, with the source entries marked as compacted-into-epic. The motion is **lossy by design** — compaction discards detail to preserve meaning-shape.

Out of scope for this spec; needs its own design pass. Listed here to name the vertical direction.

## Proposed Substrate — MemPalace

**Project**: https://github.com/mempalace/mempalace (memory entry: `reference_mempalace.md`)

MemPalace is a Python CLI + library + MCP server that provides verbatim-text storage with semantic search and a temporal entity-relationship graph. Its architecture maps near 1:1 onto the historian role's requirements:

| Historian requirement | MemPalace primitive |
|---|---|
| Per-agent diary (matches `[Elohim as specialist subagents]`) | Wings — each agent gets its own scoping container |
| Pattern signature schema (Open Q #1) | Vector embeddings (ChromaDB default; pluggable backend) |
| Continuous comparison cadence (Open Q #2) | Auto-save hooks fire at Claude Code compaction boundaries; `wake-up` for on-demand context |
| Archive indexing | `mempalace sweep` — idempotent, resume-safe, one drawer per message pair |
| Epic-graph traversal | SQLite temporal entity-relationship graph with validity windows |
| Surface annotation | `mempalace search` over historian's wing returns candidate precedents to inject into active plan |

**Performance**: 96.6% R@5 on LongMemEval raw; 98.4% with hybrid pipelines. Local inference by default.

### What's resolved by adopting MemPalace

- Pattern signature schema (Open Q #1) — embeddings replace the hand-designed signature
- Historian cadence (Open Q #2) — auto-save + on-demand `wake-up` replaces a custom polling loop
- Per-agent scoping — wings cover the elohim-specialist pattern without new infrastructure

### What MemPalace does NOT decide

- **Storage location**. By default likely `~/.mempalace/`; we need to choose deliberately given we already have `.claude-config/` and `genesis/docs/`-tracked content. Memory data isn't versioned content, so probably a third location.
- **Embedding model**. The "nothing leaves your machine" property depends on running a local model. Need to verify the local-inference path before committing.
- **Integration with existing memory-kit**. MemPalace belongs to the **history perspective**. Memory-kit stays where it is (development-present, stdlib + markdown). Do NOT absorb memory-kit into MemPalace.
- **Integration with `.claude/archive/`**. Cleanup writes to `.claude/archive/<date>/`. We need an adapter that runs `mempalace sweep` on the archive periodically — likely a follow-up of cleanup-apply, or its own dated cron.

### Why not absorb memory-kit into MemPalace

Memory-kit operates on the present-slice: MEMORY.md and topic files must stay human-readable, git-trackable, conflict-resolvable. ChromaDB's binary store is wrong for that role. The two systems serve different perspectives on the temporal triad and should remain distinct.

### Pilot before commitment

Before integration, run a small pilot:
1. Install mempalace locally with a non-default storage location
2. Create a `historian` wing
3. `mempalace sweep` against `.claude/archive/` (currently small but growing)
4. Run a sample pattern search against an active plan — does the recall surface anything useful?
5. If yes, decide on integration shape (CLI calls, MCP tools, or hook integration). If no, the substrate isn't earning its weight yet.

The pilot is a separate sprint. This spec names mempalace as the proposed substrate but does not commit to adoption.

## Relationship to Existing Tooling

| Existing | Perspective served | Stays | Future relationship |
|---|---|---|---|
| `memory-kit` | development (present) | yes | unchanged after realignment lands |
| `converge` | development → roadmap bridge | yes | feeds roadmap-walker eventually |
| `cleanup` | development (archives to past) | yes | feeds historian's archive index |
| `dedupe-memory` | development | yes | output remains advisory |
| `path-update` | development | yes | independent |
| `skill-audit` | development (context budget) | yes | independent |
| `memory-review` (new) | development | yes | reports lifecycle health |

| Proposed | Perspective served | Status |
|---|---|---|
| `historian` | history (past → present surface) | future — needs pattern-signature schema |
| `epic-timeline` | history (reading-the-protocol-arc) | future — read-only walk of epic-graph git history |
| `roadmap-walker` | roadmap (future) | future — projects next epic from trajectory |
| `promote-to-epic` | wisdom-compaction (vertical) | future — needs its own spec |

## Open Questions

1. ~~**Pattern signature schema.**~~ **Resolved by MemPalace** — vector embeddings replace the hand-designed signature. Embedding model choice (local vs hosted) remains open.

2. ~~**Historian cadence.**~~ **Resolved by MemPalace** — auto-save hooks fire at Claude Code compaction boundaries; `wake-up` provides on-demand context. Custom polling loop unneeded.

3. **Epic-graph traversal model.** Read the epic.md git history as plain commit walk, or build a derived graph with semantic linking parsed from epic body text? MemPalace's temporal graph supports the latter; commit walk is a simpler floor. Still requires decision.

4. **`promote-to-epic` operator gating.** How aggressive should the system be about proposing wisdom-compaction? The memory-damage safeguards in converge already encode the conservative answer for *merge*; promote-to-epic is structurally similar but with a different destination. Orthogonal to substrate choice.

5. **Roadmap-walker as separate tool or natural extension of converge?** Today converge produces a next-actions menu (the present-to-roadmap bridge). Walking the future further (multiple epics ahead) is a longer projection — possibly a different tool, possibly converge with a `--horizon` flag. Orthogonal to substrate choice.

6. **MemPalace storage location.** By default likely `~/.mempalace/`. We need a deliberate choice — memory data isn't versioned content (rules out `genesis/`), isn't harness-managed (rules out `.claude-config/` proper), and shouldn't pollute the project tree. Suggestion: `/projects/.mempalace/` parallel to `.claude-config/`.

7. **Embedding model verification.** Confirm MemPalace's local-inference path actually runs offline. The "nothing leaves your machine" property is foundational for the protocol's values; verify before adopting.

8. **Pilot scope.** What's the smallest pilot that proves historian's recall is real? Suggestion: sweep `.claude/archive/`, create a sample plan that intentionally rhymes with an archived failure, see whether mempalace search surfaces it without prompting.

## Out of Scope

- Implementation. This spec is a frame, not a build plan. Each proposed tool needs its own design pass.
- Modifying epic content. The epic-graph is read-as-source-of-truth; this spec doesn't propose epic-authoring automation.
- Replacing converge or memory-kit. Both stay; this spec gives them context.
- Auto-memory (Claude's harness-managed chat-side memory). Out of scope; we complement, we don't reach into it.

## Memory Pointers

- `project_three_temporal_perspectives.md` — the triad in user's own framing
- `project_wisdom_resolves_into_epics.md` — the vertical-lifecycle observation
- `project_historian_pattern_surface_agent.md` — the historian role
