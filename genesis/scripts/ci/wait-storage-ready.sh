#!/bin/bash
# Pre-seed readiness RE-gate (genesis #1182 Cluster A).
#
# The seed target passes "Verify Target Health" (verify-doorway-readiness.sh —
# conductor.connected + healthy), but RE-ENTERS the `catching-up` admission
# state after the Upload-Blob-Backed-Content + Seed-Custody stages backlog the
# projector. So the LATE Seed REA Commitments sub-steps (operator-bindings,
# stewardship, projections) hit `503 {"status":"catching-up","retryAfter":30}`
# and exhaust the seeder client's own 503-retry budget (doorway-client.ts
# ~775-827). This gate WAITS for the projector to catch up — checking the
# `p2p.caughtUp` + `projection.writer` fields the early gate does NOT — so those
# seeds fire into a ready node and their existing retries land.
#
# SOFT by design: a timeout WARNS and proceeds (the seeder's 503-retry is the
# backstop; aborting the stage is strictly worse than today). Never exits non-0.
# Degrades to healthy-only on older images whose /health lacks p2p.caughtUp.
#
# CPS note: bash lives here, not inline in genesis/Jenkinsfile (64KB CPS method
# limit — see CLAUDE.md "Jenkinsfile Size Limit").
#
# Arg: $1 = doorway base URL (external https://… or internal http://host:port) — the seed target.
set -uo pipefail

DOORWAY_HOST="${1:?doorway host required}"
MAX_ATTEMPTS="${READY_MAX_ATTEMPTS:-36}" # ~3 min at 5s
INTERVAL="${READY_INTERVAL_SECS:-5}"

command -v jq >/dev/null || { echo "⚠️  jq missing — skipping readiness re-gate"; exit 0; }

echo "⏳ pre-seed readiness re-gate: waiting for ${DOORWAY_HOST}/health to clear catching-up (caughtUp + projection.writer)…"
for i in $(seq 1 "$MAX_ATTEMPTS"); do
  BODY=$(curl -sf --max-time 8 "${DOORWAY_HOST}/health" 2>/dev/null || echo '{}')
  HEALTHY=$(echo "$BODY" | jq -r '.healthy // false' 2>/dev/null || echo false)
  CAUGHT=$(echo "$BODY" | jq -r '.p2p.caughtUp // empty' 2>/dev/null || echo '')
  WRITER=$(echo "$BODY" | jq -r '.projection.writer // empty' 2>/dev/null || echo '')

  if [ "$HEALTHY" = "true" ] && [ "$CAUGHT" = "true" ] && [ "$WRITER" = "true" ]; then
    echo "✅ storage caught up (healthy + caughtUp + projection.writer) after ~$(((i - 1) * INTERVAL))s — seeding can proceed"
    exit 0
  fi
  # Older image without the p2p.caughtUp field: cannot gate on it — degrade to
  # the early gate's healthy-only signal rather than block forever on a missing field.
  if [ "$HEALTHY" = "true" ] && [ -z "$CAUGHT" ]; then
    echo "⚠️  /health has no p2p.caughtUp field (older image?) — degrading to healthy-only; proceeding"
    exit 0
  fi
  echo "  catching-up: healthy=${HEALTHY} caughtUp=${CAUGHT:-null} writer=${WRITER:-null} (attempt ${i}/${MAX_ATTEMPTS})"
  sleep "$INTERVAL"
done

echo "⚠️  storage did not clear catching-up within $((MAX_ATTEMPTS * INTERVAL))s — proceeding anyway (seeder 503-retry is the backstop; genesis #1182 Cluster A)"
exit 0
