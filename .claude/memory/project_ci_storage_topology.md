---
name: CI build storage topology — openebs-jiva on hp-micro10, builds on operations/edge
description: Build PVCs use openebs-jiva-csi-default which replicates backing data to the storage node while staying network-accessible from build nodes
type: project
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
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
