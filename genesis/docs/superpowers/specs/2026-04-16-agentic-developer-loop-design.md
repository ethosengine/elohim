# Agentic Developer Loop — Design

**Status:** Design — approved, pending implementation plan
**Date:** 2026-04-16
**Branch:** `dev` (brainstormed session on `dev`)
**Related:**
- `elohim/brit/docs/plans/phases/phase-2a-build-attestation-primitives.md`
- `rakia/docs/plans/build-attestation-integration.md`
- `.claude/skills/pipeline-diagnostics/SKILL.md` (superseded in part by this system)
- `.claude/skills/ci-triage/SKILL.md` (superseded in part by this system)

## Summary

A first-class agentic developer that closes the loop on Jenkins pipeline failures through Objective-driven iteration. A human kicks off a shift with a named Objective; the agent iterates — observe, orchestrate, attempt, measure, judge — until the Objective's predicate holds (with stability) or the agent bails with an explicit question. Every shift produces a single markdown artifact that serves as journal, handoff, and retrospective input.

The design targets overnight operation without the human being paged for permission prompts, while keeping the blast radius bounded and the judgment surface always in the top-tier model.

## Non-goals

- Not a build monitor or log summariser in isolation. The loop body produces real changes (commits, retriggers, reports) against real Objectives.
- Not a substitute for the pre-push quality gates that handle lint/format/typo failures. The agent's scope is complex integration and deployment failures that reach Jenkins despite clean pre-push.
- Not concurrent — v1 runs one Objective at a time.
- Not webhook-driven in v1 (platform does not yet support inbound webhooks to this workspace). `/loop dynamic` is the kickoff mechanism; webhook is the north-star trigger model.

## Principles

These are the load-bearing design commitments. Depart from them only with explicit cause.

### P1 — First-class agentic developer, not a watcher

The system is a developer with the full dev cycle: claim work (Objective), understand (observe + verify), fix (attempt), prove (measure), close (declare done or bail cleanly). Language and framing throughout should reflect agency and ownership. Never "babysitter," "sitter," or "monitor."

### P2 — Iterations are the scarce resource, not tokens

One iteration ≈ one real pipeline run (up to an hour wall-clock + real compute + possible deployment side effects). A cheap model that needs 5 iterations is far more expensive than an expensive model that needs 2. Therefore:

- Opus orchestrates every iteration and attempts the work.
- Haiku's role is strictly data reduction (log compression, structured summaries) — never decisions.
- Sonnet's role, when invoked, is as Opus's delegate for specific sub-tasks — not autonomous orchestration.
- There is no "periodic Opus safety cadence" because every iteration is already an Opus iteration.

### P3 — Judgment governs loop length, budget is the safety net

Opus can bail at any iteration with an explicit question and a reason. Budget (iteration count, wall clock) exists only as a backstop against runaway loops. The primary exit signal is agent judgment about trajectory.

### P4 — Done is measurable, stable, and predicate-shaped

"Done" is an Objective's predicate evaluating true against a trusted measurement, with stability — two consecutive passing measurements with at least one on a fresh trigger. A single green build does not close the shift.

### P5 — The allowlist is the trusted authority surface

`.claude/settings.json` + `.claude/settings.local.json` entries are pre-approved. The pain is specificity, not trust. Make patterns as broad as safely possible (within a published safety taxonomy) rather than adding hundreds of near-duplicate literal entries.

### P6 — Wishlist feedback loops refine the environment

Every shift produces structured wishlists — commands the agent wanted but couldn't run, and pipeline output patterns that wasted the agent's effort. Humans action these between sprints; each shift leaves the environment slightly better for the next. No extra agent calls; both wishlists are passive byproducts of work already happening.

### P7 — The agent may not edit the judge

Opus may not modify the Objective, the measurement command, the files the measurement command reads, the journal schema, the test runners, or the test fixtures against which pass-rate is measured. Fixing a failing test by weakening the test is a bail with a proposed change, not an autonomous edit.

