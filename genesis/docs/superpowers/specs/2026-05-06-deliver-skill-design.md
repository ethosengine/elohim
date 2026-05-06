# `/deliver` — The Finisher Skill

**Date:** 2026-05-06
**Branch:** dev
**Status:** Design (peer to `/shift`; spec → plan → implementation cycle to follow)
**Related:**
- `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md` — `/shift`'s loop design (sibling)
- `genesis/docs/superpowers/specs/2026-05-05-visual-validation-design.md` — visual-validation as integration-mode dimension
- `genesis/docs/superpowers/specs/2026-05-06-haiku-visual-triage-design.md` — tier-1 visual triage (consumed by tier-3 done check)

## TL;DR

`/deliver` is the finisher. It runs **after** a sprint is supposedly complete, when the human visits the app and the promised feature isn't actually there. It re-orients on the original plan and manifesto, renders the experience, judges screenshot-vs-promise as a tier-3 stewardship verdict, and grinds through whatever's missing — story authorship, glue-writing, debugging, content seeding — until the screenshot proves delivery.

It is a peer to `/shift`. They differ in shape:

| | `/shift` | `/deliver` |
|---|---|---|
| Goal | numerical Objective + measure cmd | FeaturePromise (manifesto + plan + scenarios + visual proof) |
| Done | stable measurement, fresh trigger | tier-3 stewardship verdict + scenarios pass + screenshot matches plan |
| Bail | "out of scope, can't move metric" | "spec genuinely uncharted AFTER exhaustive search" or "consent required" |
| Scope | drafted upfront, defensive | unbounded by default; agent commits to "I'll touch whatever delivers" |
| Iteration | observe → act → measure | refresh → render → tier-3 judge → diagnose → fix |
| Kickoff | interview user for Objective | exhaustive plan-anchored context binding |
| Use-case | drive a CI metric to green | actually deliver a feature the prior sprint claimed was complete |

## Problem

Sprints land. CI goes green. The user visits the app and the feature they expected to see isn't there — or it's there but broken, or it's there but doesn't match what the plan said it should look like, or the visible state is dominated by error overlays that make the feature unusable.

Today's options for closing this gap are bad:

- **Manual debugging session.** The user hand-walks the failure surface, looking up the prior plan, finding gaps, dispatching agents one-off. High cognitive load; doesn't scale; doesn't document the closure for future reference.
- **`/shift` invocation.** Tries to drive a CI metric down. The metric drops one or two scenarios, then bails because root-causes are "outside scope" — content seed data, missing scenarios, missing routes, missing manifesto-anchored design calls. The bail-with-proposal mechanism is procedurally correct but materially premature: the FEATURE work isn't done; the metric just couldn't move further within drafted scope.
- **New sprint with new plan.** Heavy. Requires brainstorming → writing-plans → implementing all over again to close what should be a small last-mile gap.

The missing skill is the **last-mile finisher** — bridges the gap between "sprint claimed done" and "human verifiably sees the delivery." Has wide latitude to touch whatever's needed (scenarios, glue, content, schema, app code), bails only when the promise itself is genuinely undefined.

## Design

### Working name

**`/deliver`** — the verb captures fulfilling-the-promise. Description prose: *"The finisher. Runs after a sprint completion to actually deliver the promised feature."*

### Kickoff — exhaustive plan-anchored binding

Distinct from `/shift`'s "interview the user for an Objective." User invokes with a handle:

```
/deliver light-up-the-topology
/deliver "I want to see the topology show up — light-up-the-topology landed but the app doesn't render it"
/deliver imagodei-recovery-flow
```

The handle resolves to **plan-anchored binding** through exhaustive search across these sources, in order:

1. **Plan(s)** — `genesis/docs/plans/*<handle>*.md` and `genesis/docs/superpowers/plans/*<handle>*.md`
2. **Spec(s)** — `genesis/docs/superpowers/specs/*<handle>*.md`
3. **Manifesto / epic narrative** — `genesis/docs/content/elohim-protocol/**/*<handle>*` and adjacent files
4. **Sprint result(s)** — `.claude/shifts/*<handle>*.sprint-result.md` (recent shift artifacts that worked on this)
5. **Existing scenarios** — `genesis/a2o/features/**/*<handle>*.feature` and grep for handle terms inside features
6. **Memory entries** — `~/.claude/projects/-projects-elohim/memory/**` matching the topic
7. **Code surface** — components, services, routes that name the handle's terms (Glob + ripgrep)

