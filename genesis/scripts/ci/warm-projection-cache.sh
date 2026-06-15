#!/bin/bash
# Warm Projection Cache — POST /admin/cache/warm on the target doorway.
#
# Externalized verbatim from genesis/Jenkinsfile (Warm Projection Cache stage) to
# keep the pipeline's single CPS dispatch method under the 64KB
# MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
set -euo pipefail

DOORWAY_HOST="$1"

echo "═══════════════════════════════════════════════════════════"
echo "🔄 WARMING PROJECTION CACHE"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
echo ""

RESP=$(curl -sf -X POST "${DOORWAY_HOST}/admin/cache/warm" || echo '{}')
STATUS=$(echo "$RESP" | jq -r '.status // "error"')
PEERS=$(echo "$RESP" | jq -r '.peers // 0')

if [ "$STATUS" = "warming" ]; then
    echo "   ✅ Cache warm-up triggered ($PEERS peer(s))"
    echo "   ⏳ Waiting 10s for warm-up to complete..."
    sleep 10
else
    echo "   ⚠️ Cache warm-up returned: $RESP"
    echo "   Doorway may need restart to refresh projection cache"
fi
