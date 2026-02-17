---
name: ci-triage
description: Quick CI/CD failure diagnosis. Use when builds fail.
---

# CI Triage

## Quick Start

```
1. getBuild jobFullName="elohim-orchestrator"
2. If failed → fetch ci-summary.json artifact
3. Use category-specific search from taxonomy
```

**ci-summary.json URL**: `https://jenkins.ethosengine.com/job/elohim-orchestrator/job/{branch}/{buildNumber}/artifact/ci-summary.json`

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
- ci-pipeline agent - Deep investigation with test results
