---
name: project_edge_deploy_restarts_genesis_conductors
title: Edge deploy restarts genesis-pair conductors
description: Edge Deploy stage unconditionally rollout-restarts every conductor StatefulSet (genesis pair included); a doorway-only fix needs the operator kubectl path.
metadata: 
  node_type: memory
  type: project
  originSessionId: 9976da73-b166-447f-a5fa-ac2fa59103b6
---

**The edge deploy restarts the genesis-pair conductors — there is no doorway-only push path.** `elohim/holochain/Jenkinsfile` "Deploy Edge Node - Alpha" stage calls `deployHumansInParallel` **unconditionally** (≈line 1830) → `deployHumanManifest` → **`Jenkinsfile:712 kubectl rollout restart statefulset/<human>`** for every human (`matthew, adam, jessica, james, pete` = default `HUMAN_ASSIGNMENTS`). `rollout restart` bounces the pod even when the edgenode image is unchanged (it patches the pod-template `restartedAt` annotation). The doorway restart that activates a doorway env/code change is a SEPARATE call right after (`Jenkinsfile:763 rollout restart deployment/elohim-doorway-<env>`).

- `DEPLOY_ONLY=true` only skips **build** stages — it still runs `deployHumansInParallel` (still restarts conductors). `RESET_HUMANS` is worse (deletes StatefulSets). No param/changeset runs the doorway deploy while skipping the human restart.
- A `claude/*` (or `feat-*`) branch push IS dispatched as dev-class by the orchestrator (`genesis/orchestrator/Jenkinsfile:873` — `claude\/.+` matches `isDevBranch`), and the Deploy stage `when` matches any changeset touching `doorway/**`/`elohim-storage/**`/`edgenode/**`. So a doorway-only code change still triggers the full human-restart deploy. See [[project_sprint_branch_not_orchestrator_indexed]].

**Why it matters (the genesis-pair brake):** the standing constraint is "never flip arc/restart/env on the matthew/adam genesis pair." A routine rolling restart rejoins the SAME DHT with the SAME agent key (no re-key — re-key only happens on `ALLOW_DNA_REINSTALL` force-reinstall, which a doorway-only change never triggers because the DNA hash is unchanged → install stale-check reads "not stale"). So a restart is *benign in steady state* — but if you're deploying an UNPROVEN bootstrap/discovery fix, forcing the genesis pair to re-bootstrap *through that fix* overnight/unwatched is the thing that arms a real partition.

**The safe landing path for a doorway-only fix (operator-owned kubectl):** build the doorway image (build stages restart nothing), then `kubectl apply -f manifests/doorway/alpha.yaml` (+ `alpha-b.yaml`) and `kubectl rollout restart deployment/elohim-doorway-alpha{,-b} -n elohim-alpha` — restarts ONLY the doorways, leaves conductors untouched. Bootstrap is a **runtime** protocol: the running conductors pick up a doorway-side bootstrap change on their next heartbeat PUT/GET (~3–30 min), no conductor restart needed. Applied to the F-BOOTSTRAP islanding fix (`BOOTSTRAP_MONGODB_DB` / `MongoK2Store`): `HANDOFF-2026-06-17-fbootstrap-deploy-gate.md`. Cluster is operator-owned — I never run kubectl from dev ([[k8s_is_not_the_architecture]]); this is the recommended-command handoff, not a self-executed step.
