---
name: converge
description: Synthesize the dev corpus into a ranked "what's next" menu — extracted from memory-kit so its planning-tier responsibilities (vision×readiness scoring, memorial-tier safeguards, search-bias compensation) stand on their own. Run after a `memory-kit` weekly sweep to convert backlog/dedupe/plan-status/sprint-digest signal into per-theme plan edits and a session-start handoff menu. Use when an operator asks "what's next?" or before kicking off a major `/shift`.
---

# Converge — Trajectory Synthesis + "What's Next" Menu

Extracted from `memory-kit` (May 2026). The hygiene tools (`cleanup`, `path-update`, `dedupe-memory`, `skill-audit`, `memory-review`) handle *maintenance*. This skill handles *synthesis*: turning the reports those tools produce into actionable plan edits and a ranked next-actions handoff.

**Spec**: `genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md`.

## When to invoke

- After a `memory-kit` weekly sweep, before a major `/shift` or `/deliver`
- When an operator types "what's next?" at session start (the menu surface this skill produces is the canonical answer)
- Never in isolation — `convergence-themes.md` is computed from `memory-kit` reports; running converge without fresh reports produces a stale menu

## Four phases

```bash
python3 .claude/scripts/converge/converge-scan.py     # Phase 1: deterministic theme detection
# (Phase 2 synthesis subagent dispatched — see prompt below)
python3 .claude/scripts/converge/converge-apply.py    # Phase 3: operator-approved plan edits
# Phase 4: session-start UX reads the produced next-actions.md
```

1. **Theme detection** (deterministic, `converge-scan.py`) — clusters BACKLOG items, dedupe pairs, plan-status cooling items, sprint-digest themes, and path-rename clusters by shared theme keywords (extracted from active plan filenames + filtered through stopwords + multi-source-type requirement). Outputs `.claude/memory-kit/<TODAY>/convergence-themes.md` with each theme's contributing items and a candidate canonical plan.

2. **Synthesis** (LLM judgment, subagent dispatch) — when this skill is invoked after a fresh scan, dispatch a `general-purpose` subagent (opus) with the prompt below. Produces:
   - Per-theme proposals at `.claude/memory-kit/<TODAY>/converge/<theme>-proposal.md` with structured edit blocks (mark-done, add-as-outstanding, merge-redundant, remove-obsolete, surface-question) — each operator-reviewable via `- [x] Accept` checkboxes.
   - The **next-actions menu** at `.claude/memory-kit/<TODAY>/next-actions.md` — top 3-5 ranked recommendations with vision × readiness scoring and pre-authored Objectives.

3. **Apply** (deterministic, `converge-apply.py`) — for each accepted edit, performs the plan modification. v1 supports `mark-done` (`- [ ]` → `- [x]` with evidence comment) and `add-as-outstanding` (insert new `- [ ]` after named anchor); other kinds are operator-manual edits in v1.

4. **Session-start UX** — when a human says "what's next?", read the latest `next-actions.md`, present conversationally, hand off to `/shift` or `/deliver` with the chosen pre-authored Objective. No new skill needed; this is a convention.

## Synthesis subagent prompt (Phase 2 dispatch)

