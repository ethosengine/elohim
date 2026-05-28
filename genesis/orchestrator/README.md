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

### Genesis Triggering
Genesis is triggered automatically after ALL dependent pipelines succeed. It auto-detects the target environment from the branch.

### Manual-Only Pipelines
`elohim-steward` is marked `manualOnly: true` - the orchestrator never triggers it automatically.

## Troubleshooting

**Q: Pipeline shows NOT_BUILT?**
- Expected! The orchestrator didn't trigger it because no relevant files changed.

**Q: Genesis not running?**
- Check if all dependencies (holochain, edge, app) succeeded.
- Genesis only runs after successful builds.

**Q: Wrong environment targeted?**
- Check the branch. Orchestrator passes branch info to pipelines.
- dev/feat-*/claude/* → alpha, staging* → staging, main → prod
