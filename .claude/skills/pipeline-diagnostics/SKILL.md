---
name: pipeline-diagnostics
description: Use when a build failed, when comparing two Jenkins builds to measure a fix's impact, when checking whether a specific commit landed in a build, when reading a2o sprint-report findings, or when retrieving any archived CI artifact. Also use when `mcp__jenkins__*` tools are absent from your tool list and you'd otherwise give up or grep the repo blindly — public Jenkins URLs + WebFetch work. Triggers: "did my fix land?", "why is the next build still failing?", "what's in the sprint-report?"
---

# Pipeline Diagnostics

This skill helps diagnose CI/CD pipeline issues for the Elohim project using Jenkins. It covers two access paths — Jenkins MCP (preferred when connected) and direct WebFetch against public URLs (fallback, always works) — plus the retrieval workflows for sprint-reports, cucumber reports, and build-to-commit correlation.

## Quickstart: "I pushed a fix — is it in a build yet and did it help?"

This is the workflow run 90% of the time. Don't grep the repo, don't guess URLs — hit these in order:

1. **Confirm the commit is on origin**
   ```
   git log --oneline origin/dev..HEAD      # empty? already pushed.
   git log --oneline -1 origin/dev         # note the SHA
   ```

2. **Find the latest build of the affected pipeline**
   Hit the job index page — it lists the last N builds with status:
   ```
   WebFetch https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/
     prompt: "List last 3 builds with build number, status, timestamp.
              Also list any archived artifacts under genesis/a2o/reports/."
   ```
   Returns build numbers like `#935`, a status, a timestamp, and the artifact tree for the latest. **The index page is your address book — always start here.**

3. **Confirm your SHA landed in that build** (optional but recommended when diagnosing regression vs. unlanded)
   ```
   WebFetch https://jenkins.ethosengine.com/job/<pipeline>/job/dev/<N>/api/json?tree=changeSets[items[commitId,msg]]
     prompt: "List commit SHAs in this build's changeset."
   ```
   If your SHA isn't there, the build ran against an older commit — wait for the next one. Don't interpret findings from a stale build as if they reflect your fix.

4. **Retrieve the sprint-report** (A2O pipeline only) for the findings delta
   ```
   WebFetch https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/<N>/artifact/genesis/a2o/reports/sprint-report.md
     prompt: "Return verbatim. Preserve summary table, pillar headers, all fingerprints."
   ```
   Compare fingerprints against the previous build's report. Fingerprints are stable across runs — the same `aac96b4f6151` today as last week, for the same error signature.

5. **Narrate the delta, not the absolute numbers.** The valuable signal is "fingerprint X dropped from 26 → 0" or "new fingerprint Y appeared" — not "99 failures is bad."

## Address book — Jenkins URL patterns

**Never guess these. Copy from here.**

| Path | Template | Example |
|------|----------|---------|
| Job index (build list) | `/job/<pipeline>/job/<branch>/` | `/job/elohim-genesis/job/dev/` |
| Specific build | `/job/<pipeline>/job/<branch>/<N>/` | `/job/elohim-genesis/job/dev/935/` |
| Latest build (alias) | `/job/<pipeline>/job/<branch>/lastBuild/` | useful when you don't know `N` |
| Last successful | `/job/<pipeline>/job/<branch>/lastSuccessfulBuild/` | skips failed builds |
| Build artifact | `/job/.../<N>/artifact/<repo-relative-path>` | `…/935/artifact/genesis/a2o/reports/sprint-report.md` |
| Console log (raw) | `/job/.../<N>/consoleText` | plain text — huge, prefer searchBuildLog via MCP |
| API (JSON) | `/job/.../<N>/api/json?tree=<fields>` | scoped query; `tree=changeSets[items[commitId]]` for SHAs |

**Base URL**: `https://jenkins.ethosengine.com`

**Branches**: `dev` (main), feature branches become `job/feat-xxxx`, etc. URL-encode slashes as `/job/`.

## Artifact address book (what lives where after a successful build)

| Pipeline | Artifact | Full path |
|----------|---------|-----------|
| `elohim-genesis` | Sprint-report (markdown) | `genesis/a2o/reports/sprint-report.md` |
| `elohim-genesis` | Sprint-report (JSON) | `genesis/a2o/reports/sprint-report.json` |
| `elohim-genesis` | Cucumber HTML | `genesis/a2o/reports/cucumber-report.html` |
| `elohim-genesis` | Cucumber JSON | `genesis/a2o/reports/cucumber-report.json` |
| `elohim-genesis` | Coverage gap report | `genesis/a2o/reports/coverage-gap-report.json` |
| `elohim-genesis` | Per-scenario console errors | `genesis/a2o/reports/console/<scenario>.json` |
| `elohim-app` | Test results | check job's test trend; surface via `mcp__jenkins__getTestResults` |

