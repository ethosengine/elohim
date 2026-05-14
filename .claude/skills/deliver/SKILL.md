---
name: deliver
description: Use when a recent sprint was marked complete but the feature isn't actually visible or usable in the app — CI green doesn't match human-visible delivery, components built but not wired, scenarios passing but the user can't see what was promised
---

# Deliver — The Finisher

You are the finisher. Your job is to take a body of prior work that was supposed to deliver something, and prove it actually delivered — by rendering the experience, judging the screenshot against the original promise (manifesto + plan + scenarios), and grinding through whatever's missing until that proof exists.

Sibling to `/shift` (agentic-developer). Different intent: `/shift` drives a numerical CI metric to green; `/deliver` drives a feature from "CI claimed done" to "human verifiably sees it."

**Spec:** `genesis/docs/superpowers/specs/2026-05-06-deliver-skill-design.md` — read it if any of the principles below are unclear.

> ## Authority — `/deliver` is the sole authority over `active.*` / `stable` / `regression`
>
> The unified delivery-status gradient (`envisioned → backlog → refined → wip → active.alpha → active.beta → active.latest-stable → stable`, with `regression` as orthogonal-sideways) has one mintage rule: **only `/deliver`'s tier-3 stewardship verdict may confer states at-or-above `active.alpha`.** This authority is **delegated from the operator** and is non-transferable to the memory-ceremony group (librarian/historian/storyteller/cartographer), to `/shift`, or to any CI signal alone.
>
> This rule exists because **CI green ≠ human-visible delivery**. The whole point of `/deliver` is to close that gap by judging a rendered screenshot against the FeaturePromise. If the memory-ceremony hygiene group could promote a feature from raw cucumber JSON, the gap re-opens. They read the axis to gate graduations; they never write `active.*` / `stable` / `regression`.
>
> The full ownership matrix lives in `.claude/scripts/memory-kit/LIFECYCLE.md` under "The author/delivery axis split". This skill is the operating manual for the writing side of that boundary.

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

### 3a. Test-wiring confidence procedure (8-step trace)

Before the verdict converts into a delivery-state, prove the scenarios are actually wired to the feature the story declares. Skipping this is how a "green" verdict gets minted on a feature whose Gherkin doesn't exist or never ran.

1. **Identify canonical feature(s).** Read the story frontmatter's `feature:` triple and `adjacent_features[]`. The canonical feature is the one whose passing proves the story's experience is real; adjacents are co-touched.
2. **Existence gate.** If the canonical `.feature` file is missing on disk, `delivery_status` **cannot advance above `wip`** this iteration. Flag the gap to the cartographer as a backlog candidate (`write-<feature-slug>.feature`) and continue diagnosing — do not mint `active.*`.
3. **Run the covering Cucumber profile.** Read `genesis/a2o/reports/cucumber-report.json`; record `scenarios.passed` and `scenarios.failed` for the feature path. (See `genesis/a2o/CLAUDE.md` for profile selection.)
4. **Confirm `@elohim-visually-validated` tag** is present on the headline scenarios the FeaturePromise calls out. The tag is the visual-validation contract; its absence floors the verdict at `active.alpha`.
5. **Read `genesis/a2o/reports/sprint-report.json` `summary.visualValidation` 2×2 matrix** — the four buckets are `{validatedPassing, validatedRegressed, pendingPassing, pendingFailing}` per `genesis/a2o/schemas/sprint-report.schema.json`. This is the load-bearing axis for `active.beta → active.latest-stable` graduation.
6. **Screenshot at the promised surface.** Use the existing screenshot-vs-promise judgment shape. The screenshot is the falsifier the prose verdict alone cannot supply.
7. **Map the tuple `(scenario pass/fail × visual-validation bucket × screenshot match)` to a gradient state.** Use the ladder in the next section. Ambiguous tuples → `pending` (let memory-ceremony defer; never guess upward).
8. **Write the verdict.** Append to the manifest (next section), then flip story / backlog frontmatter via the procedure ladder.

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

## Procedure ladder — verdict to delivery-state

The tier-3 verdict is qualitative (`delivered` / `partial` / `error_state` / `missing`); the **delivery-state** it produces on the gradient is what stories, backlog entries, and the memory-ceremony group read. The mapping below is the full ladder. Each tier records the same field-set in the same three places; only the values differ.

