---
status: proposal
date: 2026-05-10
---

# `/converge` Skill Design — Closing the Dreaming → Execution Loop

**Status**: proposal
**Date**: 2026-05-10
**Sibling artifacts**:
- Memory lifecycle spec: `genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md`
- Living memory epic: `genesis/docs/content/elohim-protocol/living_memory/epic.md`
- Memory-kit toolkit (consumes its outputs): `.claude/skills/memory-kit/SKILL.md`

## Why

The `memory-kit` toolkit produces six dated reports per sweep — `cleanup-backlog-refresh.md`, `dedupe-clusters.md`, `plan-status.md`, `sprint-digest.md`, `path-update-proposals.md`, `skill-audit.md`. Each surfaces real engineering signal:
- BACKLOG items still relevant but cold
- Concepts that should merge
- Plans cooling mid-stream with open items
- Aggregated open questions across sprints
- Path drift from active restructures

But the operator currently has to **manually weave these into a coherent next-step**. That weaving — finding the trajectory across themes, merging redundant designs, surfacing what's designed-but-outstanding, updating the canonical plan — is exactly the synthesis the dreaming layer was built for. Without it, the toolkit is a diagnostic dashboard but not a forward step.

`/converge` is the missing tool. It consumes the toolkit's outputs and **updates the canonical plan(s) so they're crystal-clear, concise, and ready for `/shift` or `/deliver` to pick up**. Each invocation moves the corpus forward: completed tasks marked done, outstanding tasks ordered by readiness, redundant sub-plans merged, newly-surfaced questions added.

The end-state vision: if `/converge` runs after every `memory-kit` sweep and `/shift` runs against its outputs, **unimplemented planned work asymptotically approaches zero**. Closed loop.

## What it does (in one sentence)

For each thematic cluster of in-flight work, find the canonical plan, synthesize what's done / designed-but-outstanding / mergeable / obsolete from the toolkit reports + git history + sprint trajectory, and propose specific edits to that plan so the next executor sees a current, ordered, ready-to-act picture.

## Scope

`/converge` operates on **plans** (mostly under `genesis/docs/superpowers/plans/`). It updates them. It does NOT modify specs, memory entries, or sprint-results — those have their own lifecycles handled elsewhere.

It is **operator-gated**: every plan edit is reviewed and accepted before being applied. Running `/converge` produces proposals; `apply` is a separate explicit step.

## The session-start UX (the end-state this enables)

Human starts a Claude Code session and types **"what's next?"**. The agent reads the latest `converge` output and responds:

```
Here are the plans we've refreshed and prepared, ranked by vision-alignment × readiness.
Top recommendation: iroh delivery cutover — gate 4 (cross-stack recovery e2e).
  Vision-alignment: 9/10 — substrate transition, 1-2 cycles to canonical
  Readiness: 8/10 — worktree exists; tasks 1-3 done; 4-7 outstanding with no blockers
  Pre-authored Objective: "Land iroh-recovery-e2e cross-stack tests on dev; gate 4 of cutover."
  Estimated cycles: 1-2 shifts

Other ready plans:
  2. iroh-seeder dual-write (Plan 4 Tasks 4-10) — vision 9, readiness 7
  3. iroh-phase12 peer-transport-manifest (codegen + merge) — vision 8, readiness 9
  4. recovery-M5 auth-portal convergence — vision 7, readiness 5 (blocked by M3 schema review)

Pick one to /shift, or 'show me #N' for details.
```

Human picks → agent invokes `/shift` (or `/deliver` for delivery-shaped work) with the pre-authored Objective.

This is the closed loop the user named: *"I just kept going until everything that was planned was delivered."* The dream cycle (`memory-kit`) feeds the synthesis (`converge`), the synthesis feeds the menu, the menu feeds execution, execution feeds back into the corpus, the next dream cycle picks up from there. Asymptotic convergence to delivered.

## Three phases (mirrors `/cleanup` shape)

### Phase 1: theme detection (deterministic)

Read all memory-kit outputs from the latest dated directory. Cluster items by shared keyword frequency:
- BACKLOG items grouped by recurring keywords in their "what's unfinished" text
- dedupe-clusters grouped by their shared key terms
- plan-status cooling items grouped by plan-name themes
- sprint-digest cross-cutting themes (already extracted)
- path-update changes grouped by repo-path-prefix
- Aggregated open questions grouped by mentioned subsystem names

Output: `convergence-themes.md` — a list of detected themes with: theme name, contributing items from each report, the candidate canonical plan(s) for that theme (most-recent-modified plan whose name contains the theme keyword).

This phase is pure rule-based. No LLM needed for detection.

