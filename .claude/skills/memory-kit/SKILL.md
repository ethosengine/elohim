---
name: memory-kit
description: Use periodically (weekly, before major sprints, or when you ask "what's next?") to run async memory-hygiene tasks across the dev corpus AND synthesize what's ready to execute. Toolkit of seven deterministic-first scripts — cleanup (archive stale), path-update (propagate renames), dedupe-memory (surface merge candidates), plan-status (plan dashboard), sprint-distill (sprint digest), skill-audit (skill catalog quality), converge (synthesize trajectory + produce ranked next-actions menu). Each surfaces operator-approvable proposals; only cleanup, path-update, and converge modify files (after explicit accept). Closes the loop dreaming → execution: the converge step produces a "what's next" menu with vision-alignment × readiness scoring and pre-authored Objectives, ready to hand off to /shift or /deliver. Pull periodically; not always-active.
---

# Memory Kit — Async Memory Hygiene Toolkit

The dev corpus is alive. Specs accumulate, plans cool mid-stream, memory entries point at renamed files, sprint-results bury open questions per-sprint, skills overlap. This kit is six bounded tools you pull out periodically to keep the corpus living rather than monotonically growing.

**Spec reference**: `genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md` — the comet-shaped memory model, the lifecycle primitives (`promote`, `compact`, `merge`, `submerge`/`surface`, `close-interval`, `memorialize`, `forget`, `quarantine`), and the design principles. This kit is the simplest deterministic+judgment cut applied to dev memory; future expansion handles `merge` and `promote` as full primitives.

**When to invoke**: weekly hygiene pass, before kicking off a major `/shift`, when "memory says X exists but X is gone" surfaces in agent output, or whenever the corpus feels noisy.

## Architecture

- **Scripts** live at `.claude/scripts/memory-kit/<tool>.py`. Pure Python stdlib; no external deps.
- **Outputs** land at `.claude/memory-kit/<YYYY-MM-DD>/<tool>-<artifact>.md`. Dated; unified location across all tools.
- **Archive destination** (cleanup only): `.claude/archive/<YYYY-MM-DD>/<original-relative-path>`. Mirrors repo structure so trajectory stays walkable backward.
- **Skill entry point**: this single skill, deferred-loaded. Body and tool details only enter context on invocation. **Not always active** — pull out periodically.

## The Six Tools

### 1. `cleanup` — archive stale specs/plans/memory

```bash
python3 .claude/scripts/memory-kit/cleanup-scan.py     # Phase 1: deterministic scan
# (judgment subagent dispatched as Phase 2 — see below)
python3 .claude/scripts/memory-kit/cleanup-apply.py    # Phase 3: operator-approved archive
```

**What it does**: three-phase workflow.