## Artifacts

Eight artifacts constitute the v1 surface.

| Artifact | Location | Git | Purpose |
|---|---|---|---|
| `/shift` slash command | `.claude/commands/shift.md` | committed | Entry point. Invokes the skill. |
| `agentic-developer` skill | `.claude/skills/agentic-developer/SKILL.md` | committed | Orchestration playbook: kickoff → iterate → close. |
| `/generalize-permissions` skill | `.claude/skills/generalize-permissions/SKILL.md` | committed | Deterministic allowlist clustering with safety taxonomy. Run standalone when the allowlist gets bloated. |
| Objective YAML | `.claude/shifts/<shift-id>.objective.yaml` | gitignored | Authored interactively at kickoff. Read each iteration. |
| Shift journal / sprint result | `.claude/shifts/<shift-id>.journal.md` | gitignored | Appended each iteration; becomes sprint result on close. |
| Durable palette | `.claude/settings.json` | committed | Patterns graduated from past shifts' wishlists. |
| Shift-scoped palette | `.claude/settings.local.json` | local, gitignored | Shift-id-tagged additions, cleaned at close. |
| Objective schema | `.claude/schemas/objective.schema.json` | committed | Validates Objective YAML at kickoff. |

**Shift id format:** `<ISO-timestamp-no-colons>-<objective-slug>` — e.g. `2026-04-16T22-30-lift-edge-pipeline-to-green`.

## Lifecycle

Three phases in the loop, plus one out-of-loop prerequisite.

### Pre-shift readiness check (per shift, first 60 seconds of kickoff)

Before iteration 1, verify the environment or abort:

- **Tokens valid.** Jenkins MCP `whoami`, `gh auth status`, `git remote` reachable.
- **Connections live.** `mcp__jenkins__getStatus` returns 200; `git push --dry-run` authenticates; any other MCP endpoints the Objective depends on respond.
- **Objective measure runs.** Execute `objective.measure` once. Confirm it returns a parseable result. Record the baseline measurement for iteration 1's delta.
- **Palette sanity.** Pattern-match Opus's planned command set against the union of `settings.json` and `settings.local.json`. Gaps become the shift-scoped palette proposal for explicit user approval.
- **Git state clean.** No uncommitted work that would get mixed into auto-commits.

Any failure → shift aborts with a readiness report. No wasted iterations on a broken environment.

### Iteration loop

See "Iteration anatomy" below. Runs until done or bail.

### Close

Three exit paths, one artifact:

- **Done.** Stability gate satisfied. Final journal stanza declares done with evidence (two consecutive passing measurements, SHA of landing commit if any).
- **Bail.** Opus writes explicit question + reason + proposed next step in the final stanza.
- **Interrupted.** `/shift` stopped externally. Partial state is serialized as a stanza.

All three produce `.claude/shifts/<shift-id>.journal.md` as the sprint result, including the proposed palette additions (wishlist + blockers) and observed anti-patterns. Shift-scoped `settings.local.json` entries are cleaned out.

### Pre-sprint pipeline readiness (out-of-loop, passive)

Orthogonal to the lifecycle. Haiku's output schema carries an `observed_anti_patterns` field. Each iteration's reductions surface pipeline output patterns that wasted effort — full logs where summaries would do, repetitive listings, pre-emptive noise, missing stack-trace extraction, unparseable stage boundaries. The sprint result surfaces them under "Proposed pipeline legibility improvements," evidence-attached. Humans (or a scoped follow-up shift) address them between sprints.

No standalone gut-check skill. Anti-pattern noting is a passive byproduct of Haiku's existing work.

## Iteration anatomy

### Steps (Opus orchestrates)

