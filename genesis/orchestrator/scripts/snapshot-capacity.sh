#!/usr/bin/env bash
# snapshot-capacity.sh — capture a live cluster snapshot for the rakia ledger.
#
# Reads `kubectl get nodes/pods/pvc` and emits a JSON document matching the
# top-level shape of genesis/data/rakia/compute-capacity-snapshot.json.
# The output is the audit-trail input to compute-capacity.json — the
# operator (or a future validator) reviews the diff before promoting
# numbers from the snapshot into the ledger.
#
# Usage:
#   genesis/orchestrator/scripts/snapshot-capacity.sh [output.json]
#
# Default output: ./compute-capacity-snapshot.json (next to repo root).
#
# Requirements:
#   - kubectl with cluster access
#   - jq
#   - metrics-server enabled if you want actuals (`kubectl top` output)
#
# Notes:
#   - This script is intentionally read-only. It never modifies cluster
#     state, never edits the ledger, and never writes outside the output
#     path you give it.
#   - Steward namespaces are an inline list; edit STEWARD_NAMESPACES below
#     when adding/removing tracked workloads.
#   - When metrics-server is unavailable, `kubectl top` lines fail gracefully
#     and the actuals block is omitted.

set -euo pipefail

OUT="${1:-compute-capacity-snapshot.json}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

STEWARD_NAMESPACES=(jenkins harbor nexus eclipse-che)

if ! command -v kubectl >/dev/null 2>&1; then
  echo "snapshot-capacity: kubectl not found on PATH" >&2
  exit 2
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "snapshot-capacity: jq not found on PATH" >&2
  exit 2
fi

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

# ---- Section 1: nodes ----------------------------------------------------
kubectl get nodes -o json | jq '
  {
    items: [.items[] | {
      name: .metadata.name,
      nodeType: (.metadata.labels."node-type" // "untyped"),
      role: ((.metadata.labels."node-role.kubernetes.io/control-plane" | if . then "control-plane" else "worker" end) // "worker"),
      ready: ([.status.conditions[] | select(.type == "Ready") | .status] | first == "True"),
      allocatable: .status.allocatable,
      capacity: .status.capacity
    }]
  }' > "$WORKDIR/nodes.json"

# ---- Section 2: cluster-wide top (live actuals) --------------------------
if kubectl top nodes --no-headers >/dev/null 2>&1; then
  kubectl top nodes --no-headers 2>/dev/null \
    | awk '{ printf "{\"name\":\"%s\",\"cpu\":\"%s\",\"cpu_pct\":\"%s\",\"mem\":\"%s\",\"mem_pct\":\"%s\"}\n",$1,$2,$3,$4,$5 }' \
    | jq -s '.' > "$WORKDIR/top-nodes.json"
else
  echo '[]' > "$WORKDIR/top-nodes.json"
fi

# ---- Section 3: per-namespace steward commitments ------------------------
echo '{}' > "$WORKDIR/stewards.json"
for ns in "${STEWARD_NAMESPACES[@]}"; do
  pod_resources=$(kubectl get pods -n "$ns" -o json 2>/dev/null | jq '
    [.items[].spec.containers[].resources] | {
      requests: {
        cpu_m: ([.[].requests.cpu // empty] | length),
        memory_Mi: ([.[].requests.memory // empty] | length)
      },
      limits: {
        cpu_m: ([.[].limits.cpu // empty] | length),
        memory_Mi: ([.[].limits.memory // empty] | length)
      },
      containerCount: length
    }' || echo '{}')

  pvc_list=$(kubectl get pvc -n "$ns" -o json 2>/dev/null | jq '
    [.items[] | {
      name: .metadata.name,
      size: .spec.resources.requests.storage,
      storageClass: .spec.storageClassName
    }]' || echo '[]')

  workloads=$(kubectl get all -n "$ns" -o json 2>/dev/null | jq '
    [.items[] | select(.kind == "Deployment" or .kind == "StatefulSet" or .kind == "DaemonSet") | {
      kind: .kind,
      name: .metadata.name,
      replicas: (.status.replicas // 0)
    }]' || echo '[]')

  jq --arg ns "$ns" \
     --argjson pods "$pod_resources" \
     --argjson pvcs "$pvc_list" \
     --argjson workloads "$workloads" \
     '. + {($ns): {namespace: $ns, podResources: $pods, pvcs: $pvcs, workloads: $workloads}}' \
     "$WORKDIR/stewards.json" > "$WORKDIR/stewards.json.tmp"
  mv "$WORKDIR/stewards.json.tmp" "$WORKDIR/stewards.json"
done

# ---- Section 4: elohim humans (cluster-wide via label) -------------------
kubectl get pods -A -l app=elohim-edgenode -o json 2>/dev/null | jq '
  {
    items: [.items[] | {
      name: .metadata.name,
      namespace: .metadata.namespace,
      human: (.metadata.labels."elohim-human" // "unknown"),
      node: .spec.nodeName,
      phase: .status.phase,
      requests: (.spec.containers[0].resources.requests // {}),
      limits: (.spec.containers[0].resources.limits // {})
    }]
  }' > "$WORKDIR/humans.json"

kubectl get pvc -A -l app=elohim-edgenode -o json 2>/dev/null | jq '
  {
    items: [.items[] | {
      namespace: .metadata.namespace,
      name: .metadata.name,
      size: .spec.resources.requests.storage,
      storageClass: .spec.storageClassName
    }]
  }' > "$WORKDIR/human-pvcs.json"

# ---- Compose final output ------------------------------------------------
jq -n \
  --arg ts "$TIMESTAMP" \
  --slurpfile nodes "$WORKDIR/nodes.json" \
  --slurpfile topNodes "$WORKDIR/top-nodes.json" \
  --slurpfile stewards "$WORKDIR/stewards.json" \
  --slurpfile humans "$WORKDIR/humans.json" \
  --slurpfile humanPvcs "$WORKDIR/human-pvcs.json" \
  '{
    snapshotTimestamp: $ts,
    snapshotMethod: "snapshot-capacity.sh — kubectl get nodes/pods/pvc + kubectl top",
    nodes: $nodes[0],
    topNodes: $topNodes[0],
    stewards: $stewards[0],
    elohimHumans: $humans[0],
    elohimPvcs: $humanPvcs[0]
  }' > "$OUT"

echo "snapshot-capacity: wrote $(wc -c < "$OUT") bytes to $OUT"
echo "snapshot-capacity: review the diff against genesis/data/rakia/compute-capacity.json before committing."