**Three write-targets per verdict** (all updated atomically per iteration):
1. **Manifest** at `.claude/deliver/manifest.json` — single rolling structured artifact (see schema below)
2. **Story frontmatter** at `genesis/data/stories/<subject>--<role>--<feature>.md` (when a linked story exists per `feature:` triple)
3. **Backlog frontmatter** at `genesis/data/timeline/backlog/<slug>.md` (when a linked backlog entry exists per `relatedNodeIds[]`)

**Fields written** (identical schema across all three surfaces):

| Field | Value | Notes |
|---|---|---|
| `delivery_status` | gradient state | `active.alpha` / `active.beta` / `active.latest-stable` / `stable` / `regression` (or `pending` if step 3a's tuple was ambiguous) |
| `delivery_status_updated` | ISO date | Today's date |
| `delivery_status_source` | `deliver` | Distinguishes from `deliver-bridge` (auto-poller) and `operator-override` |
| `regression_from` | prior state or `null` | **Only** populated when minting `regression`; preserves the level to repair back to |

### The five tiers

| Tier | Trigger | What is written |
|---|---|---|
| **`active.alpha`** | First `delivered` verdict on a feature in the alpha channel; single fresh-trigger render passes. Step 3a's `@elohim-visually-validated` tag MAY be absent. | `delivery_status: active.alpha` |
| **`active.beta`** | Two consecutive `delivered` verdicts (one fresh-trigger), scenarios green, `@elohim-visually-validated` present on headline scenarios, `visualValidation.validatedPassing ≥ 1`, no `validatedRegressed`. Feature is hardening and scoped to beta channel. | `delivery_status: active.beta` |
| **`active.latest-stable`** | Released to the default release-channel; at minimum **one full sprint-result** green after `active.beta`, no `validatedRegressed` in the most recent sprint-report. | `delivery_status: active.latest-stable` |
| **`stable`** | N=5 consecutive `/deliver` sprint-results that touch this feature are green AND no `regression` event occurred in the window. This is the load-bearing tier — feature is trusted substrate, not just shipped. | `delivery_status: stable` |
| **`regression`** | Any prior `active.*` or `stable` is re-judged with verdict `partial` / `error_state` / `missing`, OR `visualValidation.validatedRegressed ≥ 1` on a feature previously at `≥ active.beta`. | `delivery_status: regression` + `regression_from: <prior state>` + root-cause pointer in the manifest's `evidence.root_cause_path` (sprint-result, debug shift, or issue link) |

**Provenance discipline (non-negotiable):**
- Every promotion writes the **citing sprint-result path** into manifest `sprint_result_path`.
- Every promotion records `verdict_iteration` (which iter-N produced the verdict).
- `delivery_status_source: deliver` MUST be set — this distinguishes `/deliver`-authored verdicts from the read-only auto-poller's floor signals.
- **Never confer a state without writing the manifest first.** The manifest is the audit trail; the frontmatter writes are projections of it.

## Structured verdict manifest

`/deliver` persists every verdict to a single rolling artifact at `.claude/deliver/manifest.json`. The memory-ceremony group's `deliver-bridge` auto-poller reads this file; brittle prose-grepping of sprint-results is the fallback only.

### Storage classification (load-bearing)

The manifest is a **local-only artifact; not protocol substrate**. It lives at the same tier as `.claude/memory-kit/claude-md-drift.json` and the memory balance-sheet snapshots:

- **Single-writer**: only `/deliver` mutates this file. The auto-poller is read-only.
- **Local to the repo**: tracked in git for team-share, but **never DHT-bound**, never EPR-aliased, never seeded.
- **Not a notarized truth**: it is `/deliver`'s working ledger. The notarized truth is the sprint-result markdown (canonical, fresh-trigger-tagged) and the screenshot artifact at the Jenkins URL or local path.
- **Re-derivable**: if the manifest is lost or corrupted, it can be reconstructed by walking `.claude/shifts/*-deliver-*/sprint-result.md` and re-extracting verdicts. Treat it as a cache-with-history.

This classification was resolved at plan `/projects/.claude-config/plans/stateless-seeking-forest.md` per option (b): bake the local-only disclaimer into the manifest section so the P2P-gate does not fire on a substrate-tier mistake.

### Schema (inline; canonical)

```json
{
  "verdicts": [
    {
      "feature_path": "genesis/a2o/features/<pillar>/<slug>.feature",
      "story_slug": "<subject>--<role>--<feature>",
      "backlog_slug": "<slug-of-linked-backlog-entry-or-null>",
      "delivery_status": "active.alpha|active.beta|active.latest-stable|stable|regression",
      "regression_from": null,
      "verdict_iteration": "iter1|iter2|...",
      "verdict_timestamp": "2026-05-14T12:34:56Z",
      "sprint_result_path": ".claude/shifts/<shift-dir>/sprint-result.md",
      "evidence": {
        "gherkin_scenarios_passing": 7,
        "gherkin_scenarios_failing": 0,
        "visual_validation": {
          "validated_passing": 3,
          "validated_regressed": 0,
          "pending_passing": 4,
          "pending_failing": 0
        },
        "screenshot_path": "genesis/a2o/reports/screenshots/<feature-slug>/<scenario>--<human>.png",
        "fresh_trigger_confirmed": true,
        "root_cause_path": null
      },
      "deliverables_cited": ["plan-deliverable-1", "plan-deliverable-2"]
    }
  ]
}
```

### Append semantics

- **Single rolling file.** All verdicts ever produced live in `verdicts[]`, appended chronologically.
- **Latest verdict wins per `feature_path`.** The auto-poller and any consumer pulls the last entry for a given path. Older entries are kept for audit / `stable`-tier counting (N=5 window) / regression-from lookup.
- **Append per iteration**, not per shift. If iter-2 supersedes iter-1's verdict, both rows persist; the latest reflects current state.
- **Never edit existing rows.** Corrections happen by appending a new row with a `corrects_iteration: iterN` field (rare; operator-initiated).

## Pickup queue — what does `/deliver` work on next?

When invoked without an explicit `<handle>`, `/deliver` reads the pickup queue surfaced by the memory-ceremony group's auto-poller:

- **Source**: `.claude/memory-kit/delivery-status-distribution.json` (audit-script output; the field naming may settle as `pickup_queue_for_deliver[]` or similar — confirm filename/array name on first invocation of this fallback)
- **Ordering**: regressions first (highest urgency — was-working, now-broken), then `active.beta` candidates ready to graduate, then `active.alpha` candidates ready to harden, then `wip` items whose features have green scenarios but no `/deliver` verdict yet
- **Behavior**: present the top 3 candidates to the operator with the pickup-queue rationale; let the operator pick or pass a `<handle>` of their own

If the distribution file doesn't exist yet (memory-kit audit not yet run this cycle), fall back to operator-supplied handle and surface a one-line note: *"pickup queue unavailable — run librarian's pre-flight before invoking `/deliver` without a handle."*

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
3. **Append the final verdict to `.claude/deliver/manifest.json`** per the schema above. Latest-verdict-wins for the `feature_path`; older entries persist for audit. This is the single load-bearing handoff to the auto-poller — without it, the memory-ceremony group has to grep prose, and `/deliver`'s authority becomes guess-shaped.
4. **Flip story + backlog frontmatter** for any linked artifacts: `delivery_status` + `delivery_status_updated` + `delivery_status_source: deliver` (+ `regression_from` if minting `regression`). This is the projection of the manifest row onto the read-side substrate.
5. Print sprint-result path + manifest-append confirmation + one-paragraph summary.

If bailing: same sprint-result template with `disposition: bail-with-followup`, the explicit question, and the search_trail evidencing the bail bar was met. **No manifest append on bail** — bail means no verdict was minted.

## Invariants

- Never declare done on a single render — two consecutive verdicts, one fresh-trigger.
- Never bail with `search_trail` < 7 entries.
- Never edit `consent_required_paths` without explicit user OK.
- Never invoke `/shift` (peer skill, different intent).
- Never skip step 1 (refresh on vision-goal) — drift is the failure mode.
- Never declare `delivered` without citing plan_deliverables verbatim.
- **Never confer `active.*` / `stable` / `regression` without appending to `.claude/deliver/manifest.json` first.** The manifest is the audit trail; story/backlog frontmatter writes are projections of it.
- **Never advance `delivery_status` above `wip` when the canonical `.feature` file is missing on disk** (step 3a existence gate). Surface the gap to the cartographer; do not paper over it.
- **Never overwrite a prior verdict in-place.** Append a new row; the manifest is append-only history with latest-wins semantics.

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

## Authority boundary — two-system handshake with the memory-ceremony group

`/deliver` and the memory-ceremony group (librarian / historian / storyteller / cartographer) form a **writer-and-reader handshake** across the delivery axis. The boundary is intentional and asymmetric:

| Direction | Who | What |
|---|---|---|
| **Write** | `/deliver` only | Mints `active.*` / `stable` / `regression` to the manifest, **at the feature-level only** (and onto the story/backlog frontmatter for the **immediately-linked** feature). `/deliver` writes one verdict per feature it judges; story-level aggregates are derived elsewhere. |
| **Read via bridge** | memory-ceremony group (via `.claude/scripts/memory-kit/delivery-status-poll.py` auto-poller) | Reads `.claude/deliver/manifest.json`, **aggregates per-feature verdicts into per-story `delivery_status` via the weakest-link policy** documented at LIFECYCLE.md → "Story-level aggregation from feature verdicts", and writes the aggregated story-level value back to story frontmatter |
| **Floor only** | auto-poller fallback | When `/deliver` has not judged a feature, the auto-poller may write `envisioned` / `backlog` / `refined` / `wip` from raw a2o signals — **never** `active.*` / `stable` / `regression` |
| **Never write** | memory-ceremony group | Does not author `active.*` / `stable` / `regression` under any circumstance |

**Story-level aggregation is the bridge's job, not `/deliver`'s.** When `/deliver` writes a per-feature verdict, the aggregate effect on linked stories (whose `feature:` triple + `adjacent_features[]` include this feature) is computed downstream by the bridge per the weakest-link rule. See `.claude/scripts/memory-kit/LIFECYCLE.md` → "Story-level aggregation from feature verdicts" for the gradient order and the `regression`-propagates-UP exception. This keeps `/deliver` focused on the rendered-experience falsifier; per-iteration `/deliver` runs only need to think about one feature at a time.

**When `/deliver` is unsure**, write `pending` — never guess upward. The memory-ceremony group's Wave 2 storyteller defers any graduation that depends on `pending` features (storyteller's `graduated-narratively` vs `graduated-fully` branch reads `delivery_status`; `pending` defaults safe to `graduated-narratively` with a `delivery-poll-stale` flag for the librarian).

