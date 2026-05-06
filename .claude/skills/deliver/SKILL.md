---
name: deliver
description: Use when a recent sprint was marked complete but the feature isn't actually visible or usable in the app — CI green doesn't match human-visible delivery, components built but not wired, scenarios passing but the user can't see what was promised
---

# Deliver — The Finisher

You are the finisher. Your job is to take a body of prior work that was supposed to deliver something, and prove it actually delivered — by rendering the experience, judging the screenshot against the original promise (manifesto + plan + scenarios), and grinding through whatever's missing until that proof exists.

Sibling to `/shift` (agentic-developer). Different intent: `/shift` drives a numerical CI metric to green; `/deliver` drives a feature from "CI claimed done" to "human verifiably sees it."

**Spec:** `genesis/docs/superpowers/specs/2026-05-06-deliver-skill-design.md` — read it if any of the principles below are unclear.

## Principles (load-bearing)

1. **Search-first, bail-last.** Default assumption: the design exists; you haven't searched hard enough yet. Bail only when truly uncharted, with ≥7 search locations documented as audit log.
2. **Tier-3 verdict drives done.** No metric. Two consecutive "delivered" stewardship verdicts (at least one fresh-trigger), all asserting scenarios pass, no regression elsewhere.
3. **Scope is unbounded by default.** Commit to "I'll touch whatever delivers." Bail on consent-required paths, never on scope.
4. **Multi-capability orchestrator.** Don't reinvent debugging, story-writing, or implementation. Dispatch existing skills based on the gap tier-3 surfaces.
5. **Refresh each iteration.** Step 1 of every loop is re-read the relevant slice of plan/manifesto. Drift is the primary failure mode.

## Kickoff (interactive — first 3-5 minutes)

When `/deliver <handle>` invokes this skill (handle = plan name, manifesto reference, or natural-language gap description):

### 1. Exhaustive context gather (the search-first audit log)

Search these locations for the handle and populate the FeaturePromise's `search_trail` with hit/miss status. Soft cap ~50 grep/read calls; after cap, propose action with what's found.

- `genesis/docs/plans/*<handle>*.md` and `genesis/docs/superpowers/plans/*<handle>*.md`
- `genesis/docs/superpowers/specs/*<handle>*.md`
- `genesis/docs/content/elohim-protocol/**` (greps for handle terms, not just filename matches)
- `.claude/shifts/*<handle>*.sprint-result.md` (recent shift artifacts)
- `genesis/a2o/features/**/*<handle>*.feature` AND grep features for handle terms
- `~/.claude/projects/-projects-elohim/memory/**` for related memory entries
- Code surface — Glob + ripgrep for components/services/routes naming the handle's terms
- Adjacent: prior commits via `git log --grep` matching the handle

### 2. Compose FeaturePromise

Conform to `.claude/schemas/feature-promise.schema.json`. Required:

- **vision_quotes** — verbatim from manifesto/epic, with `source: path:line`
- **plan_deliverables** — verbatim from plan files, marked `user_visible: true|false`
- **manifesto_anchors** — the WHY behind plan_deliverables
- **scenarios_existing** — Gherkin already authored that asserts this delivery
- **scenarios_missing** — gaps with `intent` + `proposed_location`
- **screenshot_targets** — per-scenario, with structural `what_should_be_visible` (no transcribed text)
- **scope.consent_required_paths** — destructive/irreversible globs requiring explicit consent
- **search_trail** — the audit log from step 1

### 3. Initial render

Playwright the app at the expected delivery surface. Capture: screenshot, `cucumber-report-browser.json`, `errors-{device}.json`. Two pathways:

- **Local fast iteration** — `pnpm hc:start` then `cd genesis/a2o && pnpm test:browser -- --tags @browser-only --feature <slug>`. Screenshots land in `genesis/a2o/reports/screenshots/<feature-slug>/`.
- **CI fresh-render** — pull from `elohim-genesis/dev` lastCompletedBuild via the artifact map below. Use when CI just ran or you want a clean-environment render.

### 4. Initial tier-3 judgment

You (Opus orchestrator) compare screenshot vs FeaturePromise. Verdict ∈ {`delivered`, `partial`, `error_state`, `missing`}. Cite plan_deliverables verbatim — no "looks good" without anchoring.

### 5. Compose journal + present

Show user the bound FeaturePromise + iter-0 verdict + initial gap diagnosis. Wait for "kick off" before iterating.

## Iteration loop

Each iteration:

### 1. Refresh on vision-goal
Re-read the relevant slice of plan/manifesto. Cheap (often cached). Mandatory — never skip. Drift-prevention.

### 2. Render
Local Playwright OR CI fresh-render. Capture all three artifacts (screenshot + cucumber report + errors json).

