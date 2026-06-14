#!/usr/bin/env bash
# Seed Content-Provide Rows — EPR Resilience Card Workstream D
#
# Seeds rea_commitments rows (action='provide', state='active',
# resource_classified_as='content:<reach>') so the resilience snapshot's
# commitment_backed_collectives count is non-zero after a reseed.
#
# Required env (from genesis Jenkinsfile):
#   DOORWAY_URL          Target doorway (e.g. https://doorway-alpha.elohim.host)
#   PEER_STORAGE_URLS    Comma-separated name=host:port pairs for /auth/me probes
#                        (from peerStorageUrlsCsv() in Jenkinsfile)
#
# Optional:
#   DOORWAY_API_KEY      API key when doorway requires auth
#   PROVIDE_REACHES      Extra reaches beyond 'commons' (comma-separated)
#   PROVIDE_HUMAN_IDS    Override the default human set (comma-separated humanIds)
#
# Exit codes (propagated from seed-provide-rows.ts):
#   0 — all targeted humans seeded or already exist
#   2 — partial: at least one human seeded, at least one skipped/failed
#   1 — total failure: ZERO humans seeded (or pre-flight error)

set -euo pipefail

# DOORWAY_URL: prefer explicit override, fall back to Jenkins env
# RESOLVED_DOORWAY_HOST (set by Resolve Target Host stage in genesis/Jenkinsfile).
export DOORWAY_URL="${DOORWAY_URL:-${RESOLVED_DOORWAY_HOST:-http://localhost:8888}}"

echo "══════════════════════════════════════════════════════════════"
echo "SEED CONTENT-PROVIDE ROWS (EPR Resilience Card — Workstream D)"
echo "══════════════════════════════════════════════════════════════"
echo "Doorway:            ${DOORWAY_URL}"
echo "Peer storage URLs:  ${PEER_STORAGE_URLS:-<unset — using PEER_STORAGE_URL_TEMPLATE default>}"
echo "Reaches:            ${PROVIDE_REACHES:-commons (default)}"
echo "Humans:             ${PROVIDE_HUMAN_IDS:-<default alpha set>}"
echo ""

cd "$(dirname "$0")/../../seeder"

npx tsx src/seed-provide-rows.ts
