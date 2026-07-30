---
name: project-ssr-first-deploy-seed-then-restart
title: SSR first deploy — seed, then restart doorway
description: "First SSR deploy of an EPR app: App pipeline seeds serverBlobHash, THEN a doorway restart materializes it; a doorway-only push won't trigger App; edge-before-seed needs one extra edge restart."
metadata: 
  node_type: memory
  title: SSR first-deploy = App seed serverBlobHash THEN doorway restart
  type: project
  originSessionId: 13b4bc62-ff46-41bb-8974-10317419acad
  modified: 2026-07-19T06:56:30.770Z
---

Getting a new SSR-capable EPR app (e.g. `lamad-spa`) actually SERVING SSR on alpha is a two-pipeline dance, and the ordering bites (verified 2026-07-19 landing lamad-spa SSR):

1. **App pipeline seeds the bundle.** The root `Jenkinsfile` `stageAndVerifyAllBundles` → `scripts/ci/stage-spa-blob.sh` runs in **two phases**: a byte-seed pass that uploads the blob but logs `⊘ byte-seed only (DO_PATCH!=1) — no head PATCH` (do NOT panic — this is phase 1), then a dedicated PATCH phase that emits `✓ patched <slug> (serverBlobHash)` + `✓ verified <slug> serverBlobHash = sha256-…`. Only after the PATCH phase is `serverBlobHash` actually on the content node.
2. **The doorway materializes `SSR_BUNDLE_SLUGS` at BOOT** from the substrate. `SSR_BUNDLE_SLUGS="elohim-host-landing,lamad-spa"` lives in `genesis/orchestrator/manifests/doorway/{alpha,alpha-b}.yaml`. An unseeded slug skips-with-warn → app stays CSR/404 until the blob ships.

**The trap — changeset dispatch + deploy-order race:**
- A **doorway-only change does NOT re-trigger the App pipeline** (orchestrator dispatch is changeset-driven), so a doorway-only push ships the SSR machinery + `SSR_BUNDLE_SLUGS` WITHOUT ever seeding the bundle. Force it with an empty `[build:app]` commit.
- If the **edge deploy (doorway restart) runs BEFORE the App seed lands**, the doorway boots with the slug still unseeded → materialize-skip → `/lamad` 404/CSR even after the seed later completes. The App pipeline does NOT restart the doorway (it only stages+patches+mount-verifies; the mount-verify 503s during the race). So you need **one more edge restart AFTER the seed** — force `[build:edge]` (new sha tag → pod restart → re-materialize) or an operator `kubectl rollout restart`.

Verify: `curl /db/content/lamad-spa` shows `serverBlobHash` (seed done); then `curl -I /lamad` for `x-ssr-rendered: 1` (live) vs `x-ssr-skipped: renderer-app-mismatch` (still unseeded on that doorway). Watch out: `/lamad` 404 + `conductor-diagnostics {"status":"catching-up"}` is transient restart churn (~20min), not a regression. Related: [[project_ssr_render_trace_and_fixed_fetcher]], [[project_prod_main_lag_vs_alpha_dev]], [[project_edge_deploy_restarts_genesis_conductors]].
