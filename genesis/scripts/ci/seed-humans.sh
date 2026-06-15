#!/bin/bash
# Seed Humans — human records for the population.
#
# Externalized verbatim from genesis/Jenkinsfile (Seed Humans stage,
# genesis/seeder dir) to keep the pipeline's single CPS dispatch method under the
# 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
# Credential (NOT in argv): API_KEY_ADMIN comes via withEnv.
set -euo pipefail

DOORWAY_HOST="$1"

echo "═══════════════════════════════════════════════════════════"
echo "SEED HUMANS"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
echo "Admin key: $( [ -n "${API_KEY_ADMIN:-}" ] && echo provided || echo 'not set' )"
echo ""
DOORWAY_URL="${DOORWAY_HOST}" npx tsx src/seed-humans.ts
