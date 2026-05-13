---
name: Verify cluster state before drafting operator runbooks
description: Manifests in genesis/manifests/ may not match cluster reality; confirm namespace, storageClass, and existing PVCs against `kubectl get` before writing runbook expected-outputs
type: feedback
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
When drafting an operator runbook that says "this will be unchanged" or "this will be created," verify against actual cluster state — not against the manifest checked into the repo. The manifest is a desired-state declaration that may have drifted from what the cluster actually has.

**Why:** On 2026-05-09 the runbook for `nix-cache-pvc.yaml` claimed the apply would produce "nix-cache-holochain unchanged / cargo-cache-holochain unchanged / sweettest-target-cache-holochain created" because both existing PVCs in the manifest declared `namespace: ethosengine`. On the cluster, those two PVCs were actually in `namespace: jenkins` (Bound for 151 days, populated). Applying the manifest verbatim would have created three orphan empty PVCs in `ethosengine` and left the next elohim-holochain/dev build failing on the unbound sweettest claim. The operator caught this at pre-flight; the runbook had to be revised before apply.

**How to apply:**
- Before writing "Expected output: <unchanged>" sections, run `kubectl get pvc -A | grep <name>` (or equivalent for the resource type) and quote the actual STATE/AGE/namespace as observed.
- Cross-reference the consuming Jenkins pod template's `serviceAccount` and explicit `namespace:` field — Jenkins kubernetes-plugin pods typically spawn in the namespace where the SA lives, NOT necessarily where the manifest claims the PVC should be.
- For runbooks targeting a Che workspace, note that the Che image generally does NOT have `kubectl` installed. The apply pattern that works: `kubectl exec <workspace-pod> -- cat <file> | kubectl apply -f -` from a host with cluster credentials.
- When the manifest disagrees with cluster reality, fix the manifest in the same commit as the runbook revision so the repo state converges with the cluster.
