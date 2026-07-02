---
name: project_prod_main_lag_vs_alpha_dev
title: Per-host doorway deploy lag (elohim.host = dev alpha-b)
description: A UI bug on one doorway host but not another is per-host deploy lag, not code; two catchError-swallowed legs (edge container + spa-blob) leave a host stale.
metadata: 
  node_type: memory
  originSessionId: b265419a-8b6b-4814-acc3-9bb106b13095
---

The doorway hosts deploy from DIFFERENT branches/legs (edge pipeline `elohim/holochain/Jenkinsfile`, stages `Deploy Edge Node - {Alpha,Staging,Prod}`):

- **`doorway-alpha.elohim.host`** ← `dev`, alpha env, doorway-A (`alpha.yaml`).
- **`elohim.host`** (bare apex) ← `dev`, alpha env, **doorway-B / alpha-b federation peer** (`alpha-b.yaml`, deploy `elohim-doorway-alpha-b`). NOT a main/prod host. This is the dual-doorway federation being proven.
- **`doorway.elohim.host`** ← `main`, prod env (`prod.yaml`) — the legacy prod doorway. `main` runs ~5018 commits behind `dev` (no release cut in months), so anything on `doorway.elohim.host` is far stale.

**Why `elohim.host` lagged while `doorway-alpha` was current** (both deploy from dev in the SAME `deployDoorwaysWithTestShape('alpha', …)` with the same `alpha.env` tags, so when both succeed they're identical): the alpha-b leg is wrapped in `catchError(buildResult: 'UNSTABLE')` (~L798-807) — it **silently swallows alpha-b deploy failures** (common cause: ingress hostname conflict on `elohim.host`), so `elohim.host` stays on its last-good image while doorway-A keeps updating. Fixed the stale header comment + pushed dev 2026-06-23 (commit `69607bd04`); a clean dev edge build where the alpha-b leg succeeds is what brings `elohim.host` current.

**Diagnostic signature:** a UI bug on one doorway host but not another is a per-host deploy lag, not a code bug. Confirm by fetching each host's hashed app CSS (`/` → `styles-*.css`) and diffing for the expected fix marker (e.g. `color-scheme: dark`) — don't re-derive the bug from source. Worked example: resilience hypercard "light text on light background" on the EPR bar — fix `05827e5ed` (2026-06-12) was live on alpha, missing on elohim.host. Deploys are operator-owned (never run kubectl); the repo manifests/pipeline are the cleanup surface. Sibling to [[feedback_che_devworkspaces_direct_to_main]] and [[feedback_frontend_review_eyes_first]].

**TWO independent per-host stale-ing legs (don't conflate them):** (1) the EDGE pipeline's alpha-b doorway/storage CONTAINER deploy (above); (2) the APP pipeline's per-host SPA-BLOB BUNDLE staging (root `Jenkinsfile` `stageSpaBlobs`/`stage-spa-blob.sh`) — PUTs the built Angular bundle + PATCHes `blobHash` onto each storage backend's content row. **A CSS/theme/bundle discrepancy is leg (2)** — the served `styles-*.css`/`main-*.js` hash IS the app bundle, staged per host (alpha→matthew, elohim.host→adam). Re-confirmed 2026-06-27: elohim.host still on `color-scheme:dark`×0 (alpha ×2) → 05827e5ed never landed there; the 2026-06-23 `69607bd04` was only a comment fix, not a redeploy. Root cause of leg (2)'s recurring invisibility: adam 503s transiently, the single-attempt leg exited non-zero, the per-(host,slug) `catchError`→UNSTABLE swallowed it, orchestrator treats UNSTABLE as success → host silently stale. **Fixed `adcb695d4` (2026-06-27):** Part A = bounded retry in `stage-spa-blob.sh` (transient 503s self-heal); Part B = `emitAppDeployJunit` emits a per-(host,slug) junit testcase (`classname=elohim-app.deploy.<env>`, `type=spa-blob-stale`) so a persistently-stale host is a NAMED test failure in the app build's test-report tab / `getTestResults`, not an invisible UNSTABLE. So: when elohim.host looks stale, FIRST check the app build's test results for a `spa-blob-stale` failure naming the host — that tells you it's leg (2) and which host. Mirrors edge `emitDeployJunit`. Still operator-owned to actually redeploy.
