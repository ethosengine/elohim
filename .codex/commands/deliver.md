---
description: The finisher — runs after a sprint completion to actually deliver the promised feature. Re-orients on plan/manifesto, renders the experience, judges screenshot vs promise as tier-3 stewardship, and grinds through whatever's missing (scenarios, glue, debugging, content) until the screenshot proves delivery.
---

# /deliver

Invokes the `deliver` skill to close the gap from "CI green / sprint claimed done" to "human verifiably sees the delivered feature."

Sibling to `/shift`. Different intent: `/shift` drives a numerical CI metric to green; `/deliver` drives a feature from "claimed done" to "actually visible."

## Usage

- `/deliver <handle>` — handle is a plan name, manifesto reference, or natural-language gap description. Examples:
  - `/deliver light-up-the-topology`
  - `/deliver "I want to see the topology show up — light-up-the-topology landed but the app doesn't render it"`
  - `/deliver imagodei-recovery-flow`
- `/deliver resume <shift-id>` — *(v2, not yet implemented)* resume a bailed run after operator answers the bail question

## What it does

1. **Exhaustive context gather** — searches plans, specs, manifesto/epics, prior sprint-results, Gherkin features, memory, and code surface for anything matching the handle. Soft cap ~50 grep/read calls. Populates a `search_trail` audit log.
2. **Composes a FeaturePromise** at `.claude/shifts/<shift-id>.feature-promise.json` per `.claude/schemas/feature-promise.schema.json` — vision quotes (verbatim from manifesto), plan deliverables (verbatim from plan), scenarios existing/missing, screenshot targets, scope philosophy, search trail.
3. **Initial render** — Playwright the app at the expected delivery surface; capture screenshot + cucumber-report-browser.json + errors-{device}.json.
4. **Initial tier-3 judgment** — orchestrator (Opus) compares screenshot vs FeaturePromise. Verdict ∈ {delivered, partial, error_state, missing}, citing plan_deliverables verbatim.
5. **Composes journal** at `.claude/shifts/<shift-id>.journal.md`; presents user with FeaturePromise + iter-0 verdict; waits for "kick off" confirmation.
6. **Iteration loop** — refresh on vision-goal → render → tier-3 judge → diagnose gap (dispatches sub-skill: debugging, /generate-scenarios, angular-architect, rust-architect, tauri-architect, page-model, content-pipeline, brainstorming, ci-observer visual-triage, ci-investigator) → fix → re-render. Loop until two consecutive `delivered` verdicts (one fresh-trigger), all scenarios pass, no regression-elsewhere.
7. **Close** — sprint result at `.claude/shifts/<shift-id>.sprint-result.md` carrying the screenshot artifact (the proof), plan-vs-delivery match summary, scenarios authored, glue written grouped by pillar, debugging journey, search trail, consent-asks made.

## Bail criteria (high bar — search-first, bail-last)

Bail only when:

1. **Design genuinely uncharted** — `search_trail` ≥ 7 locations with hit/miss; bail-with-proposal text takes the form *"Searched [list]. Found [partial refs]. Cannot reconcile to delivery shape because [specific gap]. Need from operator: [specific call]."*
2. **Destructive consent required** — irreversible op outside auto-mode authorization.

Never bail on scope, "I don't know how to fix this," or iteration count alone.

## See also

- Skill: `.claude/skills/deliver/SKILL.md`
- Spec: `genesis/docs/superpowers/specs/2026-05-06-deliver-skill-design.md`
- Schema: `.claude/schemas/feature-promise.schema.json`
- Sibling: `/shift` (peer skill, different intent)
- Required pre-read inside the skill: `pipeline-diagnostics` skill (canonical Jenkins-fetch patterns)

## Loading the skill

Use the `Skill` tool with `skill: deliver`.