### Phase 2: per-theme synthesis (subagent dispatch)

For each detected theme above a relevance threshold (e.g. ≥3 contributing items across reports), dispatch a `general-purpose` subagent (opus). Each subagent gets:
- The theme name and contributing items
- The candidate canonical plan path
- Read access to the full corpus

Each subagent does:
1. Read the canonical plan in full
2. Read all contributing items (their source files, not just the report excerpts)
3. Walk recent git log for the theme's repo paths
4. Walk recent sprint-results mentioning the theme
5. Walk recent dev-intent.jsonl entries
6. Synthesize a structured proposal:
   - **Mark-done**: tasks in the plan whose deliverables are now in the repo (with evidence: file paths, commit hashes, scenarios passing)
   - **Add-as-outstanding**: tasks that have been designed (in specs/dev-intent/sprint open-questions) but aren't yet in the canonical plan, ordered by readiness (no blockers first)
   - **Merge-redundant**: sub-plans or sections that overlap and should collapse, with the merged proposal text
   - **Remove-obsolete**: tasks whose context is gone (e.g. superseded by an entirely different approach), with evidence
   - **Surface-question**: open questions across reports that need a decision before the plan is executable

Each subagent writes its proposal to `.claude/memory-kit/<YYYY-MM-DD>/converge/<theme>-proposal.md`.

### Phase 3: operator review + apply

Operator reviews each per-theme proposal. Marks `- [x] Accept` on edit blocks they approve. Apply step:
- For each accepted edit block, performs the edit in the canonical plan file
- Mark-done: replaces `- [ ]` with `- [x]` and adds an evidence comment
- Add-as-outstanding: inserts new `- [ ]` items in the right section
- Merge-redundant: replaces multiple sections with the merged version
- Remove-obsolete: comments out (preserves trajectory) rather than deletes
- Surface-question: adds an "Open Questions" section if not present, appends the question with citations

Apply preserves the plan's existing structure and frontmatter.

### Phase 4: the "what's next" menu

After per-theme synthesis (and optionally after operator-applied plan edits — for highest accuracy), converge produces a ranked **next-actions menu** at `.claude/memory-kit/<YYYY-MM-DD>/next-actions.md`. This is the surface the session-start UX consumes.

For each refreshed plan that's ready-to-execute, compute two scores:

**Vision-alignment (0-10)** — how directly does completing this plan advance the manifesto / canonical epics?
- Read `genesis/docs/content/elohim-protocol/manifesto.md` and the relevant epics
- Check whether the plan's deliverables are explicitly named in vision documents (high alignment) vs are infrastructure/enablement (medium) vs are tooling/hygiene (lower)
- This is judgment-laden; the synthesis subagent assigns the score with brief reasoning

**Readiness (0-10)** — how close is this plan to "an agent can pick this up and ship"?
- All blockers explicitly resolved or absent (high)
- Worktree or branch exists with partial work (high)
- Open items are well-scoped and ordered (high)
- Open items have unresolved design questions or external dependencies (low)
- Plan has no checkbox structure / unclear next step (low)

**Recommendation = vision × readiness**, sorted descending. Top 3-5 surface in the menu.

Each menu entry includes:
- Plan path
- Both scores with one-sentence reasoning each
- A **pre-authored Objective** (1-2 sentences, ready to drop into `/shift` or `/deliver`)
- Estimated cycles (sprints) to completion based on open-item count + recent velocity from sprint-distill
- Any blockers (if readiness < 8)
- Recommended skill (`/shift` for build work, `/deliver` for delivery-shaped work)

The menu is not interactive — it's a markdown file. The session-start UX is "agent reads the file and presents it conversationally." This keeps the data-layer simple and operator-readable.

## File locations

- Skill: `.claude/skills/converge/SKILL.md` (extracted from memory-kit 2026-05-13 — open question answered: own dir)
- Scripts: `.claude/scripts/converge/converge-scan.py` + `.claude/scripts/converge/converge-apply.py`
- Per-theme proposals: `.claude/memory-kit/<YYYY-MM-DD>/converge/<theme>-proposal.md`
- Theme report (Phase 1 output): `.claude/memory-kit/<YYYY-MM-DD>/convergence-themes.md`
- **Next-actions menu (Phase 4 output)**: `.claude/memory-kit/<YYYY-MM-DD>/next-actions.md` — the "what's next" surface

## Session-start integration

The "what's next" UX requires almost nothing new on the agent side — it's a convention, not a skill:

- Human asks "what's next" (or similar); agent reads `.claude/memory-kit/<latest-date>/next-actions.md` (find latest dated dir under memory-kit)
- If the file is older than the current week, gently suggest a fresh `/memory-kit + /converge` cycle first — the menu's value decays with corpus drift
- Present the top-3 ranked items conversationally; offer details on demand
- On selection, invoke `/shift` or `/deliver` with the pre-authored Objective

