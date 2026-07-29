#!/bin/bash
# Seed Household Formation — canonical rung-3 ceremony driver.
#
# This gives operators the same entry point as the Genesis pipeline without
# requiring a full Genesis build or a raw `npx tsx` invocation. Run from
# genesis/seeder (the just target there does this):
#   CONDUCTOR_URLS='ws://peer-a:4445,ws://peer-b:4445' just seed-household-formation
#
# Args:
#   $1 = comma-separated conductor app WebSocket URLs
#
# Optional environment is deliberately inherited by the ceremony driver:
#   STORAGE_URL / DOORWAY_URL                  projection probe and settle wait
#   CONTENT_BLOB_HASH / CONTENT_BLOB_SIZE_BYTES custody ceremony layer
#   INSTALLED_APP_ID / HOUSEHOLD_SALT / HOUSEHOLD_NONCE_PREFIX
set -euo pipefail

CONDUCTOR_URLS="${1:?usage: seed-household-formation.sh <conductor-app-ws-urls>}"

echo "═══════════════════════════════════════════════════════════"
echo "SEED HOUSEHOLD FORMATION (rung-3 ceremony)"
echo "═══════════════════════════════════════════════════════════"
echo "Conductors: ${CONDUCTOR_URLS}"
echo "Projection: ${STORAGE_URL:-${DOORWAY_URL:-<unset — re-run convergence probe disabled>}}"
echo "Custody blob: ${CONTENT_BLOB_HASH:-<unset — custody layer will self-skip>}"
echo ""

CONDUCTOR_URLS="${CONDUCTOR_URLS}" npx tsx src/seed-household-formation.ts
