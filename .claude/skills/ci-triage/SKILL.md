---
name: ci-triage
description: Surface-level CI/CD failure diagnosis from build logs — under 5 minutes per build. Use when "why did the build fail?", "is this red?", or quick health check on a pipeline. For multi-build flake pattern analysis or deeper investigation, escalate to ci-investigator.
---

# CI Triage

## Quick Start

```
1. getBuild jobFullName="elohim-orchestrator"
2. If failed → fetch ci-summary.json artifact
3. Use category-specific search from taxonomy
```

**ci-summary.json URL**: `https://jenkins.ethosengine.com/job/elohim-orchestrator/job/{branch}/{buildNumber}/artifact/ci-summary.json`

## Auth model

MCP runs as anonymous against this Jenkins (OIDC realm — explicit auth would redirect-loop). Read tools work freely; `triggerBuild` and `updateBuild` will fail. To re-run a build, push an empty commit with a `[build:<pipeline>]` tag — see pipeline-diagnostics for the full list.

## Search Strategy

**Rule: Search first, paginate second. Never fetch full logs.**

| Category | Pattern | ctx | max |
|----------|---------|-----|-----|
| DNA_BUILD | `error\[E` | 5 | 10 |
| APP_BUILD | `error TS\d+` | 3 | 20 |
| INFRASTRUCTURE | `hApp.*not found` | 5 | 10 |
| SEEDING | `PREFLIGHT\|ETIMEDOUT` | 10 | 3 |

**Quick scan**: `searchBuildLog pattern="ERROR|FAILED" maxMatches=5`
**Tail logs**: `getBuildLog limit=-100`

## Dependency Order

When multiple fail, check upstream first:
```
holochain → edge → app → genesis
```

| Combo | Root Cause |
|-------|------------|
| holochain + edge | holochain (Rust/WASM) |
| edge + genesis | edge (container/deploy) |
| genesis only | environment/connectivity |

## Health Checks

```bash
curl -s https://doorway-alpha.elohim.host/health | jq
curl -s https://alpha.elohim.host -o /dev/null -w '%{http_code}'
```

## See Also

- `.claude/data/failure-taxonomy.json` - Full category definitions
- `ci-observer` agent (Haiku) - Always-first absorber of Jenkins MCP data; returns structured summary
- `ci-investigator` agent (Sonnet) - Deep investigation when observer confidence is low or cross-build correlation is needed
