---
name: project_alpha_auth_portal_baseline_2026_09_04
title: Alpha auth-portal baseline — DEV_MODE registration + held act:i
description: "On alpha, DEV_MODE binds every new hosted registrant to Matthew's Human; act:i a2o is HELD on the fleet unless cluster-state is overridden — bites on any registration/agency work"
metadata: 
  node_type: memory
  title: Alpha auth-portal baseline — DEV_MODE registration + held act:i
  type: project
  originSessionId: e0407b6c-164b-489a-aa86-da20d9fa1b8b
  modified: 2026-09-04T23:13:41.870Z
---

Auth-portal flow review + live a2o baseline against https://doorway-alpha.elohim.host (2026-09-04).

- **Registration on alpha returns Matthew's profile.** `POST /auth/register` (agencyPhase defaults to `hosted`) with a fresh identifier came back with `profile.displayName: "Matthew"`, his bio and affinities. Cause: every deployed doorway runs `DEV_MODE=true`, so the hosted branch skips per-user provisioning and calls `create_human` on the singleton conductor, which answers "Agent already has a Human profile" and the handler "recovers" the existing Human. Every hosted registrant shares that human_id/agent key (the genesis #1105 class). Fix is posture (turn dev_mode off / provision per user), operator-owned.
- **a2o act:i is HELD on the fleet.** `@act:i` maps to `owned-substrate`, `available: false` in `genesis/manifests/cluster-state.yaml`. A bare run of the auth lane against alpha executes ~5 scenarios. To baseline the flow on alpha: `ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=genesis/manifests/cluster-state.act1-household.yaml A2O_ALLOW_DESTRUCTIVE=0 E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host npx cucumber-js --tags '@e2e and @auth and not @wip'` (the override would otherwise switch destructive scenarios ON — keep `A2O_ALLOW_DESTRUCTIVE=0`). Result that day: API 41/53 pass (fails: operator bootstrap-key mismatch = fixture env, logout WS console.error); browser 12/18 (3 agency-badge fails = app shell blank).
- **Doorway-app portal (`/threshold/*`) works on alpha; the elohim-app shell did not** (`main-EAKNZDUP.js` 404 = [[project_doorway_shell_stale_head_incident_2026_09_04]]), so app-side portal scenarios cannot be judged on the fleet until that cure deploys.
- Code-level flow bugs handed to a worker (fix branch): threshold-register posts snake_case keys (display name silently dropped); handle_register validates AFTER create_human/provision; agency.service has no `hosted-steward` arm (visitor residency, "Unknown" summary); login.component authority chip blank because it prefetches `/auth/me` while anonymous; in-app register gated on a conductor socket anonymous visitors can't open; doorway-account ticks "Steward" for a hosted-steward.

**Why:** the fleet's DEV_MODE + held act:i means green CI says nothing about the registration experience; the two facts above are what a session needs to get a real baseline in minutes.
**How to apply:** for any auth/registration/agency work, run the forced baseline command first, then judge the deployed portal only at `/threshold/*` until the shell incident is closed. See [[project_app_pipeline_e2e_ghost_and_apex_seed_gate]], [[feedback_local_mesh_first_cadence]].
