---
name: project_pipeline_dispatch_ordering
title: Pipeline dispatch & deploy ordering (umbrella)
description: "Dispatch/deploy ordering traps: live-target gates deadlock their own fix; a same-wave dispatch bakes the PREVIOUS happ (edge pulls floating dev-latest; DNA is fire-and-forget); new path-deps need Dockerfile COPY; coordinator changes keep the DNA hash; elohim-edgenode runs che MAIN (ff it), a hand-written gitlink can be a fabricated SHA, Jenkins param defaults lag one build, sweettest shards need 24Gi ephemeral on 0.7."
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

**2026-09-02 — commit tags are read from the TIP commit only.** `[build:*]`, `[conductor-roll]`,
`[dna:migrate]`, `[edge:validate-only]` are read by the orchestrator (`git log -1`) and by the edge
deploy's `resolveConductorWorkloadImage` from the tip of the push, not from every commit in the
range. A tagged commit pushed under a later untagged commit (a fmt fix, a sibling session's
commit) dispatches nothing. Put the tag on the last commit — an empty `git commit --allow-empty`
at the tip is the honest carrier when the tagged work is already committed.

**Re-hit 2026-09-03 (edge #1421):** adding `elohim-ark-core` as a storage path dep without extending the storage image COPY set (`elohim/elohim-storage/Dockerfile` + `scripts/ci/build-storage-image.sh`) broke the edge build; the local gate cannot see it. Every delegated-task prompt that may add a path dep must carry this rule explicitly — the memory alone did not reach the executor.
- **Conductor-image dispatch traps (2026-09-03, holochain 0.7 cutover):** (1) the `elohim-edgenode` Jenkins job runs
  che-devworkspaces **main**'s Jenkinsfile/Dockerfile — pushing a che branch and bumping the gitlink is NOT enough;
  fast-forward che `main` to the branch (`git push origin origin/<branch>:main`) or the job builds the old tx5-era
  recipe ("no HC_REF → no pin tag"). (2) A gitlink written by hand can carry a FABRICATED full SHA whose 12-char
  prefix matches the real commit; git accepts it, CI's exact-commit fetch dies with `upload-pack: not our ref`.
  Verify every gitlink with `git -C <submodule> cat-file -t <full-sha>` (must print `commit`) before pushing.
- **Jenkins parameter-default LAG (2026-09-03):** a job's `string(defaultValue:)` change in its Jenkinsfile takes effect
  only AFTER one build has run the new Jenkinsfile (the `properties` step rewrites the job definition mid-run, after the
  parameters were bound). elohim-edgenode #28 ran the 0.7 Jenkinsfile but bound the tx5-era `HC_FEATURES` default and
  died on "none of the selected packages contains these features". Check `getJob … parameterDefinitions` before
  re-dispatching; the first build after a default change is a sacrificial one unless the caller passes the value.
- **DNA pipeline on the 0.7 line (2026-09-03):** sweettest shard pods need ≥12Gi/24Gi ephemeral storage (holonix 0.7
  closure + 945 MB nextest archive) or kubelet evicts them mid-run and "N tests run" is ONE shard's count; the
  orchestrator fires elohim-holochain fire-and-forget and starts elohim-edge right after the app build, and edge pulls
  the FLOATING `elohim-happ:dev-latest` — so a DNA change needs its own round (`[build:dna]`), and only then an edge
  round; an edge failure cascades `Aborting` through the orchestrator. The 0.7 client family compiles OpenSSL from
  source (openssl-src via keystore/sqlcipher) — builder images need full `perl`, not perl-base.
