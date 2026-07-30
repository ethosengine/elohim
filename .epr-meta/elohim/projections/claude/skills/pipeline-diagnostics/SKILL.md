---
name: pipeline-diagnostics
description: "Use when a build failed, when comparing two Jenkins builds to measure a fix's impact, when checking whether a specific commit landed in a build, when reading a2o sprint-report findings, or when retrieving any archived CI artifact. Also use when `mcp__jenkins__*` tools are absent from your tool list (some subagent contexts) and you'd otherwise give up or grep the repo blindly — public Jenkins URLs + WebFetch work as a fallback. Triggers: \"did my fix land?\", \"why is the next build still failing?\", \"what's in the sprint-report?\""
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/pipeline-diagnostics"
---

# Pipeline Diagnostics

Diagnoses CI/CD pipeline issues for the Elohim project using Jenkins. Covers two access paths — Jenkins MCP (preferred, primary) and direct WebFetch against public URLs (fallback for contexts where MCP isn't loaded) — plus retrieval workflows for sprint-reports, cucumber reports, and build-to-commit correlation.

## Auth model — read this first

Jenkins is OIDC-protected, with two access paths and sharply different guardrails:

**1. Anonymous (default — MCP and unauthenticated WebFetch both run this way).** No `Authorization` header — explicit auth triggers an interactive OIDC login flow and breaks the connection. Consequences:
- **All read tools/URLs work.** `getBuild`, `getBuildLog`, `searchBuildLog`, `getJob`, `getJobs`, `getBuildChangeSets`, `getTestResults`, `getFlakyFailures`, `getStatus`, `getBuildScm`, `getJobScm` — anonymous has Overall.Read + Job.Read on `https://jenkins.ethosengine.com`. Same for public WebFetch URLs (address book below) — no auth needed for reads.
- **Write tools fail.** `mcp__jenkins__triggerBuild` and `mcp__jenkins__updateBuild` don't work — anonymous lacks `Job.Build`. For retriggers, push a commit (optionally `[build:<pipeline>]` tagged — see "Triggering builds" below).

**2. Authenticated (`JENKINS_USERNAME` + `JENKINS_TOKEN`, orchestrator-only).** A direct `curl -u "$JENKINS_USERNAME:$JENKINS_TOKEN"` call can do what anonymous can't, including **parameterized builds** (e.g. `RESET_STORAGE=true` for schema-drift recovery, where `[build:*]` tags are insufficient because they carry pipeline membership but not parameter values). Reserved for the shift Opus orchestrator, usable autonomously (no per-use user confirmation) but only against **verified** Jenkins state — the orchestrator must KNOW from actual reads that triggering won't cause interruptions or storms. Guessing is disqualifying. Full guardrails and workflow: "Parameterized rebuild (authenticated)" below.

**MCP vs WebFetch:** prefer MCP (`mcp__jenkins__*`) when loaded — it's structured (log search, test-result enumeration, changeset queries) and always anonymous-mode. Fall back to WebFetch against the URL patterns below when MCP isn't in your tool list (subagent contexts that don't inherit the parent's MCP set, or a session before postStart finishes). Detect by checking your tool list for `mcp__jenkins__*`; if absent, go straight to WebFetch rather than spending a turn hoping it appears.

## Quickstart: "I pushed a fix — is it in a build yet and did it help?"

This is the workflow run 90% of the time. Don't grep the repo, don't guess URLs — hit these in order:

1. **Confirm the commit is on origin**
   ```
   git log --oneline origin/dev..HEAD      # empty? already pushed.
   git log --oneline -1 origin/dev         # note the SHA
   ```

2. **Find the latest build of the affected pipeline** — the job index page lists the last N builds with status:
   ```
   WebFetch https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/
     prompt: "List last 3 builds with build number, status, timestamp.
              Also list any archived artifacts under genesis/a2o/reports/."
   ```
   Returns build numbers like `#935`, status, timestamp, and the artifact tree for the latest. **The index page is your address book — always start here.**

3. **Confirm your SHA landed in that build:**
   ```
   WebFetch https://jenkins.ethosengine.com/job/<pipeline>/job/dev/<N>/api/json?tree=changeSets[items[commitId,msg]],actions[causes[shortDescription]]
     prompt: "List commit SHAs in this build's changeset, and the build cause."
   ```
   - Your SHA appears in `changeSets[].items[].commitId`: build reflects your fix — if findings haven't changed, the fix itself is wrong.
   - Your SHA is NOT there: the build ran against an older commit. Wait for the next one; don't read its findings as proof-of-fix. `actions[].causes[]` shows what triggered it (upstream pipeline, scheduled run, your push, or a manual `triggerBuild`).
   - No changesets at all: triggered manually — the SCM revision under `actions` tells you which commit it actually ran against.

   To check whether several commits all landed, diff `git log --oneline origin/dev~10..origin/dev` against the build's `changeSets` list — anything above the last listed commitId lands in the *next* build.

4. **Retrieve the sprint-report** (A2O pipeline only) for the findings delta:
   ```
   WebFetch https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/<N>/artifact/genesis/a2o/reports/sprint-report.md
     prompt: "Return verbatim. Preserve summary table, pillar headers, all fingerprints."
   ```
   Compare fingerprints against the previous build's report — fingerprints are stable across runs (the same hash repeats for the same error signature across builds).

5. **Narrate the delta, not the absolute numbers.** "Fingerprint X dropped from 26 → 0" or "new fingerprint Y appeared" is the signal — not "99 failures is bad."

## Address book — Jenkins URL patterns

**Never guess these. Copy from here.** Base URL: `https://jenkins.ethosengine.com`. Branches: `dev` (main), feature branches become `job/feat-xxxx`, etc. — URL-encode slashes as `/job/`.

| Path | Template | Example |
|------|----------|---------|
| Job index (build list) | `/job/<pipeline>/job/<branch>/` | `/job/elohim-genesis/job/dev/` |
| Specific build | `/job/<pipeline>/job/<branch>/<N>/` | `/job/elohim-genesis/job/dev/935/` |
| Latest build (alias) | `/job/<pipeline>/job/<branch>/lastBuild/` | useful when you don't know `N` |
| Last successful | `/job/<pipeline>/job/<branch>/lastSuccessfulBuild/` | skips failed builds |
| Build artifact | `/job/.../<N>/artifact/<repo-relative-path>` | `…/935/artifact/genesis/a2o/reports/sprint-report.md` |
| Console log (raw) | `/job/.../<N>/consoleText` | plain text — huge, prefer `searchBuildLog` via MCP |
| API (JSON) | `/job/.../<N>/api/json?tree=<fields>` | `tree=changeSets[items[commitId]]` for SHAs |

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

## Pipeline reference

Flow: `elohim-orchestrator` (webhook-triggered changeset analysis) dispatches `elohim-holochain`, `elohim-edge`, `elohim-app` in parallel, each feeding `elohim-genesis` (seed + a2o verification), then post-deploy health checks. `elohim-steward` is manual-only (Tauri desktop).

| Job name | Purpose | Produces | Typical duration |
|----------|---------|----------|-------------------|
| `elohim-orchestrator` | Changeset analysis, downstream trigger | dispatch decision | 3-5 min — supersedes an in-flight run if a new push lands first |
| `elohim-holochain` | DNA compilation, hApp packaging + k8s deploy | `elohim.happ` | 15-25 min — fast build, slower deploy (pod restart + readiness probes) |
| `elohim-edge` | Docker builds: doorway + elohim-storage (heavy `cargo build --release`) + deploy | `doorway:tag`, `storage:tag` images | 25-35 min — the long pole; start here when estimating end-to-end time |
| `elohim-app` | Angular build, static assets | `dist/elohim-app` | — |
| `elohim-genesis` | Content seeding + a2o cucumber run + sprint-report | seed verification, sprint-report | 10-15 min — dominated by E2E scenario timeouts; a broken alpha inflates this |
| `elohim-steward` | Tauri desktop app | — | manual trigger only |
| **Full cascade** (push → sprint-report) | | | **~45-60 min** — budget a full hour from `git push` to reading a fresh sprint-report |

Despite the name, `elohim-edge` uses `elohim/holochain/Jenkinsfile` (not the DNA-only one) — it's what builds Rust images *and* deploys. The DNA-only job is `elohim-holochain`, using `elohim/holochain/dna/Jenkinsfile`. Authoritative name→jenkinsPath→changePatterns mapping: `genesis/orchestrator/Jenkinsfile` `PIPELINES` map.

**Key Jenkinsfile locations:** root orchestrator `/projects/elohim/Jenkinsfile`; pipeline controller `/projects/elohim/genesis/orchestrator/Jenkinsfile`; DNA/hApp builds `/projects/elohim/holochain/Jenkinsfile`; seeding pipeline `/projects/elohim/genesis/Jenkinsfile`; desktop app `/projects/elohim/steward/Jenkinsfile`.

### Environment mapping

| Branch pattern | Environment | Doorway URL |
|---------------|-------------|-------------|
| `dev`, `feat-*`, `claude-*` | Alpha | doorway-alpha.elohim.host |
| `staging-*` | Staging | doorway-staging.elohim.host |
| `main` | Production | doorway.elohim.host |

### Wakeup-delay guidance

Use `ScheduleWakeup.delaySeconds` per what you're waiting for — don't poll inside a turn:

- Waiting for orchestrator to finish analysis: 300s (5 min) — then you know which downstream pipelines fired.
- Waiting for edge to finish compiling: 1200-1800s (20-30 min) — you won't learn anything new mid-compile.
- Waiting for the whole cascade after a push touching Rust + manifests: 2400-3000s (40-50 min); re-sleep shorter if you wake up too early.
- Waiting for a cascade where edge is already green and only genesis needs to run: 900s (15 min).
- Build running but stuck/queued: don't wake sooner than the p95 duration above.

### Coordination gotchas

- **Orchestrator can supersede a prior in-flight build.** Pushing twice quickly marks the first orchestrator run "Not built / Superseded" and can cancel its downstream triggers. The second run analyzes the cumulative diff since the last green, so downstreams still fire correctly, but any in-flight edge/holochain from the first run may abort mid-way.
- **Edge and holochain run in parallel, not serial.** Holochain may go SUCCESS before edge publishes new images, meaning the deploy ran against the OLD storage image tag (`dev-latest` points at last green). holochain-green + edge-not-yet-green = the deploy is running old code — needs either a fresh holochain trigger after edge, or a rollout restart to pick up the new tag.
- **`dev-latest` tag race.** StatefulSet pods only re-pull on pod restart. With `imagePullPolicy: Always` + `dev-latest`, k8s pulls the new image only when the pod restarts — a rolling restart happens on spec change (env var, mount, resource bump), NOT on image tag content change alone. A deploy relying on `dev-latest` moving must also change something in the pod spec, or include an explicit `kubectl rollout restart statefulset/...`.
- **Orchestrator skips pipelines it thinks are up-to-date**, even when a changed path matches a pipeline's `changePatterns` — it compares against each pipeline's own last-built baseline commit and skips if that baseline already contained the file in its current state. A manifest-only annotation change can look "new" to git but not to the pipeline's baseline. Escape hatch: the `[build:*]` tag (below).

## Triggering builds — webhook + commit tags, not MCP

`mcp__jenkins__triggerBuild` is unavailable to the anonymous MCP user. The canonical trigger surface is the GitHub webhook landing on `elohim-orchestrator`, which analyzes the changeset and dispatches downstream pipelines.

**Retry a failed build for the same commit** — empty commit + push:
```
git commit --allow-empty -m "ci: retrigger [build:edge]"
git push
```

**Force a pipeline whose changeset analysis missed** — the orchestrator's webhook-trigger branch parses the HEAD commit message for `[build:<pipeline>]` tags and force-adds matching pipelines regardless of changeset analysis:

| Tag | Pipeline |
|-----|----------|
| `[build:edge]` | elohim-edge (Rust doorway + storage + deploy) |
| `[build:dna]` | elohim-holochain (DNA/hApp only) |
| `[build:app]` | elohim (Angular) |
| `[build:genesis]` | elohim-genesis (a2o + seed) |
| `[build:sophia]` | elohim-sophia |
| `[build:steward]` | elohim-steward |
| `[build:all]` | every non-manualOnly pipeline |

Comma-separated forms work: `[build:edge,genesis]`. Use this when the orchestrator's changeset analysis is verifiably wrong, you need a rolling restart the normal deploy wouldn't cause, or you're testing CI changes without meaningful source changes. Don't abuse `[build:all]` — each pipeline costs minutes; prefer the narrowest tag.

**Trigger with parameters:** the webhook path only uses pipeline-defined defaults — it can't vary `SKIP_SEEDING`, `ENVIRONMENT`, `RESET_STORAGE`, etc. For that, use the authenticated path below (editing Jenkinsfile defaults and pushing also works, but only for permanent changes, not one-off operational unblocks).

## Parameterized rebuild (authenticated)

The only sanctioned way for Claude to trigger parameterized Jenkins builds. Credentials live in env (`JENKINS_USERNAME`, `JENKINS_TOKEN`, `JENKINS_URL`) and authenticate against `Job.Build`, which anonymous MCP lacks. Read both halves below before reaching for the curl.

**Who may use this:** only the shift Opus orchestrator — even within /shift, never subagents. ci-observer and ci-investigator are read-only instruments that feed Jenkins state to the orchestrator; they never invoke triggers.

**When it's appropriate:** the `[build:*]` tag mechanism forces pipeline membership but can't pass parameter values. Use the authenticated path when a parameterized stage gates the actual fix (canonical case: `elohim-genesis` Seed Database stage's `RESET_STORAGE=true`, which clears `content.db` to recover from schema drift (see `feedback_seed_lock_means_schema_drift`) — tag-based retrigger inherits the default `false` and reproduces the failure), when a diagnostic rebuild needs a non-default value (`SKIP_SEEDING=true`, narrowed `STEPS=...`, alternate `ENVIRONMENT`), or when the webhook path simply can't carry the operational intent. Don't reach for it when an empty `[build:<pipeline>]` commit would do.

**Hard guardrails** — structural preconditions, not procedural confirmations:

1. **Verified safety, not user consent, is the gate.** Auto mode covers the routine call; per-use confirmation isn't required. What's required: the orchestrator KNOWS from actual `getJob`/`getStatus` reads that triggering won't stomp a concurrent build or cause a storm. "Guessed," "should be fine," "MCP didn't respond but probably ok" → do not curl; re-check after a wait or bail with an explicit question.
2. **Queue/in-flight verification.** Before triggering, read each alpha-touching pipeline (`elohim-orchestrator`, `elohim-genesis`, `elohim-edge`, `elohim-holochain`, `elohim`) via `mcp__jenkins__getJob jobFullName="<pipeline>/dev"` and confirm `lastBuild.building: false`. Any failed or ambiguous read means the precondition isn't met — don't proceed. Record the queue snapshot in the journal as evidence.
3. **Build-storm prevention.** Don't re-trigger the same pipeline within its typical build-cycle time of a prior trigger (genesis ~10-15 min, edge ~25-35 min — see Pipeline reference above). Track recent trigger timestamps in the shift journal; if a recent trigger is in flight or just-finished, examine its result first.
4. **Trigger leaves, not roots.** Never authenticated-trigger `elohim-orchestrator` with parameters — it dispatches downstream pipelines, multiplying storm risk. Stick to leaf pipelines (`elohim-genesis` is canonical; `elohim-edge` is acceptable but rarely needs parameters).
5. **Destructive-parameter awareness.** `RESET_STORAGE=true` runs `kubectl exec rm content.db && kubectl delete pod` for each alpha human (`genesis/Jenkinsfile:285-334`) — it stomps any concurrent reader/writer. Tighten guardrail #2 to its strictest reading: if anything alpha-touching is `building: true`, defer.
6. **Token never appears in logs, journals, transcripts, commit messages, or skill files.** Reference only as `$JENKINS_TOKEN` (same for `$JENKINS_USERNAME`/`$JENKINS_URL`). If a curl needs quoting in the journal, use the env-var placeholders, never resolved values — capture only the resulting build number / queue id.
7. **First-use credential verification.** Before the first authenticated request in a session:
   ```bash
   curl -sS -u "$JENKINS_USERNAME:$JENKINS_TOKEN" "$JENKINS_URL/api/json?tree=mode" | head -c 200
   ```
   Jenkins JSON back (not an HTML login page or 403) means credentials are good. Otherwise journal the failure mode and bail with a question — don't retry blindly. This is a sanity check, not a permission gate.

**The workflow:** (1) Diagnose — Opus, with ci-observer/ci-investigator evidence, identifies a parameterized rebuild as the right move. (2) Verify preconditions — guardrails #2-#5: queue check on each alpha pipeline, recent-trigger check, leaf-not-root check, destructive-parameter strict reading; defer or bail on partial verification, never curl on it. (3) Verify credentials (first use of session only — guardrail #7). (4) Issue the curl (pattern below); record the invocation in the journal using env-var placeholders. (5) Capture the response — Jenkins returns a queue item `Location` header on success (`Location: .../queue/item/<n>/`); record the queue id and resulting build number, and update the recent-trigger timestamp for guardrail #3. (6) Re-enter the observation loop — normal observer/investigator dispatch applies from here.

**Curl pattern** — Jenkins API tokens generally bypass CSRF crumb requirements:
```bash
curl -sS -X POST \
  -u "$JENKINS_USERNAME:$JENKINS_TOKEN" \
  -D /tmp/jenkins-trigger-headers.txt \
  "$JENKINS_URL/job/elohim-genesis/job/dev/buildWithParameters?RESET_STORAGE=true"

# Check the Location header for the queue item
grep -i '^Location:' /tmp/jenkins-trigger-headers.txt
```
If the install requires a crumb (403, "No valid crumb"):
```bash
CRUMB=$(curl -sS -u "$JENKINS_USERNAME:$JENKINS_TOKEN" \
  "$JENKINS_URL/crumbIssuer/api/json" | jq -r '.crumb')

curl -sS -X POST \
  -u "$JENKINS_USERNAME:$JENKINS_TOKEN" \
  -H "Jenkins-Crumb: $CRUMB" \
  "$JENKINS_URL/job/elohim-genesis/job/dev/buildWithParameters?RESET_STORAGE=true"
```
The exact crumb requirement is install-specific — first-use verification (guardrail #7) tells you which path applies.

Pass multiple parameters as separate query params: `.../buildWithParameters?RESET_STORAGE=true&SKIP_SEEDING=false&STEPS=all` (URL-encode any value with spaces/special characters). Branched job paths mirror the multibranch structure, e.g. `/job/elohim-genesis/job/dev/`, `/job/elohim-edge/job/dev/`, `/job/elohim-orchestrator/job/dev/` — append `buildWithParameters?...` for parameters or `build` for defaults.

**Failure modes:**

| Symptom | Likely cause | Recovery |
|---|---|---|
| 401 Unauthorized | Wrong username/token | Check env vars; do not retry blindly |
| 403 Forbidden, "No valid crumb" | CSRF protection on, token alone insufficient | Use the crumb pattern above |
| 403 Forbidden, "missing Job.Build permission" | Token lacks build permission | Stop. User must regenerate token with correct scope |
| 404 Not Found | Wrong job path (branch encoding, pipeline name) | Verify with `curl ... /api/json?tree=jobs[name]` |
| 200 OK but no Location header | Build was queued but routing changed | Check `getJob` for the new build |

## Red flags — anti-patterns observed when agents DON'T use this skill

| Thought | Reality |
|--------|---------|
| "Jenkins MCP isn't connected, I can't check the build." | MCP is anonymous-mode and connects automatically on workspace start. If truly missing in a subagent, fall back to WebFetch — public Jenkins reads need no auth. |
| "Let me call `mcp__jenkins__triggerBuild` to retry." | Anonymous can't trigger builds. Push an empty commit with `[build:<pipeline>]`; the orchestrator webhook dispatches it. |
| "I'll add `Authorization: Basic <token>` to make MCP authenticated." | Don't — OIDC intercepts the attempt and 302s into a redirect loop. Anonymous reads cover the entire diagnostic workflow. |
| "Let me grep the repo for the error." | The repo doesn't know what happened in CI. Fetch the sprint-report or console log. |
| "Let me spawn a ci-investigator subagent to check." | It may or may not have MCP loaded; either way it falls back to WebFetch if missing. Prefer `ci-observer` (Haiku) for surface scans; reserve `ci-investigator` (Sonnet) for cross-build correlation or low-confidence escalation. |
| "I'll assume the build ran against my latest commit." | Assume nothing — correlate SHA via `changeSets[].items[].commitId` before reading findings as proof-of-fix. |
| "99 → 90 failures means my fix barely worked." | Check fingerprints, not raw counts. A cascade-root fix drops several fingerprints at once; net failures can stay flat if pendings shift. |
| "The build status is green, so we're done." | Sprint-report aggregator is non-blocking — build can be SUCCESS with scenario failures. Always pull the report. |
| "I'll just read cucumber-report.json, it has everything." | Hundreds of KB of raw scenarios. Start with sprint-report.md (ranked, deduplicated); drill into cucumber only for specific stack traces. |

## Sprint-report delta analysis

The aggregator (`genesis/a2o/scripts/build-sprint-report.ts`) produces fingerprinted, deduplicated findings. Fetch baseline and new reports via the artifact address book above, then:
- Look at the `Summary` table — `scenarios / passed / failed / pending` gives the gross signal.
- Scan fingerprints from baseline — for each, check if it's absent, reduced, or unchanged in the new report. Flag any *new* fingerprint as a regression you introduced.
- **Cascade collapse:** a single infra/config fix can drop dozens of failures at once because downstream scenarios all depended on a blocked prerequisite — e.g. a foundational `POST /auth/register 503` fingerprint means every scenario needing a registered human also fails (surfacing as `POST /auth/login 401 Invalid credentials` because the fixture human was never created). Fixing the root often collapses most of the report in one shot; expect `occurrences:` to drop across multiple fingerprints together.

**Reading a fingerprint:** each finding has a 12-hex fingerprint (SHA-256 of the normalized error), a normalized error message, an occurrence count, and the affected-scenarios list. Same error across runs → same fingerprint, even with different runtime IDs/timestamps (the aggregator strips UUIDs, ports, hashes) — a structural change to the error message is what produces a different fingerprint.

## Common failure patterns

Map the failed Jenkins stage name to the component at fault: "Build DNAs" → Rust/WASM; "Build App" → Angular/TypeScript; "Seed Content" → doorway/conductor connection; "Deploy" → k8s/Docker; "E2E VERIFICATION (API)" → a2o scenarios (pull sprint-report.md).

| Class | Grep the log for | Common causes | Fix |
|---|---|---|---|
| DNA / WASM build | `error[E`, `cannot find`, `unresolved` | Missing RUSTFLAGS for getrandom backend; incompatible dep versions; zome syntax errors | Confirm `RUSTFLAGS='--cfg getrandom_backend="custom"'`; verify `Cargo.lock` committed; check zome source |
| Angular build | `error TS`, `Cannot find module` | Type mismatches after model changes; missing imports; circular deps | Run `npm run build` locally; check type sync between elohim-service and elohim-app; verify imports resolve |
| Seeding — connection | `ETIMEDOUT`, `WebSocket`, `connection refused` | Doorway not ready; wrong admin URL; network policy blocking | Check doorway health endpoint; verify `HOLOCHAIN_ADMIN_URL`; check k8s pod status |
| Seeding — schema | `missing required`, `validation failed` | Content files missing id/title fields | Run `npm run validate` in genesis/seeder; fix content files |
| Docker build | `COPY failed`, `RUN failed`, `denied` | Missing build artifacts from a prior stage; Harbor registry auth; Dockerfile syntax | Check prior-stage artifacts; verify Harbor credentials; lint the Dockerfile |

## MCP tool quick reference

```
mcp__jenkins__getStatus                                          # overall Jenkins health
mcp__jenkins__getJobs                                             # list jobs
mcp__jenkins__getJob jobFullName="elohim-genesis/dev"             # job info (omit buildNumber args for latest below)
mcp__jenkins__getBuild jobFullName="elohim-genesis/dev" buildNumber=935
mcp__jenkins__getBuildLog jobFullName="elohim-genesis/dev" limit=-200        # tail
mcp__jenkins__searchBuildLog jobFullName="elohim-genesis/dev" pattern="error|failed|Exception|panic" ignoreCase=true contextLines=3
mcp__jenkins__getTestResults jobFullName="elohim-app/dev" onlyFailingTests=true
mcp__jenkins__getBuildChangeSets jobFullName="elohim-genesis/dev" buildNumber=935
mcp__jenkins__getFlakyFailures jobFullName="elohim-genesis/dev"
```

After genesis completes, verify deployment health via `stats:dev`/`stats:prod` commands, doorway health endpoints, and application smoke tests.