## Two access paths — pick based on what's available in the session

### Path A — Jenkins MCP (when `mcp__jenkins__*` tools appear in your tool list)

Use MCP tools for anything involving large log searches or test-result enumeration. They return structured data and handle auth.

```
mcp__jenkins__getJob jobFullName="elohim-genesis/dev"
mcp__jenkins__getBuild jobFullName="elohim-genesis/dev" buildNumber=935
mcp__jenkins__searchBuildLog jobFullName="elohim-genesis/dev" pattern="error|failed" ignoreCase=true contextLines=3
mcp__jenkins__getTestResults jobFullName="elohim-app/dev" onlyFailingTests=true
mcp__jenkins__getBuildChangeSets jobFullName="elohim-genesis/dev" buildNumber=935
mcp__jenkins__triggerBuild jobFullName="elohim-genesis/dev"
```

### Path B — WebFetch (when MCP isn't connected, e.g., many agent contexts, some Che sessions)

**This is proven to work against the public Jenkins — no auth needed for read.** Use the URL patterns from the address book above with `WebFetch`. Pros: always available, no session-bootstrap dance. Cons: no log search, no test-result extraction, no trigger.

Detection: check your tool list for `mcp__jenkins__*`. If absent, **do not** waste a turn asking an agent to try MCP — it'll fail the same way. Go straight to WebFetch.

## Red flags — anti-patterns observed when agents DON'T use this skill

| Thought | Reality |
|--------|---------|
| "Jenkins MCP isn't connected, I can't check the build." | WebFetch against the public Jenkins works. The URL patterns in the address book above don't need auth for reads. |
| "Let me grep the repo for the error." | The repo doesn't know what happened in CI. Fetch the sprint-report or console log. |
| "Let me spawn a ci-pipeline subagent to check." | Subagent has the same MCP availability you do. If it's missing for you, it's missing for them. Use WebFetch directly. |
| "I'll assume the build ran against my latest commit." | Assume nothing. Correlate SHA via `changeSets[].items[].commitId` before reading findings as proof-of-fix. |
| "99 → 90 failures means my fix barely worked." | Check fingerprints, not raw counts. A cascade-root fix drops 5 fingerprints at once; net failures may stay flat if pendings change. |
| "The build status is green, so we're done." | Sprint-report aggregator is non-blocking — build can be SUCCESS with 90 scenario failures. Always pull the report. |
| "I'll just read cucumber-report.json, it has everything." | 800KB of raw scenarios. Start with sprint-report.md (12KB, ranked, deduplicated). Drill into cucumber only for specific stack traces. |

## Workflow — sprint-report delta analysis

The sprint-report aggregator (`genesis/a2o/scripts/build-sprint-report.ts`) produces fingerprinted, deduplicated findings. When comparing two builds, the useful operations are:

```
# Baseline (before your fix)
WebFetch …/<baseline>/artifact/genesis/a2o/reports/sprint-report.md

# New (after your fix)
WebFetch …/<new>/artifact/genesis/a2o/reports/sprint-report.md
```

Then:
- Look at the `Summary` table — `scenarios / passed / failed / pending` gives the gross signal.
- Scan fingerprints from baseline — for each one, check if it's absent, reduced, or unchanged in the new report.
- Flag any *new* fingerprints — those are regressions you introduced.
- Cascade collapse: a single infra/config fix can drop 50+ failures at once because downstream scenarios all depended on a blocked prerequisite. Expect the `occurrences:` count to drop multiple fingerprints together.

### Cascade awareness

A2O scenarios form a dependency tree: auth → content → delivery. When a foundational fingerprint like `POST /auth/register 503` is in the report, every scenario that depended on a registered human will also fail (surfacing as `POST /auth/login 401 Invalid credentials` because the fixture human was never created). **Fixing the root often collapses ~80% of the report in one shot.** Sprint-report #934 → #935 went from 99 → 90 failures for the a2o fixes but would have gone to ~10 if the conductor URL fix had worked — that's the cascade.

### Reading a findings fingerprint

Each finding has: a 12-hex fingerprint, a normalized error message, an occurrence count, and the list of affected scenarios. Fingerprints are produced by SHA-256 of the normalized error, so:
- Same error across runs → same fingerprint (safe to track across builds)
- Different runtime IDs/timestamps → still same fingerprint (aggregator strips UUIDs, ports, hashes)
- Structural change to the error message → different fingerprint (treat as new)

