# RUNBOOK — Iroh Rollback Drill (Gate #11)

**Drill scenario authored:** 2026-05-10
**Drill executed:** _(date — fill in after drill run)_
**Gate:** #11 of iroh cutover (see master plan `2026-05-10-iroh-delivery-master.md`)

---

## Purpose

Validate that the cluster can revert from iroh dual-stack to libp2p-only at any
time without data loss, service interruption, or permanent state corruption.
The drill also validates the forward flip back to iroh after confirming libp2p
stability.

This is a planned exercise, not an emergency procedure. Run it on the alpha
cluster while the gate #7 soak is active.

---

## What the env-flip does and does not do

### What DOES flip (runtime-only)

Setting `TRANSPORT_BACKEND=libp2p` on elohim-storage:

- Selects libp2p as the exclusive transport for all planes immediately on
  next elohim-storage startup (or hot-reload if supported).
- Stops iroh from being offered in future identity-handshake ALPNs.
- Causes the dual-stack selector to return libp2p on every plane decision.

No rebuild is required. The binary already contains both transports.
`TransportBackend` is a runtime selector, not a compile-time flag.

### What does NOT automatically rollback

These items persist across the env-flip and must be managed explicitly:

| Item | Persists through flip? | Management |
|---|---|---|
| **Diesel migrations** | Yes — all schema changes from Plans 1-5 remain. The `peer_transport_manifest` table and all iroh-related columns stay present. | Migrations are additive; no rollback migration exists or is needed. |
| **On-disk iroh state** | Yes — iroh's blob store, BLAKE3-addressed objects, and the secret key file at `IROH_SECRET_KEY_PATH` all remain on disk. | No action needed. If iroh is re-enabled later, it resumes from existing state. |
| **peer_transport_manifest rows** | Yes — every row recording a peer's observed iroh/libp2p plane support remains in SQLite. | Rows are not deleted. When libp2p-only mode is active, iroh_supports_json columns are simply not consulted by the selector. |
| **Iroh secret key** | Yes — the Ed25519 key used for iroh's node identity persists at `IROH_SECRET_KEY_PATH`. | This key is NOT rotated by the env-flip. If you need to rotate it (e.g., suspected key compromise), that is a separate procedure. |
| **QUIC listener socket** | No — iroh's UDP listener is closed when elohim-storage restarts in libp2p-only mode. The port is freed. | No action needed. |
| **Active iroh connections** | No — in-flight QUIC streams are gracefully closed on shutdown. No data loss for completed transfers. | In-flight transfers that had not completed may need to be retried by the caller. |

**Key insight:** the rollback is a config change + pod restart. State is safe.
The only operational risk is in-flight transfers at the moment of restart.

---

## Rollback procedure (iroh → libp2p)

Run from a node with `kubectl` access to the `elohim-alpha` namespace.

### Step 1 — Record pre-flip state

```bash
# Capture /status counters before the flip for comparison.
for POD in $(kubectl get pods -n elohim-alpha -l app=elohim-edgenode -o name); do
  echo "=== $POD pre-flip ==="
  kubectl exec -n elohim-alpha $POD -c elohim-storage \
      -- curl -s http://localhost:8090/status
done > rollback-drill-pre-flip-$(date +%Y%m%d-%H%M%S).json
```

### Step 2 — Flip transport backend to libp2p

```bash
kubectl set env statefulset/elohim-edgenode-alpha \
    -n elohim-alpha \
    -c elohim-storage \
    TRANSPORT_BACKEND=libp2p

# Watch rolling restart:
kubectl rollout status statefulset/elohim-edgenode-alpha -n elohim-alpha
```

Expected: pods restart one at a time (RollingUpdate strategy). Total restart
time depends on readiness probe delays (~30–60s per pod × replica count).

### Step 3 — Verify libp2p-only operation

Wait 5 minutes after rollout completes, then verify:

```bash
# All planes should show transport=libp2p, none=iroh.
for POD in $(kubectl get pods -n elohim-alpha -l app=elohim-edgenode -o name); do
  echo "=== $POD post-flip-to-libp2p ==="
  kubectl exec -n elohim-alpha $POD -c elohim-storage \
      -- curl -s http://localhost:8090/status
done > rollback-drill-post-flip-libp2p-$(date +%Y%m%d-%H%M%S).json

# Check for errors:
kubectl logs -n elohim-alpha -l app=elohim-edgenode -c elohim-storage \
    --since=10m | grep -E "error|TRANSPORT|iroh" | head -40
```

Success criterion: `blobs.iroh_served` counter is 0 on all new requests;
`blobs.libp2p_served` counter is incrementing; no "no shared transport" errors.

### Step 4 — Record latency during libp2p-only window

Run a spot check of blob fetch latency from a client device:

```bash
# Quick timing check — 10 fetches via libp2p-only alpha peer.
for i in $(seq 1 10); do
  curl -s -o /dev/null -w "%{time_total}\n" \
      https://doorway-alpha.elohim.host/blob/EXAMPLE_HASH_HERE
done
```

Record mean and max. These are the libp2p baseline numbers for comparison when
iroh is re-enabled.

---

## Forward flip back to iroh (libp2p → dual-stack)

After confirming libp2p stability (minimum 10 minutes), flip back to dual-stack:

### Step 1 — Re-enable iroh

```bash
kubectl set env statefulset/elohim-edgenode-alpha \
    -n elohim-alpha \
    -c elohim-storage \
    TRANSPORT_BACKEND=dual-stack

kubectl rollout status statefulset/elohim-edgenode-alpha -n elohim-alpha
```

### Step 2 — Verify iroh re-establishment

Wait 5 minutes for iroh identity-handshakes to complete between peers, then:

```bash
for POD in $(kubectl get pods -n elohim-alpha -l app=elohim-edgenode -o name); do
  echo "=== $POD post-flip-to-dual-stack ==="
  kubectl exec -n elohim-alpha $POD -c elohim-storage \
      -- curl -s http://localhost:8090/status
done > rollback-drill-post-flip-dual-stack-$(date +%Y%m%d-%H%M%S).json
```

Success criterion: `blobs.iroh_served` is incrementing again; no errors.

---

## Drill scenario

**Scenario:** "Alpha cluster has been on dual-stack for 24 hours. We flip to
libp2p-only, verify all blobs are served via libp2p, then flip back to
dual-stack and verify iroh resumes."

Timeline:

1. T+0: Record pre-flip /status counters.
2. T+2 min: Flip to libp2p-only. Rolling restart begins.
3. T+10 min: All pods restarted. Verify libp2p-only operation.
4. T+15 min: Run latency spot check (10 fetches).
5. T+20 min: Flip back to dual-stack. Rolling restart begins.
6. T+30 min: All pods restarted. Verify iroh re-established.
7. T+35 min: Record post-drill /status counters.
8. T+40 min: Commit drill report to this file.

---

## Drill report (fill in after execution)

**Date/time executed:**
**Operator:**

| Metric | Pre-flip (dual-stack) | During libp2p-only | Post-flip (dual-stack) |
|---|---|---|---|
| iroh_served (total across peers) | | 0 (expected) | |
| libp2p_served (total across peers) | | | |
| "no shared transport" errors | | 0 (expected) | 0 (expected) |
| Blob fetch p50 latency (ms) | | | |
| Blob fetch p99 latency (ms) | | | |
| Rolling restart duration (total) | n/a | | |

**Outcome:** PASS / FAIL / PARTIAL

**Notes:**

**Gate #11 closed:** _(date; drill report filled; signed off by)_
