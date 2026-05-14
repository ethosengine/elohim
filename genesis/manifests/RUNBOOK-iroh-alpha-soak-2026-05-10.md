# RUNBOOK — Iroh Alpha-Cluster Soak (Gate #7)

**Soak window opened:** 2026-05-10
**Soak window closes when:** 7 days with zero "no shared transport" errors and
both `/status` blob counters incrementing on every peer
**Gate:** #7 of iroh cutover (see master plan `2026-05-10-iroh-delivery-master.md`)

---

## Cluster state (from manifests — operator must verify actual state)

The alpha cluster topology (per memory `project_alpha_topology_bootstrap_pair`):

| Peer | Role | Manifest | Namespace |
|---|---|---|---|
| adam | Bootstrap pair (node 0) | `edgenode/alpha.yaml` StatefulSet replica 0 | `elohim-alpha` |
| matthew | Bootstrap pair (node 1) + doorway operator | `edgenode/alpha.yaml` StatefulSet replica 1 | `elohim-alpha` |
| jessica | Household peer | `edgenode/alpha.yaml` StatefulSet replica 2 | `elohim-alpha` |
| terrance | Stewarded-child peer | `edgenode/alpha.yaml` StatefulSet replica 3 | `elohim-alpha` |
| shem-* | Multi-tenant shem peers (2) | `edgenode/alpha.yaml` StatefulSet replicas 4-5 | `elohim-alpha` |

Che workspace does not have `kubectl`. The operator must verify actual state
before applying. Expected state after manifest apply:

```
# Verify all elohim-storage pods show TRANSPORT_BACKEND=dual-stack
kubectl get statefulset elohim-edgenode-alpha -n elohim-alpha -o jsonpath='{.spec.template.spec.containers[?(@.name=="elohim-storage")].env}' | python3 -m json.tool

# Verify pods are running (not in CrashLoopBackOff after env change)
kubectl get pods -n elohim-alpha -l app=elohim-edgenode
```

**IMPORTANT:** This runbook documents manifest state. Actual cluster state may
differ. Per memory `feedback_verify_cluster_state_before_runbook`: always
quote actual `kubectl get` output before and after applying. Do not assume
the manifest matches the running cluster.

### How to apply

The manifest change is in
`genesis/orchestrator/manifests/edgenode/alpha.yaml` — `TRANSPORT_BACKEND`
env var added to the `elohim-storage` container spec (Gate #7 comment inline).

Apply from a node with `kubectl` access:

```bash
kubectl apply -f genesis/orchestrator/manifests/edgenode/alpha.yaml
# StatefulSet rolling update will restart pods one at a time.
# Watch rollout:
kubectl rollout status statefulset/elohim-edgenode-alpha -n elohim-alpha
```

---

## Transport backend selector

`TRANSPORT_BACKEND` on elohim-storage selects the active transport at runtime:

| Value | Behaviour |
|---|---|
| `libp2p` | All planes use libp2p only (pre-cutover default) |
| `dual-stack` | All planes try iroh first, fall back to libp2p (soak mode) |
| `iroh` | All planes iroh-only (post-cutover, not yet enabled during soak) |

The soak uses `dual-stack`. This means every blob fetch, gossip publish, sync
delta, and EPR announce attempts iroh first. If iroh fails, libp2p handles it
transparently. Zero data loss is expected regardless of transport outcome.

---

## Success criteria (7-day window)

All three must hold continuously for 7 days:

1. **Every blob fetch served** — either iroh or libp2p; no `"no shared transport"`
   errors in elohim-storage logs on any of the 6 alpha peers.
2. **Both counters incrementing** — `/status` on each peer shows
   `blobs.iroh_served` and/or `blobs.libp2p_served` growing over time.
   A peer that sees only libp2p growth is acceptable (iroh may not have a
   direct path through that NAT) but must not show zero total blob activity.
3. **Zero "no shared transport" errors** — grep elohim-storage logs:
   ```bash
   kubectl logs -n elohim-alpha -l app=elohim-edgenode -c elohim-storage \
       --since=168h | grep "no shared transport"
   # Expected: no output
   ```

### Monitoring commands

```bash
# Check /status on each peer via doorway
for PEER in 0 1 2 3 4 5; do
  echo "=== Pod $PEER ==="
  kubectl exec -n elohim-alpha elohim-edgenode-alpha-$PEER \
      -c elohim-storage -- curl -s http://localhost:8090/status | python3 -m json.tool
done

# Scan for transport errors (run daily)
kubectl logs -n elohim-alpha -l app=elohim-edgenode -c elohim-storage \
    --since=24h 2>&1 | grep -E "no shared transport|iroh.*error|transport.*fail"
```

---

## Escalation

- **"no shared transport" errors**: check peer-map rows for the failing peer
  pair. The `peer_transport_manifest` table should show both peers supporting
  at least one common plane. If not, the identity-handshake plane may not
  have completed — check `iroh_identity_handshake_*` test suite.
- **One counter always zero**: acceptable if the peer is isolated (e.g., shem
  peers behind strict NAT). Document which archetype; this informs gate #9
  consumer-grade decisions.
- **Pod crash after env change**: rollback immediately —
  `kubectl set env statefulset/elohim-edgenode-alpha -n elohim-alpha TRANSPORT_BACKEND=libp2p -c elohim-storage`
  then investigate logs before re-applying.

---

## Gate closure

| Date (UTC) | Iroh served | Libp2p served | "no shared transport" errors | Notes |
|---|---|---|---|---|
| 2026-05-10 | — | — | — | Soak window opened; manifest applied |
| (fill daily) | | | | |

**Gate #7 closed:** _(date, kubectl get statefulset output quoted, signed off by)_