## Pipeline Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                     Elohim CI/CD Orchestration                    │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  GitHub Push                                                      │
│      ↓                                                            │
│  Orchestrator  ←── Analyzes changesets, triggers dependencies     │
│      ↓                                                            │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Parallel Builds (if dependencies met):                     │ │
│  │                                                              │ │
│  │  elohim-holochain  →  DNA builds, hApp packaging            │ │
│  │  elohim-edge       →  Doorway, storage containers           │ │
│  │  elohim-app        →  Angular build                         │ │
│  └─────────────────────────────────────────────────────────────┘ │
│      ↓                                                            │
│  elohim-genesis  ←── Seed validation & deployment                 │
│      ↓                                                            │
│  Health Checks  ←── Post-deployment verification                  │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

## Jenkins Job Reference

| Job Name | Purpose | Triggers |
|----------|---------|----------|
| `elohim-orchestrator` | Changeset analysis, pipeline coordination | GitHub webhook |
| `elohim-holochain` | Rust DNA compilation, hApp packaging | Orchestrator |
| `elohim-edge` | Docker containers for doorway, storage | Orchestrator |
| `elohim-app` | Angular build, static assets | Orchestrator |
| `elohim-genesis` | Content seeding, verification | After app/edge |
| `elohim-steward` | Tauri desktop app (manual) | Manual only |

## Using MCP Tools for Diagnostics

### Check Jenkins Status
```
Use mcp__jenkins__getStatus to check Jenkins health
```

### Get Job Information
```
Use mcp__jenkins__getJob with jobFullName="elohim-holochain"
Use mcp__jenkins__getJob with jobFullName="elohim-app"
Use mcp__jenkins__getJob with jobFullName="elohim-genesis"
```

### Get Build Details
```
Use mcp__jenkins__getBuild with jobFullName="elohim-holochain"
  (omit buildNumber for latest)

Use mcp__jenkins__getBuild with jobFullName="elohim-holochain" buildNumber=123
  (specific build)
```

### Analyze Build Logs
```
Use mcp__jenkins__getBuildLog with jobFullName="elohim-holochain"
  limit=-100  (last 100 lines)

Use mcp__jenkins__searchBuildLog with:
  jobFullName="elohim-holochain"
  pattern="error|failed|Error"
  ignoreCase=true
  contextLines=3
```

### Check Test Results
```
Use mcp__jenkins__getTestResults with jobFullName="elohim-app"
  onlyFailingTests=true
```

## Common Failure Patterns

### DNA Build Failures

**Pattern: WASM compilation error**
```
Search logs for: "error\[E" or "cannot find" or "unresolved"
```

**Common causes:**
- Missing RUSTFLAGS for getrandom backend
- Incompatible dependency versions
- Syntax errors in zome code

**Fix checklist:**
1. Check `RUSTFLAGS='--cfg getrandom_backend="custom"'` is set
2. Verify `Cargo.lock` is committed
3. Check zome source for compile errors

### Angular Build Failures

**Pattern: TypeScript errors**
```
Search logs for: "error TS" or "Cannot find module"
```

**Common causes:**
- Type mismatches after model changes
- Missing imports
- Circular dependencies

**Fix checklist:**
1. Run `npm run build` locally
2. Check for type sync between elohim-service and elohim-app
3. Verify all imports resolve

### Seeding Failures

**Pattern: Connection timeout**
```
Search logs for: "ETIMEDOUT" or "WebSocket" or "connection refused"
```

**Common causes:**
- Doorway not ready
- Wrong admin URL
- Network policy blocking

**Fix checklist:**
1. Check doorway health endpoint
2. Verify HOLOCHAIN_ADMIN_URL environment variable
3. Check K8s pod status

**Pattern: Schema validation**
```
Search logs for: "missing required" or "validation failed"
```

**Fix:**
1. Run `npm run validate` in genesis/seeder
2. Check content files for missing id/title fields

### Docker Build Failures

**Pattern: Image build error**
```
Search logs for: "COPY failed" or "RUN failed" or "denied"
```

**Common causes:**
- Missing build artifacts from previous stage
- Harbor registry auth issues
- Dockerfile syntax

## Environment Mapping

| Branch Pattern | Environment | Doorway URL |
|---------------|-------------|-------------|
| `dev`, `feat-*`, `claude-*` | Alpha | doorway-alpha.elohim.host |
| `staging-*` | Staging | doorway-staging.elohim.host |
| `main` | Production | doorway.elohim.host |