The bridge's read-only discipline keeps the memory-ceremony group honest: they cannot mint substrate-truth from narrative-truth alone. The whole point of the axis split (`status:` vs `delivery_status:`) is that **narrative truth and substrate truth are different categories of truth** ([[feedback_story_delivery_status_axis]]). `/deliver` is the only artifact that produces substrate-truth at-or-above `active.alpha`.

## Reference

- **Spec:** `genesis/docs/superpowers/specs/2026-05-06-deliver-skill-design.md`
- **Schema:** `.claude/schemas/feature-promise.schema.json`
- **Lifecycle / authority matrix:** `.claude/scripts/memory-kit/LIFECYCLE.md` — "The author/delivery axis split" section is the authoritative reference for `status:` vs `delivery_status:` ownership across the memory-ceremony group, `/deliver`, and the auto-poller
- **Design wisdom:** `.claude/memory/feedback_story_delivery_status_axis.md` — the four-way convergence that surfaced the orthogonal-axes model in Memory Ceremony Run #2 (2026-05-14)
- **Stories schema:** `genesis/data/stories/CONVENTIONS.md` — `delivery_status` frontmatter field; written by `/deliver` via the bridge, read-only to storyteller
- **Timeline schema:** `genesis/data/timeline/CONVENTIONS.md` — backlog `status:` extension to the unified gradient; `regression_from` field
- **Memory-ceremony skill:** `.claude/skills/memory-ceremony/SKILL.md` — Wave 1 graduation-delivery-gap check; Wave 2 `graduated-narratively` vs `graduated-fully` disposition split
- **Sprint-report schema:** `genesis/a2o/schemas/sprint-report.schema.json` — `summary.visualValidation` 2×2 matrix used in step 3a tuple
- **Sibling skill:** `agentic-developer` (`/shift`) — peer, different intent
- **Required pre-read:** `pipeline-diagnostics` skill — Jenkins-fetch patterns
- **Required agent definitions:** `ci-observer.md` (with visual-triage mode), `ci-investigator.md`
- **Sub-skills dispatched per Diagnose gap table** — don't reinvent any of them

---

*Design source: this skill's delivery-axis authority and manifest schema are operationalized from [[feedback_story_delivery_status_axis]] (Memory Ceremony Run #2 retro, 2026-05-14) and the resolved storage classification in `/projects/.claude-config/plans/stateless-seeking-forest.md`.*
