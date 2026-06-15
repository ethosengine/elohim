#!/bin/bash
# Seed a single genesis peer via direct SQLite write.
#
# Externalized verbatim from genesis/Jenkinsfile (runContentSeedStage,
# genesis-peer loop, genesis/seeder dir) to keep the pipeline's single CPS
# dispatch method under the 64KB MethodTooLargeException limit — see
# CLAUDE.md "Jenkinsfile Size Limit".
#
# Args:
#   $1 = genesis peer humanId (for log lines)
#   $2 = genesis peer storage URL (host:port, no scheme)
set -euo pipefail

GP_HUMAN_ID="$1"
GP_STORAGE_URL="$2"

echo "═══════════════════════════════════════════════════════════"
echo "🌱 SEEDING GENESIS PEER: ${GP_HUMAN_ID} (${GP_STORAGE_URL})"
echo "═══════════════════════════════════════════════════════════"
# Direct seed via SQLite write; replica peers receive it through
# P2P identity-driven replication. Idempotent across genesis
# peers — each peer's SQLite settles to the same content set.
STORAGE_URL="http://${GP_STORAGE_URL}" npx tsx src/seed-sqlite.ts
echo "✅ ${GP_HUMAN_ID} seeded successfully"