1. **Ground.** Read Objective, last 2-3 journal stanzas, current palette. Decide iteration type: *observe-only*, *act-on-hypothesis*, *retrigger*, *verify-done-candidate*, or *bail*.
2. **Observe.** Dispatch Haiku to fetch + summarise the current state (`objective.measure` output, supporting artifacts). Haiku returns the schema below.
3. **Verify (optional).** If Haiku's `confidence` is low, or if Opus suspects the summary (contradicts prior finding, missing critical detail, suspiciously clean), dispatch Sonnet with a specific directive.
4. **Act.** Do one of: write patch (Edit/Write), commit + push via palette-approved git commands, retrigger build via Jenkins MCP, or nothing (waiting on prior action).
5. **Measure.** Wait for build/test completion (ScheduleWakeup delay ≈ pipeline run time + buffer). Run `objective.measure`; capture numeric result.
6. **Judge.** Decide: *progress* (measurement moved toward target), *stall* (no delta N iterations running), *novel* (unexpected symptom), *done-candidate* (predicate holds — triggers verification iteration), *bail*.
7. **Journal.** Append iteration stanza.
8. **Next.** Done (stability satisfied) → exit. Bail → exit. Otherwise `ScheduleWakeup`.

### Haiku output schema

```yaml
iteration: <int>
measurement:
  value: <number>
  delta: <signed-number>     # vs. previous iteration
  baseline: <number>
  target: <number>
context:
  build_id: <int>
  status: <passed|failed|running>
  first_failing_stage: <string or null>
primary_failure:
  error_class: <short tag>
  evidence: |
    <5-10 most relevant log lines, extracted>
  files_mentioned:
    - <path>
observed_anti_patterns:
  - pattern: <short description>
    evidence: <one-line excerpt>
confidence: <low|medium|high>
```

`confidence: low` is Opus's prompt to dispatch Sonnet.

### Sonnet delegation contract

Sonnet is invoked with (a) a specific directive and (b) a copy of the current palette. Sonnet:

- **Pattern-matches intended commands against the palette before invoking bash.** No match → does not try. Returns finding + `wanted_commands: []` describing each blocked intent with its purpose.
- **Does not escalate directly.** All permission signal flows back to Opus, which is the only voice the journal speaks with.

Opus curates Sonnet's `wanted_commands` into one of three tiers (see Authority model → Wishlist tiers).

### Action types

| Action | Requires | Guardrail |
|---|---|---|
| Write/edit file | Path in `objective.scope.paths` AND not in high-risk denylist AND not a measurement-oracle file | Oracle immutability (P7) applies |
| Commit + push | Palette has `git add`, `git commit`, `git push origin <branch>` | Never force-push, amend, rebase, branch delete |
| Retrigger build | Palette has `mcp__jenkins__triggerBuild` | Counts against iteration budget |
| Dispatch Sonnet | Always allowed | Logged in journal with directive |
| Bail | Always allowed | Produces question + reason stanza |

### Stability tracking

Journal header carries a stability counter. Increments on passing measurement, resets on failing. Opus declares done only when:

- `counter >= objective.target.stability.consecutive` (default 2)
- AND at least one of those measurements came from a fresh trigger path (not a manual retry of the prior build)

First passing measurement → iteration is tagged *done-candidate*, next iteration is an explicit verification (retrigger + wait + re-measure) before done is declared.

## Objective schema

Shape is binary + progressive metric + baseline floor + stability. Minimum viable example:

```yaml
objective:
  name: lift-edge-pipeline-to-green
  description: >
    Get the edge pipeline's alpha-deploy stage to green after the StatefulSet
    drift fix; verify stability across two consecutive fresh triggers.

  measure:
    type: cmd
    run: "pnpm exec jenkins-digest <buildId> | jq '.stages[\"alpha-deploy\"].passed'"
    # returns 1 (pass) or 0 (fail)

  target:
    predicate: ">="
    value: 1                          # binary degenerate case of progressive
    stability:
      consecutive: 2
      across_triggers: true

  baseline:
    predicate: ">="
    value: 0                          # must not regress below this floor

  budget:
    iterations: 10
    wall_clock_min: 480

  scope:
    paths:
      - "genesis/orchestrator/**"
      - "elohim/holochain/**"
```

