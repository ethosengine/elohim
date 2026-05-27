---
name: project-generous-probes-pattern
description: "Canonical k8s health-probe pattern for elohim manifests — timeoutSeconds: 15, failureThreshold: 5 baseline; readiness initialDelaySeconds: 120 for containers running embedded Holochain install_app."
metadata: 
  node_type: memory
  type: project
  originSessionId: f5ed8831-8faa-47bc-a508-fa91142db0de
---

The canonical k8s health-probe pattern across `genesis/orchestrator/manifests/**` is:

```yaml
# Embedded-Holochain containers (elohim-node consolidated, legacy edgenode):
livenessProbe:
  initialDelaySeconds: 60..180   # 60 for socat/conductor TCP probe; 180 for elohim-storage /health
  periodSeconds: 30
  timeoutSeconds: 15
  failureThreshold: 5
readinessProbe:
  initialDelaySeconds: 120        # keeps probe out of install_app + WASM compile window
  periodSeconds: 10
  timeoutSeconds: 15
  failureThreshold: 5

# Lightweight HTTP services (doorway, doorway-app, agent-sdk, storybook, NATS, mongodb):
livenessProbe / readinessProbe:
  initialDelaySeconds: 5..30      # cold-start dependent
  periodSeconds: 10..30
  timeoutSeconds: 15
  failureThreshold: 5
```

**Why:** The default `timeoutSeconds: 1` is too tight under CFS throttling for any pod sharing a contended node. 2026-05-27 incident shape: on shem (24 cores, 48% loaded, 13 of 14 alpha pods + a doorway replica), 9 of 14 storage pods went CrashLoopBackOff or got SIGKILL'd by liveness probes. Two distinct exit codes told the story:
- **Exit 1 (CrashLoopBackOff)** — daniel: `Error: install_app failed: Websocket error: Timeout`. Embedded conductor's admin WS handshake blew its budget because the conductor was CPU-throttled mid-WASM-compile (cpu: 1, install_app cycle ~30–60s).
- **Exit 137 (SIGKILL by kubelet liveness)** — nancy, eve: pod was alive and gossiping (`Serving content inventory count=3190`), but `/health` couldn't return within the 1s probe timeout under CFS throttling. 5 consecutive 1s misses × 30s period = 150s before kill.

15s is the upper end of "transient under load" while staying short of "actively broken." A `/health` endpoint that genuinely needs 25s to respond is sick; 15s gives 100× margin over typical millisecond response and 3× margin over heavy-WASM-compile spikes.

**How to apply:**
- Any new k8s manifest under `genesis/orchestrator/manifests/` adopts `timeoutSeconds: 15` + `failureThreshold: 5` on every probe by default.
- Containers running embedded Holochain (`elohim-node`, legacy `edgenode`, `elohim-storage` sidecar) bump readiness `initialDelaySeconds` to 120 so the install_app phase doesn't get scored.
- Containers with V8/Angular cold start (doorway, doorway-app) keep their `startupProbe` budget (24 × 5s = 120s) and add `timeoutSeconds: 15` on the regular probes.

**Related:** install_app CPU contention is also addressed by bumping `edgenodeCpuLimit` from `1000m` → `2000m` in `genesis/orchestrator/data/deployments.json` for 11 humans (jessica, james, pete, frank, gertrude, susan, caleb, daniel, emma, eve, nancy). Floors are preserved in `$recycledLaptopFloor` / `$chromebookFloor` records — restoration is gated on (a) shem CPU contention resolution (anti-affinity / topology spread) AND (b) the `elohim_storage::inventory` InvalidHashFormat verifier fix landing (`elohim/elohim-storage/src/p2p/inventory_gossip.rs:132–134`). See [[feedback_ci_evidence_underdetermines_cluster_diagnosis]] for why the symptom looked cluster-only at first and [[feedback_structural_verify_canonical_wire_shape]] for the verifier bug that's also part of the same incident.