1. **Scan** (deterministic) — surfaces archive candidates (`status: superseded`, `status: cancelled`, completed plans, stale proposals) and dangling-reference flags. Outputs `cleanup-proposals.md`.
2. **Judge** (LLM + semantic search) — invoke this skill and dispatch a `general-purpose` subagent with the prompt below. The subagent investigates each candidate (semantic search for successor specs, recent git activity in adjacent paths, dev-intent mentions, sprint references) and classifies as ARCHIVE / BACKLOG / KEEP-FRESH / OPERATOR-CALL. Writes `cleanup-proposals-judged.md` and `cleanup-backlog-refresh.md` (the latter surfaces unfinished work that's still relevant — the inverse of forgetting).
3. **Apply** — operator marks `- [x] Accept` on confirmed-stale entries; apply moves them to dated archive. Trajectory-preserving (not delete).

**Judge phase subagent prompt**: read `cleanup-proposals.md`. For each candidate: read it, semantic-search the corpus for successor specs / sibling plans / recent git activity in adjacent paths / dev-intent mentions / sprint references / MEMORY entries / TODO comments. Classify as ARCHIVE (cite successor or abandonment evidence), BACKLOG (cite continued-relevance evidence), KEEP-FRESH (cite active references — false positive on the deterministic flag), or OPERATOR-CALL (ambiguous; summarize considerations). Write `cleanup-proposals-judged.md` (refined proposals with judgments + Accept checkboxes only on ARCHIVE entries) and `cleanup-backlog-refresh.md` (BACKLOG items sorted by recency-of-related-activity, with what's-unfinished / why-it-still-matters / suggested-next-step).

**Boundary**: only tool besides path-update that modifies files. Modifies via *moves*, never deletes.

### 2. `path-update` — propagate renames into stale citations

```bash
python3 .claude/scripts/memory-kit/path-update-scan.py    # detect renames + scan
python3 .claude/scripts/memory-kit/path-update-apply.py   # apply approved replacements
```

**What it does**: `git log --diff-filter=R --name-status` for the last year captures rename pairs. For each, ripgrep finds documents still citing the OLD path. Plus a small set of inferred renames (suffix-drop heuristics — currently `*-view.schema.json` → `*.schema.json`). Outputs `path-update-proposals.md` with `- [ ] Accept` checkboxes per (old → new) pair. Apply does in-file string replacement.

**Caveats** (worth knowing): `text.replace` is unscoped — could over-replace if a doc intentionally cites a historical path in a code sample. Operator should spot-check before bulk-accepting. The suffix-drop heuristic is hand-curated; new patterns require explicit additions.

**Why it matters**: most cleanup KEEP-FRESH cases are path drift. This tool fixes those mechanically — closes the loop on the largest class of cleanup flags.

**Boundary**: only modifies path strings; never archives, never changes content meaning.

### 3. `dedupe-memory` — surface merge candidates

```bash
python3 .claude/scripts/memory-kit/dedupe-memory-scan.py [--threshold 0.30]
```

**What it does**: TF-IDF cosine similarity across all MEMORY topic files. Clusters pairs above threshold. Outputs `dedupe-clusters.md` with similarity scores and shared key terms (interpretable evidence — the actual products contributing to the score, not generic keywords).

**Calibration note**: default threshold is 0.55 per spec, but the corpus is diverse — practical threshold for surfacing actionable clusters is closer to 0.30. Use `--threshold` to tune.

**Boundary**: read-only surface. Does NOT perform merges. Merge is a future tool that consumes these clusters.

### 4. `plan-status` — dashboard of plan state

```bash
python3 .claude/scripts/memory-kit/plan-status.py [--no-include-complete]
```

**What it does**: scans `genesis/docs/plans/` and `genesis/docs/superpowers/plans/`. For each plan: extracts checkbox state, frontmatter, age, last-modified. Classifies as `active` / `in-progress-cooling` / `stalled` / `complete` / `no-checkboxes`. Outputs `plan-status.md` with open-item text extracted per plan (the gold — what's actually unfinished).

**Headline pattern this surfaces**: if `complete: 0` keeps showing up, your team isn't completing checkboxes when work ships — `cleanup` will never see graduating plans. Fix is cultural (mark boxes), not tooling.

**Boundary**: read-only. Surfaces what NOT to archive (still-active work) and what's gone cold (cooling plans needing operator attention).

### 5. `sprint-distill` — digest sprint-results

```bash
python3 .claude/scripts/memory-kit/sprint-distill.py [--since YYYY-MM-DD]
```

**What it does**: finds sprint-result files at `.claude/shifts/**` (handles both flat and per-shift-directory conventions). Extracts structured fields per sprint (objective, outcome, what shipped, what blocked, learnings, open questions). Cross-cuts themes (word frequency across titles/objectives, stopword-filtered to project-domain terms). Aggregates ALL open-questions across ALL sprints into one "Open Questions" section.

**The aggregated open-questions section is the gold** — those are the explicit "pick this up next" pointers that otherwise stay buried per-sprint. Pair this with `plan-status` cooling and `cleanup-backlog-refresh.md` to see what's actually queued.

**Boundary**: read-only. Doesn't propose memory or spec updates (that's the future loop-closer).

### 6. `skill-audit` — skill catalog quality

```bash
python3 .claude/scripts/memory-kit/skill-audit.py
```

**What it does**: scans `.claude/skills/*/SKILL.md`. Three issue classes: vague descriptions (too short / generic / no `Use when` framing), trigger-overlap pairs (skills competing on the same conversational cues), stale-by-mtime (>90 days untouched).

**Why it matters**: skill metadata is always-loaded into Claude's context. Vague descriptions cost tokens and clutter trigger-matching. Overlapping triggers mean two skills both want to fire on the same cue — usually means consolidation or differentiation is needed.

**Boundary**: read-only diagnostic. Doesn't rewrite descriptions or merge skills (those are operator decisions).

## Recommended Periodic Workflow

**Weekly hygiene + synthesis (~25 minutes)**:
1. `python3 .claude/scripts/memory-kit/cleanup-scan.py`
2. Invoke this skill so cleanup judgment phase runs (subagent classifies candidates)
3. `python3 .claude/scripts/memory-kit/path-update-scan.py`
4. `python3 .claude/scripts/memory-kit/plan-status.py`
5. `python3 .claude/scripts/memory-kit/sprint-distill.py --since <last-sweep-date>`
6. `python3 .claude/scripts/memory-kit/converge-scan.py` (Phase 1 of converge)
7. Invoke this skill again so converge synthesis subagent runs (produces per-theme proposals + next-actions menu)
8. Review the dated `.claude/memory-kit/<today>/` directory — start with `next-actions.md` (the menu)
9. Mark `- [x] Accept` on cleanup, path-update, and converge entries you approve
10. Run the apply scripts: `cleanup-apply.py` → `path-update-apply.py` → `converge-apply.py`
11. **Hand off**: pick the top next-action and invoke `/shift` (or `/deliver`) with the pre-authored Objective

**Session-start ("what's next?") UX**:
- Human types "what's next?" at session start
- Agent reads `.claude/memory-kit/<latest>/next-actions.md`
- If older than ~7 days, suggest a fresh weekly cycle first
- Otherwise present the top recommendation conversationally + offer detail on others
- On selection, invoke `/shift` or `/deliver` with the pre-authored Objective

**Monthly deeper sweep (add to the weekly)**:
- `python3 .claude/scripts/memory-kit/dedupe-memory-scan.py --threshold 0.30`
- `python3 .claude/scripts/memory-kit/skill-audit.py`
- Review the cluster pairs and skill issues; act on what's actionable.

**Before a major `/shift`**: at minimum run cleanup + path-update so the agent sees a corpus without dangling refs.

## Design Principles

- **Deterministic-first.** Five of six tools are pure rule-based at the surfacing layer. Only cleanup uses judgment (Phase 2 subagent), and even that is operator-gated at apply.
- **Operator-approved.** No tool modifies files without an explicit `- [x] Accept` from the operator. Even cleanup's apply step skips entries the operator hasn't checked.
- **Archive, never delete.** Matches the lifecycle spec — `close-interval` is structurally distinct from `forget`. Future passes may eventually `forget` items archived > N months old; that's its own cycle.
- **Read-only by default.** The most common interaction is "scan → review the report → close it" without any modification at all. Modification is the exception.
- **Trajectory-preserving.** Archived items remain queryable. Dated directories make "what got cleaned when" walkable backward.
- **Bounded scope.** This kit covers specs, plans, memory, sprint-results, skill catalog. Not code, not test files, not CI artifacts. Resist scope creep — when the deterministic rules outgrow this, that's when the broader `/dream` skill (forthcoming) is the right next artifact.
- **Single skill entry point.** This one skill. Six tools. Deferred-loaded. Not always-active. Pull out periodically.

### 7. `converge` — synthesize trajectory + produce "what's next" menu

```bash
python3 .claude/scripts/memory-kit/converge-scan.py     # Phase 1: deterministic theme detection
# (judgment subagent dispatched as Phase 2 — see below)
python3 .claude/scripts/memory-kit/converge-apply.py    # Phase 3: operator-approved plan edits
```

**Why it matters**: this is the loop-closer. Every dreaming pass produces a sharper plan; every plan converges to delivery. The end-state UX: human asks **"what's next?"** at session start; agent reads the converge menu and presents a ranked list of refreshed plans with pre-authored Objectives ready for `/shift` or `/deliver`.

**Spec**: `genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md`.

**Four phases:**

1. **Theme detection** (deterministic, `converge-scan.py`) — clusters BACKLOG items, dedupe pairs, plan-status cooling items, sprint-digest themes, and path-rename clusters by shared theme keywords (extracted from active plan filenames + filtered through stopwords + multi-source-type requirement). Outputs `convergence-themes.md` with each theme's contributing items and a candidate canonical plan (most-recent-modified plan with the theme keyword).

2. **Synthesis** (LLM judgment, subagent dispatch) — when this skill is invoked after a fresh scan and the operator asks for synthesis, dispatch a `general-purpose` subagent (opus) using the prompt template below. The subagent produces:
   - Per-theme proposals at `.claude/memory-kit/<TODAY>/converge/<theme>-proposal.md` with structured edit blocks (mark-done, add-as-outstanding, merge-redundant, remove-obsolete, surface-question) — each edit is operator-reviewable with `- [x] Accept` checkboxes.
   - The **next-actions menu** at `.claude/memory-kit/<TODAY>/next-actions.md` — top 3-5 ranked recommendations with vision × readiness scoring and pre-authored Objectives.

3. **Apply** (deterministic, `converge-apply.py`) — for each accepted edit, performs the plan modification. v1 supports `mark-done` (`- [ ]` → `- [x]` with evidence comment) and `add-as-outstanding` (insert new `- [ ]` after named anchor); other kinds are operator-manual edits in v1.

4. **Session-start UX** — when a human says "what's next?", read the latest `next-actions.md`, present conversationally, hand off to `/shift` or `/deliver` with the chosen pre-authored Objective. No new skill needed; this is a convention.

**Synthesis subagent prompt (Phase 2 dispatch):**

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

**Boundary**: only modifies plans (after operator approval). Never edits specs, memory entries, sprint-results, or skills. Never invents tasks — every add-as-outstanding cites a source.

#### Biases of the search approach (read this; it shapes what surfaces AND what your work shapes for future cycles)

`converge-scan.py` uses classical IR (TF-IDF + bigrams + DF auto-stopwords + TextRank co-occurrence centrality) over a corpus of plan filenames + frontmatter + section headings + memory titles + sprint titles. This approach has known biases. **Consuming skills (especially the Phase 2 synthesis subagent) must understand them — both to compensate when investigating, and to author future content that stays legible to the search.**

**Surfacing biases (themes the search favors or misses):**

| Bias | What it means | What to compensate |
|---|---|---|
| **Filename + heading weighting** | A theme must appear in plan filenames or section headings to surface. Themes living only in body text are invisible. | Synthesis subagent: when reading a canonical plan, check whether topical concepts in the *body* match the filename/heading themes; flag mismatches as "search-invisible work" needing operator review. |
| **Bigram boost (1.6×)** | Multi-word phrasal themes (e.g. `cross stack`, `phase 12`) get a score boost; but their constituent unigrams often dominate edges. | Don't assume bigrams = most important. If a unigram (e.g. `iroh`) absorbs all connectivity, its bigrams (e.g. `iroh recovery`) may be undersurfaced even though they're more specific. Investigate sub-themes inside top unigrams. |
| **TF-IDF + DF auto-stopwords** | Pervasive vocabulary (in >40% of docs) drops out as noise. So once a concept becomes universally adopted, it stops surfacing as a theme. | If `iroh` gets DF-capped (e.g. mentioned in every memory entry six months from now), it'll be invisible. Treat the DF cap as a signal: high-DF terms are *successfully integrated*, not *unimportant*. Cross-reference with manifesto/epics to confirm. |
| **TextRank centrality** | Themes in dense co-occurrence clusters score higher; isolated themes lose. | Important-but-isolated work (a niche but load-bearing thread) may rank low. The synthesis subagent should not equate "low convergence rank" with "unimportant." |
| **Multi-source-type requirement (≥2)** | Themes must appear in 2+ memory-kit report types. New themes only in one report don't surface. | First-touch themes (e.g. a new arc just opened in one sprint) are invisible until they propagate. Synthesis subagent should explicitly check sprint-digest's recent-themes section for things that haven't yet surfaced in convergence-themes. |
| **No semantic similarity (no embeddings)** | Synonyms (`recover` ≠ `recovery`) don't cluster. Conceptually-adjacent themes with disjoint vocabulary stay separate. | Synthesis subagent should manually fold related themes if they share a canonical plan — convergence might list them as separate. |

**Reflexive biases (how the agent's own work shapes future search):**

The corpus IS the agent's accumulated work. Today's writes become tomorrow's search inputs. Three feedback loops to be aware of:

1. **Filename choice = future theme visibility.** If you write a spec named `2026-05-15-async-mesh-design.md`, the term `async-mesh` will surface as a candidate theme next cycle. If you write it as `2026-05-15-design.md`, `async-mesh` is invisible until you put it in a heading. **Be deliberate**: the filename is the strongest signal converge sees.

2. **Heading discipline = topical legibility.** Plans/specs/memories with rich, specific section headings produce richer signal than narrative-only docs. If you author a long body without heading structure, your work won't surface as a theme even if it's load-bearing. Use headings; use specific terms.

3. **Successful delivery = silent disappearance.** When work ships and stops being mentioned in new sprint-results, its themes lose signal each cycle. Eventually the search forgets. **This is correct for tactical work** (we don't want delivered tactical items occupying the menu). **It's wrong for principles** that need to stay surfaced. Counter via memorialization — promote completed-but-foundational work to manifesto/epic content where it stays anchored regardless of recent activity.

4. **Convergence = DF inflation.** As more docs cite a theme, its DF rises; eventually it hits the cap and disappears. So the more successful a theme is, the closer it gets to invisibility. Counter: when a theme starts appearing in >30% of new docs, that's the moment to *promote it upstream* (into manifesto-tier permanence) rather than relying on convergence to keep surfacing it.

**For coherence + focus**: the search rewards tight clustering and consistent vocabulary. Disjoint-but-related work (multiple aliases for the same concept) reads as low-centrality noise. So coherence in naming and focus in scope BOTH help the search find the work AND help the search drop the noise. The skill helps you when you help the corpus stay legible.

**For the synthesis subagent specifically**: when scoring a theme, also note in your reasoning *whether the search shaped what you saw or whether you discovered things outside the search's view*. That meta-observation is itself useful operator signal — it tells us when the search is keeping up with the work and when it's lagging.

## What This Kit Is NOT

- Not a general memory consolidator (no merge, no promote, no compact in current tools — those need their own design)
- Not destructive (archive, never delete; modifications are limited to path-string substitution)
- Not autonomous (operator approval is structural for any file modification)
- Not always-active (single skill, deferred-loaded, periodic invocation)

## Related

- `genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md` — lifecycle spec (the design language all tools share)
- `genesis/docs/content/elohim-protocol/living_memory/epic.md` — narrative on what living memory means in the protocol
- `.claude/scripts/memory-kit/` — all tool scripts
- `.claude/memory-kit/<YYYY-MM-DD>/` — dated outputs (operator review surface)
- `.claude/archive/<YYYY-MM-DD>/` — cleanup destination (preserves trajectory)
