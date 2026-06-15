#!/bin/bash
# Seed Custody Commitments — default custody pair set for a content blob.
#
# Externalized verbatim from genesis/Jenkinsfile (Seed Custody Commitments stage,
# genesis/seeder dir) to keep the pipeline's single CPS dispatch method under the
# 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = doorway host base URL (e.g. https://doorway-alpha.elohim.host)
#   $2 = content blob hash
#   $3 = content blob size in bytes
set -euo pipefail

DOORWAY_HOST="$1"
BLOB_HASH="$2"
BLOB_SIZE="$3"

echo "═══════════════════════════════════════════════════════════"
echo "SEED CUSTODY COMMITMENTS (default custody pair set — seeder prints the actual pairs)"
echo "═══════════════════════════════════════════════════════════"
echo "Doorway: ${DOORWAY_HOST}"
echo "Blob:    ${BLOB_HASH} (${BLOB_SIZE} bytes)"
echo ""
DOORWAY_URL="${DOORWAY_HOST}" \
CONTENT_BLOB_HASH="${BLOB_HASH}" \
CONTENT_BLOB_SIZE_BYTES="${BLOB_SIZE}" \
  npx tsx src/seed-commitments.ts
