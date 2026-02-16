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

```groovy
PIPELINES = [
    'elohim-holochain': [
        changePatterns: ['holochain/dna/', 'holochain/holochain-cache-core/'],
        dependsOn: [],
        triggersGenesis: true
    ],
    'elohim-edge': [
        changePatterns: ['doorway/', 'doorway-app/', 'holochain/edgenode/', 'holochain/manifests/'],
        dependsOn: ['elohim-holochain'],
        triggersGenesis: true
    ],
    'elohim': [
        changePatterns: ['elohim-app/', 'elohim-library/', 'VERSION'],
        dependsOn: ['elohim-holochain'],
        triggersGenesis: true
    ],
    'elohim-genesis': [
        changePatterns: ['genesis/'],
        dependsOn: ['elohim-holochain', 'elohim-edge', 'elohim'],
        triggersGenesis: false
    ],
    'elohim-steward': [
        changePatterns: ['steward/'],
        dependsOn: ['elohim-holochain'],
        manualOnly: true
    ]
]
```

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
