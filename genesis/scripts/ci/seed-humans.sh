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
# Seed only into a ready node: a fleet roll leaves storage `catching-up` for minutes
# (genesis #1625: 503 circuit-open on every write). Soft — never exits non-0.
bash "$(dirname "${BASH_SOURCE[0]}")/wait-storage-ready.sh" "${DOORWAY_HOST}"
DOORWAY_URL="${DOORWAY_HOST}" npx tsx src/seed-humans.ts
