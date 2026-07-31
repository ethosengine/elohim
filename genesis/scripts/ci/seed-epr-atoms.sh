#!/bin/bash
# Seed EPR Atoms — the elohim-host-landing atom family.
#
# Mints five content-addressed EPR atoms (schema · governance state ·
# stewardship claim · custody claim · the landing atom itself) so the landing
# surface's nav-context resolves and its backing claims are REAL: the
# governance coupling leg points at a governance atom that exists, and the
# claims list holds the CIDs of claim atoms this run also stored.
#
# Wired here because the seeder existed but was reachable from NO pipeline
# stage and NO package script — the exact shape of the genesis #1079
# stewardship gap (a seeder nobody ran, asserted on by nobody).
#
# Externalized from genesis/Jenkinsfile to keep the pipeline's single CPS
# dispatch method under the 64KB MethodTooLargeException limit — see CLAUDE.md
# "Jenkinsfile Size Limit".
#
# Idempotent by construction: the atom CIDs are derived from the canonical
# bytes, so a re-run re-PUTs the same five CIDs. elohim-storage's
# LocalEprStore::put short-circuits when the CID exists with identical
# canonical bytes; it errors only on a genuine collision (same CID, different
# bytes), which a deterministic seed cannot produce.
#
# Args:
#   $1 = storage base URL (e.g. http://storage.elohim-alpha.svc:8090)
#        Falls back to $STORAGE_URL, then the doorway host.
# Credential (NOT in argv): DOORWAY_API_KEY comes via withEnv.
set -euo pipefail

# `${1:-…}` alone is not enough: Jenkins passes an EMPTY positional when neither
# STORAGE_URL nor RESOLVED_DOORWAY_HOST is set in the pipeline env, and an empty
# set argument defeats the `:-` default. Collapse in two steps.
TARGET_URL="${1:-}"
TARGET_URL="${TARGET_URL:-${STORAGE_URL:-${RESOLVED_DOORWAY_HOST:-http://localhost:8090}}}"

echo "═══════════════════════════════════════════════════════════"
echo "SEED EPR ATOMS (elohim-host-landing atom family)"
echo "═══════════════════════════════════════════════════════════"
echo "Target:  ${TARGET_URL}"
echo "Admin key: $( [ -n "${DOORWAY_API_KEY:-}" ] && echo provided || echo 'not set' )"
echo ""

STORAGE_URL="${TARGET_URL}" npx tsx src/seed-epr-atom.ts
