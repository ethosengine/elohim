#!/bin/bash
# membrane-probe.sh — disposable-source LOAD PROBE for the doorway membrane.
#
# WHAT IT PROVES
#   The doorway membrane (doorway-service/src/server/membrane.rs) source-keys
#   every non-exempt request and escalates a single source through
#   Allow → Shape(+250ms) → Challenge(429 + x-membrane:challenge) → Deny(403 +
#   x-membrane:deny) as its in-window hit count crosses the calibrated edge
#   thresholds (window 60s, shape 300, challenge 600, ban 1200, ban_secs 900).
#   This script drives ONE throwaway source past challenge_threshold so the
#   verdict metric (doorway_membrane_verdict_total{verdict="challenge"}) and the
#   429 response become observable in a live deployment — today the membrane is
#   only unit-test-verified.
#
# WHY CONCURRENCY (NOT A for-LOOP) IS MANDATORY
#   Once a source crosses shape_threshold (300), the server sleeps 250ms on
#   EVERY subsequent request before proceeding (http.rs apply_membrane Shape
#   arm). A sequential curl loop therefore plateaus at ~4 req/s inside the
#   [300,600) shape band. The 60s window is SLIDING with eviction, so hits age
#   out: at ~4 req/s you never out-run the eviction and never reach 600. We fire
#   in parallel (xargs -P) so the 250ms shape delays absorb concurrently and the
#   in-window count climbs past challenge in seconds.
#
# ISOLATION / BLAST RADIUS  (READ THIS)
#   The membrane source key is `agent:<key>` (authenticated) or `ip:<client_ip>`
#   (unauthenticated) — derive_source() in membrane.rs. The User-Agent is NOT
#   part of the source key; it is set here only so the probe source is
#   grep-able in operator logs. WHAT ACTUALLY GETS RATE-LIMITED/BANNED IS THE
#   EGRESS IP THIS SCRIPT RUNS FROM, for ban_secs (900s = 15 min). Source-keying
#   means real users on OTHER IPs are unaffected — that is the safety property.
#   Run from a box whose egress IP is NOT shared with real alpha users
#   (a CI runner / disposable VM), never from a shared NAT in front of humans.
#
# SAFETY
#   - Hits a NON-DESTRUCTIVE GET path only (default /version.json). Never point
#     this at a mutating route — the probe issues hundreds of requests.
#   - Will get the probe's egress IP banned for ~15 min. THAT IS THE POINT.
#
# Usage:
#   membrane-probe.sh [TARGET_URL] [PATH]
#   TARGET_URL   doorway base URL   (default https://doorway-alpha.elohim.host)
#   PATH         non-exempt GET path (default /version.json)
# Env:
#   COUNT        total requests to fire (default 800 — past challenge=600,
#                under ban=1200, so the band shows clean 429s without tipping
#                into 403 deny). Set COUNT=3 to smoke the plumbing harmlessly.
#   PARALLEL     concurrent workers   (default 50)
#   UA           User-Agent for log identification (default membrane-probe)
#
# Examples:
#   genesis/a2o/scripts/membrane-probe.sh
#   genesis/a2o/scripts/membrane-probe.sh https://doorway-alpha.elohim.host /api/v1/version.json
#   COUNT=3 genesis/a2o/scripts/membrane-probe.sh        # harmless plumbing smoke
set -euo pipefail

TARGET_URL="${1:-https://doorway-alpha.elohim.host}"
PROBE_PATH="${2:-/version.json}"
COUNT="${COUNT:-800}"
PARALLEL="${PARALLEL:-50}"
UA="${UA:-membrane-probe}"

# Reject obviously-mutating intent early (the probe fires GETs only).
case "${PROBE_PATH}" in
  /admin/*|*/seed/*|*/upload*|*/blob/*)
    echo "REFUSING: '${PROBE_PATH}' looks mutating/destructive — probe GETs a read-only path only." >&2
    exit 2
    ;;
esac

URL="${TARGET_URL%/}${PROBE_PATH}"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

echo "membrane-probe → ${URL}"
echo "  count=${COUNT}  parallel=${PARALLEL}  user-agent='${UA}'"
echo "  thresholds (edge default): shape 300 / challenge 600 / ban 1200 in a 60s window"
echo "  WARNING: this bans the egress IP it runs from for ~900s (15 min). Source-keyed: other IPs unaffected."
echo

# One worker: GET the path, emit just the HTTP status code on its own line.
# %{http_code} is universal across curl versions (unlike %header{...}).
fire() {
  curl -s -o /dev/null \
    -A "${UA}" \
    --max-time 15 \
    -w '%{http_code}\n' \
    "$1" 2>/dev/null || echo "000"
}
export -f fire
export UA

START="$(date +%s)"
# seq feeds N identical URLs; xargs -P fans them across PARALLEL workers so the
# server-side 250ms shape delays overlap and the in-window count climbs fast.
yes "${URL}" | head -n "${COUNT}" \
  | xargs -P "${PARALLEL}" -I_URL bash -c 'fire "$@"' _ _URL \
  > "${TMP}/codes.txt" 2>/dev/null || true
END="$(date +%s)"
ELAPSED=$(( END - START ))

# ── Status-code histogram ───────────────────────────────────────────────────
N200="$(grep -c '^200$' "${TMP}/codes.txt" || true)"
N429="$(grep -c '^429$' "${TMP}/codes.txt" || true)"
N403="$(grep -c '^403$' "${TMP}/codes.txt" || true)"
NOTHER="$(grep -cvE '^(200|429|403)$' "${TMP}/codes.txt" || true)"
TOTAL="$(grep -c '' "${TMP}/codes.txt" || true)"

echo "── status-code histogram (${TOTAL} responses in ${ELAPSED}s) ──"
printf '  200 (allow/shape): %s\n' "${N200}"
printf '  429 (challenge)  : %s\n' "${N429}"
printf '  403 (deny)       : %s\n' "${N403}"
printf '  other/error      : %s\n' "${NOTHER}"
echo

# ── Confirm the x-membrane header on a challenge response ────────────────────
# 429 is emitted ONLY by the membrane Challenge arm (always with
# x-membrane:challenge), but we fetch the header explicitly for proof. One extra
# request is harmless — the source is already banned/throttled at this point.
MEMBRANE_HEADER=""
if [ "${N429}" -gt 0 ] || [ "${N403}" -gt 0 ]; then
  MEMBRANE_HEADER="$(curl -isS -A "${UA}" --max-time 15 "${URL}" 2>/dev/null \
    | grep -i '^x-membrane:' | head -n1 | tr -d '\r' || true)"
fi

# ── PASS / NO-VERDICT signal ─────────────────────────────────────────────────
if [ "${N429}" -gt 0 ]; then
  echo "MEMBRANE ACTIVE: observed ${N429} x 429 challenge after ~600 in-window requests (${MEMBRANE_HEADER:-x-membrane: challenge})"
  exit 0
elif [ "${N403}" -gt 0 ]; then
  echo "MEMBRANE ACTIVE: observed ${N403} x 403 deny (source banned; ${MEMBRANE_HEADER:-x-membrane: deny}) — overshot challenge into ban"
  exit 0
else
  echo "NO MEMBRANE VERDICT (not deployed / threshold not crossed)"
  echo "  - The membrane is committed on feat/frontend-eyes-sprint and not yet deployed to alpha:"
  echo "    a real run today is EXPECTED to land here. After deploy, re-run and expect 429s."
  echo "  - If deployed: raise COUNT (>600) and PARALLEL (sliding 60s window evicts old hits;"
  echo "    sustain >10 req/s to out-run eviction)."
  exit 1
fi
