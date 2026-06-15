#!/bin/bash
# Verify Seeding — assessment-content spot-check on a genesis peer.
#
# Externalized verbatim from genesis/Jenkinsfile (runVerifySeedingStage) to keep
# the pipeline's single CPS dispatch method under the 64KB MethodTooLargeException
# limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = genesis peer storage URL (host:port, no scheme)
#   $2 = genesis peer humanId (for log lines)
set -euo pipefail

GENESIS_URL="$1"
CHECK_PEER_HUMAN_ID="$2"

echo ""
echo "🔬 Verifying assessment content on genesis peer (${CHECK_PEER_HUMAN_ID})..."
for ASSESSMENT_ID in assessment-personal-values assessment-values-hierarchy; do
    ASSESSMENT=$(curl -sf "http://${GENESIS_URL}/db/content/$ASSESSMENT_ID" || echo '{}')
    FMT=$(echo "$ASSESSMENT" | jq -r '.contentFormat // empty')
    if [ -z "$FMT" ]; then
        echo "   ⚠️ Assessment '$ASSESSMENT_ID' not found"
        continue
    fi
    if [ "$FMT" = "markdown" ]; then
        echo "   ❌ Assessment '$ASSESSMENT_ID' has format 'markdown' (likely defaulted from unknown format)"
        exit 1
    fi
    HAS_WIDGETS=$(echo "$ASSESSMENT" | jq -r '.contentBody' | jq -e '.[0].content.widgets | length > 0' 2>/dev/null && echo "yes" || echo "no")
    if [ "$HAS_WIDGETS" = "yes" ]; then
        echo "   ✅ Assessment '$ASSESSMENT_ID': format=$FMT, widgets present"
    else
        echo "   ⚠️ Assessment '$ASSESSMENT_ID': format=$FMT, no widgets in content body"
    fi
done
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "✅ SEEDING VERIFIED"
echo "═══════════════════════════════════════════════════════════"
