#!/bin/bash
# Seed Projections — EPR project-epr commitments for the doorway router.
#
# Externalized verbatim from genesis/Jenkinsfile (seedProjectionsStage,
# genesis/seeder dir) to keep the pipeline's single CPS dispatch method under the
# 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
# Credential (NOT in argv): DOORWAY_API_KEY comes via withEnv.
#
# The doorway host only names the scope. Commitment WRITES go to the steward's
# own storage peer (storageUrlForHuman, PEER_STORAGE_URLS-aware) so each
# deterministic project-epr id is rooted by exactly one cell — posting through a
# load-balanced doorway name double-rooted project-epr-98f0d59051751497.
set -euo pipefail

DOORWAY_HOST="$1"

echo "═══════════════════════════════════════════════════════════"
echo "SEED PROJECTIONS (EPR project-epr commitments)"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
echo "Admin key: $( [ -n "${DOORWAY_API_KEY:-}" ] && echo provided || echo 'not set' )"
echo ""
DOORWAY_URL="${DOORWAY_HOST}" npx tsx src/seed-projections.ts
