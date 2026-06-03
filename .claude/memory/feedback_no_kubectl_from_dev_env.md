---
id: feedback-no-kubectl-from-dev-env
name: feedback-no-kubectl-from-dev-env
description: "Don't run kubectl from this dev environment for cluster-state mutations or even reads — operator owns cluster access; agent owns code-level changes only."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 41ea186d-062d-48b9-8772-9a4a3d9763d9
cites:
  - genesis/manifests
---

Don't run kubectl from this Eclipse Che dev environment. When the operator says "clean up the ingresses" or similar cluster-touching task, that means: **make sure the repo manifests are coherent / clean / non-conflicting** so the next pipeline run (or operator-side apply) reconciles to the right state. The operator owns cluster access directly.

**Why:** keeps blast-radius narrow — operator can hand-trigger cluster ops with their own context (cordoned nodes, partial deploys in flight, etc.); agent contributes by ensuring the code-of-record is what the operator wants to apply.

**How to apply:** for any "clean up X resource" or "fix the live ingress / configmap / deployment" request:
- Grep the repo for the resource type + the conflicting name/host/label
- Make sure no manifest in the repo claims state that conflicts with the desired end state
- Surface stale manifests for deletion/edit OR fix the manifest to match desired
- Never run `kubectl apply|delete|edit|patch` from this env, even for "just one ingress"
- Even `kubectl get` reads are operator-side; use Jenkins MCP or in-repo manifests as the read surface

Concrete trigger from 2026-05-23: asked to "make sure ingresses are cleaned up" — first tried `kubectl get ingress` from the dev env; operator rejected and clarified "just make sure you've cleaned up the ingresses in the code." The repo IS the cleanup surface; live cluster is the operator's surface.