Progressive example (pass-rate goal):

```yaml
objective:
  name: lift-gherkin-pass-rate
  measure:
    run: "pnpm run gherkin:report --json | jq '.rate'"
  target:
    predicate: ">="
    value: 0.20
    stability:
      consecutive: 2
      across_triggers: true
  baseline:
    predicate: ">="
    value: 0.05
  budget: { iterations: 10, wall_clock_min: 480 }
  scope:
    paths:
      - "genesis/a2o/**"
      - "app/elohim-app/src/**"
```

## Authority model

Four layers, checked in order. An action must pass all.

### L1 — Paths allowlist (scoped by Objective)

`objective.scope.paths` is a glob allowlist. Edit/Write tool calls outside these patterns are refused at tool-call time.

### L2 — High-risk path denylist (hardcoded)

Inside `scope.paths`, the following are untouchable without human consent — touching them triggers bail with a proposed change, not autonomous edit:

- `Jenkinsfile`, `**/Jenkinsfile`
- `.husky/**`
- `pnpm-lock.yaml`, `Cargo.lock`
- `package.json` version fields
- `.github/**`
- `CLAUDE.md`, `**/CLAUDE.md`
- `.claude/objectives/<current>.yaml` (no moving goalposts)
- `.claude/shifts/*` files other than Opus's own journal
- `.claude/settings.json` (durable base; shift-scoped entries go to `settings.local.json` only)

### L3 — Measurement-oracle bail gate (categorical)

The agent may not edit the judge (P7). Modifying the Objective, the `measure` command, files the `measure` command reads from, test runners, or test fixtures measured against requires bail + proposed change.

### L4 — Palette (command-level allowlist, two-tier)

- `.claude/settings.json` — committed, durable. Patterns graduated from past shifts.
- `.claude/settings.local.json` — partially local. Shift-scoped additions tagged with `// shift:<id>`, cleaned at close.

Opus checks intended commands against the union before invoking bash. No match = no attempt.

#### Palette safety taxonomy

Patterns follow the taxonomy:

- **Broadly safe to wildcard:** `cargo`, `pnpm`, `npm`, `vitest`, `jest`, `pytest`, `eslint`, `stylelint`, `prettier`, `tsc`, `rustup`
- **Subcommand-scoped:** `git` (add/commit/diff/status/log/show/fetch/pull — safe; push/reset --hard/branch -D/checkout --/--force — prompt or deny). `kubectl` (get/describe — safe; apply/delete/rollout — prompt).
- **Never wildcard:** `rm`, `sudo`, `curl`, `ssh`, `aws`, `gcloud`, `gh delete`, `npm publish`, `cargo publish`

`/generalize-permissions` skill encodes this taxonomy. Its proposals are bulk (10 literal entries collapse to 1 pattern) and never applied silently — always user-approved. Run it standalone when the allowlist gets bloated.

### Wishlist tiers

Three tiers, curated by Opus from its own reflection and from Sonnet's returned `wanted_commands`:

| Tier | Meaning | Sprint result treatment |
|---|---|---|
| **Redirect** | Opus found an approved alternative | Journal note only; not surfaced to human |
| **Wishlist** | Convenience — Opus worked around it | Surfaced: "approve for next shift, low priority" |
| **Blocker** | No workaround; iteration stalled or shift bailed | Surfaced first: "approve before next shift, high priority — this actually stopped work" |

Each wishlist/blocker entry in the sprint result includes:

- Narrow literal pattern + proposed generalization
- Purpose
- Iteration(s) where it arose
- Safety taxonomy note

## Retrospective hooks

The spec makes shift results structured so retros are cheap and evidence-based. Retrospective execution is out of scope for v1 (human process; compiler automation is v2).