```
You are the synthesis phase of /converge. Read .claude/memory-kit/<TODAY>/convergence-themes.md
to see the themes detected from the latest memory-kit dreaming pass.

For each theme above noise (skip themes with low total signal or that look like
generic vocabulary like "execution", "peer" alone — focus on substantive ones
like "iroh", "epr", "recovery", "doorway", "topology", "storage"):

PER-THEME WORK
==============

1. Read the theme's canonical plan candidate fully.
2. Read each linked BACKLOG item's source spec/plan.
3. `git log --since="60 days ago" --oneline` for the theme's repo paths.
4. Search recent sprint-results in .claude/shifts/ for theme mentions.
5. Read .claude/data/dev-intent.jsonl entries (if exists) mentioning the theme.

Produce a per-theme proposal at .claude/memory-kit/<TODAY>/converge/<theme>-proposal.md
with structured edit blocks. Each edit block has form:

  ### N. `<kind>` — <one-line description>
  **Plan**: `<canonical-plan-path>`
  **Edit**:
  ```edit
  target: <text of the existing checkbox to mark done>
  evidence: <citation: file paths, commit hashes, scenarios>
  ```
  - [ ] Accept  <!-- id: <plan-path>:<edit-id> -->

Edit kinds:
  mark-done — checkbox in plan should be checked (target = checkbox text; evidence = citation)
  add-as-outstanding — new outstanding item should be added (anchor = section heading; new_item = text)
  merge-redundant — sections to collapse (manual review in v1; describe in proposal)
  remove-obsolete — task no longer relevant (manual; describe + cite)
  surface-question — open question to add to plan (manual; cite source)

Be conservative on mark-done: only mark when the deliverable is unambiguous
(file exists at expected path; scenario passing; commit message references the task).

NEXT-ACTIONS MENU
=================

After per-theme proposals are written, produce .claude/memory-kit/<TODAY>/next-actions.md.
This is the "what's next" surface.

For each refreshed plan that's ready-to-execute, score:

VISION-ALIGNMENT (0-10) — does completing this plan advance the manifesto's design principles?

Read genesis/docs/content/elohim-protocol/manifesto.md (Part II Design Principles
1-6: Distributed Architecture / Graduated Intimacy / Values Alignment / Community
Governance / Wealth as Circulation / Living Memory). Walk each principle, mark
YES/SOMEWHAT/NO/TANGENTIAL for whether the plan's deliverables advance it.

  Score guide:
    9-10: directly advances 3+ principles substantively (substrate-level work)
    7-8:  directly advances 1-2 principles substantively (cross-cutting feature)
    5-6:  enables Principle 1 (infrastructure / tooling)
    3-4:  hygiene / operational
    1-2:  internal-tooling-only

  Substrate work multiplies. Tactical work is linear.
  Cite which principle(s) the plan advances in the reasoning.

READINESS (0-10) — how close is this plan to "an agent ships in 1-2 cycles"?

  Signals (additive):
    Worktree exists with partial work: +3
    All blockers explicitly resolved or absent: +3
    Open items well-scoped + ordered (mechanical work, not design): +2
    Recent commit activity in adjacent area: +1
    No pending operator decisions: +1
    Plan has clear next step at top of unfinished list: +1

  Subtractive:
    Has unresolved design questions in Open Questions section: -2
    Requires external dependency (review, vendor, governance): -2
    Plan has no checkbox structure (narrative-only): -3

RECOMMENDATION = vision × readiness, sorted descending.

When picking the TOP recommendation, prefer concrete gates over master plans —
master plans are too coarse to be a single Objective; recommend the next gate
the master plan unblocks. When scores tie, prefer the plan closer to "ships in
1-2 cycles" over "ships in 5 cycles, more vision impact" — closes the loop faster.

Output format (top 3-5 entries):

  ## Top recommendation: <plan name>
  - **Plan**: `<path>`
  - **Vision-alignment**: N/10 — <one-sentence reasoning citing principles>
  - **Readiness**: N/10 — <one-sentence reasoning citing signals>
  - **Pre-authored Objective**: <1-2 sentences, action-oriented, ready to drop into /shift>
  - **Estimated cycles**: 1-2 / 2-3 / 4+
  - **Recommended skill**: /shift or /deliver
  - **Blockers** (if readiness < 8): <brief list>

  ## Other ready plans
  ... (rank 2-5)

The menu is what the human reads at session start. Make every line load-bearing.
Total length: ≤80 lines for top recommendation, ≤30 lines per other entry.

When done, print: counts of themes synthesized, proposals written, plans surfaced
in the menu, and the path of next-actions.md.

MEMORY-DAMAGE SAFEGUARDS (read before proposing any mark-done, merge, or remove)
================================================================================

The biggest failure mode of this skill is *aggressive consolidation that loses gold*.
The opposite failure (hoarding) is also bad, but it's a slow leak; aggressive
consolidation is irreversible damage. When uncertain, default to PRESERVATION,
not action.

Concrete safeguards:

1. **Memorial-tier check before any mark-done or remove.** Before proposing to
   mark a checkbox done OR remove an item as obsolete, search for citations of
   that item or its theme in:
   - genesis/docs/content/elohim-protocol/ (manifesto, epics, constitution)
   - The latest two manifesto-tier MEMORY entries
   If cited there, the item is foundational-adjacent. Use OPERATOR-CALL with
   reasoning instead of auto-proposing the change. Note: "high citation in
   vision documents — operator may want to memorialize before any cleanup."

2. **Convergent-insight respect.** When dedupe-clusters surfaces same-concept
   entries from different conversations or sprints, do NOT default to "merge."
   The rediscovery itself is signal — the insight is robust enough to surface
   multiple independent times. Default action: leave both, propose only as
   OPERATOR-CALL with the framing "convergent insight — multiple independent
   reach for the same point; operator may prefer to memorialize as a principle
   rather than merge as duplication."

3. **Quiet-but-foundational surface (additive section in next-actions.md).**
   In addition to the top 3-5 ranked recommendations, include a separate
   section titled "## Quiet but load-bearing." This lists items that the
   search ranks LOW but that you (the synthesis subagent) judge as important:
   - Plans untouched in >60 days but referenced in manifesto/epics
   - Memory entries with low DF but high vision-alignment
   - Specs whose deliverable hasn't shipped but whose principle is cited elsewhere
   This counters the search's recency-and-frequency bias. The operator may
   want to refresh attention on these even if they don't dominate the menu.

4. **High-DF promotion warning.** If you notice during investigation that a
   theme has hit or is approaching the DF auto-stopword cap (>30% of docs),
   flag it explicitly: "{theme} is approaching DF saturation — convergence
   will lose visibility soon. Recommend memorialization to manifesto/epic
   tier before that happens." This addresses the reflexive bias where
   *successful* themes become invisible.

5. **Self-aware uncertainty disclosure.** For any proposal where you are not
   confident (mixed evidence, ambiguous deliverable status, principle-level
   work that doesn't fit checkboxes), use OPERATOR-CALL with explicit
   reasoning. Do not force a verdict to fill the proposal slot. Empty
   proposals are better than wrong proposals; the operator can always run
   another cycle.

6. **The lifetime-memory principle.** Humans keep certain memories for life
   regardless of recency or frequency — formative experiences, foundational
   relationships, things that shaped who they are. The protocol equivalents
   are manifesto principles, vision statements, and explicitly memorialized
   work. These should NEVER be merged, archived, or marked-done by automatic
   logic — only by explicit operator action with reviewer pass. If you find
   yourself proposing changes to manifesto-tier content, STOP and reframe as
   OPERATOR-CALL with maximum caution.

7. **Default toward preservation when uncertain.** Cleanup is reversible
   in principle (archived items can be restored) but expensive in practice
   (operator attention is finite; once attention moves on, archived gold
   stays buried). Merge is structurally more dangerous (lossy by design;
   per the lifecycle spec, c ≈ 0.6-0.95 of inputs). When the evidence is
   ambiguous, propose nothing rather than propose merge. Empty per-theme
   proposals are valid output.

These safeguards apply across all five edit kinds (mark-done, add-as-outstanding,
merge-redundant, remove-obsolete, surface-question). The skill exists to make
the corpus more legible, not to compress it for compression's sake. Be
conservative; the operator can always tell you to be more aggressive next cycle.
```

