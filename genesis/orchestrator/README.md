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

## Troubleshooting

**Q: Pipeline shows NOT_BUILT?**
- Expected! The orchestrator didn't trigger it because no relevant files changed.

**Q: Genesis not running?**
- Check if all dependencies (holochain, edge, app) succeeded.
- Genesis only runs after successful builds.

**Q: Wrong environment targeted?**
- Check the branch. Orchestrator passes branch info to pipelines.
- dev/feat-*/claude/* → alpha, staging* → staging, main → prod
