---
id: "backlog-deploy-resilience-jenkinsfile-wiring"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Wire the deploy-to-healthy write-readiness pre-probe + post-stage serve-verify + tri-state junit into the root Jenkinsfile (scripts already landed; Groovy deferred — CPS-limit + locally-unverifiable)"
slug: "deploy-resilience-jenkinsfile-wiring"
written: "2026-06-30"
author: "pipeline-shakeout shift (overnight)"
status: "open"
priority: "high"
ci_status: backlog
jobs: [elohim]
tags: [ci, deploy, stage-spa-blob, write-readiness, deploy-to-healthy, junit, jenkinsfile, cps-limit, elohim-host, fail-open, all-skip-floor]
cites:
  - scripts/ci/probe-write-readiness.sh
  - scripts/ci/verify-host-serves.sh
  - scripts/ci/stage-spa-blob.sh
  - Jenkinsfile
  - genesis/data/timeline/backlog/deploy-spa-blob-silent-unstable-on-degraded-node-shallow-health.md
---

# Deploy-resilience: wire the (already-committed) probe/verify scripts into the app deploy

## Status: scripts LANDED + verified; Jenkinsfile wiring DEFERRED

The deploy-resilience LOGIC is committed and verified (`bash -n` clean; the probe's fail-open
contract hand-verified):
- `scripts/ci/probe-write-readiness.sh` — per-host WRITE-readiness pre-probe. Exits `2` ONLY on a
  clear degraded signal (doorway reachable on `/health` < 500 BUT both `/db/stats` AND
  `/db/content/{slug}` return 5xx/000 on all retries). **Critic guard (a) FAIL-OPEN is built in:**
  every ambiguous outcome (probe network error, timeout, 404, doorway unreachable, unexpected code)
  exits `0` = deploy-and-let-the-real-stage-fail-loudly. Never skips a deployable host on ambiguity.
- `scripts/ci/verify-host-serves.sh` — post-stage proof: `{host}/` → 200 AND served `blobHash`
  non-null AND == the staged hash.
- `scripts/ci/stage-spa-blob.sh` — now publishes the staged content hash to `$STAGE_HASH_OUT`
  (opt-in: a no-op when unset, so it is **inert** until the Jenkinsfile sets it — every current
  caller is byte-for-byte unaffected).

**The Jenkinsfile wiring was reverted** (an in-flight agent stalled mid-edit; the partial Groovy
pushed the root `Jenkinsfile` to 1659 lines — over the documented ~1596-line `MethodTooLargeException`
CPS-breach proxy — and Groovy is not locally verifiable). Shipping a half-edited deploy pipeline is
the exact fail-closed/blocks-all-deploys hazard to avoid. So the scripts are committed inert; the
wiring is this backlog.

## The wiring to add (root `Jenkinsfile`, heredoc-free, minimal — CPS-64KB-aware)

In the per-host deploy loop (the `stageAndVerifyAllBundles` helper, ~`:421`):
1. **Pre-probe + deploy-to-healthy:** before staging host `i`, call
   `sh(returnStatus: true, script: "bash '${env.WORKSPACE}/scripts/ci/probe-write-readiness.sh' '${url}' 'elohim-host-landing'")`.
   `returnStatus` (never throws). If `== 2` → add to a `skippedHosts` set and `continue` (skip). Any
   other value → deploy (fail-open). **Only `2` skips.**
2. **Publish the staged hash:** in `stageSpaBlobs` (~`:293`), set
   `STAGE_HASH_OUT="${env.WORKSPACE}/.deploy-hash-${sanitized(outcomeKey)}"` in the `withEnv` around
   the `stage-spa-blob.sh` call (sanitize: `replaceAll(/[^A-Za-z0-9._-]/, '-')`).
3. **Post-stage verify:** after a host stages, call `verify-host-serves.sh` (reconstruct the same
   hash path); if it fails, downgrade that host's outcome to `failed` (host staged but serves stale).
4. **Tri-state junit** (`emitAppDeployJunit`, ~`:452`): emit `passed` / `failed` (`<failure/>`) /
   `skipped` (`<skipped/>`). `<skipped/>` is NOT counted red (a known-degraded host must not drag a
   healthy multi-host deploy to red).
5. **Critic guard (b) ALL-SKIP FLOOR (REQUIRED):** if `skippedHosts.size() == doorwayEprUrls.size()`
   (every host skipped → zero deployed), emit a hard `<failure/>` — NEVER green. Otherwise
   "skipped-not-red" becomes a silent-green dumping ground that masks "nothing deployed."

Constraints: bash bodies stay in `scripts/ci/*.sh` (no inline `sh """…"""` heredocs — they inflate
the single CPS dispatch method toward the 64KB ceiling); keep added Groovy minimal; additive only —
never remove the deploy-of-healthy-hosts path. **Operator must review the Groovy + watch the first
app deploy** (no local Jenkins to compile-check it).

## Why (the bug this closes)

Today a failed stage leg is `catchError`-swallowed → the build goes merely UNSTABLE while
`elohim.host` silently 404s (a host left serving a STALE bundle). `/health` is too shallow to catch
a dead storage write path behind an up doorway. Extends
[[deploy-spa-blob-silent-unstable-on-degraded-node-shallow-health]] (this adds: probe the WRITE
path, deploy-to-healthy, make STALE loud not silent).
