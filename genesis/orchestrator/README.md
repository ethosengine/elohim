# Elohim Orchestrator

The **central controller** for all Elohim CI/CD pipelines. This is the ONLY pipeline that receives GitHub webhooks - all other pipelines are triggered by the orchestrator.

## Architecture

```
GitHub Webhook → Orchestrator → Analyze Changesets → Trigger Pipelines → Report
                                       ↓
                           Health checks & notifications
```

## How It Works

1. **Receive webhook** - GitHub pushes trigger the orchestrator
2. **Analyze changesets** - Determine which files changed
3. **Map to pipelines** - Match changed files to pipeline patterns
4. **Trigger in order** - Respect dependency graph (holochain → edge/app → genesis)
5. **Report status** - Update build description with results

## Pipeline Configuration

Each pipeline declares its metadata in a `build-manifest.json` file. The orchestrator uses `graph-walker.mjs` to walk these manifests and build a dependency graph.

### Example: `elohim/holochain/build-manifest.json`

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-holochain",
  "jenkinsPath": "Jenkinsfile",
  "manualOnly": false,
  "triggersGenesis": true,
  "cascades": true,
  "dependsOn": [],
  "changePatterns": [
    "holochain/dna/",
    "holochain/holochain-cache-core/"
  ],
  "steps": {
    "build": { "stage": "Build", "step": "cargo build" },
    "test": { "stage": "Test", "step": "cargo test" }
  },
  "gate": {
    "metric": "testCoverage",
    "minimum": 75
  },
  "deployment": {
    "environments": ["alpha", "staging", "prod"]
  }
}
```

### How to Add a New Pipeline

1. Create `<project>/build-manifest.json` with `pipeline`, `jenkinsPath`, `changePatterns`, `dependsOn`, and other metadata
2. Ensure the Jenkinsfile at `jenkinsPath` exists and validates `UpstreamCause` or `UserIdCause`
3. Run `node genesis/orchestrator/scripts/generate-pipeline-list.mjs` to update the Bash-consumable artifact
4. Commit both files

The orchestrator automatically discovers all manifests at startup.

## Dependency Graph

```
elohim-holochain ──┬──► elohim-edge ────┐
                   ├──► elohim (app) ───┼──► elohim-genesis
                   └──► elohim-steward  │
                        (manual only)   └──────────────────►
```

## Health Endpoints

The orchestrator monitors these endpoints after deployments:

| Endpoint | URL |
|----------|-----|
| doorway-dev | https://doorway-alpha.elohim.host/health |
| doorway-prod | https://doorway.elohim.host/health |
| alpha | https://alpha.elohim.host |
| staging | https://staging.elohim.host |
| prod | https://elohim.host |

## Key Behaviors

### Skipped Pipelines
Individual pipelines check if they were triggered by the orchestrator. If triggered directly by webhook (not orchestrator), they show `NOT_BUILT` instead of running.

> **Measure semantics — `NOT_BUILT`/`ABORTED`/superseded are NOT success and NOT failure.** A `NOT_BUILT` child means "didn't run", not "passed". Do not let a downstream readiness/measure step read `NOT_BUILT`/`ABORTED`/`UNSTABLE` as green or as 0-failures — that is a *lossy* measure that has repeatedly masked real regressions. When `abortPrevious` preempts an in-flight child (a new push superseding an older one), the superseded build lands `ABORTED`; reading that as a pass is the single most common false "it's fixed" signal. Tighten any pass-test to require `lastBuild.commit == HEAD` AND a non-`NOT_BUILT`/non-`ABORTED` result before trusting it.

### Baseline state
Each pipeline carries a per-pipeline baseline (the last build the orchestrator considers known-good). **Watch-out — baseline-rollback over-build:** a `FAILURE`/`ABORTED` result can invalidate the per-pipeline baseline and roll back to the *global* baseline, which then fans out into a full cascade rebuild; `lastSuccessful()` can pin an ancient green build that no longer reflects HEAD. The baseline should advance only on a *confirmed-downstream-success*, never on a dispatch that merely started. (Backlog: convert the baseline into an explicit state machine + a `build-manifest ⊆ orchestrator changePatterns` drift test — see the recurring-anti-patterns museum record below.)

### Genesis Triggering
Genesis is triggered automatically after ALL dependent pipelines succeed. It auto-detects the target environment from the branch.

### Manual-Only Pipelines
`elohim-steward` is marked `manualOnly: true` - the orchestrator never triggers it automatically.

### Triggers — webhook double-fire
**Watch-out — one dev push can produce two orchestrator builds.** When a Jenkinsfile declares an explicit `triggers { githubPush() }` AND the job is a Multibranch item (which fires its own implicit branch-indexing trigger), a single push fires *both*; the first build is immediately superseded (lands `ABORTED` — see measure semantics above) and looks like a phantom failure. The fix is to drop the explicit `triggers { githubPush() }` and rely on the Multibranch implicit trigger alone. A sibling variant is timer/cron collision with the webhook window (a scheduled build colliding with a late-EDT/PDT push) — reschedule the cron off that window. (Backlog: `orchestrator-trigger-dedup`.)

## Recurring CI/orchestrator anti-patterns

The frequency-ranked, curated catalog of recurring CI/orchestrator/build failure modes (measure-semantics, baseline-rollback, Dockerfile/build-manifest completeness, `HUSKY=0`-is-non-functional, sccache-poisons-rustc-output, `#[ignore]`-is-a-CI-no-op, cucumber parse-abort, CPS method-size, webhook double-fire) lives in the history museum:
**`genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`**.
Read it before debugging a "regression" or proposing a measure/baseline change — most of these have cost real shift time more than once and each carries the specific symptom + the fix.

