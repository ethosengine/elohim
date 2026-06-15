#!/bin/bash
# Wait for a replica peer to catch up via P2P replication (never fatal).
#
# Externalized verbatim from genesis/Jenkinsfile (runContentSeedStage, replica
# catch-up loop) to keep the pipeline's single CPS dispatch method under the 64KB
# MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Invoked via sh(returnStatus: true, ...): exit 0 = caught up, exit 1 = lagging.
# NOTE: `set -uo pipefail` (no -e) — preserved exactly from the original.
#
# Args:
#   $1 = peer humanId (for log lines)
#   $2 = peer storage URL (host:port, no scheme)
set -uo pipefail

PEER_HUMAN_ID="$1"
PEER_STORAGE_URL="$2"

echo "⏳ WAITING FOR REPLICATION: ${PEER_HUMAN_ID} (${PEER_STORAGE_URL})"
DEADLINE=$((SECONDS + 600))
while [ $SECONDS -lt $DEADLINE ]; do
    STATUS=$(curl -sf "http://${PEER_STORAGE_URL}/p2p/status" 2>/dev/null || echo '{}')
    CAUGHT_UP=$(echo "$STATUS" | jq -r '.replication.caughtUp // false')
    COMPLETED=$(echo "$STATUS" | jq -r '.replication.completed // 0')
    PENDING=$(echo "$STATUS" | jq -r '.replication.pending // 0')
    # COMPLETED=0 is valid when the peer already had everything.
    if [ "$CAUGHT_UP" = "true" ] && [ "$PENDING" = "0" ]; then
        echo "✅ ${PEER_HUMAN_ID} caught up: $COMPLETED replicated this run, 0 pending"
        exit 0
    fi
    echo "   ${PEER_HUMAN_ID}: $COMPLETED completed, $PENDING pending"
    sleep 10
done
echo "⏭️  ${PEER_HUMAN_ID} replication not caught up within 600s"
exit 1
