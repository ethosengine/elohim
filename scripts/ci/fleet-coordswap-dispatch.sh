#!/usr/bin/env bash
# fleet-coordswap-dispatch.sh — CI leg of the rung-1 coordinator hot-swap
# vehicle (backlog upgrade-propagation-p2p-design-arc).
#
# Called by the DNA pipeline after elohim.happ is packed and has passed
# integration: derives the target fleet's storage peers from
# genesis/orchestrator/data/deployments.json and runs the rolling driver
# (scripts/ci/fleet-coordswap.sh --apply) against them.
#
# SAFE BY CONSTRUCTION, WARN-ONLY BY POLICY:
#   - The storage endpoint refuses bundles from a different DNA lineage
#     per role (integrity changes are NEVER hot-swapped) and no-ops when
#     there is no coordinator drift — so running this on every DNA build
#     is idempotent: coordinator-only diffs sweep the fleet in minutes,
#     everything else reports and changes nothing.
#   - This wrapper ALWAYS exits 0. A failed rollout prints a loud
#     "COORDSWAP:" summary for the build log; it must not fail the DNA
#     build while the vehicle earns trust. Peers still on a pre-endpoint
#     storage binary answer 404 and read as failed-apply — expected until
#     the edge roll that delivers the endpoint.
#   - Kill-switch: COORDSWAP_ENABLE=false (or 0) skips entirely.
#
# TRAJECTORY (do not extend this scaffold): the k8s-shaped parts of this
# vehicle are ONLY the initiator (CI push) and the roster (service DNS from
# deployments.json). The durable, protocol-shaped machinery is server-side:
# each peer applies coordinators to ITS OWN conductor via its own storage
# runtime (lineage guard, drift check, verify). Rung 5 of the arc replaces
# this push with delivery THROUGH the p2p network: the bundle travels the
# storage plane as elected content and each peer pulls/verifies/adopts at
# its own pace. Peer-selection or ordering intelligence must NOT accrete
# here — that behavior belongs to the adoption election, not this script.
#
# Usage: fleet-coordswap-dispatch.sh <path/to/elohim.happ> <target-env>
#   env DEPLOYMENTS_JSON overrides the humans source (for local testing).
set -uo pipefail

HAPP="${1:?usage: fleet-coordswap-dispatch.sh <happ> <target-env>}"
TARGET_ENV="${2:?usage: fleet-coordswap-dispatch.sh <happ> <target-env>}"
DEPLOYMENTS_JSON="${DEPLOYMENTS_JSON:-genesis/orchestrator/data/deployments.json}"
DRIVER="$(dirname "$0")/fleet-coordswap.sh"

case "${COORDSWAP_ENABLE:-true}" in
  false|0|no) echo "COORDSWAP: skipped (COORDSWAP_ENABLE=${COORDSWAP_ENABLE})"; exit 0 ;;
esac

if [ ! -s "$HAPP" ]; then
  echo "COORDSWAP: SKIP — happ bundle missing or empty: $HAPP"
  exit 0
fi
if [ ! -r "$DEPLOYMENTS_JSON" ]; then
  echo "COORDSWAP: SKIP — deployments source not readable: $DEPLOYMENTS_JSON"
  exit 0
fi

# Peer derivation mirrors the edge Jenkinsfile's resolveHumanAssignments +
# computeStorageUrls: resourcePrefix = elohim-<name>-<env>, namespace =
# elohim-<env>, storage HTTP on :8090. Suspended humans are the roster's
# source of truth (scope-reconcile) — never target them.
PEERS="$(jq -r --arg env "$TARGET_ENV" '
  .humans[]
  | select((.suspended // false) | not)
  | "\(.name)=http://elohim-\(.name)-\($env).elohim-\($env).svc.cluster.local:8090"
' "$DEPLOYMENTS_JSON" | paste -sd, -)"

if [ -z "$PEERS" ]; then
  echo "COORDSWAP: SKIP — no active peers derived for env '$TARGET_ENV'"
  exit 0
fi

echo "COORDSWAP: rolling coordinator hot-swap → env=$TARGET_ENV"
echo "COORDSWAP: peers: $PEERS"

rc=0
bash "$DRIVER" --happ "$HAPP" --peers "$PEERS" --apply --timeout 180 || rc=$?

if [ "$rc" -eq 0 ]; then
  echo "COORDSWAP: SUCCESS — fleet coordinators in sync with this build's bundle"
else
  echo "COORDSWAP: INCOMPLETE (driver rc=$rc) — see the rollout table above."
  echo "COORDSWAP: warn-only by policy; the DNA build is NOT failed by this."
fi
exit 0
