#!/bin/bash
# Seed Accounts — /account/import across active compute peers.
#
# Externalized verbatim from genesis/Jenkinsfile (Seed Accounts stage,
# genesis/seeder dir) to keep the pipeline's single CPS dispatch method under the
# 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
#   $2 = comma-separated target peer storage URLs (may be empty)
set -euo pipefail

DOORWAY_HOST="$1"
TARGET_PEERS="$2"

echo "═══════════════════════════════════════════════════════════"
echo "SEED ACCOUNTS"
echo "═══════════════════════════════════════════════════════════"
if [ -n "${TARGET_PEERS}" ]; then
  echo "Targets: ${TARGET_PEERS}"
else
  echo "Doorway: ${DOORWAY_HOST}  (single target; set SEEDER_TARGET_PEERS to split)"
fi
echo ""
DOORWAY_URL="${DOORWAY_HOST}" \
SEEDER_TARGET_PEERS="${TARGET_PEERS}" \
  npx tsx src/seed-accounts.ts