This keeps the integration simple: no new always-loaded skill, no harness changes. The convention is documented in the memory-kit SKILL.md and (eventually) in CLAUDE.md as the recommended session-start flow.

## Why this closes the loop

Each `memory-kit` + `/converge` + `/shift` cycle:
1. **dreaming**: memory-kit surveys the corpus, surfaces what's stale, unfinished, mergeable
2. **converge**: synthesizes the trajectory; updates canonical plans so they're current and ordered; ranks by vision × readiness; pre-authors Objectives
3. **session-start UX**: human asks "what's next?"; agent presents the ranked menu; human picks or accepts the top recommendation
4. **execution**: `/shift` or `/deliver` picks up the chosen plan with the pre-authored Objective and ships against it
5. New sprint-results feed back into the corpus
6. Next dreaming cycle picks up from the new state

Unimplemented planned work shrinks each cycle. The corpus converges to delivered.

The vision named by the user: *"I just kept going until everything that was planned was delivered (no more unreviewed plans outstanding marked unimplemented)."* AND *"hey what's next" → here's the plans we dreamed about ready to go, here's the most important to reach the vision, review, and go.* This is the closed-loop autonomous-agent end-state.

## Boundary discipline

- ONLY modifies plans. Not specs, not memory entries, not sprint-results, not skills.
- ONLY surfaces edits as proposals. Apply is explicit and operator-gated.
- The synthesis subagents are read-mostly. They write only their per-theme proposal file.
- No new tasks invented from thin air — every "add-as-outstanding" cites a source (spec section, dev-intent entry, sprint open-question).
- "Remove-obsolete" never deletes; it comments out with a reason. Preserves plan trajectory.

## Open questions (to resolve before building)

1. **Living next to memory-kit, or separate skill?** Two reasonable options:
   - Sub-doc within memory-kit (single skill, `/memory-kit converge` invocation pattern)
   - Separate skill (`/converge` standalone, but always-loaded description bloat)
   I lean toward sub-doc within memory-kit — the kit's narrative is "weekly hygiene, then converge into next plan." One skill description.

2. **Theme detection threshold.** What counts as a "theme worth synthesizing"? ≥3 contributing items across reports? ≥2 if they're high-confidence? Tunable via arg.

3. **What's a "canonical plan"?** When multiple plan files share a theme keyword, which is canonical? Heuristic: most-recently-modified plan with the keyword in name AND in active state (per plan-status). May need explicit operator hint for ambiguous cases.

4. **How aggressive is mark-done?** A task with shipped code in the repo is clearly done. But a task whose deliverable is "draft design doc X" — is the draft sufficient, or does it need review-passed status? Conservative default: only mark done when the deliverable is unambiguous (file exists at expected path; scenario passes; commit message references the task).

5. **Cross-theme merges.** What if two themes are actually one (e.g. "iroh" and "iroh-recovery" should be one umbrella theme)? Phase 1 surfaces both; Phase 2 might propose collapsing them. Worth handling but adds complexity. Defer to v2.

6. **Plan structure assumptions.** The apply step assumes plans have checkbox-list structure. Plans without checkboxes (`no-checkboxes` per plan-status) are out of scope for v1.

## Acceptance criteria for v1

- Phase 1 scan correctly identifies themes from a fresh memory-kit sweep
- Phase 2 produces actionable per-theme proposals with citations on every claim
- Phase 3 apply preserves plan structure; idempotent; reversible from git
- Running it on the current corpus produces at least one usable proposal for the iroh theme (the most-active current cluster)
- Operator can read a per-theme proposal in <5 minutes and decide which edits to accept

## Future directions (post v1)

- **Cross-theme synthesis**: detect when two themes should converge into one
- **Auto-promotion of new specs**: when a sprint produces something genuinely new not yet in any plan, propose a new plan stub
- **Backwards trajectory walk**: from a delivered feature, walk backward through plan→spec→memory to identify which earned its way upstream
- **Convergence delta tracking**: per-cycle, report "we converged N tasks this cycle" — visible progress toward the end-state

## What this is NOT

- Not autonomous (operator approval at apply)
- Not a planner (doesn't generate new work; consolidates existing work)
- Not a spec editor (only plans)
- Not a code editor (only plan markdown)

## Sources

- Memory lifecycle design (sibling spec)
- Memory-kit SKILL.md and forthcoming kit usage patterns
- User's articulation of the closed-loop vision: dreaming → execution → completion, asymptotic to zero unimplemented planned work
