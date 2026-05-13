---
name: build-manifest.json is required for new orchestrator pipelines
description: Adding a pipeline to the PIPELINES groovy map alone won't trigger it — the build graph (build-manifest.json files) is now authoritative
type: feedback
originSessionId: cc51fa69-af87-4c58-a30c-b86120b754fc
---
When adding a new orchestrator-managed pipeline, you MUST create a `build-manifest.json` for the new project — registering the pipeline in `genesis/orchestrator/Jenkinsfile`'s `PIPELINES` map alone is not enough.

**Why:** Per `genesis/orchestrator/Jenkinsfile:726`, "legacy PIPELINES disagrees with build graph (legacy algorithm is no longer authoritative)". The orchestrator runs both decision systems and treats the build graph (`graph-walker.mjs` reading per-project `build-manifest.json` files) as the source of truth. PIPELINES is kept for advisory cross-checking only. Without a build-manifest.json, the graph returns SKIP and the new pipeline never triggers, even though the PIPELINES groovy entry says BUILD.

Caught by orchestrator build #811 on 2026-05-04 when the elohim-storybook pipeline was registered in PIPELINES but had no build-manifest.json — divergence detected, graph won, new pipeline never fired. Required a follow-up commit (`e2f770a0`) creating `app/elohim-library/build-manifest.json` before the pipeline could trigger.

**How to apply:** Any plan that adds a new orchestrator pipeline MUST include a build-manifest.json task. The minimum shape is:
```json
{
  "manifestVersion": "1.0",
  "pipeline": "<name matching PIPELINES key>",
  "description": "...",
  "steps": { "<step-name>": { "inputs.sources": [...], "buildProcess": ["<Jenkinsfile path>"], ... } },
  "gate": { "projects": { "<pipeline-name>": { "dir": "<project dir>", "steps": [...] } } },
  "deployment": { "targets": { "alpha": { "healthCheck": "..." } } }
}
```

Validate with `cd genesis/orchestrator && pnpm exec node validate-manifests.mjs`.