## Biases of the search approach

`converge-scan.py` uses classical IR (TF-IDF + bigrams + DF auto-stopwords + TextRank co-occurrence centrality) over a corpus of plan filenames + frontmatter + section headings + memory titles + sprint titles. **Consuming skills (especially the Phase 2 synthesis subagent) must understand its biases** — both to compensate when investigating, and to author future content that stays legible to the search.

**Surfacing biases:**

| Bias | What it means | What to compensate |
|---|---|---|
| **Filename + heading weighting** | A theme must appear in plan filenames or section headings to surface. Themes living only in body text are invisible. | When reading a canonical plan, check whether topical concepts in the *body* match the filename/heading themes; flag mismatches as "search-invisible work" needing operator review. |
| **Bigram boost (1.6×)** | Multi-word phrasal themes get a score boost; but their constituent unigrams often dominate edges. | Don't assume bigrams = most important. Investigate sub-themes inside top unigrams. |
| **TF-IDF + DF auto-stopwords** | Pervasive vocabulary (>40% of docs) drops out as noise. Once a concept becomes universally adopted, it stops surfacing as a theme. | Treat the DF cap as a signal: high-DF terms are *successfully integrated*, not *unimportant*. Cross-reference with manifesto/epics to confirm. |
| **TextRank centrality** | Themes in dense co-occurrence clusters score higher; isolated themes lose. | Important-but-isolated work may rank low. Do not equate "low convergence rank" with "unimportant." |
| **Multi-source-type requirement (≥2)** | Themes must appear in 2+ memory-kit report types. New themes in only one report don't surface. | First-touch themes are invisible until they propagate. Explicitly check sprint-digest's recent-themes section for things that haven't yet surfaced in convergence-themes. |
| **No semantic similarity (no embeddings)** | Synonyms (`recover` ≠ `recovery`) don't cluster. | Manually fold related themes if they share a canonical plan. |

**Reflexive biases** (today's writes shape tomorrow's search):

1. **Filename choice = future theme visibility.** Be deliberate; the filename is the strongest signal converge sees.
2. **Heading discipline = topical legibility.** Use specific section headings, not narrative-only docs.
3. **Successful delivery = silent disappearance.** Correct for tactical work; *wrong* for principles. Counter via memorialization — promote completed-but-foundational work to manifesto/epic content.
4. **Convergence = DF inflation.** As more docs cite a theme, its DF rises; eventually it hits the cap and disappears. Counter: when a theme starts appearing in >30% of new docs, promote it upstream into manifesto-tier permanence.

## Boundary

- Only modifies plans (after operator approval). Never edits specs, memory entries, sprint-results, or skills.
- Never invents tasks — every add-as-outstanding cites a source.
- Reads from `.claude/memory-kit/<date>/` (shared report dropoff with memory-kit) and writes its own outputs there. The output directory naming is legacy from when converge lived inside memory-kit; rename is follow-up scope.
- Hands off via the `next-actions.md` convention to `/shift` or `/deliver`. Not autonomous.

## Related

- `.claude/skills/memory-kit/SKILL.md` — produces the reports converge consumes
- `genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md` — the design rationale and end-state vision
- `genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md` — the lifecycle primitives this skill operates within
