---
name: memory-ceremony
description: Orchestrate four agents (one per temporal scope) through six coordinated waves — pre-flight, parallel survey, four-lens debate, future projection, operator review, retrospective. Invoke when working-memory byte-budget breaches, when scanner substrate changes, when 3+ sprint-results land in one wing, on manifesto edits, or on operator request. Each agent speaks for its temporal scope; the ceremony resolves disposition decisions a single lens cannot.
---

# /memory-ceremony — Six-Wave Coordinated Memory Hygiene

This skill orchestrates the four-agent memory team (librarian, historian, storyteller, cartographer) through a coordinated ceremony. Each wave has a specific shape; Wave 4 is the operator-interaction checkpoint; balance-sheet snapshots at Wave 0 + Wave 6 give cross-cycle delta evidence.

**When to invoke**:
- Signal: MEMORY.md ≥ 22.5KB (90% of budget); audit-substrate edit; 3+ sprint-results in one wing within 14 days; manifesto edit
- Operator: when memory health is in question, before a major /shift, or to feel a settling baseline
- Floor: monthly if no signal fires; ceiling: biweekly

**Substrate references**:
- `.claude/scripts/memory-kit/LIFECYCLE.md` — ownership matrix, dispositions, cadence
- `genesis/data/timeline/CONVENTIONS.md` — backlog/roadmap/chronicle schema
- `genesis/data/stories/CONVENTIONS.md` — canonical stories schema
- `genesis/scripts/memory-balance.sh` — Wave 0 + Wave 6 balance sheet capture

## Wave 0 — Pre-flight (~1 min)

Verify in parallel:
- `mempalace status` returns 4 wings, >10K drawers
- `genesis/data/stories/CONVENTIONS.md`, `INDEX.md`, ≥1 story file present
- `genesis/data/timeline/CONVENTIONS.md` + chronicle/, roadmap/, backlog/ subdirs
- All four agents present at `.claude/agents/{librarian,historian,storyteller,cartographer}.md`
- `git status` reasonably clean
- `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` IF planning to use real TeamCreate at Wave 2 (otherwise single-agent four-lens runs without)

**Capture balance sheet baseline**:
```
genesis/scripts/memory-balance.sh
```
Persists snapshot to `.claude/memory-kit/balance-sheets/<ts>.{json,txt}`. Diff column shows delta from most-recent prior snapshot.

State one-line readiness; bail if anything fails.

## Wave 0.5 — Conditional: retroactive MemPalace cleanup (~5-10 min)

Only run if a substantive in-place refactor has happened since last ceremony (e.g., persona rename, schema change touching memory entry content). Frozen embeddings need re-mining.

```
mempalace status                                                          # baseline
mempalace init /projects/elohim/.claude/memory --no-llm --yes --auto-mine
mempalace init /projects/elohim/genesis/plans --no-llm --yes --auto-mine
mempalace status                                                          # delta
mempalace search "<recent rename term>"                                   # sanity
```

Skip `shifts` (historical) and `elohim-protocol` unless they were touched. Content hashing skips unchanged files automatically.

## Wave 1 prologue — Horizon-scan freshness check (~1 min)

Before dispatching Wave 1 subagents, check the latest report at `.claude/memory-kit/horizon-scans/`. If the latest report's `next_recommended_scan` is ≤ today, OR no report exists: dispatch the cartographer to invoke `/mem-horizon-scan` first. Most ceremonies skip this step (quarterly cadence). When triggered, the cartographer's Wave 1 output prepends a "Horizon delta" section based on the new scan's summary.

## Wave 1 prologue — Story-coverage audit run (~1 min, every ceremony)

Run `.claude/scripts/memory-kit/story-coverage-audit.py` BEFORE dispatching Wave 1 subagents. Cheap (filesystem walk + frontmatter parse); deterministic; idempotent. Output goes to `.claude/memory-kit/story-coverage-audit.json` + dated `.md` report — neutral coverage data (features-on-disk, features-orphan, per-orphan leverage_score, sourcing-completeness flags, dangling references).

Surface the numbers as **Wave 0 metrics** to the operator (one line: "story-coverage: M of N features orphan; K canonical-anchored"). Do not pre-interpret what they mean for downstream waves; each Wave 1 subagent reads the same data and reaches its own conclusion per its own lens.

## Wave 1 — Parallel survey (~5-10 min) — plain Task dispatch

Dispatch THREE subagents in **one message** (librarian + historian + cartographer). Storyteller does NOT participate in Wave 1.

### Subagent 1: librarian — hygiene pass