### Shift-result sections designed for retrospective harvest

| Section | Captures | Consumer |
|---|---|---|
| **Observed anti-patterns** | Pipeline output that wasted Haiku's effort | Jenkinsfile (short term); migration-bridge to **brit attestation shape** (long term) |
| **Palette wishlist + blockers** | Commands Opus/Sonnet needed | `.claude/settings.json` promotion; migration-bridge to **rakia step-level surface** (long term) |
| **Judgment calls log** | Iterations where Opus bailed, invoked Sonnet for verification, or distrusted a measurement | Objective schema improvements + `agentic-developer` skill playbook |
| **Measurement trustworthiness notes** | Low-confidence Haiku calls; regressions after "done"; oracle skepticism events | Objective predicate tightening; stability-gate re-tuning |

### Cross-shift aggregation template

Sprint-end retrospective reads all sprint shift results and produces `docs/retrospectives/sprint-<date>.md` with:

- **Top anti-patterns by frequency** (permanent section)
- **Graduated palette entries** — wishlist items recurring across shifts, promoted in one batched PR (permanent section)
- **Implications for brit** — observed pain → proposed attestation field/shape. *Migration-bridge; remove once brit Phase 2a + rakia attestation-consumption lands.*
- **Implications for rakia** — observed orchestrator gaps → proposed rakia behavior. *Migration-bridge; remove once rakia is the primary orchestrator.*
- **Implications for the agentic-developer itself** — v1.1 priorities

A `docs/retrospectives/TEMPLATE.md` with these sections as fillable bodies ships with this spec. The two migration-bridge sections are explicitly annotated as temporary in the template header.

## Open questions / deferred items

### v1-scope open

- **Shift resumption.** If Opus bails with a question and the human answers in the morning, is there a `/shift resume <id>` that reads the old journal + the answer and continues, or is each shift atomic (human starts a new one with a tighter Objective)? Needs decision before implementation begins.
- **Journal context window strategy.** Does the journal carry a "trajectory summary" header always reflecting the last 3 iterations, or does Opus re-ground from the full journal every iteration? Matters for long shifts' context window. Default proposal: trajectory summary header, full journal on request.

### v2 deferred

- **Objective catalog.** Promote recurring Objectives to `.claude/objectives/*.yaml` once shift results show patterns repeat.
- **Webhook trigger.** Replace `/loop dynamic` kickoff with Jenkins post-build webhook once platform supports it.
- **Multiple concurrent shifts.** One Objective at a time is a v1 limit, not a design limit.
- **Retrospective compiler.** An agentic shift whose Objective is "produce sprint retrospective from this sprint's shift results."
- **Stability defaults review.** Current defaults (`consecutive=2`, `across_triggers=true`) are guesses; re-tune from false-positive rate after first sprints.
- **Sonnet delegation playbook.** Common Opus→Sonnet directives graduate to a small named catalog over time.

## Decision log

Key design commitments made during brainstorming, preserved for later audit.

| Decision | Rationale |
|---|---|
| Opus orchestrates every iteration | Iterations are the scarce resource; cheap orchestration that needs more iterations is net more expensive (P2). |
| No periodic Opus safety cadence | Every iteration is already an Opus iteration. |
| Sonnet never escalates directly | Keeps the journal single-voiced; Opus is the sole palette curator. |
| Dropped line-change budget | Arbitrary; paths-allowlist + denylist + oracle gate are the real guardrails. |
| Pre-sprint gut-check via passive Haiku observation, not a standalone skill | Cheaper (zero extra calls); evidence-based (flagged from real logs). |
| Retrospective compiler out of scope for v1 | Let first few retros be human-written; compiler can learn from their shape. |
| Brit/rakia retrospective sections marked migration-bridge | Those downstream consumers exist only while Jenkins is orchestrator; delete once attestations and rakia land. |