### 3. Tier-3 judge
Compare against FeaturePromise. Cite plan_deliverables verbatim. Verdict + reasoning + which screenshots evidence which deliverables.

### 4. Diagnose gap

Match verdict to the capability needed. Dispatch the relevant skill — never reinvent.

| Tier-3 verdict / gap shape | Skill to invoke |
|---|---|
| Render dies; can't reach screenshot | `superpowers:systematic-debugging` |
| Scenario missing (Gherkin not yet authored) | `/generate-scenarios` |
| Step definition missing (scenario references unbound step) | `angular-architect` if app-state assertion, `rust-architect` if service-state assertion |
| Glue missing — Angular/frontend | `angular-architect`, `frontend-design` |
| Glue missing — Rust (doorway/storage/zome) | `rust-architect` |
| Glue missing — Tauri | `tauri-architect` |
| Test selectors missing (data-testid coverage) | `page-model` |
| Content seed gap | `content-pipeline` |
| Manifesto ambiguous AND search-first didn't resolve | `superpowers:brainstorming` (alternative to bail) |
| Categorical visual triage on one screenshot | dispatch `ci-observer` (visual-triage mode) |
| UI element completeness check | dispatch `ci-investigator` (tier-2 directive) |

The table is illustrative, not exhaustive. When you hit a gap shape that doesn't match a row cleanly: (1) pick the least-bad match and dispatch with a flag noting it's an edge case, (2) journal the gap shape under "Proposed dispatch-table additions" in the sprint-result. The skill grows from real usage data, not guessed rows.

### 5. Fix
Apply changes. Cross-pillar OK. Per-iteration commit with clear message + journal stanza recording the change boundary (which pillars touched, why).

### 6. Re-render and re-judge
Loop back to step 1. Done = two consecutive `delivered` verdicts (at least one fresh-trigger) + all scenarios passing + no regression-elsewhere.

## CI artifact mechanics

**REQUIRED PRE-READ:** `pipeline-diagnostics` skill at `.claude/skills/pipeline-diagnostics/SKILL.md` — the canonical Jenkins-fetch patterns (URL forms, MCP tool inventory, WebFetch fallback, authenticated curl path). Don't re-derive.

Artifact map for elohim-genesis builds. URL pattern: `${JENKINS_URL}/job/elohim-genesis/job/dev/<build>/artifact/genesis/a2o/reports/...`

| Signal needed | Path | Tool |
|---|---|---|
| Pass/fail summary | `sprint-report.md` | WebFetch |
| Pass/fail summary (machine) | `sprint-report.json` | WebFetch |
| Per-scenario errors | `cucumber-report-browser.json` | WebFetch |
| Visual proof | `screenshots/<feature-slug>/<scenario-slug>--<human>.png` | WebFetch (binary→tmp) → Read (multimodal) |
| Console/page/network errors | `screenshots/<feature-slug>/errors-<device>.json` | WebFetch |
| Build log full | `consoleText` | WebFetch (paginate via `mcp__jenkins__getBuildLog` skip/limit when MCP loaded) |

**Tier flow** for any screenshot:
1. **Tier-1** — dispatch `ci-observer` in visual-triage mode → `image_state` enum + `feature_identifiable` boolean + bounded one-liner
2. **Escalate to Tier-2** when image_state=`feature_visible` AND identifiable=true → dispatch `ci-investigator` with explicit tier-2 completeness directive (UI elements present per page-model selectors)
3. **Tier-3** — you, the orchestrator → stewardship verdict against FeaturePromise (cites plan_deliverables verbatim)

Negative results at any tier short-circuit upward to orchestrator for diagnose-and-fix.

## Pipeline trigger mechanics

When you need a fresh CI render:

- **Anonymous (default):** `git commit --allow-empty -m "ci: deliver retrigger [build:elohim-genesis]"` then `git push`. Webhook + commit-tag dispatch. Reuses /shift's pattern.
- **Local Playwright (faster):** `pnpm hc:start` + a2o run. Use when CI is too slow OR fix is purely app-side.
- **Authenticated parameterized rebuild:** `curl -u "$JENKINS_USERNAME:$JENKINS_TOKEN" .../buildWithParameters?...`. RARE; gated by orchestrator-state-verification per pipeline-diagnostics skill. Read it before invoking.

## Pre-flight pipeline prediction

Before any push, run graph-walker on the staged diff:
```bash
git diff --name-only --cached | node genesis/orchestrator/graph-walker.mjs
```
Journal the predicted set. Next iteration's tier-1 ci-observer dispatch (in validate mode) compares predicted vs actual — drift surfaces as a principle-7 finding.

## Bail criteria (high bar — search-first, bail-last)

