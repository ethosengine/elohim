---
name: project_pipeline_dispatch_ordering
title: Pipeline dispatch & deploy ordering (umbrella)
description: "Dispatch/deploy ordering traps: live-target gates deadlock their own fix; a same-wave dispatch bakes the PREVIOUS happ; new path-deps need Dockerfile COPY; coordinator changes keep the DNA hash."
metadata:
  node_type: memory
  type: project
---

# Pipeline dispatch & deploy ordering (umbrella)

Folds the CI dispatch-ordering and deploy-sequencing trap cluster. Members:

- [[project_cascade_deadlock_live_target_gate]] — A live-target E2E gate on the only waited-on Level-0 pipeline deadlocks the edge deploy that fixes that target; fixed via catchError→UNSTABLE.
- [[project_edge_happ_fetch_race]] — Edge bakes elohim-happ:dev-latest fetched mid-build; same-wave dispatch with the DNA pipeline ships the PREVIOUS bundle — dependsOn is not wave-ordered.
- [[project_edge_deploy_restarts_genesis_conductors]] — Edge Deploy restarts conductors; genesis pair skips on STS-unchanged (9f9c4aec4), happ-digest stamp keeps real DNA moves restarting; doorway-only fix = operator kubectl path.
- [[project_new_path_dep_needs_dockerfile_copy]] — A new path-dep (even transitive) needs COPY+sed in BOTH edge Dockerfiles, workspace-field inline for storage, and manifest watch-path — else edge breaks at dev.
- [[project_saga_banking_validate_only_gate]] — Bank saga/notary measures via [edge:validate-only] (exists since 2026-07-30); the quiesce gate reads MATTHEW only, not the shem trio — bites on every banking attempt
- [[project_dna_hash_blind_to_coordinator_zomes]] — Holochain DNA hash covers only integrity zomes + modifiers — coordinator-only changes need the update_coordinators hot-swap path, not reinstall
- **Fire-and-forget red is invisible to the level guard (2026-08-27, orchestrator #1733).** A `longRunning` pipeline (DNA, elohim-library) is dispatched `wait:false` and `dispatchResult` records success at dispatch, so `levelFailed` never sees its FAILURE — edge #1386 and genesis still ran while holochain #1403 was red (dead sccache key). Two consequences: (1) don't predict "level-0 red withholds edge" from the Jenkinsfile guard alone — check the downstream job; (2) a DNA red is SILENT at the orchestrator (baseline advances optimistically) — the DNA lane can stay red for days with green orchestrator runs. Inverse of the wait:true trap: a short pipeline's red DOES abort the level, and a push during a wait:true dispatch cascade-kills it (bug #5).