> Drive a memory-kit hygiene pass over the present-tense substrate. Workflow: run memory-review.py for baseline; read `.claude/memory-kit/claude-md-drift.json` for drift; run cleanup-scan; run dedupe-memory-scan (confirm top hits with `mempalace_check_duplicate` for embedding-grade verification); run claude-md-audit; run skill-audit and agent-audit; run story-coverage-audit.py.
>
> **NEW (Run #2 retro) — graduation-delivery-gap check**: run the `deliver-bridge` auto-poller (`.claude/scripts/memory-kit/delivery-status-poll.py` if present; otherwise fall back to reading `/deliver` sprint-results manually from `.claude/shifts/*-deliver-*/`). For each canonical story whose linked feature has `delivery_status < active.latest-stable` (per `/deliver`'s last verdict, or floor signal if `/deliver` hasn't judged), surface as `graduation-delivery-gap`. **Authority reminder**: librarian only READS the delivery axis here — `/deliver` is the only authority that mints `active.*`/`stable`/`regression`. Do not author those states.
>
> **NEW (Round 3) — story-coverage-audit + sourcing-completeness**: read `.claude/memory-kit/story-coverage-audit.json` (regenerated by the Wave 1 prologue). Surface the neutral coverage numbers (`features_on_disk`, `features_orphan`, top-N orphans by `leverage_score`) in your output for downstream lenses to weigh independently. Do not pre-interpret. Also check per-canonical-story `sourced_from:` blocks for the 5 keys; flag any keys empty without inline rationale comment as a per-story currency-audit flag.
>
> **NEW (Run #5 retro) — cross-substrate impact map (MANDATORY, not optional)**: for every finding you report — drift item, archive candidate, dedupe cluster, CLAUDE.md edit candidate, anything about to mutate gospel-tier substrate — also surface the **cross-substrate impact map**: which `.claude/agents/*.md` cite the affected content, which `.claude/skills/*/SKILL.md` reference the affected pattern, which `.claude/memory/*.md` entries depend on the affected substrate, which `genesis/data/stories/*.md` or backlog entries cite it. Use grep + mempalace_search to build the map. **The single-substrate finding alone is insufficient output** — Run #5's CLAUDE.md OVER-BUDGET regression (1 → 3) happened because cross-substrate impact wasn't part of the standard report shape. Coherence across substrates is the ceremony's job; report what would drift if your finding is acted on, so cartographer's Wave 3 plan and Wave 6 dispatch can include the sweep.
>
> Per LIFECYCLE.md: write working memory + apply tiny corrections during dedupe; do NOT write timeline entries (handoff to historian/storyteller/cartographer). OBSERVATION ONLY in this wave — Wave 4 is the apply step.
>
> Output ≤ 600 words per shape (raised from 500 to accommodate impact maps): health summary, top findings WITH cross-substrate impact maps, archive candidates WITH impact maps, dedupe candidates, CLAUDE.md drift surfaces WITH impact maps, **graduation-delivery-gap list**, **story-coverage numbers (features-on-disk, features-orphan, top orphans by leverage_score)**, **per-story sourcing-completeness flags**, chronicle-worthy moments, signals worth carrying forward.

### Subagent 2: historian — precedent surface

> Surface resonant precedents for the team's current trajectory. Workflow: read recent context (git log 7d, latest shift-results, dev-intent.jsonl, stories/INDEX, timeline conventions, MEMORY.md). Identify 3-4 in-flight themes. For each, `mempalace_search` across all four wings; apply 6-layer progressive recall ladder when widening (search → get_drawer → read source → kg_timeline → find_tunnels → git log). Filter to SPECIFIC, LOAD-BEARING, NON-OBVIOUS precedents.
>
> **NEW (Run #5 retro) — cross-substrate drift precedent (MANDATORY surface)**: include at least one precedent specifically about **cross-substrate coherence failures** when they exist — prior cycles where a single-substrate fix caused downstream citations to go stale (CLAUDE.md trim orphaning agent citations; skill rename orphaning memory references; memory graduation orphaning skill descriptions). These are the load-bearing "we've been here before, single-substrate hygiene isn't enough" cases that cartographer needs to weight when planning Wave 3 stasis actions. If no such precedent surfaces this cycle, say so explicitly — "no archived cross-substrate drift precedent for this shape" is a useful negative result.
>
> Per LIFECYCLE.md: in-conversation annotations only this wave; chronicle entries are Wave 4-approved.
>
> Output ≤ 450 words (raised from 400 to accommodate cross-substrate precedent): trajectory read, max 3 resonant precedents (archived-at, what happened, why now, suggested action), **cross-substrate drift precedent (or explicit no-resonance)**, no-resonance themes.

### Subagent 3: cartographer — future surface

> Surface forward-leaning themes that have accumulated readiness signal. NEW in 2026-05-14 retro — cartographer joins Wave 1 to give the disposition-pen (storyteller) three lenses to score against, not two. Workflow: read manifesto principles for vision-axis; check existing backlog/roadmap for active commitments; search for plans nearing readiness; identify manifesto edits not yet propagated to backlog; surface vision-cited content lacking active work.
>
> **NEW (Round 3) — story-coverage audit as substrate**: read `.claude/memory-kit/story-coverage-audit.json` (regenerated in the Wave 1 prologue). The data — `features_on_disk`, `features_orphan`, per-orphan `leverage_score` — informs your vision×readiness ranking per your own per-cycle judgment. No predetermined multiplier, no fixed prohibition on vision-projection. Some cycles the coverage gap may dominate your read; other cycles other signals may dominate. Weigh independently.
>
> Output ≤ 200 words: top 3-5 forward-themes with vision×readiness scores + one-line readiness rationale each. This becomes Wave 2 input for storyteller's disposition decisions.

After all three return, summarize to operator in 5-6 lines.

## Wave 2 — Disposition debate (~10-15 min) — storyteller leads

Dispatch storyteller as Wave 2 lead. Storyteller decides operating shape:

- **Single-agent four-lens (default for routine, ≤10 candidates, no obvious disagreement)**: storyteller carries librarian/historian/cartographer/storyteller lenses inline, producing the triage directly. Faster, dispositions still rigorous if discriminators are clean.
- **Real team-debate via TeamCreate (contested, >10 candidates, or lens-disagreement predicted)**: storyteller spawns librarian/historian/cartographer as teammates; debates via mailbox; synthesizes. Higher fidelity for contested decisions. Requires `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` in session.

Hand storyteller the three Wave-1 reports verbatim + substrate refs. Hard rules enforced:
- **Tiny-delete**: librarian-proposes + storyteller-confirms (two-signature)
- **Graduate**: storyteller-proposes + named story exists at `status: canonical`. **Now branches by delivery-axis** (Run #2 retro):
  - **`graduated-narratively`** (story canonical + linked feature `delivery_status < active.latest-stable`): graduation completes (narrative carries the lesson), but cartographer is handed a `delivery-debt` flag to refine into a backlog entry (e.g., "author `<feature>.feature` + run through `/deliver`")
  - **`graduated-fully`** (story canonical + linked feature `delivery_status >= active.latest-stable` per `/deliver`'s verdict): graduation completes; no debt flag
- **Graduate-pending** (added 2026-05-14): story exists at `status: draft` — entry waits on operator-canonical flip
- **Memorialize**: historian-confirms forensic value beyond what archive preserves (discriminator: "does memorialize add retrievability archive doesn't already give?")
- **Archive-without-graduation**: historian does NOT confirm forensic value beyond archive
- **No-consensus**: valid output; route to Wave 4 operator decision

**Authority reminder**: the storyteller reads `delivery_status` from story frontmatter to decide between `graduated-narratively` and `graduated-fully`. The storyteller does **not** author `delivery_status` — that's `/deliver`'s authority, written by the `deliver-bridge` auto-poller. If `delivery_status` is missing or stale, the storyteller defaults to `graduated-narratively` (safe default) and flags `delivery-poll-stale` for the librarian to refresh.

Output sections (storyteller pen): COVERED-graduated-fully, COVERED-graduated-narratively, MEMORIALIZE, HOLD, GRADUATE-PENDING, ARCHIVE-WITHOUT-GRADUATION, TINY-DELETE, NEEDS-NEW-STORY, NO-CONSENSUS, debate signals for Wave 5 retro.

**Round 3 — AUTHOR-CANONICAL-STORY available disposition class**: AUTHOR-CANONICAL-STORY is one disposition class in the storyteller's repertoire. If the storyteller's lens reads canonical-story authoring as the right move for this cycle (weighing story-coverage audit data against the rest of the Wave 1 substrate), append the section to the disposition output:

```
AUTHOR-CANONICAL-STORY — proposed authorings for this cycle:
1. (subject, role, feature) — proposed title — sourced_from preview — rationale
2. ...
3. ...
```

Cap at ~3 (storyteller is one Opus seat; per `storyteller-coverage-sprint` calibration, 6 stories per sprint is the upper edge — so ~3 per ceremony cycle is sustainable). Whether to use this disposition class — and how to balance it against disposition triage — is the storyteller's per-cycle judgment, not pre-allocated by signal. These authorings, if proposed, become Wave 4 operator-decision items alongside the other disposition outputs.

## Wave 3 — Synthesis + stasis plan (~5-10 min) — cartographer

Dispatch cartographer with all three Wave 1+2 reports verbatim. Wave 3 produces DRAFTS only; writes happen in Wave 4 and execution happens in Wave 6.

Workflow:
1. Cluster findings into themes; note convergences (high signal when 3+ agents flag same area)
2. Score each cluster by vision-alignment × readiness × quiet-but-load-bearing (0.4 / 0.4 / 0.2 — convergence is one input, not the ranker; see [[project_three_temporal_perspectives]] convergence-bias caveat). The story-coverage audit data is part of the substrate you weigh; how it informs your ranking is your per-cycle judgment.
3. Draft top 3-5 backlog Objectives with full frontmatter + body per timeline/CONVENTIONS.md
4. Draft 1-3 roadmap entries for longer-horizon themes if your read supports it
5. For each Wave 2 NEEDS-NEW-STORY item, evaluate elevating as backlog Objective ("write canonical story of X")
6. For each Wave 2 AUTHOR-CANONICAL-STORY item (if storyteller produced any), evaluate whether to elevate one as the recommended /shift Objective — pre-author it with subject/role/feature triple, sourced_from preview, and your rationale
7. Recommend single best /shift Objective with 2-sentence justification. What that recommendation is depends on what Wave 1+2+3 actually surfaced this cycle; do not pre-commit to a category.
8. **Stasis implementation plan (REQUIRED, Round 4+)** — for each audited dimension in LIFECYCLE.md "Dimensions with stasis targets", produce a row in the stasis plan. This is the substrate Wave 6 executes against — **not aspirational, binding**. Authoring discipline:
   - **20% is a floor, not a ceiling.** Default ambition is the 50% bite when scope allows; the floor is the worst acceptable outcome, not the target. Silently aiming for the floor IS bailing on the larger goal.
   - **Concrete diffs, not "propose action."** Where substrate is well-defined (CLAUDE.md line edits, MEMORY.md entry trims, specific dead-path removals), name the lines/files/edits in the action column. Wave 6 dispatches must be paste-ready scope, not "go figure it out."
   - **CLAUDE.md drift is always-attempt.** Enumerate each affected CLAUDE.md file with its own concrete diff; one operator-approval question in Wave 4 covers the whole set as flow-through. "Operator pre-approval gate" is NOT a defer reason — drafting the diff is part of the plan, approval is one inline question, applying is Wave 6.
   - **Cross-substrate sweep is part of every gospel-touching action (Run #5 retro).** When a stasis-plan row's action touches a CLAUDE.md, an agent definition, a skill description, or any gospel-tier file, the row MUST include a cross-substrate sweep sub-action — informed by the librarian's Wave 1 impact maps. Example: a "CLAUDE.md drift" row's action is `(1) edit root + 3 over-budget files per diff; (2) sweep .claude/agents/ for citations of the removed content; (3) sweep .claude/skills/ for descriptions referencing the removed patterns; (4) sweep .claude/memory/ for entries depending on the removed substrate; (5) apply mechanical fixes; surface judgment calls for Wave 4`. The action's effort tier accounts for the full sweep, not just the named-file edit. Wave 6 dispatches the full sweep as one bounded scope.
   - **All-CLAUDE.md-files invariant.** When the action touches any CLAUDE.md, ALWAYS check every CLAUDE.md file in the tree (`find . -name 'CLAUDE.md' -not -path '*/node_modules/*'`) for budget/drift status, not just the drift-listed ones. Run #5's OVER-BUDGET regression (1 → 3) happened because the librarian was only watching root while two others crossed 200 lines unsupervised.
   - **Aim for over-delivery on adjacent dimensions.** When librarian is already in a CLAUDE.md file, neighboring dead-path noise gets cleaned in the same dispatch — same blast radius, more value. When tightening MEMORY.md, sweep for stale memorializations to graduate.
   - **The plan is binding.** Wave 5 retros will reckon dimension-by-dimension against it AND against cross-substrate coherence as a first-class outcome; Wave 6 will execute it; the chronicle will record per-dimension achievement. Padding the defer column to escape work shows up in the next-cycle 3-cycle-clock as a forced-attempt.

### Wave 3 stasis-plan output template

```
## Stasis implementation plan — Round N

| # | Dimension | Current | Target | 20%-floor | Proposed action | Impact map | Resolution agent | Effort tier | Status |
|---|---|---|---|---|---|---|---|---|---|
| 1 | CLAUDE.md DRIFTED-FACTUAL count | 18 | 0 | 14 | Reword the 4 most-stale lines in app/elohim-app/CLAUDE.md per audit's per-line evidence | §1 | librarian | small | proposed |
| 2 | CLAUDE.md OVER-BUDGET count | 2 | 0 | (n/a — count of 2; advance = touch at least 1) | Trim root CLAUDE.md by ~200 lines via section consolidation; ALL-CLAUDE.md invariant sweep | §2 | librarian | medium | proposed |
| 3 | Cleanup-scan review flags | 67 | 0 | 53 | Run cleanup-judge subagent over the 14 most-stale entries; apply operator-confirmed ARCHIVE | §3 | librarian | small | proposed |
| 4 | Agent catalog real findings (post-FP filter) | 3 | 0 | 2 | Fix dead-path citations in 2 agent prompts | §4 | librarian | small | proposed |
| 5 | Skill catalog overlap pairs | 1 | 0 | 1 | Disambiguate /memory-kit vs /converge descriptions | §5 | librarian | small | proposed |
| 6 | Story orphan-ratio | 0.42 | 0 | 0.34 | (deferred-with-rationale: storyteller-coverage-sprint owns this; ceremony surface ≤3/cycle) | — | storyteller | medium | deferred |
| 7 | Story sourcing-completeness flags | 2 | 0 | 2 | Backfill sourced_from on james-son + david-and-the-stewarded-hub | §7 | storyteller | small | proposed |
| 8 | MEMORY.md byte size | 30,025 | 24,400 | 28,900 | Tighten 6 longest index entries; graduate 2 stale memorialized topics | §8 | librarian | medium | proposed |
| 9 | Surface:Archive ratio | (current) | <100:1 | (current × 0.8) | (operator-confirmed graduations from Wave 4) | §9 | storyteller + librarian | small | proposed |
| 10 | /deliver pickup queue | (current) | drain over time | (current × 0.8) | (deferred-with-rationale: cycle has no /deliver-ready features this round) | — | /deliver | small | deferred |

## Impact maps (cross-substrate citations to sweep)

Every row with `Status: proposed` AND a gospel-touching action MUST have an impact-map block. Rows with `Status: deferred-with-rationale` MAY omit the map (action isn't happening this cycle). Per-row blocks are paste-ready for Wave 6 dispatches — the executing agent reads them as part of the dispatch prompt body, not as separate documentation.

### §1 — CLAUDE.md DRIFTED-FACTUAL — impact map
- **Files about to change**: app/elohim-app/CLAUDE.md (4 lines), [enumerate the specific line ranges per audit evidence]
- **Cited by — agents** (grep'd by librarian Wave 1): .claude/agents/angular-architect.md (line 22 cites the testing pattern), .claude/agents/code-reviewer.md (line 47 cites the lint setup)
- **Cited by — skills**: .claude/skills/page-model/SKILL.md (description references the strict-template pattern at line 38 of the affected CLAUDE.md)
- **Cited by — memory**: .claude/memory/feedback_signature_changes_grep_callers.md (refs the RUSTFLAGS section), .claude/memory/project_lit_wc_pivot_design_for_generation.md
- **Cited by — stories/backlog**: none surfaced
- **Sweep actions**: (a) apply diff to elohim-app/CLAUDE.md; (b) update angular-architect.md line 22 if pattern reworded; (c) update code-reviewer.md line 47; (d) confirm page-model SKILL description still accurate or update; (e) surface any judgment calls in (b)–(d) to operator
- **No-action affordance**: if a sweep target's citation is still valid after the diff, mark verified-no-change in Wave 6 report — don't silently skip

### §2 — CLAUDE.md OVER-BUDGET — impact map
- **Files about to change**: root CLAUDE.md (281→<200 lines via section consolidation)
- **All-CLAUDE.md invariant sweep**: `find . -name 'CLAUDE.md' -not -path '*/node_modules/*'` and report current line count per file; any file ≥190 lines gets the same trim treatment in this dispatch
- **Cited by — agents**: [librarian Wave 1 fills with grep results — every agent that cites root CLAUDE.md content]
- **Cited by — skills**: [as above]
- **Cited by — memory**: [as above]
- **Sweep actions**: (a) trim root; (b) sweep any other CLAUDE.md file at-or-near budget; (c) update cited substrate per drift surfaces

(§3, §4, §5, §7, §8, §9 — same shape, populated by cartographer from librarian's Wave 1 impact maps. The librarian's Wave 1 IS the source for these blocks; cartographer's job is to organize them into the plan, not invent them.)
```

Columns:
- **#** — row index; referenced by the per-row impact-map block below the table
- **Dimension** — name from LIFECYCLE.md
- **Current** — value from this cycle's audits
- **Target** — value from the dimensions table
- **20%-floor** — minimum advance this cycle = `current - 0.2 × (current - target)`. For binary/very-small counts, "advance = touch at least 1" is acceptable.
- **Proposed action** — bounded scope; achievable in one Wave 6 dispatch; gospel-touching actions must include the cross-substrate sweep as part of the action
- **Impact map** — `§N` pointer to the per-row block below the table, OR `—` for deferred rows. Populated from librarian's Wave 1 impact-map output. Wave 6 dispatch reads this block verbatim.
- **Resolution agent** — librarian / storyteller / historian / /deliver
- **Effort tier** — small (≤1 dispatch); medium (≤3 dispatches); large (needs split across cycles → at least one split must land this cycle; backlog the remainder)
- **Status** — `proposed` (default; will be executed in Wave 6) or `deferred-with-rationale` (cycle cannot advance this dimension; rationale required; subject to defer-budget + 3-cycle clock)

### The deferred-with-rationale option (bounded; not the easy out)

Some dimensions are genuinely unactionable in a given cycle. To preserve agent agency without enabling silent demotion-by-defer, the option is **bounded** along three axes:

**Defer budget**: at most **2 dimensions per cycle** may carry `status: deferred-with-rationale`. If cartographer's Wave 3 plan defers a third, the cartographer must demote one of the existing defers to `proposed` (find a way to take a bite this cycle, however small).

**3-cycle clock**: a dimension deferred for 3 consecutive cycles becomes `forced-attempt` in cycle 4 — cartographer must propose action; `deferred-with-rationale` is unavailable. Track via the chronicle's "Stasis-progress per dimension" table across cycles. A `forced-attempt` dimension's action may be deliberately small ("touch at least 1 file") but cannot be omitted.

**Valid defer reasons** (narrowed):
- **Substrate dependency** — upstream tool/script not yet built (e.g., `delivery-status-poll.py` absent). Pairs with: a backlog entry to build the dependency must already exist or be filed this cycle.
- **Out-of-cycle ownership** — dimension only moves when a non-ceremony actor runs (e.g., `/deliver` pickup queue drains when `/deliver` runs). The chronicle still records the dimension's value.

**NOT valid defer reasons** (these are bailing, not deferring):
- **"Effort tier = large"** — split into 2+ dispatches; at least one split MUST land this cycle. File a backlog entry for the remainder.
- **"Operator pre-approval gate"** — draft the diff in Wave 3, surface as one Wave 4 question (flow-through-ready), apply in Wave 6. Approval gating is procedure, not blocker.
- **"Convergence pulled focus elsewhere"** — same-day variance-collapse is a known footgun, not a defer reason.
- **"Cycle is busy already"** — every cycle is busy. The 20%-floor exists precisely because near-100%-coverage of a regressed-toward-stasis state is harder than the first 20%, not easier.

If a candidate defer doesn't pass these gates, the dimension gets a `proposed` row with a smaller action — never a disappearance into the rationale column. The substrate's purpose is to make bailing visible and costly across cycles, not to reward "explicit defer not silent" as if it were the same as advancing.

## Wave 4 — Operator review + apply (~5-15 min depending on mode)

Present compact summary to operator (≤20 lines, raised from 15 to fit impact-map surfacing):
- Librarian: archive/dedupe/CLAUDE.md drift counts
- Historian: precedent count + names (including any cross-substrate drift precedents)
- Storyteller: disposition counts per category
- Cartographer: backlog/roadmap draft counts + convergent themes
- NO-CONSENSUS cases needing operator call
- **Stasis-plan summary**: N rows proposed, M deferred (with reasons), 20%-floor advances expected per dimension
- **Cross-substrate sweep scope per gospel-touching row**: when an approval covers a gospel-touching action, surface the impact-map sweep targets explicitly in the summary — not "approve fix-CLAUDE.md-drift" but "approve §2 = trim root CLAUDE.md + sweep [N agent files] + [M skill descriptions] + [K memory entries]". The operator needs to see what cross-substrate work the approval authorizes, or the gate is rubber-stamping.

### Impact-map surfacing in operator questions

When `AskUserQuestion` is invoked for a gospel-touching row, the option label/description MUST name the sweep scope:

- ❌ Wrong: "Approve: Fix CLAUDE.md DRIFTED-FACTUAL — reword 4 lines in app/elohim-app/CLAUDE.md"
- ✅ Right: "Approve §1: edit app/elohim-app/CLAUDE.md (4 lines) + sweep .claude/agents/{angular-architect, code-reviewer} + .claude/skills/page-model + 2 memory entries citing the reworded patterns"

Operator can then choose: (a) approve the full sweep, (b) approve the edit but defer the sweep with rationale, (c) edit the impact-map (remove a sweep target, add one), (d) decline entirely. Option (b) requires the operator to write the rationale — sweep-deferral is still a defer that the 3-cycle clock counts.

### Mode determination — default to flow-through; pause only where operator judgment is actually needed

Before invoking `AskUserQuestion`, classify each Wave 1+2+3 surfacing into one of these treatments. The default is flow-through; pause is the exception, not the ritual.

| Surfacing class | Wave 4 treatment |
|---|---|
| NO-CONSENSUS items between lenses | `AskUserQuestion` required for that specific item |
| High-blast-radius actions (≥5 archives in one batch; CLAUDE.md or schema-file edits; /shift launches with significant scope; content-deletion of >1 file) | `AskUserQuestion` required for those specific items |
| Cartographer /shift recommendation with multiple equally-ranked options | `AskUserQuestion` required for the /shift pick only |
| Operator-decision-required items (canonical-flips on stories, role-record creation, new vocabulary additions, anything that mutates substrate gospel) | `AskUserQuestion` required for those specific items |
| Two-signature confirmed dispositions (tiny-delete, memorialize, graduate, graduate-pending where the disposition matrix made the call cleanly) | **Flow-through** |
| Unambiguous backlog/roadmap drafts (cartographer drafts are paste-ready; no operator decisions remain) | **Flow-through** |
| Everything else — low-blast-radius, unambiguous, non-gospel-mutating | **Flow-through** |

### When the entire cycle is flow-through

Announce in one compact message: "applying these now: [enumerated list]; surfacing the rest as Wave 6 substrate." Then proceed to dispatch librarian/cartographer for execution. The operator interrupts if they disagree — they don't need to be prompted. Most substrate-stable cycles will land here.

### When some items are flow-through and others require pause

Announce the flow-through items first ("applying X, Y, Z without question"), then `AskUserQuestion` for the pause-required items only. Don't pad the question set with categories that have nothing to decide.

### Question-set rules (when AskUserQuestion is required)

- Set is **derived from what Wave 1+2+3 actually surfaced** — not pre-canned
- Set is **as small as the cycle warrants** — one or two questions, not four-for-ritual
- No option pre-flagged as "recommended default" unless cartographer's per-cycle ranking genuinely produced a clear winner with no operator-decision dimension
- AUTHOR-CANONICAL-STORY items (when present from storyteller's Wave 2) surface as their own question class — present neutrally per the agency-and-clarity principle (see post-Run-#2 wisdom)

### Execute approvals serially (no parallel writes to `.claude/memory/`)

- **Librarian dispatched once** for all memory-tier dispositions (flow-through + operator-approved)
- **Cartographer dispatched once** for backlog/roadmap writes (or operator writes directly if cartographer drafts are paste-ready)
- **Inline actions** (e.g., cascade-root code fixes) dispatched to appropriate coding agent
- **Chronicle write deferred to Wave 6** (historian, after retrospective)

## Wave 5 — Retrospective (~5 min) — parallel dispatch

Dispatch all four agents in parallel with compact Wave 4 summary AND cartographer's Wave 3 stasis plan AND the Wave 6 achievement table (so retros can reckon against what was actually planned vs. delivered). Each reflects on:
1. What worked (1-2 sentences)
2. What hurt (1-2 sentences)
3. **Anti-bail self-reckoning** — Where did your lens contribute to bailing on the larger goal? Was the bail honest substrate-truth (the dimension genuinely couldn't move) or convenience (the defer was easier than the bite)? Name one specific case if applicable. Be self-honest — the substrate doesn't punish honest reckoning, but it does compound when bails go unexamined.
4. **Cross-substrate coherence reckoning (Run #5 retro)** — Did the cycle's gospel-touching actions produce cross-substrate sweeps, or did changes land in one substrate while citations elsewhere went stale? Specifically: did your Wave 1 / Wave 3 / Wave 6 work surface and resolve cross-substrate impact, or did you treat your substrate as an island? Cross-substrate coherence is the ceremony's job — single-substrate hygiene that orphans downstream citations is the failure mode the four-agent collaboration is supposed to prevent. Name one cross-substrate sweep that landed (or should have landed and didn't).
5. Signal candidates (max 2) — concrete hook/accumulator/cron with trigger + threshold + action
6. Cadence proposal — how often + what should trigger
7. Was Wave 2 operating shape (single-agent four-lens vs TeamCreate) the right call?

Output ≤ 300 words each (raised to accommodate items 3 + 4).

## Wave 6 — Stasis execution + chronicle (~10-20 min depending on dispatch scope) — orchestrator

Wave 6 is **execution**, not just synthesis. The stasis-progress invariant (locked in after Run #3's three-cycle silent demotion) requires every cycle to make measurable advance against each audited dimension that has a non-deferred target. Wave 6 lands those advances and the chronicle records the actual achievement, dimension by dimension.

### Phase 6a — Group dispatch per cartographer's stasis plan

Read cartographer's stasis implementation plan from Wave 3 output. For each dimension with `status: proposed` (i.e., not `deferred-with-rationale`):

1. Determine the resolution agent (librarian / storyteller / historian / /deliver).
2. Determine file-scope of the proposed action — what files will be touched.
3. Group dispatches by scope-conflict:
   - **Parallel** when scopes are disjoint (e.g., two librarian dispatches touching different CLAUDE.md files; storyteller backfill + librarian dedupe)
   - **Serial** when scopes overlap (e.g., two dispatches both editing MEMORY.md or the same agent prompt)
4. Dispatch with **anti-bail framing AND the row's impact map verbatim baked into the prompt**. Use this template (adapt per-dispatch):

   > "**Stasis-plan row #{N}: {dimension}.** Current: {value}. Target: {value}. 20%-floor: {value}. Proposed action: {action verbatim from Wave 3}.
   >
   > **Cross-substrate impact map (paste verbatim from Wave 3 §{N})**:
   > {paste the entire per-row impact-map block here — the files about to change, agents/skills/memory/stories that cite the affected content, sweep actions, no-action affordance}
   >
   > **Execution mandate**: Advance this dimension to at least its 20%-floor. **20% is the worst acceptable outcome, not the target — 50% bite is the default ambition when your lens supports it.** Execute the proposed action AND the sweep actions in the impact map. Do not treat the named-file edit as sufficient — the row's full scope is edit + sweep.
   >
   > **Do NOT return with 'I propose to do X.' Return with X executed.** If scope feels too large to fully execute in one dispatch: split it. Do enough this cycle to clear the 20%-floor (which now includes a baseline sweep across cited substrate); surface the remainder as a backlog entry. Returning with 'this is too big to attempt' is not an available outcome — return with the executable bite plus the split-for-next-cycle plan.
   >
   > **For each impact-map sweep target**: confirm the citation is still valid after your edit OR update it OR surface as judgment-call to operator. Mark each target verified-no-change / updated / surfaced in your return report — do not silently skip.
   >
   > **Over-delivery on adjacent dimensions is encouraged.** If you are in a CLAUDE.md file fixing drift, sweep neighboring dead-path noise in the same edit — same blast radius, more value. If you are tightening MEMORY.md entries, look for stale memorializations to graduate while you're there. The chronicle records actual achievement; under-delivery is a quality signal Wave 5 will examine."

Each agent executes to achieve at least the 20% threshold for their dimension; the dispatch prompt instructs them to aim higher when their lens supports it. **Particularly for the librarian** (resolver for most stasis-plan rows): the dispatch prompt arrives with concrete diffs from Wave 3 **AND the row's impact map verbatim**, so execution is "apply this paste-ready set + sweep these specific citations in these specific files," not "go decide what to do."

The impact map is the breadcrumb trail produced by librarian's Wave 1 fact-finding + historian's precedent surface + cartographer's Wave 3 organization. Phase 6a's job is to preserve it through the handoff into execution — not to compress it back into a generic action description.

### Phase 6b — Confirm achievement

After dispatches return, re-run the relevant audits to measure post-stasis state:

| Dimension touched | Audit to re-run |
|---|---|
| CLAUDE.md drift counts | `.claude/scripts/memory-kit/claude-md-audit.py` |
| Cleanup-scan flags | `.claude/scripts/memory-kit/cleanup-scan.py` |
| Agent catalog findings | `.claude/scripts/memory-kit/agent-audit.py` |
| Skill catalog | `.claude/scripts/memory-kit/skill-audit.py` |
| Story coverage | `.claude/scripts/memory-kit/story-coverage-audit.py` |
| MEMORY.md size | `.claude/scripts/memory-kit/memory-review.py` |
| Surface:Archive ratio | `genesis/scripts/memory-balance.sh` (also Phase 6c) |
| Delivery pickup queue | `delivery-status-poll.py` if present, else read `/deliver` sprint-results |

Measure each dimension's new value vs target. Mark each dimension as one of:
- **achieved** — advanced past 20%-floor or hit target
- **partial** — advanced but not past 20%-floor (rare; investigate why dispatch under-delivered)
- **deferred-with-rationale** — from Wave 3 plan; rationale carries forward

A `partial` outcome is a quality signal — Wave 5 retrospective should examine why. Either the 20%-floor was unrealistic this cycle (re-calibrate threshold), the dispatch scope was off (re-calibrate Wave 3), or the agent under-delivered (signal candidate for next ceremony).

### Phase 6c — End-state balance sheet

```
genesis/scripts/memory-balance.sh
```
Persists end-state snapshot; diff column reflects post-stasis state against Wave 0 baseline.

### Phase 6d — Post-ceremony coherence verification (Run #5 retro)

The ceremony's actual success metric is NOT "did the audit numbers move" — it's **"is the substrate now a good context-primer for the next implementation sprint?"** Audit numbers measure islands; sprint agents read across islands. Coherence verification is the cross-island check.

Workflow:

1. **Identify the substrate touched this cycle**: list every file Wave 6 mutated (Phase 6a return reports give you this — CLAUDE.md files, agent definitions, skill descriptions, memory entries, stories).

2. **Identify 1-2 plausible downstream sprint topics**: what implementation sprint might fire next? Pull from cartographer's Wave 3 backlog drafts or the recommended /shift Objective. Examples: "feature authoring for stewarded-device-sync", "qahal collective-governance scenarios", "doorway SSR resource floor."

3. **Dispatch a research subagent** (Explore, fresh context — NOT one of the four memory agents, which are inside-the-ceremony and may carry confirmation bias):

   > "You are a downstream implementation-sprint agent about to start work on **{topic}**. Read these files as your primed context:
   > - MEMORY.md
   > - {affected CLAUDE.md files}
   > - {affected agent definitions, if topic-relevant}
   > - {affected skill descriptions, if topic-relevant}
   > - {affected memory entries, if topic-relevant}
   > - {relevant stories/backlog entries}
   >
   > Report: (a) any contradictions across these files that would mislead your sprint? (b) any gaps where the context references something you cannot find? (c) any stale citations (e.g., a CLAUDE.md mentions a pattern, an agent prompt cites the pattern, but the pattern was just removed)? (d) any redundancies where the same fact lives in N places, and you can't tell which is authoritative?
   >
   > You are NOT executing the sprint — you are evaluating whether the context primes you well or poorly. Return a coherence score: GREEN (clean, no issues), YELLOW (minor noise, recoverable), RED (contradictions or gaps that would derail the sprint). Cite specific files + line evidence for any non-GREEN finding."

4. **Read the verification report**. If GREEN: chronicle records the coherence check passed. If YELLOW: chronicle records the noise as a Wave 5 retro signal AND the librarian closes the noise inline if mechanical. If RED: **the ceremony is not done** — re-dispatch the relevant Phase 6a agent to resolve the contradictions before Phase 6e chronicle write. RED outcomes are the highest-value signal the ceremony can produce; they mean the cross-substrate sweep missed something and the next sprint would have suffered for it.

5. **Optional: rotate downstream topics across cycles**. Each ceremony's coherence check picks 1-2 topics from cartographer's backlog drafts; over time, the coverage of "context that would prime a sprint well" expands. The check is sampling, not exhaustive — but it's the closest thing to a downstream-test the ceremony can produce without actually running a sprint.

Phase 6d is the gate between "we changed substrate" and "we changed substrate well." Without it, the ceremony has no feedback loop on whether the cross-substrate sweep was sufficient — Wave 5 retros are self-reckoning (useful but inside-the-cycle), while Phase 6d is independent observation by a fresh-context agent (the closest proxy we have for the next sprint's experience).

### Phase 6e — Historian writes chronicle

Standard chronicle frontmatter + body per `genesis/data/timeline/CONVENTIONS.md`.

**Required new section — "Stasis-progress per dimension"** (added Round 4):

```
## Stasis-progress per dimension

| Dimension | Target | Starting value | Ending value | Actual % advance | Status | Rationale (if deferred) |
|---|---|---|---|---|---|---|
| CLAUDE.md DRIFTED-FACTUAL count | 0 | 18 | 13 | 27.8% | achieved | — |
| CLAUDE.md OVER-BUDGET count | 0 | 2 | 1 | 50% | achieved | — |
| Cleanup-scan review flags | 0 | 67 | 51 | 23.9% | achieved | — |
| Agent catalog real findings | 0 | 3 | 1 | 66.7% | achieved | — |
| Skill catalog overlap pairs | 0 | 1 | 0 | 100% | achieved | — |
| Story orphan-ratio | 0 | 0.42 | 0.42 | 0% | deferred | Storyteller-coverage-sprint owns; ≤3/cycle ceiling |
| Story sourcing-completeness flags | 0 | 2 | 0 | 100% | achieved | — |
| MEMORY.md byte size | ≤24,400 | 30,025 | 28,800 | 21.7% | achieved | — |
| Surface:Archive ratio | <100:1 | (current) | (post) | (%) | (status) | — |
| /deliver pickup queue | drains | (current) | (current) | 0% | deferred | No /deliver-ready features this cycle |
```

The chronicle is now an honest record of what advanced and what didn't, with explicit deferred-with-rationale for anything that fell short. If a dimension shows `partial` status, the rationale column explains why (under-delivered dispatch / unrealistic floor / blocker discovered mid-execution).

**Required new section — "Cross-substrate coherence verification"** (added Run #5 retro): the chronicle records Phase 6d's coherence check result for each sampled downstream topic — topic, files-read, score (GREEN/YELLOW/RED), specific findings if non-GREEN, and resolution (closed inline / surfaced for Wave 5 / re-dispatched). This is the ceremony's audit-of-the-audit: did the substrate actually improve as a sprint-primer, or did we just move numbers?

### Phase 6f — Next-ceremony substrate

Consolidate Wave 5 outputs into:
1. 3-5 proposed hooks/automations (concrete trigger + threshold + action)
2. Cadence recommendation (synthesized)
3. New memory entries to write (draft `feedback_*` or `project_*` capturing wisdom)
4. Agent prompt updates (concrete diffs)
5. Final state diff including balance-sheet snapshot

**Delivery-status transitions this cycle**:

```
| Artifact | Before | After | Source | Notes |
|---|---|---|---|---|
| story:james-and-the-spoke | undelivered | undelivered | deliver-bridge-floor | feature not yet authored; delivery-debt flag remains |
| backlog:write-stewarded-device-sync | (new) | refined | cartographer | spawned from delivery-debt flag |
| (future row when /deliver judges X) | wip | active.alpha | deliver-bridge | first tier-3 delivered verdict |
```

Entries where `delivery_status` did not change are not listed (output-discipline). Surface `graduation-delivery-gap` count for the next-ceremony's librarian to triage.

Present consolidated summary to operator. Dispatch historian to write `genesis/data/timeline/chronicle/YYYY-MM-DD-<slug>.md` with the consolidated report. Include both the balance-sheet table AND the stasis-progress per dimension table — together they form the evidence that the ceremony advanced the substrate.

## End state criteria

- All Wave 4-approved actions executed
- Wave 2 team dissolved cleanly (if TeamCreate was used)
- **Wave 6 stasis dispatches completed** (no agent left mid-execution; each `proposed` dimension has been touched)
- **Chronicle contains the stasis-progress per dimension table** (the new Round 4 invariant)
- **Any deferred dimensions have explicit rationale recorded** in the chronicle's stasis-progress table
- Chronicle entry exists with balance-sheet table AND stasis-progress table
- Retrospective memory entries written
- Final balance-sheet snapshot persisted (next ceremony's prior)
- All TaskCreate tasks closed

Operator should then have a populated backlog ready to feed `/shift`, concrete signal-driven automation proposals, and deterministic delta evidence over the last cycle — including measurable stasis-progress per audited dimension.

## Known footguns

- **One team per lead at a time**: if Wave 2 uses TeamCreate, the team must dissolve before Wave 3 dispatches.
- **Single-agent four-lens ventriloquy risk**: storyteller running inline four-lens may compress the least-fluent lens (historian's forensic voice in first ceremony). Compensate by reading peer agent definitions explicitly before the inline debate.
- **Convergence bias**: three-perspective convergence is strong cascade-root signal but down-ranks forward-leaning items only cartographer sees. Apply 0.4/0.4/0.2 weighting (convergence/vision/quiet).
- **MEMORY.md re-index grows the file**: folding orphan files in adds bytes; tightening alone can't claw it back. Real compression is umbrella consolidation or graduation, not entry-tightening.
- **Path resolution self-reinforcement**: if a script writes to a buggy doubled-path, the next walk-up may satisfy on that artifact. Co-anchor on two markers (`.claude/` + `.git/`), never one. See [[feedback_self_reinforcing_path_bug_class]].
- **Anti-bail discipline**: `deferred-with-rationale` is bounded (max 2/cycle, 3-cycle clock, narrowed valid reasons) precisely because the silent failure mode is bailing on hard dimensions while reporting "explicit defer not silent." Run #3's three-cycle silent demotion taught this; Run #4's 3-of-6 cleared-bar was the calibration point. The 20%-floor exists so near-100%-coverage cycles still make measurable advance — bailing on the floor IS the regression the ceremony was written to prevent. The aggressive-bite framing in Wave 3 + the anti-bail dispatch template in Phase 6a + the anti-bail self-reckoning in Wave 5 are three reinforcing surfaces; weakening any one of them lets the failure mode return.
- **CLAUDE.md gospel-edit deferral**: historically the easiest dimension to bail on (cited "operator pre-approval gate" as a defer reason). Cartographer must draft concrete diffs in Wave 3, Wave 4 absorbs them as one flow-through-ready question, Wave 6 applies. Drafting + approval-gate + apply is procedure, not blocker — and CLAUDE.md drift is low-hanging fruit that should get attention every cycle until it converges.

## Related

- `.claude/scripts/memory-kit/LIFECYCLE.md` — ownership matrix + cadence + disposition definitions
- `.claude/scripts/memory-kit/CLAUDE.md` — substrate overview
- `.claude/skills/memory-kit/SKILL.md` — librarian's solo toolkit
- `.claude/skills/converge/SKILL.md` — cartographer's solo toolkit
- `.claude/agents/{librarian,historian,storyteller,cartographer}.md` — agent definitions
- `.claude/memory/feedback_first_memory_team_ceremony.md` — wisdom from inaugural ceremony
- `.claude/memory/feedback_memory_balance_sheet_pattern.md` — balance-sheet artifact
- `genesis/data/timeline/CONVENTIONS.md` — timeline schema
- `genesis/data/stories/CONVENTIONS.md` — stories schema