## Before Editing Orchestrator Dispatch Logic

Read the substrate pieces before touching Execute Builds, dispatch ordering, or trigger logic:
- `graph-walker.mjs` (`walkGraph`) — JS change-detection for local pre-push; reads `build-manifest.json` source-globs to compute which pipelines are affected
- `build-graph.groovy` (`walkBuildGraph`) — the server-side Groovy mirror of the same manifest-walk; runs in Jenkins
- `preview.mjs` — `just ci-preview`, imports `pipeline-registry.mjs` + `graph-walker.mjs` to print predicted dispatch locally pre-push
- `Jenkinsfile` `groupByDependencyLevel` and `triggerPipeline` (Groovy dispatch loop)

Key invariants that naive edits break: `levelFailed` guard must abort downstream; baselines advance only after confirmed success; `cascades: false` pipelines (sophia, epr) opt out of downstream auto-include; Genesis is intentionally outside the levels loop. Any edit to `orderByDependencies`, `groupByDependencyLevel`, `propagateDependencies`, or pipeline metadata in `build-manifest.json` files must be reflected in `build-graph.groovy`.

Note: `graph-walker.mjs` is per-pipeline manifest-step gating (change detection + lint/test); `groupByDependencyLevel` is orchestrator-level dispatch ordering. Different layers, different concerns.

## Predictive Build-Graph Vision

The long-term target: predict what will run before you push, reconcile against what actually ran, and treat every disconnect as an investigation. Three-hour build runs hide cascading failures inside a single opaque "Execute Builds" stage; visible structure surfaces drift before it becomes a mystery.

The substrate already exists — don't rebuild it:
- `preview.mjs` (`walkGraph` via `graph-walker.mjs`) computes the predicted dispatch graph locally pre-push
- `pipeline-registry.mjs` is the single source of pipeline metadata (loaded from `build-manifest.json` files)

Planned iterations (each safe to land independently):
1. **Visibility** — nest stages inside Execute Builds so Blue Ocean shows level structure with per-pipeline timing. Presentation only, no behavior change.
2. **Reconciliation artifact** — emit `predicted-build-graph.json` (from `walkGraph`) and `actual-build-graph.json` (from Execute Builds results), then diff in a Reconcile stage.
3. **Drift escalation** — any predicted-vs-actual disconnect marks UNSTABLE with an investigation pointer.

## Seed Stage — Per-Peer, Not All-or-Nothing

When one peer's conductor admin WebSocket is down, the seeder continues against ready peers and reports the unready one as partial — it does not abort the whole stage. Partial-cluster is the steady state in P2P architecture; an all-or-nothing seeder pretends the substrate is monolithic and masks per-peer health.

Rules:
- Readiness probes belong at the per-peer level; gate nothing globally on one pod's health.
- Record a per-peer readiness snapshot at start; seed ready peers; surface unready peers in the report.
- `actual-build-graph.json` `results` map carries per-peer status; downstream advisories decide whether partial-seed warrants UNSTABLE or informational.
- E2E tests targeting a specific peer should skip-with-reason if that peer was unready, not fail-cascade.

## Troubleshooting

**Q: Pipeline shows NOT_BUILT?**
- Expected! The orchestrator didn't trigger it because no relevant files changed.

**Q: Genesis not running?**
- Check if all dependencies (holochain, edge, app) succeeded.
- Genesis only runs after successful builds.

**Q: Wrong environment targeted?**
- Check the branch. Orchestrator passes branch info to pipelines.
- dev/feat-*/claude/* → alpha, staging* → staging, main → prod