Out of this, the skill composes a **FeaturePromise** artifact:

- **vision_quotes** — one or more verbatim manifesto/epic quotes capturing the WHY
- **plan_deliverables** — verbatim list from the plan(s) of what was supposed to land, marked user_visible / internal
- **manifesto_anchors** — the WHY behind plan_deliverables
- **scenarios_existing** — Gherkin scenarios already authored that assert this delivery
- **scenarios_missing** — gaps where scenarios should exist but don't (with intent + proposed location)
- **screenshot_targets** — per-scenario expected visual proof, with what_should_be_visible structural description
- **scope philosophy** — "unbounded; agent commits to whatever delivers"
- **consent_required_paths** — the only opt-out: destructive paths still need explicit user OK
- **search_trail** — every place searched, with hit/miss status (the bail bar's audit log)

After binding, the skill:

1. **Initial render** — Playwright run, navigate to the expected delivery surface, take baseline screenshot
2. **Initial tier-3 judgment** — stewardship comparison: what's actually there vs what was promised; iter-0 delivery-gap diagnosis
3. **Compose journal + present** — show user the bound FeaturePromise + iter-0 gap diagnosis
4. **User confirms "kick off?"** — same gate as `/shift`. User can edit the FeaturePromise before running.

Kickoff is heavier than `/shift`'s by design. The exhaustive search trail IS the bail bar; doing the search at kickoff means the agent already knows where to look when ambiguity arises mid-iteration.

### Iteration loop — refresh → render → judge → fix

Each iteration:

1. **Refresh on vision-goal.** Re-read the relevant slice of plan/manifesto. Cheap (often cached). Mandatory — don't skip. Drift-prevention.

2. **Render the experience.** Two pathways depending on iteration context (see "CI artifact mechanics" section for the specific tool calls):
   - **CI fresh-render** — pull the latest genesis pipeline build's screenshot artifact for the relevant feature slug. Use this when CI just ran and is the source of truth.
   - **Local fresh-render** — Playwright run against the local dev server (`pnpm hc:start` + a2o run). Use this when CI is stale, when iterating fast, or when CI hasn't seen the latest fix yet.

   Either way, capture: the screenshot, the cucumber-report-browser.json (per-scenario state), and the per-scenario `errors-{device}.json` console/network artifacts. All three feed tier-3 judgment.

3. **Tier-3 stewardship judgment.** Opus orchestrator (the skill itself) compares screenshot against the FeaturePromise's plan_deliverables + vision_quotes. Verdict:
   - **delivered** — matches the promise, can identify the feature, no error overlays, scenarios pass
   - **partial** — feature visible but missing elements (specific list)
   - **error_state** — error overlay / blank / wrong route — feature unreachable
   - **missing** — feature affordances are entirely absent from the UI

4. **Diagnose gap.** Tier-3 verdict drives which capability is needed. The skill is a **multi-capability orchestrator** — invokes existing skills:
   - **Story missing** → `/generate-scenarios` writes the Gherkin shape from FeaturePromise.scenarios_missing
   - **Glue missing** → `frontend-design`, `angular-architect`, `rust-architect` for implementation work in their domains
   - **Implementation broken** → `superpowers:systematic-debugging`
   - **Content data gap** → `content-pipeline` skill for seed data fixes
   - **Test selectors missing** → `page-model` skill for data-testid coverage
   - **Manifesto/spec genuinely ambiguous** → `superpowers:brainstorming` (alternative to bail-with-proposal)
   - Tier-1 visual triage (Haiku) and tier-2 completeness (Sonnet) feed into the tier-3 judgment as evidence

5. **Fix.** Apply changes. Often cross-pillar (app + content + scenarios + maybe schema). Per-iteration commit with explicit boundary noted in journal stanza. Search-first discipline holds — if any sub-skill says "spec unclear," the orchestrator pushes back: "have you searched [list]? proceed with what's documented."

6. **Re-render and re-judge.** Loop until tier-3 verdict is **delivered** for two consecutive renders, at least one fresh.

The skill does NOT terminate on first successful change. Only on stable positive stewardship verdict.

### Done criterion

**Two consecutive tier-3 verdicts of "delivered"**, at least one from a fresh render — fresh CI screenshot OR fresh Playwright run after the agent's last fix. Single positive verdict = done-candidate; two = done.

The verdict is produced by the orchestrator itself (skill runs as Opus, making it tier-3 by definition). For a verdict to qualify as "delivered" it must satisfy ALL of:

1. **Visual proof** — screenshot matches the plan_deliverables and vision_quotes. Tier-3 reasoning must cite plan_deliverables verbatim — no "looks good" without anchoring.
2. **Scenarios pass** — every scenario in `verification.scenarios_existing` is in passing state in the latest cucumber-report. Newly-authored scenarios from `scenarios_missing` count once they're committed and passing.
3. **No regression elsewhere** — the change set must not have broken other passing scenarios in the same feature pillar (compared against the iter-0 baseline).

The screenshot-vs-plan match is the primary check; the scenarios-pass requirement makes the verdict falsifiable. There is no parallel "is the metric ≤ N" gate.

### Bail criterion (high bar — search-first, bail-last)

Bail only when:

1. **Design genuinely uncharted** — and the journal documents where the agent searched, with results. Bail journal must list ≥7 search locations with hit/miss status. Bail-with-proposal text:
   > *"Searched [list]. Found [3 partial references]. Can't reconcile to a delivery shape because [specific gap]. Need from operator: [specific design call]."*

   If the agent can't show its search trail, it hasn't earned the bail.

2. **Destructive action consent required** — irreversible operation outside auto-mode's authorization (DB migrations, force-pushes, prod deploys, manifest changes affecting other peers, crypto operations). Ask, don't act.

NEVER bail on:
- **Scope.** Commitment is "I'll touch whatever delivers." If a fix is in `genesis/data/lamad/`, edit it. If it's in `elohim/holochain/dna/`, edit it (with consent if it changes notarized law).
- **"I don't know how to fix this."** That's debugging skill territory — invoke it, don't avoid it.
- **Iteration count alone.** Budget is a soft signal. Agent reaches bail-with-proposal only IF nothing has moved AND search hasn't surfaced new direction.

### Schema — FeaturePromise

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/agentic/feature-promise.schema.json",
  "title": "FeaturePromise",
  "description": "Bound artifact for /deliver invocations. Composed during kickoff via exhaustive plan-anchored search.",
  "type": "object",
  "required": ["name", "handle", "promise", "verification", "scope", "budget", "search_trail"],
  "properties": {
    "name": { "type": "string", "pattern": "^[a-z][a-z0-9-]{2,60}$" },
    "handle": { "type": "string", "description": "The argument passed to /deliver" },
    "description": { "type": "string", "minLength": 10 },
    "promise": {
      "type": "object",
      "required": ["vision_quotes", "plan_deliverables"],
      "properties": {
        "vision_quotes": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["source", "text"],
            "properties": {
              "source": { "type": "string", "description": "path:line or path" },
              "text": { "type": "string" }
            }
          }
        },
        "plan_deliverables": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["plan_path", "deliverable", "user_visible"],
            "properties": {
              "plan_path": { "type": "string" },
              "deliverable": { "type": "string" },
              "user_visible": { "type": "boolean" }
            }
          }
        },
        "manifesto_anchors": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["source", "text"],
            "properties": {
              "source": { "type": "string" },
              "text": { "type": "string" }
            }
          }
        }
      }
    },
    "verification": {
      "type": "object",
      "required": ["scenarios_existing", "scenarios_missing", "screenshot_targets"],
      "properties": {
        "scenarios_existing": {
          "type": "array",
          "items": { "type": "string", "description": "feature_path: scenario_name" }
        },
        "scenarios_missing": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["intent", "proposed_location"],
            "properties": {
              "intent": { "type": "string" },
              "proposed_location": { "type": "string" }
            }
          }
        },
        "screenshot_targets": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["feature_slug", "what_should_be_visible"],
            "properties": {
              "feature_slug": { "type": "string" },
              "scenario_name": { "type": "string" },
              "what_should_be_visible": {
                "type": "string",
                "description": "Structural description, not transcribed text. Tier-3 judges screenshot against this."
              }
            }
          }
        }
      }
    },
    "scope": {
      "type": "object",
      "required": ["philosophy"],
      "properties": {
        "philosophy": {
          "const": "unbounded; agent commits to whatever delivers"
        },
        "consent_required_paths": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Globs requiring explicit user consent before edit (destructive/irreversible paths)."
        }
      }
    },
    "budget": {
      "type": "object",
      "required": ["iterations", "wall_clock_min"],
      "properties": {
        "iterations": { "type": "integer", "minimum": 5, "maximum": 50, "default": 20 },
        "wall_clock_min": { "type": "integer", "minimum": 30, "maximum": 1440, "default": 480 },
        "search_calls": { "type": "integer", "minimum": 10, "maximum": 200, "default": 50, "description": "Soft cap on grep/read calls in any single search-first phase. After this, agent must propose action with what it has." }
      }
    },
    "search_trail": {
      "type": "array",
      "description": "Audit log of every search location visited during kickoff and any mid-iteration ambiguity-resolution searches. Bail-with-proposal requires this to be non-empty and to have ≥7 unique locations.",
      "items": {
        "type": "object",
        "required": ["location", "result"],
        "properties": {
          "location": { "type": "string", "description": "path/glob/grep pattern searched" },
          "result": { "enum": ["hit_relevant", "hit_partial", "miss"] },
          "iteration": { "type": ["integer", "null"] }
        }
      }
    }
  }
}
```

### Sprint-result shape

The `/deliver` sprint-result.md carries different fields than `/shift`'s metric-driven outcome:

- **Final tier-3 verdict** — delivered / partial / error_state / missing (with reasoning)
- **Screenshot artifact** — path/URL of the proof image
- **Plan-vs-delivery match summary** — per-deliverable: ✓ delivered / ⚠ partial (gap: …) / ❌ missing
- **Scenarios authored** — Gherkin scenarios written during the run
- **Glue written** — implementation files added/modified, grouped by pillar
- **Debugging journey** — number of debug sessions, root causes found
- **Search trail** — bail audit log
- **Consent-asks made** — paths the agent stopped at; what was authorized vs declined

## Integration with existing skills

`/deliver` is a **multi-capability orchestrator**. It dispatches to existing skills based on which gap the tier-3 judgment surfaced:

| Skill | When invoked |
|---|---|
| `superpowers:systematic-debugging` | Render dies; can't reach screenshot. Per-iteration possible. |
| `superpowers:writing-plans` | Major redesign needed (rare; the prior sprint's plan should suffice) |
| `superpowers:brainstorming` | Manifesto-ambiguous AND search-first didn't resolve. Alternative to bail. |
| `frontend-design` | App-side UI glue-writing |
| `angular-architect` | Angular service/component glue |
| `rust-architect` | Doorway/storage/zome glue |
| `tauri-architect` | Desktop integration glue |
| `/generate-scenarios` | Story authorship for missing scenarios |
| `page-model` | Test selector coverage (data-testid gaps blocking Playwright) |
| `content-pipeline` | Content seed data fixes |
| ci-observer (tier-1 visual triage) | Categorical screenshot state — feeds tier-3 |
| ci-investigator (tier-2 completeness) | UI element completeness — feeds tier-3 |
| `/shift` | NOT invoked. Different intent (peer skill). User picks the right tool. |
| `/story-harvest` | NOT invoked. Different timing (post-debug). `/deliver` may produce harvestable patterns; harvesting is operator-discretion. |

The orchestrator (the skill itself, running as Opus) does:
- Tier-3 stewardship judgment per iteration
- Capability-to-skill routing
- Search trail maintenance
- Cross-pillar change boundary tracking
- Sprint-result composition at close

## CI artifact mechanics — how `/deliver` reaches the signals it needs

`/deliver` does not invent CI plumbing. It inherits the artifact-pulling, subagent-dispatch, and pipeline-trigger patterns that `/shift` and `agentic-developer` already use. The runtime skill text MUST point at these explicitly so the agent doesn't re-derive them per invocation.

### Reference: pipeline-diagnostics skill

`/deliver` runs the `pipeline-diagnostics` skill (`.claude/skills/pipeline-diagnostics/SKILL.md`) when it needs Jenkins data. That skill is the canonical reference for:

- Jenkins MCP tool inventory (`mcp__jenkins__getBuild`, `getBuildLog`, `searchBuildLog`, `getTestResults`, `getFlakyFailures`, etc.)
- Public-Jenkins URL patterns + WebFetch fallback when MCP isn't loaded in the session (a real constraint — e.g. some subagent contexts don't get MCP)
- The `JENKINS_URL` env var + curl pattern for measure scripts
- Authenticated parameterized rebuild via `JENKINS_TOKEN` (rare; gated)

The runtime skill text should say: *"Read the pipeline-diagnostics skill before any Jenkins fetch. If MCP tools aren't in your tool list, fall back to WebFetch on the public URL pattern documented there."*

### Artifact map — what to pull from where

For genesis pipeline (`elohim-genesis/dev`):

| Signal needed | Artifact | URL pattern (relative to `${JENKINS_URL}/job/elohim-genesis/job/dev/<build>/`) | Tool |
|---|---|---|---|
| Pass/fail summary | `sprint-report.md` | `artifact/genesis/a2o/reports/sprint-report.md` | WebFetch |
| Pass/fail summary (machine) | `sprint-report.json` | `artifact/genesis/a2o/reports/sprint-report.json` | WebFetch |
| Per-scenario error data | `cucumber-report-browser.json` | `artifact/genesis/a2o/reports/cucumber-report-browser.json` | WebFetch |
| Visual proof | screenshot PNG | `artifact/genesis/a2o/reports/screenshots/<feature-slug>/<scenario-slug>--<human>.png` | WebFetch (saves binary to tmp) → Read (renders multimodally) |
| Console/page/network errors | `errors-<device>.json` | `artifact/genesis/a2o/reports/screenshots/<feature-slug>/errors-<device>.json` | WebFetch |
| Build log (full) | `consoleText` | `consoleText` | WebFetch (paginate via `mcp__jenkins__getBuildLog` `skip`/`limit` when MCP loaded) |
| Seed-phase output | embedded in console | grep within `consoleText` for `[+]` / `[X]` / `[=]` markers | WebFetch + grep |

For a feature slug, the convention is `<pillar>-<feature-name>` (e.g. `lamad-learning-journey`, `elohim-presence`, `deployment-staging-validation`).

For the screenshot binary pathway:
1. WebFetch on the PNG URL — the harness detects binary content-type and saves it to a tmp path (e.g. `/projects/.claude-config/.../tool-results/webfetch-<id>.bin`)
2. Read the tmp path — multimodal rendering produces an image the agent can describe
3. Inline tier-3 judgment OR pass-through to ci-observer (tier-1) / ci-investigator (tier-2) for the bounded categorical / completeness check before tier-3

### Subagent dispatch — which agent for which signal

| Signal needed | Agent | Tier | Cost |
|---|---|---:|---:|
| Categorical visual state — blank? loading? error_overlay? feature_visible? | `ci-observer` (visual-triage mode, just landed) | 1 | $ |
| Pipeline build summary, dispatch drift, anti-pattern catalog match | `ci-observer` (summarize/validate mode — existing) | 1 | $ |
| UI element completeness against page-model selectors | `ci-investigator` (with explicit tier-2 directive) | 2 | $$ |
| Specific quoted errors from cucumber/console | `ci-investigator` (existing pattern) | 2 | $$ |
| Cross-build correlation (flake history, regression bisection) | `ci-investigator` (existing pattern) | 2 | $$ |
| Stewardship verdict — does screenshot match plan/manifesto? | Opus orchestrator (the skill itself) | 3 | $$$ |
| Plan-vs-delivery match summary | Opus orchestrator | 3 | $$$ |

**Tier flow** for any screenshot the orchestrator wants to evaluate:
1. Tier-1 first — Haiku categorical triage. Cheap, fast, parallelizable across N screenshots.
2. If `image_state ∈ {feature_visible}` AND `feature_identifiable: true` — escalate to tier-2.
3. Tier-2 — Sonnet completeness check against page-model selectors / FeaturePromise.screenshot_targets.
4. If tier-2 returns `complete` — tier-3 (orchestrator) does final stewardship verdict.
5. Negative results at any tier short-circuit upward to orchestrator for diagnose-and-fix.

### Pipeline trigger mechanics — for forcing fresh CI renders

When `/deliver` needs a fresh CI render (because the agent just landed a fix and wants to see it through real CI):

**Anonymous path (default):** empty commit with build tag, push:
```bash
git commit --allow-empty -m "ci: deliver retrigger [build:elohim-genesis]"
git push
```

The orchestrator's webhook + commit-tag dispatch picks this up. Reuses `/shift`'s pattern.

**Authenticated path (rare, gated by orchestrator-state-verification):** parameterized rebuild via `curl -u "$JENKINS_USERNAME:$JENKINS_TOKEN" .../buildWithParameters?...`. Used only when a parameter must be set that the Jenkinsfile default doesn't carry (canonical case: `RESET_STORAGE=true` for elohim-genesis schema-drift recovery). Same guardrails as `/shift`'s parameterized-rebuild path — read `pipeline-diagnostics` skill before invoking. Token never logged.

**Local Playwright path (faster iteration):** when CI is too slow or the fix is purely app-side and doesn't need full pipeline rebuild, run Playwright locally against `pnpm hc:start` + a2o:
```bash
cd /projects/elohim
pnpm hc:start                         # background, conductor + storage + doorway
cd genesis/a2o
pnpm test:browser -- --tags @browser-only --feature <slug>
```

Screenshots land in `genesis/a2o/reports/screenshots/<feature-slug>/`. Read directly via Read tool (already local).

### Pre-flight pipeline prediction

Before any push that triggers CI, run graph-walker on the staged diff to predict which pipelines the orchestrator should dispatch:

```bash
git diff --name-only --cached | node genesis/orchestrator/graph-walker.mjs
```

Journal the predicted set in the iteration stanza. Next iteration's tier-1 ci-observer dispatch (in validate mode) compares predicted vs actual — if drift, surface as a principle-7 finding (CI dispatch correctness is part of deliverability).

### Measure script reuse

`/deliver` does NOT have a single measure script (no numerical Objective). But it inherits `/shift`'s patterns for safe artifact reading:

- `genesis/agentic/scripts/jenkins-measure-genesis-findings.sh` — reads `failed` count from `sprint-report.md`. `/deliver` may run this to track regression-elsewhere as part of done criterion check #3 ("no regression elsewhere").
- Custom one-off scripts for plan-deliverable verification (e.g. "did topology load by querying `<topology-component>`?") may be drafted ad-hoc by the agent; not gated on a pre-existing script library.

## Failure modes

| Failure mode | Why it happens | Mitigation |
|---|---|---|
| **Drift across iterations** | Agent forgets the original promise as it chases gaps; loop wanders into adjacent concerns | Mandatory step 1 every iteration: refresh on vision-goal. Skipping it is a discipline violation. |
| **Cargo-cult fixes** | Agent fixes things that LOOK broken (lints, types) but don't block delivery | Only tier-3 verdict can flip done. No "looks good, ship it." Lint/type fixes that don't move the screenshot don't count toward done. |
| **Cross-pillar runaway** | Agent commits to "I'll touch whatever delivers" → touches DNA + zomes + content + app + tests in one iteration → review surface explodes | Per-iteration commits with explicit boundary in journal stanzas; cross-pillar changes get a "review-summary" callout at close. Sprint-result groups changes by pillar. |
| **Search-forever** | Exhaustive search becomes a way to avoid acting; agent reads 100 files, never commits | Search has its own budget (default 50 grep/read calls per phase). After budget, agent MUST propose action with what it has. |
| **Story authorship without manifesto fidelity** | Agent writes Gherkin that makes screenshot pass but doesn't actually express the promise (e.g. asserts "page renders" without checking the topology data is meaningful) | Any new scenario triggers a tier-3 sanity check — "does this scenario express what the manifesto says?" — before commit. Failure rejects the scenario. |
| **Bail-bar erosion** | Agent gets sloppy; bails with thin search trail | Sprint-result template requires `search_trail` to have ≥7 unique locations for any bail. Validation hook can enforce. |
| **Tier-3 hallucination** | Opus orchestrator says "delivered" when screenshot doesn't actually match (the verdict is judgment-prone) | Two-render stability requirement. AND tier-3's reasoning must cite plan_deliverables verbatim — no "looks good" without anchoring. AND scenarios_existing must pass alongside the screenshot match. |
| **Visual-triage misclassification** | Tier-1 says "feature_visible" when it's actually partial_render; tier-3 trusts and over-reports delivered | Tier-3 always re-runs Tier-1 + Tier-2 inline as part of its judgment; disagreement triggers re-render or escalation. |

## Implementation pointers

(Spec only; not implementation. Pointers for the plan-writer.)

1. **Schema** at `.claude/schemas/feature-promise.schema.json` — the FeaturePromise shape above
2. **Skill** at `.claude/skills/deliver/SKILL.md` — same location pattern as `agentic-developer` (which sits under `.claude/skills/`)
3. **Slash command** at `.claude/commands/deliver.md` invoking the skill
4. **Sprint-result template** at `genesis/docs/shifts/DELIVER-SPRINT-RESULT-TEMPLATE.md`
5. **Journal template** at `genesis/docs/shifts/DELIVER-JOURNAL-TEMPLATE.md`
6. **Readiness check** at `genesis/agentic/deliver-readiness.mjs` — ensure prereqs (Playwright runnable, target app reachable, git clean enough)
7. **Search-trail validator** wired into the close phase — refuses to write sprint-result with insufficient search trail when status=bail
8. **Runtime skill text MUST point at `pipeline-diagnostics` skill explicitly** — that's where the Jenkins URL/MCP/auth patterns live. `/deliver` should read it before any Jenkins fetch, NOT re-derive the patterns. Same applies for `agentic-developer` cross-references (e.g. ci-observer/ci-investigator dispatch shapes).

The skill text should be roughly the same shape as `agentic-developer/SKILL.md` — principles, kickoff routine, iteration loop steps, close routine, invariants. Length-wise, expect parity with `agentic-developer/SKILL.md`.

### Subagent prerequisites

`/deliver` requires these existing agent definitions to be present and functional. The implementation plan must verify each:

- `.claude/agents/ci-observer.md` — including the **Visual triage mode** section landed in commit `0eac5fdd`. If a future change strips it, `/deliver`'s tier-1 dispatch breaks.
- `.claude/agents/ci-investigator.md` — for tier-2 completeness checks. The agent definition should accept an explicit "tier-2 completeness check" directive shape; if not, that's an implementation-blocking gap to fix first.
- The schema at `.claude/schemas/haiku-output.schema.json` with the `visual_triage` field — same dependency surface.

If any of these is missing or stale, `/deliver` kickoff readiness check FAILS the same way `/shift`'s readiness check does.

## Validation criteria

This spec succeeds when, after implementation, a smoke test of `/deliver light-up-the-topology` exhibits:

1. **Kickoff binds the FeaturePromise** within 2-3 minutes of search, finding the topology plan + any related specs + scenarios + manifesto context.
2. **Initial render + tier-3 judgment runs** before user confirmation, producing a coherent gap diagnosis.
3. **First iteration acts on the highest-leverage gap** — likely glue-writing or scenario authorship, not random file edits.
4. **Cross-iteration vision-goal refresh visibly anchors the journal** — diagnoses cite plan deliverables verbatim.
5. **Done criterion is tier-3 stable verdict + scenarios pass + screenshot match**, not a metric-drop.
6. **Bail (if it occurs) shows a search trail of ≥7 locations** with hit/miss status.

A real `/deliver` run on the topology example should plausibly produce a screenshot showing the topology actually rendered, not "Content Not Found" — that's the user's smoke-test scenario and the spec's primary validation.

## Open questions / follow-ups

- **Naming finalization:** "/deliver" is the working name. Hebrew/biblical alternative `/shem` (name/realize) was considered; sticking with `/deliver` for legibility unless implementation reveals a better fit.
- **Relationship to `/shift` mode tags:** could `/shift` add a `mode: feature-completion` toggle that turns on `/deliver`-shaped kickoff/loop? Decided against (Approach 2 in brainstorm); revisit if implementation surfaces large code-share opportunities.
- **Search-trail enforcement strictness:** ≥7 search locations as the bail bar is a starting heuristic. Tunable after first usage data — could go up if agents bail with thin trails, down if it's blocking legitimate quick bails.
- **Tier-3 orchestrator-as-judge concern:** the skill runs as Opus AND does tier-3 judgment, which can look like marking-its-own-homework. The two-render stability + scenarios-must-pass + plan-verbatim-citation discipline is the mitigation. Real usage will tell us if it holds.
- **Multi-feature delivery:** can `/deliver` take multiple handles (e.g. `/deliver light-up-the-topology recovery-flow`)? Out of scope for v1; revisit when single-feature works well.
- **Resume support:** `/deliver resume <id>` for picking up a bailed run after operator answers? Same v2 as `/shift resume`.