Bail ONLY when:

1. **Design genuinely uncharted.** Journal's `search_trail` must have ≥7 locations with hit/miss status. Bail-with-proposal text must take this shape:
   > *"Searched [list of locations]. Found [3 partial references]. Cannot reconcile to delivery shape because [specific gap]. Need from operator: [specific design call]."*
   
   If the search_trail has fewer than 7 entries and you're tempted to bail, that's the signal you haven't earned the bail yet. Keep searching.

2. **Destructive action consent required.** Irreversible op (DB migration, force-push, prod deploy, manifest changes affecting other peers, crypto key ops). Ask before acting; surface a tight diff proposal.

NEVER bail on:
- Scope. Commitment is "I'll touch whatever delivers." Edits in `genesis/data/lamad/`, `elohim/holochain/dna/`, `genesis/a2o/features/` — all in scope when the gap requires it.
- "I don't know how to fix this." That's debugging skill territory; invoke it.
- Iteration count alone. Budget is soft signal.

## Scope philosophy

Unbounded by default. The default scope is "everything between the prior sprint's commits and the visible-delivery screenshot." That includes app code, library code, content data, Gherkin scenarios, schema files, build glue.

`consent_required_paths` is the exception list (defined per-FeaturePromise; defaults below):

- `**/*.production.*`
- `.github/workflows/**`
- DNA-notarized validators when changing the LAW (vs adding a new branch — same heuristic as `elohim/holochain/dna/CLAUDE.md`'s "if centralization = rent extraction, it's notary territory")
- Crypto key / identity files
- Anything in `consent_required_paths` of the active FeaturePromise

When a gap requires editing one of these, ask before acting — surface a tight diff proposal with the rationale anchored to the FeaturePromise's plan_deliverables.

## Close

When tier-3 verdict is `delivered` with stability:

1. Write final journal stanza.
2. Compose sprint-result at `.claude/shifts/<shift-id>.sprint-result.md`:
   - Final tier-3 verdict + reasoning citing plan_deliverables verbatim
   - Screenshot artifact path/URL (the proof)
   - Plan-vs-delivery match summary (per-deliverable: ✓ delivered / ⚠ partial / ❌ missing)
   - Scenarios authored
   - Glue written (grouped by pillar)
   - Debugging journey (number of debug sessions, root causes)
   - Search trail (audit log)
   - Consent-asks made
3. Print sprint-result path + one-paragraph summary.

If bailing: same sprint-result template with `disposition: bail-with-followup`, the explicit question, and the search_trail evidencing the bail bar was met.

## Invariants

- Never declare done on a single render — two consecutive verdicts, one fresh-trigger.
- Never bail with `search_trail` < 7 entries.
- Never edit `consent_required_paths` without explicit user OK.
- Never invoke `/shift` (peer skill, different intent).
- Never skip step 1 (refresh on vision-goal) — drift is the failure mode.
- Never declare `delivered` without citing plan_deliverables verbatim.

## Common mistakes

| Mistake | Why it happens | Fix |
|---|---|---|
| Bailing on first ambiguity | Search felt thorough but wasn't | Hit the canonical 7+ locations before declaring uncharted |
| One-render done | Tired-agent's instinct (the agent that has been grinding for an hour is the one most likely to call a flake a win) | Two-render stability is non-negotiable; cost of one extra render < cost of declaring done on a flake |
| Cargo-cult fixes | Lint/type fixes feel productive | Only tier-3 verdict can flip done; if the screenshot didn't move, the fix didn't matter |
| Cross-pillar runaway in one commit | "I'll touch whatever delivers" → 12-file commit | Per-iteration commits with explicit boundary; review surface stays manageable |
| Story authorship without manifesto fidelity | Easy to write Gherkin that makes the screenshot pass without expressing the promise | Every new scenario gets a tier-3 sanity check before commit: "does this scenario express what the manifesto says?" |
| Search-forever | Exhaustive search becomes a way to avoid action | Soft cap (~50 calls); after cap, propose action with what you've found |
| Tier-3 hallucination | "Looks delivered" without grounding | Every verdict must cite plan_deliverables verbatim; two-render stability + scenarios-pass is the falsifier |

## Reference

- **Spec:** `genesis/docs/superpowers/specs/2026-05-06-deliver-skill-design.md`
- **Schema:** `.claude/schemas/feature-promise.schema.json`
- **Sibling skill:** `agentic-developer` (`/shift`) — peer, different intent
- **Required pre-read:** `pipeline-diagnostics` skill — Jenkins-fetch patterns
- **Required agent definitions:** `ci-observer.md` (with visual-triage mode), `ci-investigator.md`
- **Sub-skills dispatched per Diagnose gap table** — don't reinvent any of them
