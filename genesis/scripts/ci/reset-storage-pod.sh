#!/bin/bash
# Reset Storage — delete content.db and restart a single human's storage pod.
#
# Externalized verbatim from genesis/Jenkinsfile (Reset Storage stage, per-human
# loop) to keep the pipeline's single CPS dispatch method under the 64KB
# MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = human service name (e.g. elohim-node-matthew)
#   $2 = human namespace
#   $3 = human humanId (for the final log line)
set -euo pipefail

HUMAN_SERVICE="$1"
HUMAN_NAMESPACE="$2"
HUMAN_ID="$3"

echo "Resetting ${HUMAN_SERVICE}..."

# Delete content.db and restart pod for fresh migrations
kubectl exec -n ${HUMAN_NAMESPACE} ${HUMAN_SERVICE}-0 -c elohim-storage -- rm -f /data/content.db 2>/dev/null || true
kubectl delete pod -n ${HUMAN_NAMESPACE} ${HUMAN_SERVICE}-0 --wait=false

echo "   ✅ ${HUMAN_ID} reset initiated"
