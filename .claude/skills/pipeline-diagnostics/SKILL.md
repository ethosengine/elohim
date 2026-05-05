---
name: pipeline-diagnostics
description: Use when a build failed, when comparing two Jenkins builds to measure a fix's impact, when checking whether a specific commit landed in a build, when reading a2o sprint-report findings, or when retrieving any archived CI artifact. Also use when `mcp__jenkins__*` tools are absent from your tool list (some subagent contexts) and you'd otherwise give up or grep the repo blindly — public Jenkins URLs + WebFetch work as a fallback. Triggers: "did my fix land?", "why is the next build still failing?", "what's in the sprint-report?"
---

# Pipeline Diagnostics

This skill helps diagnose CI/CD pipeline issues for the Elohim project using Jenkins. It covers two access paths — Jenkins MCP (preferred, primary) and direct WebFetch against public URLs (fallback for contexts where MCP isn't loaded) — plus the retrieval workflows for sprint-reports, cucumber reports, and build-to-commit correlation.

## Auth model — read this first

Jenkins is OIDC-protected. There are **two access paths**, with sharply different guardrails:

**1. Anonymous (default — used by the MCP and unauthenticated WebFetch).** No `Authorization` header — explicit auth would trigger an interactive OIDC login flow and break the connection. The MCP transport runs in this mode. Consequences:
- **All read tools work.** `getBuild`, `getBuildLog`, `searchBuildLog`, `getJob`, `getJobs`, `getBuildChangeSets`, `getTestResults`, `getFlakyFailures`, `getStatus`, `getBuildScm`, `getJobScm` — the anonymous role has Overall.Read and Job.Read on `https://jenkins.ethosengine.com`. Use these freely.
- **Write tools fail.** `mcp__jenkins__triggerBuild` and `mcp__jenkins__updateBuild` won't work via the MCP — anonymous lacks `Job.Build`. For un-parameterized retriggers, use the webhook-driven path: push a commit (optionally with a `[build:<pipeline>]` tag — see "Forcing a pipeline" below).

**2. Authenticated (`JENKINS_USERNAME` + `JENKINS_TOKEN`, orchestrator-only).** Devspace env carries `JENKINS_USERNAME`, `JENKINS_TOKEN`, and `JENKINS_URL`. A direct `curl -u "$JENKINS_USERNAME:$JENKINS_TOKEN"` call against the Jenkins API can do what anonymous can't — **including parameterized builds** (e.g. `RESET_STORAGE=true` for elohim-genesis schema-drift recovery, where the `[build:*]` tag mechanism is insufficient because tags carry pipeline membership but not parameter values).

This second path is **reserved for the shift Opus orchestrator** and may be used autonomously — no per-use user confirmation — but only against **verified** Jenkins state. The orchestrator must KNOW (from reading actual Jenkins responses) that the trigger won't cause pipeline interruptions or build storms. Guessing, "should be fine," or "probably nothing else is running" is disqualifying. See "Parameterized rebuild (authenticated)" below for what verified means, the workflow, and the curl pattern.

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

## Two access paths — MCP first, WebFetch as fallback

### Path A — Jenkins MCP (primary)

The MCP is registered as anonymous in the devspace devfile and connects on workspace start. All read tools work; trigger tools don't (see "Auth model" above). Use MCP for anything involving large log searches, test-result enumeration, or structured changeset queries.

```
mcp__jenkins__getJob jobFullName="elohim-genesis/dev"
mcp__jenkins__getBuild jobFullName="elohim-genesis/dev" buildNumber=935
mcp__jenkins__searchBuildLog jobFullName="elohim-genesis/dev" pattern="error|failed" ignoreCase=true contextLines=3
mcp__jenkins__getTestResults jobFullName="elohim-app/dev" onlyFailingTests=true
mcp__jenkins__getBuildChangeSets jobFullName="elohim-genesis/dev" buildNumber=935
```

To trigger a build, do not call `mcp__jenkins__triggerBuild` — anonymous lacks Job.Build. Push a commit, optionally with a `[build:<pipeline>]` tag (see below) to force the orchestrator to dispatch.

### Path B — WebFetch (fallback)

Use when MCP isn't loaded — typically subagent contexts that don't inherit the parent's MCP set, or new Che sessions before postStart finishes. The URL patterns in the address book above don't need auth for reads. Pros: always available, no session-bootstrap dance. Cons: no log search, no test-result extraction.

Detection: check your tool list for `mcp__jenkins__*`. If absent in a subagent, **do not** waste a turn re-dispatching to ask MCP — it won't appear retroactively. Go straight to WebFetch.

## Pipeline timing reference

Observed durations for the `elohim-*` pipelines on `dev`. Use these to set `ScheduleWakeup.delaySeconds` — not for polling inside a turn.

| Pipeline | Typical duration | Notes |
|----------|-----------------|-------|
| `elohim-orchestrator` | **3-5 min** | Changeset analysis + downstream trigger. Fast because it's mostly pattern-matching file paths. Will supersede an in-flight build if a new push lands before it completes. |
| `elohim-edge` | **25-35 min** | Doorway + elohim-storage Docker image builds. Heavy Rust compilation (cargo build --release) and image publish. The long pole — start here when estimating end-to-end time. |
| `elohim-holochain` | **15-25 min** | hApp packaging + k8s deploy (StatefulSet rollout). Fast build, slower deploy phase (pod restarts + readiness probes). |
| `elohim-genesis` | **10-15 min** | A2O cucumber run + sprint-report generation. Duration is dominated by the E2E scenario timeouts; a broken alpha inflates this. |
| **Full cascade** (push → sprint-report) | **~45-60 min** | Serial-ish: orchestrator blocks trigger, edge runs parallel with holochain, genesis waits for deploy. Budget a full hour from `git push` to when you should read a fresh sprint-report. |

### Wakeup-delay guidance

Pick based on what you're waiting for:

- **Just pushed, waiting for orchestrator to finish analysis:** 300s (5 min) — then you know which downstream pipelines fired.
- **Waiting for edge to finish compiling:** 1200-1800s (20-30 min) — don't poll inside the compile window, you're not going to learn anything new.
- **Waiting for the whole cascade after a push that touches Rust + manifests:** 2400-3000s (40-50 min) — come back after genesis has had a chance to run; if you wake up too early, re-sleep with a shorter delay.
- **Waiting for a cascade where edge is already green and only genesis needs to run:** 900s (15 min).
- **Build is running but stuck/queued:** don't wake up sooner than the p95 duration; you'll just re-sleep.

### Coordination gotchas

- **Orchestrator can supersede a prior in-flight build.** If you push twice quickly, the first orchestrator run is marked "Not built / Superseded" and its downstream triggers can be cancelled. The second run analyzes the cumulative diff since the last green — so downstreams still fire correctly, but any in-flight edge/holochain from the first run may abort mid-way.
- **Edge and holochain run in parallel, not serial.** Holochain may finish (SUCCESS) before edge publishes new images, meaning the deploy ran against the OLD storage image tag (`dev-latest` points at last green). Watch for this: holochain green + edge not-yet-green = Adam is running old code. You need either a fresh holochain trigger after edge, or a rollout restart to pick up the new image tag.
- **`dev-latest` tag race.** StatefulSet pods only re-pull images on pod restart. With `imagePullPolicy: Always` + `dev-latest`, k8s WILL pull the new image, but ONLY when the pod restarts. A StatefulSet rolling restart happens on spec change (env var, mount, resource bump) — NOT on image tag content change. Deploys that rely on `dev-latest` moving must also change something in the pod spec, or include an explicit `kubectl rollout restart statefulset/...` call.
- **Pipeline names vs file paths are not 1:1.** Despite the name, the Jenkins job `elohim-edge` uses `elohim/holochain/Jenkinsfile` (not the DNA-only Jenkinsfile). The DNA-only job is `elohim-holochain`, which uses `elohim/holochain/dna/Jenkinsfile`. `elohim-edge` is what actually builds Rust images *and* deploys humans. Check `genesis/orchestrator/Jenkinsfile` `PIPELINES` map for the authoritative name→jenkinsPath→changePatterns mapping.
- **Orchestrator skips pipelines it thinks are up-to-date.** Even when a file path matches a pipeline's `changePatterns`, the orchestrator uses per-pipeline baselines: if the pipeline was built at a commit that already contained the file in its current state, it skips re-triggering. This bit during a manifest-only restart-annotation push — the annotation was "new" but edge's baseline already included the surrounding manifest, so edge didn't retrigger despite the path match. Escape hatch below.

### Forcing a pipeline with `[build:*]` commit tags

The orchestrator's webhook-trigger branch parses the commit message for `[build:<pipeline>]` tags and adds matching pipelines to `FORCE_BUILD_PIPELINES` regardless of changeset analysis. Supported tags:

| Tag | Pipeline |
|-----|----------|
| `[build:edge]` | elohim-edge (Rust doorway + storage + deploy) |
| `[build:dna]` | elohim-holochain (DNA/hApp only) |
| `[build:app]` | elohim (Angular) |
| `[build:genesis]` | elohim-genesis (a2o + seed) |
| `[build:sophia]` | elohim-sophia |
| `[build:steward]` | elohim-steward |
| `[build:all]` | every non-manualOnly pipeline |

Comma-separated forms work: `[build:edge,genesis]`. The tag must appear in the commit message of the HEAD commit at push time (webhook triggers only). Use this when:
- The orchestrator's changeset analysis is wrong and you've verified path matching should have triggered
- You need a rolling restart that the normal deploy wouldn't cause (pod spec unchanged, but you know the image tag moved)
- You're testing CI changes without meaningful source changes

Don't abuse `[build:all]` — each pipeline costs minutes. Prefer the narrowest tag that'll do the job.

## Red flags — anti-patterns observed when agents DON'T use this skill

| Thought | Reality |
|--------|---------|
| "Jenkins MCP isn't connected, I can't check the build." | MCP is anonymous-mode and connects automatically on workspace start. If it's truly missing in a subagent, fall back to WebFetch — public Jenkins reads need no auth. |
| "Let me call `mcp__jenkins__triggerBuild` to retry." | Anonymous can't trigger builds. Push an empty commit with `[build:<pipeline>]` in the message; the orchestrator webhook will dispatch. |
| "I'll add `Authorization: Basic <token>` to make MCP authenticated." | Don't. OIDC will intercept the auth attempt and 302 you into a redirect loop. Anonymous reads cover the entire diagnostic workflow. |
| "Let me grep the repo for the error." | The repo doesn't know what happened in CI. Fetch the sprint-report or console log. |
| "Let me spawn a ci-investigator subagent to check." | Subagent may or may not have MCP loaded. If missing, it falls back to WebFetch. Either way, scope its task tightly. Prefer `ci-observer` (Haiku) for surface scans; reserve `ci-investigator` (Sonnet) for cross-build correlation or low-confidence escalation. |
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

## Triggering Builds — webhook + commit-tags, not MCP

`mcp__jenkins__triggerBuild` is unavailable to the anonymous MCP user. The canonical trigger surface is the GitHub webhook landing on `elohim-orchestrator`, which analyzes the changeset and dispatches downstream pipelines.

### Retry a failed build for the same commit
Make an empty commit (or amend with `--no-edit` and force-push only on a feature branch you own) and push:
```
git commit --allow-empty -m "ci: retrigger [build:edge]"
git push
```

### Force a pipeline whose changeset analysis missed
Push a commit (or amend the head commit's message) containing a `[build:<pipeline>]` tag:
```
[build:edge]      # elohim-edge (Rust doorway + storage + deploy)
[build:dna]       # elohim-holochain (DNA/hApp only)
[build:app]       # elohim (Angular)
[build:genesis]   # elohim-genesis (a2o + seed)
[build:sophia]    # elohim-sophia
[build:steward]   # elohim-steward
[build:all]       # every non-manualOnly pipeline
```
Comma-separated forms work: `[build:edge,genesis]`. See "Forcing a pipeline with `[build:*]` commit tags" above for full details.

### Trigger with parameters
The webhook path doesn't accept arbitrary build parameters — those are pipeline-defined defaults. To vary `SKIP_SEEDING`, `ENVIRONMENT`, `RESET_STORAGE`, etc., use the **authenticated path** below. (Editing Jenkinsfile defaults and pushing also works for permanent changes, but is wrong for one-off operational unblocks.)

## Parameterized rebuild (authenticated)

This section governs the only sanctioned way for Claude to trigger parameterized Jenkins builds. The credentials live in env (`JENKINS_USERNAME`, `JENKINS_TOKEN`, `JENKINS_URL`) and authenticate against `Job.Build` permission, which the anonymous MCP lacks. Power comes with a strict guardrail surface — read both halves before reaching for the curl.

### Who may use this

**Only the shift Opus orchestrator** — and even within /shift, only the orchestrator itself, never the subagents. ci-observer and ci-investigator are read-only instruments that feed Jenkins state to the orchestrator; they do not invoke triggers. The orchestrator owns diagnosis, verification, and the curl.

### When it's appropriate

The `[build:*]` tag mechanism on commit messages forces pipeline membership but cannot pass parameter values. Use the authenticated path when:

- A parameterized stage gates the actual fix. Canonical case: `elohim-genesis` Seed Database stage's `RESET_STORAGE=true` parameter, which clears `content.db` to recover from schema drift (see `feedback_seed_lock_means_schema_drift` memory). Tag-based retrigger inherits the default `false` and reproduces the failure.
- A diagnostic rebuild needs a value other than the Jenkinsfile default (`SKIP_SEEDING=true`, narrowed `STEPS=...`, alternate `ENVIRONMENT`).
- The webhook path can't carry the operational intent.

Do **not** reach for the authenticated path when an empty commit with `[build:<pipeline>]` would do — that path is the default for the same reason.

### Hard guardrails

These are structural preconditions, not procedural confirmations. The orchestrator decides — but only on verified Jenkins state, never on guesses.

1. **Verified safety, not user consent, is the gate.** Auto mode covers the routine call; per-use user confirmation is **not** required. What's required is that the orchestrator KNOWS (from actual `getJob`/`getStatus` reads) that triggering won't stomp a concurrent build or cause a build storm. "Guessed," "should be fine," "MCP didn't respond but probably ok" → do not curl. Either re-check after a wait, or bail with an explicit question.

2. **Queue/in-flight verification.** Before triggering, read each alpha-touching pipeline (`elohim-orchestrator`, `elohim-genesis`, `elohim-edge`, `elohim-holochain`, `elohim`) and confirm `lastBuild.building: false`:

   ```
   mcp__jenkins__getJob jobFullName="<pipeline>/dev"
   ```

   If any read fails or returns ambiguous data, the precondition is **not met** — do not proceed. Record the queue snapshot in the journal as the evidence basis for the trigger.

3. **Build-storm prevention.** Don't re-trigger the same pipeline within its typical build-cycle time of a prior trigger you (or anyone) issued. Genesis is ~10-15 min, edge is ~25-35 min — see "Pipeline timing reference" above. Track recent trigger timestamps in the shift journal so successive iterations don't compound. If a recent trigger is in flight or just-finished, examine its result first.

4. **Trigger leaves, not roots.** Do not authenticated-trigger `elohim-orchestrator` with parameters — orchestrator dispatches downstream pipelines, multiplying any storm risk. Stick to leaf pipelines (`elohim-genesis` is the canonical leaf for this pattern; `elohim-edge` is acceptable but rarely needs parameterized triggers).

5. **Destructive-parameter awareness.** `RESET_STORAGE=true` does `kubectl exec rm content.db && kubectl delete pod` for each alpha human (`genesis/Jenkinsfile:285-334`). That **stomps any concurrent reader/writer** — a parallel test run against the same pod will see corruption or 503s. Tighten the in-flight check to its strictest reading: if anything alpha-touching is `building: true`, defer.

6. **Token never appears in logs, journals, transcripts, commit messages, or skill files.** Reference only as `$JENKINS_TOKEN`. Same for `$JENKINS_USERNAME` and `$JENKINS_URL`. If a curl needs to be quoted in the journal, use env-var placeholders, never resolved values. Capture only the resulting build number / queue id, not the request headers.

7. **First-use credential verification.** Before the first authenticated request in a session, run a no-op authenticated read to confirm credentials work:

   ```bash
   curl -sS -u "$JENKINS_USERNAME:$JENKINS_TOKEN" "$JENKINS_URL/api/json?tree=mode" | head -c 200
   ```

   If that returns Jenkins JSON (not an HTML login page or 403), credentials are good — proceed. If not, journal the failure mode and bail with a question; do not retry blindly. This is sanity, not permission — failed creds mean the env isn't set up right, not that user consent is needed.

### The workflow

1. **Diagnose.** Opus, with ci-observer and ci-investigator evidence, identifies a parameterized rebuild as the right next move. Subagents surface evidence; the orchestrator owns the call.

2. **Verify preconditions.** Per guardrails #2, #3, #4, #5: queue check on each alpha pipeline, recent-trigger check, leaf-not-root check, destructive-parameter strict-reading. If any fails: defer (re-check after a delay) or bail (explicit question to user). Do not curl on partial verification.

3. **Verify credentials** (first use of the session only — guardrail #7). One no-op authenticated read.

4. **Issue the curl.** See pattern below. Record curl invocation in the journal using env-var placeholders, never resolved values.

5. **Capture the response.** Jenkins responds with a queue item Location header on success (`Location: .../queue/item/<n>/`). Record the queue id and the resulting build number. Update the recent-trigger timestamp in the journal for guardrail #3 on next iteration.

6. **Re-enter the observation loop.** From here, normal observer/investigator dispatch applies — wait for the build to run, ci-observer summarizes, ci-investigator escalates if needed.

### Curl pattern

Jenkins API tokens generally bypass CSRF crumb requirements. The minimal trigger:

```bash
curl -sS -X POST \
  -u "$JENKINS_USERNAME:$JENKINS_TOKEN" \
  -D /tmp/jenkins-trigger-headers.txt \
  "$JENKINS_URL/job/elohim-genesis/job/dev/buildWithParameters?RESET_STORAGE=true"

# Check the Location header for the queue item
grep -i '^Location:' /tmp/jenkins-trigger-headers.txt
```

If the install requires a crumb (some hardened setups do), the call will return 403 with a `No valid crumb` message. Recovery:

```bash
CRUMB=$(curl -sS -u "$JENKINS_USERNAME:$JENKINS_TOKEN" \
  "$JENKINS_URL/crumbIssuer/api/json" | jq -r '.crumb')

curl -sS -X POST \
  -u "$JENKINS_USERNAME:$JENKINS_TOKEN" \
  -H "Jenkins-Crumb: $CRUMB" \
  "$JENKINS_URL/job/elohim-genesis/job/dev/buildWithParameters?RESET_STORAGE=true"
```

The exact crumb requirement is install-specific — first-use verification (guardrail #5) tells you which path applies.

### Multiple parameters

Pass each as a separate query parameter:

```
.../buildWithParameters?RESET_STORAGE=true&SKIP_SEEDING=false&STEPS=all
```

URL-encode any value containing spaces or special characters.

### Branched job paths

The job path mirrors the multibranch pipeline structure. For `dev`:

| Pipeline | Path fragment |
|---|---|
| elohim-genesis on dev | `/job/elohim-genesis/job/dev/` |
| elohim-edge on dev | `/job/elohim-edge/job/dev/` |
| elohim-orchestrator on dev | `/job/elohim-orchestrator/job/dev/` |

Append `buildWithParameters?...` to invoke a parameterized build, or `build` to invoke with defaults (which the webhook path also does).

### Failure modes

| Symptom | Likely cause | Recovery |
|---|---|---|
| 401 Unauthorized | Wrong username/token | Check env vars; do not retry blindly |
| 403 Forbidden, "No valid crumb" | CSRF protection on, token alone insufficient | Use the crumb pattern above |
| 403 Forbidden, "missing Job.Build permission" | Token lacks build permission | Stop. User must regenerate token with correct scope |
| 404 Not Found | Wrong job path (branch encoding, pipeline name) | Verify with `curl ... /api/json?tree=jobs[name]` |
| 200 OK but no Location header | Build was queued but routing changed | Check `getJob` for the new build |

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
