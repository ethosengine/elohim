#!/usr/bin/env bash
# pre-roll-reading.sh — record the divergence a peer carries INTO a deploy, so
# scripts/ci/peer-roll-gate.sh can release it as settled-at-baseline instead of
# holding the roll on a standing divergence that is not the roll's debt
# (edge #1486: four of six peers burned 450-599s on `no-movement`).
#
# Called by the edge Jenkinsfile (recordPreRollReading) once per peer, BEFORE
# the deploy touches it (storage apply/restart, then the sequenced conductor
# roll). One read of `GET <storage>/p2p/status`; on a usable
# projectionReconcile block it APPENDS two lines to <state-file>:
#     PRE_DIVERGENT_<peer>=<divergentAnchor>
#     PRE_HEALED_<peer>=<healedTotal>
# Absent, unreachable or malformed → writes NOTHING and says so; the gate then
# keeps its converged/healing legs only. Never fails a deploy: exit 0 on every
# reading outcome, exit 1 only on a usage error.
#
# Runs in the deploy container: bash + coreutils ONLY (scripts/ci/.epr-meta).
# projectionReconcile is a flat object (ProjectionReconcileStatus in
# elohim-storage src/p2p/projection_reconcile.rs), so grep/sed read it.
#
# Usage: pre-roll-reading.sh <peer> <storage-url> <state-file>
# Env:   PRE_ROLL_CURL_TIMEOUT_SECS (default 20)
set -uo pipefail

if [ "$#" -ne 3 ] || [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ]; then
  echo "Usage: $(basename "$0") <peer> <storage-url> <state-file>" >&2
  exit 1
fi
PEER="$1"; STORAGE_URL="${2%/}"; STATE_FILE="$3"

raw=$(curl -fsS --max-time "${PRE_ROLL_CURL_TIMEOUT_SECS:-20}" "${STORAGE_URL}/p2p/status" 2>/dev/null) || raw=""
# The projectionReconcile object only: from its opening brace to its closing one.
block=$(printf '%s' "$raw" | tr -d '\n' | grep -o '"projectionReconcile"[[:space:]]*:[[:space:]]*{[^}]*}' | head -n 1)
field() { printf '%s' "$block" | grep -o "\"$1\"[[:space:]]*:[[:space:]]*[0-9][0-9]*" | head -n 1 | sed 's/.*:[[:space:]]*//'; }
divergent=$(field divergentAnchor)
healed=$(field healedTotal)
# An unmeasured sweep (db unavailable) publishes divergentAnchor=0 with
# peersAsked=0 — that is no reading, not a zero baseline.
peers_asked=$(field peersAsked)
if [ "${peers_asked:-0}" = "0" ]; then divergent=""; fi

if [ -z "$divergent" ] || [ -z "$healed" ]; then
  echo "pre-roll-reading[${PEER}]: no usable projectionReconcile at ${STORAGE_URL}/p2p/status (absent, unreachable or malformed) — writing nothing; the roll gate keeps its converged/healing legs only for this peer"
  exit 0
fi
# One printf = one small O_APPEND write: parallel storage branches share the file.
printf 'PRE_DIVERGENT_%s=%s\nPRE_HEALED_%s=%s\n' "$PEER" "$divergent" "$PEER" "$healed" >> "$STATE_FILE"
echo "pre-roll-reading[${PEER}]: divergentAnchor=${divergent} healedTotal=${healed} recorded in ${STATE_FILE}"
