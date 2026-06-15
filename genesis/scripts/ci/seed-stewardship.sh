#!/bin/bash
# Seed Stewardship Allocations — affinity-based content→steward allocations.
#
# Externalized verbatim from genesis/Jenkinsfile (seedStewardshipStage,
# genesis/seeder dir) to keep the pipeline's single CPS dispatch method under the
# 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
# Credential (NOT in argv): DOORWAY_API_KEY comes via withEnv.
set -euo pipefail

DOORWAY_HOST="$1"

echo "═══════════════════════════════════════════════════════════"
echo "SEED STEWARDSHIP ALLOCATIONS (affinity-based content→steward)"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
echo "Admin key: $( [ -n "${DOORWAY_API_KEY:-}" ] && echo provided || echo 'not set' )"
echo ""
DOORWAY_URL="${DOORWAY_HOST}" npx tsx src/seed-stewardship.ts
