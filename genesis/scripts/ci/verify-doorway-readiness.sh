#!/bin/bash
# Doorway readiness + preflight checks before seeding.
#
# Externalized verbatim from genesis/Jenkinsfile (the "Verify Target Health"
# stage) to keep the pipeline's single CPS dispatch method under the 64KB
# MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit"
# (helpers stay heredoc-free; bash bodies live in scripts/ci/*.sh).
#
# Args:
#   $1 = internal doorway host:port (e.g. elohim-doorway-alpha....svc:PORT)
#   $2 = external doorway base URL (e.g. https://doorway-alpha.elohim.host)
#   $3 = internal storage base URL (host:port, no scheme)
set -euo pipefail

INTERNAL_DOORWAY="$1"
DOORWAY_HOST="$2"
INTERNAL_STORAGE_URL="$3"

MAX_ATTEMPTS=30
ATTEMPT=1
DOORWAY_READY=false
DOORWAY_BASE_URL=""

# First, wait for doorway to be reachable
while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
    # Try internal DNS first
    if curl -sf -o /dev/null "http://${INTERNAL_DOORWAY}/health" 2>/dev/null; then
        DOORWAY_BASE_URL="http://${INTERNAL_DOORWAY}"
        echo "✅ Doorway responding (via internal DNS)"
        break
    fi
    # Fall back to external DNS
    if curl -sf -o /dev/null "${DOORWAY_HOST}/health" 2>/dev/null; then
        DOORWAY_BASE_URL="${DOORWAY_HOST}"
        echo "✅ Doorway responding (via external DNS)"
        break
    fi
    echo "Waiting for Doorway... (attempt $ATTEMPT/$MAX_ATTEMPTS)"
    sleep 5
    ATTEMPT=$((ATTEMPT + 1))
done

if [ -z "$DOORWAY_BASE_URL" ]; then
    echo "❌ Doorway not responding after $MAX_ATTEMPTS attempts"
    exit 1
fi

# Now use /status for comprehensive preflight checks
echo ""
echo "════════════════════════════════════════════════════════════"
echo "🔍 PREFLIGHT CHECKS"
echo "════════════════════════════════════════════════════════════"

PREFLIGHT_ATTEMPTS=0
PREFLIGHT_MAX=24  # 2 minutes with 5s intervals
CONDUCTOR_READY=false

while [ $PREFLIGHT_ATTEMPTS -lt $PREFLIGHT_MAX ]; do
    # /health returns JSON; /status returns HTML dashboard
    STATUS_JSON=$(curl -sf "$DOORWAY_BASE_URL/health" 2>/dev/null || echo '{}')

    # Parse health response
    CONDUCTOR_CONNECTED=$(echo "$STATUS_JSON" | jq -r '.conductor.connected // false')
    CONDUCTOR_WORKERS=$(echo "$STATUS_JSON" | jq -r '.conductor.connected_workers // 0')
    HEALTHY=$(echo "$STATUS_JSON" | jq -r '.healthy // false')
    STATUS=$(echo "$STATUS_JSON" | jq -r '.status // "unknown"')
    ERROR_MSG=$(echo "$STATUS_JSON" | jq -r '.error // empty')

    echo "  Healthy: $HEALTHY  Status: $STATUS"
    echo "  Conductor: connected=$CONDUCTOR_CONNECTED workers=$CONDUCTOR_WORKERS"
    if [ -n "$ERROR_MSG" ]; then
        echo "  ⚠️  $ERROR_MSG"
    fi

    # Check if ready — conductor must have workers connected
    if [ "$CONDUCTOR_CONNECTED" = "true" ] && [ "$HEALTHY" = "true" ]; then
        echo ""
        echo "✅ Doorway ready for seeding"
        CONDUCTOR_READY=true
        break
    fi

    PREFLIGHT_ATTEMPTS=$((PREFLIGHT_ATTEMPTS + 1))
    if [ $PREFLIGHT_ATTEMPTS -lt $PREFLIGHT_MAX ]; then
        echo "  ⏳ Waiting for conductor/storage... ($PREFLIGHT_ATTEMPTS/$PREFLIGHT_MAX)"
        sleep 5
    fi
done

if [ "$CONDUCTOR_READY" != "true" ]; then
    echo ""
    echo "❌ PREFLIGHT FAILED: Doorway not ready for seeding"
    echo "   Conductor connected: $CONDUCTOR_CONNECTED"
    echo "   Storage healthy: ${STORAGE_HEALTHY:-}"
    echo ""
    echo "Check doorway and elohim-storage logs for issues."
    exit 1
fi

# Verify SQLite database is available on elohim-storage (edgenode service)
echo ""
echo "🔍 Checking SQLite database availability..."
STORAGE_URL="http://${INTERNAL_STORAGE_URL}"
DB_STATS=$(curl -sf "$STORAGE_URL/db/stats" 2>/dev/null || echo '{}')
DB_READY=$(echo "$DB_STATS" | jq -r 'has("contentCount")' 2>/dev/null || echo 'false')

if [ "$DB_READY" = "true" ]; then
    echo "✅ SQLite database ready at $STORAGE_URL"
    CURRENT_CONTENT=$(echo "$DB_STATS" | jq -r '.contentCount // 0')
    CURRENT_TAGS=$(echo "$DB_STATS" | jq -r '.uniqueTags // 0')
    echo "   Current state: $CURRENT_CONTENT content, $CURRENT_TAGS tags"
else
    echo "❌ SQLite database not available at $STORAGE_URL"
    echo "   Ensure ENABLE_CONTENT_DB=true is set on elohim-storage"
    exit 1
fi

echo "════════════════════════════════════════════════════════════"
