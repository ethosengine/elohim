#!/bin/bash
# Seed Collectives — collective records for the population.
#
# Externalized verbatim from genesis/Jenkinsfile (Seed Collectives stage,
# genesis/seeder dir) to keep the pipeline's single CPS dispatch method under the
# 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
set -euo pipefail

DOORWAY_HOST="$1"

echo "═══════════════════════════════════════════════════════════"
echo "SEED COLLECTIVES"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
echo ""
DOORWAY_URL="${DOORWAY_HOST}" npx tsx src/seed-collectives.ts
