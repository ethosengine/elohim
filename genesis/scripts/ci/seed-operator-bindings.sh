#!/bin/bash
# Seed Operator Bindings — REA operate-doorway commitments for the operator.
#
# Externalized verbatim from genesis/Jenkinsfile (Seed Operator Bindings stage,
# genesis/seeder dir) to keep the pipeline's single CPS dispatch method under the
# 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
set -euo pipefail

DOORWAY_HOST="$1"

echo "═══════════════════════════════════════════════════════════"
echo "SEED OPERATOR BINDINGS (M5 matthew → alpha-elohim-host + apex-elohim-host)"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
echo ""
DOORWAY_URL="${DOORWAY_HOST}" \
  npx tsx src/seed-operator-bindings.ts