## Build-to-commit correlation

When a fix doesn't seem to have worked, first rule out "the build didn't run your commit":

```
# 1. Your local state
git log --oneline -5 origin/dev

# 2. What's in the build
WebFetch https://jenkins.ethosengine.com/job/<pipeline>/job/dev/<N>/api/json?tree=changeSets[items[commitId,msg]],actions[causes[shortDescription]]
  prompt: "List commit SHAs and the build cause."
```

Expected outcomes:
- Your SHA appears in `changeSets[].items[].commitId`: build reflects your fix. If findings haven't changed, the fix itself is wrong.
- Your SHA is NOT there: build ran against an older commit. Wait for the next build. Look at `actions[].causes[]` to see what triggered this one — was it an upstream pipeline, a scheduled run, or your push?
- No changesets at all: the build was triggered manually (e.g., via `mcp__jenkins__triggerBuild`). The SCM revision under `actions` tells you which commit it actually ran against.

### Example: "I pushed 6 commits — did they all land in #935?"

```
git log --oneline origin/dev~10..origin/dev
WebFetch …/935/api/json?tree=changeSets[items[commitId,msg]]
```

Diff the two lists. Commits above `changeSets[-1].commitId` are not in this build — they'll appear in #936+.

## Diagnostic Workflow

### 1. Identify the build
```
MCP:      mcp__jenkins__getJobs, then mcp__jenkins__getBuild
WebFetch: /job/<pipeline>/job/<branch>/  → read build numbers + status
```

### 2. Confirm your commit is in it (see "Build-to-commit correlation" above)

### 3. Get error context
```
MCP:      mcp__jenkins__searchBuildLog pattern="error|failed|Exception|panic" ignoreCase=true contextLines=3
WebFetch: /job/.../<N>/consoleText  — plain text log, use only for small targeted reads
          (MCP's searchBuildLog is dramatically better — prefer it when available)
```

### 4. Analyze stage
Look at the failed stage name to determine which pipeline component failed:
- "Build DNAs" → Rust/WASM issues (RUSTFLAGS, missing deps, zome compile errors)
- "Build App" → Angular/TypeScript issues (type sync, import resolution)
- "Seed Content" → Doorway/conductor connection issues (admin WS port, auth)
- "Deploy" → K8s/Docker issues (image pull, resource limits, service mapping)
- "E2E VERIFICATION (API)" → a2o scenarios — pull sprint-report.md for ranked findings

### 5. Check artifacts (if the stage produced output)
The sprint-report is the fastest path to understanding which tests failed and why. Don't read cucumber-report.json directly unless you need the full stack — the sprint-report has the fingerprinted findings ranked by occurrence.

## Triggering Builds

### Retry Failed Build
```
Use mcp__jenkins__triggerBuild with jobFullName="elohim-holochain"
```

### Trigger with Parameters
```
Use mcp__jenkins__triggerBuild with:
  jobFullName="elohim-genesis"
  parameters={"SKIP_SEEDING": "false", "ENVIRONMENT": "dev"}
```

## Key Jenkinsfile Locations

| File | Purpose |
|------|---------|
| `/projects/elohim/Jenkinsfile` | Root orchestrator |
| `/projects/elohim/genesis/orchestrator/Jenkinsfile` | Pipeline controller |
| `/projects/elohim/holochain/Jenkinsfile` | DNA/hApp builds |
| `/projects/elohim/genesis/Jenkinsfile` | Seeding pipeline |
| `/projects/elohim/steward/Jenkinsfile` | Desktop app |

## Artifact Flow

```
elohim-holochain
    ↓ elohim.happ
elohim-edge
    ↓ doorway:tag, storage:tag
elohim-app
    ↓ dist/elohim-app
elohim-genesis
    ↓ seed verification
```

Each pipeline fetches artifacts from upstream jobs. Check artifact availability if builds fail at fetch stages.

## Quick Diagnostics Commands

### Check all pipeline health
```
1. mcp__jenkins__getStatus (overall Jenkins)
2. mcp__jenkins__getJobs (list jobs)
3. For each job: mcp__jenkins__getBuild (latest status)
```

### Investigate specific failure
```
1. mcp__jenkins__getBuild with jobFullName + buildNumber
2. mcp__jenkins__getBuildLog with limit=-200 (tail)
3. mcp__jenkins__searchBuildLog with error patterns
4. mcp__jenkins__getTestResults if tests failed
```

### Check deployment health
```
After genesis completes, verify via:
- stats:dev / stats:prod commands
- Doorway health endpoints
- Application smoke tests
```
