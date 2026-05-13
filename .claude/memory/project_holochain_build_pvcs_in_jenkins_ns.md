---
name: Holochain build PVCs live in `jenkins` namespace
description: nix-cache-holochain, cargo-cache-holochain, sweettest-target-cache-holochain all in jenkins ns; ethosengine ns has zero of these
type: project
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
The three persistent caches consumed by the elohim-holochain/dev pipeline live in `namespace: jenkins`:

- `nix-cache-holochain` (50Gi, openebs-jiva-csi-default)
- `cargo-cache-holochain` (20Gi, same SC)
- `sweettest-target-cache-holochain` (20Gi, same SC, added 2026-05-09)

**Why:** Jenkins kubernetes-plugin spawns build pods in the namespace where its service account (`ee-jenkins`) lives, which is `jenkins`. Cross-namespace PVC mounts are not a thing — the PVC must be co-namespaced with the consuming pod.

**How to apply:**
- Any new PVC consumed by the elohim-holochain pipeline (or any Jenkins-spawned build pod) must declare `namespace: jenkins`. Earlier docs / drafts of `genesis/manifests/nix-cache-pvc.yaml` had `namespace: ethosengine`, which is wrong; commit `023443370` corrected this.
- When verifying state from outside: `kubectl get pvc -n jenkins -l app=holochain-build` is the canonical query.
- Don't conflate `ethosengine` (production namespace where conductor / storage / doorway run) with `jenkins` (CI namespace). Production PVCs (e.g. `data-elohim-matthew-alpha-0`, `data-elohim-jessica-alpha-0`) DO live in elohim-alpha; CI build caches live in jenkins.
- Storage class `openebs-jiva-csi-default` places backing data on hp-micro10 storage node regardless of namespace; that's a cluster-wide CSI behavior.
