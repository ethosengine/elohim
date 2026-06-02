---
description: Kick off an agentic developer shift — interactive Objective authoring, pre-shift readiness check, iteration loop, sprint result on close.
---

# /shift

Invokes the `agentic-developer` skill to run an agentic developer shift.

## Usage

- `/shift` — interactive kickoff (author Objective live, compose palette, start iteration)
- `/shift resume <shift-id>` — *(v2, not yet implemented)* resume a bailed shift after operator answers the bail question

## What it does

1. Interviews the user for the Objective (name, measure command,
   baseline, scope, budget, and whether the shift is
   visual-delivery-gated).
2. **FRONT fire point (born-linked + born-oriented discovery).**
   Surfaces prior canonical seed(s) + history watch-outs for the
   Objective via the lexical `spec-coherence-index.py --query` floor
   plus a JIT-scoped MemPalace semantic recall, so the shift is born
   linked to its seed and warned of known anti-patterns. **Then orients
   the Objective on the two standing maps** (additively — same FRONT
   fire), each a plain-text read folded into the shift journal's opening
   context:
   - **MAP-PATH** — reads
     `genesis/docs/content/elohim-protocol/architecture/MAP.md` and names
     the concern-**domain D#** (Section 1) the Objective lives in
     (*"this shift works in domain D#"*), its owning architecture
     seed(s), the **pillar reading order** (Section 2's walk —
     default to the Household Living Core path for care/recovery/memory
     work), and any **Gap Ledger** row (Section 3) it collides with. MAP
     is the walk over INDEX's graph; orient on MAP.
   - **ROADMAP-PRIORITY** — reads
     `genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md`
     and names where the Objective sits in vision × readiness: a ranked
     **Sprint-N** (§1, with its readiness verdict), the **verification
     track** (§2 — if the Objective is CLAIMED-ONLY/built-but-unverified,
     the shift's job is *verify*, not rebuild), **BLOCKED-BY-ENV** (§3 —
     a HELD Objective needing harbor/alpha/shem must NOT be picked; the
     readiness check in step 5 will also gate it), or **vision-deferred**
     (network-scale breadth ranked DOWN of the single-household seed).
   - **CAPTURE-COMPLEMENTARY** — a shift executes *one* Objective. When
     iteration surfaces adjacent/supportive work (a dependency, a fix it
     would benefit from, a neighboring gap), do NOT widen the Objective
     (scope-bloat is how a shift becomes a dump) and do NOT drop it (a
     dropped discovery is a dump): capture a one-line item to
     `genesis/data/timeline/backlog/`, linked to its domain D# + roadmap
     rung, so it queues as a future shift. The executed shift stays
     genuine; the complementary work plays nice as the next roadmap entry.
   - **Staleness guard:** both maps regenerate each ceremony, not live —
     if the roadmap prose disagrees with `placement-audit.py --ledger` /
     `--focus`, trust the audit numbers and say so. Never let a stale
     ranking authorize rebuilding verified work or picking a HELD
     Objective.

   All four lenses (lexical, semantic, MAP-PATH, ROADMAP-PRIORITY) trace
   to the compaction-loop spec
   (`genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md`,
   §4): legibility (MAP) and prioritization (ROADMAP) are promoted into
   the same FRONT-fire discovery the prior-art lenses already run.
3. Composes a shift id, writes Objective YAML to
   `.claude/shifts/<shift-id>.objective.yaml`, writes initial journal
   to `.claude/shifts/<shift-id>.journal.md`.
4. Pattern-matches the predicted command palette against current
   allowlists; proposes shift-scoped additions to
   `.claude/settings.local.json` for user approval.
5. Runs `pnpm run agentic:readiness -- --objective <path>`. Aborts
   on any readiness failure with a report.
6. Enters the iteration loop, using `ScheduleWakeup` to pace between
   iterations until done, bail, or budget exhaustion.
7. On terminal state, writes a sprint result markdown at
   `.claude/shifts/<shift-id>.journal.md` and prints its path.
8. **BACK fire point (decompose-self, on concluded work).** Runs
   `decompose-self` as the shift's closing act: the concluded plan(s)
   dissolve to zero residue — durable gotchas/anti-patterns route to
   canonical specs (inline watch-out) + curated history, open issues
   to backlog, narration body to git — then MemPalace re-mines the
   cleaned surface (spec §5). No parked plan is left behind.

## See also

- Skill: `.claude/skills/agentic-developer/SKILL.md`
- Spec: `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md`
- Compaction-loop spec (the FRONT/BACK fire points this command realizes):
  `genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md`
- MAP-PATH (legibility — the developer's walk):
  `genesis/docs/content/elohim-protocol/architecture/MAP.md` (INDEX.md = the graph)
- ROADMAP-PRIORITY (prioritization — vision × readiness sprints):
  `genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md`
- Templates: `genesis/docs/shifts/`, `genesis/docs/retrospectives/TEMPLATE.md`

## Loading the skill

Use the `Skill` tool with `skill: agentic-developer`.
