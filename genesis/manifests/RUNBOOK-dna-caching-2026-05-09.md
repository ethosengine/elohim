# Runbook — DNA caching PVC apply (2026-05-09)

**Target cluster:** the K8s cluster that hosts `elohim-holochain` Jenkins builds (operations + edge nodes for compute, hp-micro10 for storage).
**Namespace:** `ethosengine`
**Owning pipeline:** `elohim-holochain/dev`
**Risk:** None to running services. This adds a new PVC consumed only by the elohim-holochain build pod, which is created on demand for each build.

## What this does

Adds one new PersistentVolumeClaim — `sweettest-target-cache-holochain` — that the next elohim-holochain build will mount at `/cargo-target` to persist compiled cargo output across builds. Without this PVC, the pod will fail to schedule with `pod has unbound immediate PersistentVolumeClaims`.

The two existing PVCs in the same manifest (`nix-cache-holochain`, `cargo-cache-holochain`) are unchanged. `kubectl apply` will be a no-op for them.

## Apply

```bash
kubectl apply -f genesis/manifests/nix-cache-pvc.yaml
```

Run this command from the workspace root (`/projects/elohim`), or pass the absolute path:

```bash
kubectl apply -f /projects/elohim/genesis/manifests/nix-cache-pvc.yaml
```

Expected output:

```
persistentvolumeclaim/nix-cache-holochain unchanged
persistentvolumeclaim/cargo-cache-holochain unchanged
persistentvolumeclaim/sweettest-target-cache-holochain created
```

## Verify

Confirm all three PVCs are present and Bound to backing volumes:

```bash
kubectl get pvc -n ethosengine -l app=holochain-build
```

Expected output (all three should show STATUS=Bound):

```
NAME                                STATUS   VOLUME                                     CAPACITY   ACCESS MODES   STORAGECLASS                AGE
cargo-cache-holochain               Bound    pvc-<id>                                   20Gi       RWO            openebs-jiva-csi-default    <existing>
nix-cache-holochain                 Bound    pvc-<id>                                   50Gi       RWO            openebs-jiva-csi-default    <existing>
sweettest-target-cache-holochain    Bound    pvc-<id>                                   20Gi       RWO            openebs-jiva-csi-default    <new>
```

If `sweettest-target-cache-holochain` shows STATUS=Pending for more than ~30 seconds, the openebs-jiva CSI provisioner did not pick it up. Check provisioner logs:

```bash
kubectl logs -n openebs-system -l openebs.io/component-name=openebs-jiva-csi-controller --tail=100
kubectl describe pvc sweettest-target-cache-holochain -n ethosengine
```

## Storage placement note

`storageClassName: openebs-jiva-csi-default` is the same class used by the existing nix-cache and cargo-cache PVCs. Per `genesis/manifests/nix-cache-pvc.yaml` header:

> Storage class: openebs-jiva-csi-default (replicated, network-attached)
>   - Replicated across nodes for availability
>   - Data stored on storage node (hp-micro10, 4TB)
>   - Slightly higher latency than hostpath, but flexible scheduling

So the 20Gi backing data will land on hp-micro10 alongside Harbor / Nexus / existing build caches. Builder pods schedule on operations or edge nodes (per `nodeAffinity` in the elohim-holochain Jenkinsfile pod template) and mount the PVC over the network. No node-affinity changes are needed in the cluster.

## What the new PVC will be used for

The next elohim-holochain build (Jenkins job `elohim-holochain/dev`) will:
1. Mount `nix-cache-holochain` at `/nix/store` (was previously declared but unmounted — this is Fix 1, lives entirely in the Jenkinsfile).
2. Mount this new `sweettest-target-cache-holochain` at `/cargo-target` and set `CARGO_TARGET_DIR=/cargo-target` (Fix 2).

The first build after apply will still cold-compile (cache is empty). The second build onward should drop the 59-minute "DNA Integration" stage to ~15 min — the 47m 48s sweettest compile turns incremental.

## Rollback

This PVC is consumed only by the elohim-holochain build pod, which is created fresh per build. To remove:

```bash
kubectl delete pvc sweettest-target-cache-holochain -n ethosengine
```

Then revert commit `ff648597f` on the `dev` branch (the Jenkinsfile changes that mount the PVC). Without the revert, the next build will fail to schedule with the missing PVC.

The other two PVCs (`nix-cache-holochain`, `cargo-cache-holochain`) and the data on them are unaffected by rollback.

## Files referenced

- Manifest: `genesis/manifests/nix-cache-pvc.yaml`
- Pipeline that consumes the PVC: `elohim/holochain/dna/Jenkinsfile`
- Commit that added everything: `ff648597f` (`fix(holochain): persist nix-store + sweettest target/ across builds`)

## Confirm back to the Jenkins pipeline owner

Once `kubectl get pvc` shows STATUS=Bound for the new PVC, the next `elohim-holochain/dev` build is safe to trigger. No coordination with running deployments needed — this affects only the build pod, not any production conductor / storage / doorway pods.
