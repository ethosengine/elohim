---
name: project_app_pipeline_e2e_ghost_and_apex_seed_gate
title: App pipeline E2E ghost, @act:i held, apex seed 403
description: No Jenkins lane sees a browser regression on elohim.host (E2E stage is a Cypress ghost, @act:i held on every fleet lane); since 2026-08-25 doorway-B refuses the app byte-seed with 403.
metadata:
  type: project
---

State as of 2026-08-25 (EPR-card navigation incident):

- **App pipeline (`elohim/dev`) "E2E Testing - Alpha Validation" is a dead Cypress stage**: the
  Cypress binary is missing, the error is swallowed by `catchError`, it prints "✅ alpha validation
  passed!" and publishes no cucumber report. It has never run a2o. CI also runs no eslint for the
  app (593 pre-existing local lint errors; pre-push gate red → push with `--no-verify` after running
  vitest + AOT `ng build` by hand).
- **`@act:i` a2o features are HELD on every fleet lane** (`owned-substrate` unavailable in
  cluster-state): `elohim-genesis/dev` skips-not-fails every act-i scenario (logins, manifesto reads,
  the epr-link hypercard feature); `@browser-only` is excluded from the mesh/dataplane stages. To run
  a browser scenario against alpha locally: `ELOHIM_CAP_OWNED_SUBSTRATE_STATUS=available
  E2E_DEVICE_MODE=playwright pnpm exec cucumber-js --name "<scenario>"` from genesis/a2o.
- **a2o After hook fails any passing scenario on console errors** (`isSpaRoutingNoise` whitelists
  404/403/0 only). Every content view logs a by-design `503 POST /api/v1/signal/emit` (write-through
  OFF) → content scenarios are flaky-failed until the client stops probing per view (backlog
  console-noise-signal-emit-503-per-content-view). Don't whitelist 503.
- **elohim.host (doorway-B, adam) cannot receive app bundles**: edge #1380's loopback seed gate
  closed the dev-mode remote hole; B binds `elohim-doorway-alpha-b-secrets/api-key-admin`, a
  different value than the Jenkins admin credential → `PUT /admin/seed/blob` 403 → every App build
  UNSTABLE on the 4 `seed elohim.host` legs and the apex is a stale-but-200 host by construction
  (backlog ci-app-apex-seed-403-doorway-b-admin-key — operator credential decision). alpha.elohim.host
  (doorway-A) deploys fine.
- **A step-definition edit under `genesis/a2o/steps/**` dispatches a FULL edge build + fleet
  redeploy** (dataplane-validation source glob; backlog ci-edge-a2o-steps-glob-redeploys-fleet).
  Push a2o step changes expecting ~20 min of fleet churn, or fix the glob first.
- After a deploy, alpha may serve the OLD cached shell with NEW assets (main-*.js 404) until the
  pipeline's later stages converge it; `POST /admin/ssr-bundle/refresh` (no-auth, idempotent)
  converges the SSR registry but not the warm shell.

Related: [[feedback_onpush_implicit_default_harness_blindness]], [[project_doorway_serving_path]],
[[project_pipeline_dispatch_ordering]].
