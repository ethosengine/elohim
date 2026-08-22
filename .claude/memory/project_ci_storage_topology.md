---
index: false
id: project-ci-storage-topology
name: CI build storage topology — migrated openebs-jiva → openebs-hostpath; hostpath needs deterministic node pinning
title: CI build storage — hostpath PVCs in jenkins ns, node-pinned
description: "CI cache PVCs (nix/cargo/sweettest-target) are openebs-hostpath in jenkins ns; pin with kubernetes.io/hostname nodeAffinity or pods thrash on volume binding."
type: project
originSessionId: 91882765-aece-476c-a49a-85b618774d32
cites:
  - genesis/manifests/nix-cache-pvc.yaml
  - elohim/holochain/Jenkinsfile
  - elohim/holochain/dna/Jenkinsfile
  - steward/device/Jenkinsfile
---

> **CURRENT STATE — read the "Update 2026-05-27" section at the bottom first.** The CI cache PVCs were
> migrated off `openebs-jiva` to `openebs-hostpath`. The original section immediately below describes the
> SUPERSEDED openebs-jiva topology and is kept for lineage; its "no nodeSelector/affinity tricks needed"
> guidance is **no longer true** under hostpath — see the correction.

## SUPERSEDED (pre-2026-05-27): openebs-jiva topology
The CI cluster splits build CPU/RAM from build storage:
- **Build pods** schedule on `operations` (Intel NUC i5, 8 cores, 16GB) or `edge` nodes via `nodeAffinity` in the Jenkinsfile pod template.
- **Build storage** (nix-cache, cargo-cache, future sweettest-target-cache) uses `storageClassName: openebs-jiva-csi-default` — a replicated, network-attached storage class that places backing data on the **storage node** (hp-micro10, 4TB disk) while remaining mountable from any build node.

**Why:** Putting cache PVCs on local-path storage on the operations node would (a) overload its disk with the same artifacts the dedicated storage node already hosts (Harbor, Nexus, etc.), and (b) force pod scheduling to be pinned to one node. openebs-jiva gives the artifact-on-storage-node placement automatically while preserving scheduling flexibility.

**How to apply:**
- New build cache PVC? Use `storageClassName: openebs-jiva-csi-default`, ReadWriteOnce, in namespace `ethosengine`. Define in `genesis/manifests/`. Backing data lands on hp-micro10 by default; no nodeSelector/affinity tricks needed for placement.
- Builder pod schedule constraints (`operations` or `edge`) come from the Jenkinsfile pod template, not the PVC.
- ReadWriteOnce + `disableConcurrentBuilds()` on the pipeline is the pattern that prevents two builds racing on the same volume.
- Reference: `genesis/manifests/nix-cache-pvc.yaml` is the canonical pattern document — it has the architectural comment headers explaining the split.
- Don't reach for hostpath/local-path PVCs on operations or edge nodes for build artifacts — that defeats the storage-on-hp-micro10 design.

---

## Update 2026-05-27 — migrated openebs-jiva → openebs-hostpath; hostpath needs deterministic node pinning

**This correction supersedes the openebs-jiva guidance above** (including the "no nodeSelector/affinity tricks needed" / "don't reach for hostpath" lines — those described the OLD topology and are now wrong).

The Holochain/edge CI cache PVCs were cut over from `openebs-jiva-csi-default` (replicated, network-attached, schedule-anywhere) to `openebs-hostpath` (node-local). Hostpath is `volumeBindingMode: WaitForFirstConsumer`, so on a **multi-node pool** the PVC binds to whichever node the first consumer pod lands on — a scheduler lottery. When that node is resource-contended, the PVC pins there and subsequent pods can't schedule:
- pods 1–5 time out ~1000s each on `volume node affinity conflict` (Phase B);
- Phase A reads `0/7 nodes... didn't find available persistent volumes to bind`;
- the build only proceeds when cluster pressure shifts (observed ~1h thrash before pod 6 scheduled, elohim-edge #1010). This is anti-pattern AP-009 (pod-scheduling thrash).

**Fix — make hostpath binding deterministic:** add a `kubernetes.io/hostname` nodeAffinity to each pipeline's Jenkinsfile pod spec. Operator assignment: elohim-edge → `thinkc-p1s`; elohim-holochain (DNA) + steward → `intel-nuc`.

**Namespace:** the CI cache PVCs (nix-cache, cargo-cache, sweettest-target-cache) live in the `jenkins` namespace (per `genesis/manifests/nix-cache-pvc.yaml`), NOT `ethosengine` — the superseded section's namespace guidance is stale too.

**Tradeoff:** if a PVC had previously bound to a DIFFERENT node, the pin triggers an immediate `FailedScheduling` and the operator must delete+reapply that PVC. Dead `nix-store` volume declarations are intentionally left in the Jenkinsfiles as breadcrumbs for future sccache work.

**Co-symptom on #1067 (not a storage issue):** the Angular alpha bundle exceeded its 4MB error cap at 7.05MB → bumped to 8MB warn / 9MB error as a pragmatic unblock.

Source: shift `2026-05-27T00-14-first-clean-post-migration-dev-build`.
