#!/usr/bin/env bash
# fleet-quiesce-gate.sh — bounded-retry gate that waits out the post-deploy
# fleet-restart churn window before any dataplane measurement is allowed to
# run. A deploy restarts the whole fleet; the reconcile sweep + gossip
# catch-up that follows can take from minutes to HOURS (precedent:
# 2026-07-19 doorway-catching-up incident; 2026-08-01 recording incident,
# divergentAnchor=1763, 2h+ catch-up). A measurement run fired mid-churn
# records a false red — this gate turns that into a bounded wait with an
# honest no-measure outcome instead.
#
# This is the bounded-retry wrapper AROUND the single-shot checks encoded by
# post-deploy-saga-probe.sh — it does not replace that probe (which proves
# the federation-failover saga's honesty fence, divergent>=1 included, as a
# one-shot post-deploy assertion). This gate answers a narrower question:
# "has the fleet stopped churning enough that a measurement run means
# anything?" — so it deliberately does NOT require divergent>=1 (see below).
#
# Usage: fleet-quiesce-gate.sh <doorway-a-url> <doorway-b-url> <content-id> <storage-a-url> <storage-b-url>
# All five positional args are REQUIRED. There is no defaulting of storage
# URLs to doorway URLs here — that defaulting is a documented bug class
# (doorway /metrics is NOT storage /metrics) that post-deploy-saga-probe.sh's
# 3-arg convenience form allows; this gate refuses to repeat it.
#
# Env knobs:
#   QUIESCE_DEADLINE_SECS   bound on total wait (default 2700 = 45min)
#   QUIESCE_POLL_SECS       seconds between polls (default 60)
#   QUIESCE_SUSTAIN_SECS    minimum separation between the two passing
#                           observations required to declare quiescence
#                           (default 330 — must exceed the 300s reconcile
#                           sweep cadence, so a fresh sweep is guaranteed to
#                           have run between them)
#
# A single PASS observation requires ALL of:
#   1. storage-A /p2p/status: pull.caughtUp === true
#   2. storage-B /p2p/status: pull.caughtUp === true
#      (missing/null/unreachable on either side is NOT a pass — keep
#      waiting; never treat a null/absent field as caught up)
#   3. storage-A /metrics: elohim_projection_reconcile_converged == 1,
#      matched by an EXACT metric-name boundary (name followed by whitespace
#      or '{'), never a bare prefix/startswith match — a startswith check
#      would also match e.g. elohim_projection_reconcile_converged_total.
#   4. doorway-A and doorway-B GET /db/content/<content-id> both return 200
#      (a 503 catching-up response is NOT a pass — keep waiting)
#
# Scope note: converged is required on storage-A ONLY. The saga's honesty
# fence binds "converged" to alpha-A (the genesis/primary peer this incident
# class is measured against); requiring it fleet-wide would over-gate B for
# a property the saga never asserts of B. B's legs here are "serving" (200
# on content) and "caught up" (pull.caughtUp) — not "converged".
#
# Scope note: this gate does NOT require divergent>=1. That honesty fence
# (proving the saga actually observed real divergence before healing it)
# belongs to the cure-proof scenarios in post-deploy-saga-probe.sh, not to
# a general quiescence gate — a fully-healed fleet with zero divergence
# outstanding must still be able to pass THIS gate, or every quiet fleet
# would wedge the pipeline forever.
#
# SUSTAINED requirement: quiescence is declared only once two PASSing
# observations are seen at least QUIESCE_SUSTAIN_SECS apart, AND storage-A's
# elohim_projection_reconcile_sweeps_total is STRICTLY GREATER at the second
# observation than at the first (proves a fresh sweep ran between them and
# still read converged — not just a stale gauge holding its last value). Any
# FAILing observation in between resets the sustain window; a new anchor
# is established on the next PASS.
#
# Exit codes:
#   0 — quiesced (prints "FLEET QUIESCENT")
#   1 — usage/config error
#   3 — deadline exceeded WITHOUT sustained quiescence (prints
#       "FLEET-CHURNING: ..." — this is a no-measure outcome, not a failure)
set -euo pipefail

usage() {
  echo "Usage: $(basename "$0") <doorway-a-url> <doorway-b-url> <content-id> <storage-a-url> <storage-b-url>" >&2
  echo "  All five arguments are required — storage URLs are never defaulted to doorway URLs." >&2
}

if [ "$#" -lt 5 ]; then
  usage
  exit 1
fi

A_DOORWAY="$1"
B_DOORWAY="$2"
CONTENT="${3:?content id required}"
A_STORAGE="$4"
B_STORAGE="$5"

for v in "$A_DOORWAY" "$B_DOORWAY" "$CONTENT" "$A_STORAGE" "$B_STORAGE"; do
  if [ -z "$v" ]; then
    usage
    exit 1
  fi
done

DEADLINE_SECS="${QUIESCE_DEADLINE_SECS:-2700}"
POLL_SECS="${QUIESCE_POLL_SECS:-60}"
SUSTAIN_SECS="${QUIESCE_SUSTAIN_SECS:-330}"
CURL_TIMEOUT="${QUIESCE_CURL_TIMEOUT_SECS:-20}"

log() {
  printf 'fleet-quiesce[%s]: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1"
}

ENC_CONTENT=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1], safe=""))' "$CONTENT")

start_ts=$(date +%s)
deadline_ts=$((start_ts + DEADLINE_SECS))

anchor_ts=""
anchor_sweeps=""

log "starting — deadline=${DEADLINE_SECS}s poll=${POLL_SECS}s sustain=${SUSTAIN_SECS}s content=${CONTENT}"

while :; do
  now=$(date +%s)
  if [ "$now" -ge "$deadline_ts" ]; then
    echo "FLEET-CHURNING: deadline ${DEADLINE_SECS}s exceeded — DID NOT MEASURE; this is a no-measure outcome, not a failure"
    exit 3
  fi

  # Fetch every leg tolerating individual curl failures — a single
  # unreachable endpoint must never kill the poll loop (or the deadline
  # check above would never get a chance to run out gracefully).
  status_a=$(curl -fsS --max-time "$CURL_TIMEOUT" "${A_STORAGE%/}/p2p/status" 2>/dev/null) || status_a=""
  status_b=$(curl -fsS --max-time "$CURL_TIMEOUT" "${B_STORAGE%/}/p2p/status" 2>/dev/null) || status_b=""
  metrics_a=$(curl -fsS --max-time "$CURL_TIMEOUT" "${A_STORAGE%/}/metrics" 2>/dev/null) || metrics_a=""
  code_a=$(curl -sS -o /dev/null -w '%{http_code}' --max-time "$CURL_TIMEOUT" "${A_DOORWAY%/}/db/content/${ENC_CONTENT}" 2>/dev/null) || code_a="000"
  code_b=$(curl -sS -o /dev/null -w '%{http_code}' --max-time "$CURL_TIMEOUT" "${B_DOORWAY%/}/db/content/${ENC_CONTENT}" 2>/dev/null) || code_b="000"
  [ -n "$code_a" ] || code_a="000"
  [ -n "$code_b" ] || code_b="000"

  parsed=$(STATUS_A="$status_a" STATUS_B="$status_b" METRICS_A="$metrics_a" python3 - <<'PYEOF'
import json, os, re

def caught_up(raw):
    if not raw:
        return False
    try:
        d = json.loads(raw)
    except Exception:
        return False
    p = d.get("pull")
    if not isinstance(p, dict):
        return False
    return p.get("caughtUp") is True

def metric_value(text, name):
    # Exact metric-name boundary — the char right after the name must be
    # whitespace (bare series) or '{' (labeled series). A startswith/prefix
    # check would also match e.g. "<name>_total" or "<name>_sum" lines and
    # silently read the wrong series.
    pat = re.compile(r'^' + re.escape(name) + r'(\s|\{)')
    best = None
    for line in text.splitlines():
        if not line or line.startswith('#'):
            continue
        if pat.match(line):
            m = re.search(r'(-?\d+(?:\.\d+)?)\s*$', line)
            if m:
                best = float(m.group(1))
    return best

a_caught_up = caught_up(os.environ.get("STATUS_A", ""))
b_caught_up = caught_up(os.environ.get("STATUS_B", ""))
metrics_a = os.environ.get("METRICS_A", "")
converged = metric_value(metrics_a, "elohim_projection_reconcile_converged")
sweeps = metric_value(metrics_a, "elohim_projection_reconcile_sweeps_total")
converged_ok = converged is not None and converged == 1

print(f"A_CAUGHT_UP={a_caught_up}")
print(f"B_CAUGHT_UP={b_caught_up}")
print(f"CONVERGED_OK={converged_ok}")
print(f"CONVERGED={converged}")
print(f"SWEEPS={sweeps}")
PYEOF
) || parsed=""

  a_caught_up=$(printf '%s\n' "$parsed" | sed -n 's/^A_CAUGHT_UP=//p')
  b_caught_up=$(printf '%s\n' "$parsed" | sed -n 's/^B_CAUGHT_UP=//p')
  converged_ok=$(printf '%s\n' "$parsed" | sed -n 's/^CONVERGED_OK=//p')
  converged=$(printf '%s\n' "$parsed" | sed -n 's/^CONVERGED=//p')
  sweeps=$(printf '%s\n' "$parsed" | sed -n 's/^SWEEPS=//p')

  pass=1
  reasons=()
  [ "$a_caught_up" = "True" ] || { pass=0; reasons+=("A-not-caughtUp"); }
  [ "$b_caught_up" = "True" ] || { pass=0; reasons+=("B-not-caughtUp"); }
  [ "$converged_ok" = "True" ] || { pass=0; reasons+=("A-not-converged(${converged:-null})"); }
  [ "$code_a" = "200" ] || { pass=0; reasons+=("A-content-${code_a}"); }
  [ "$code_b" = "200" ] || { pass=0; reasons+=("B-content-${code_b}"); }
  # The sustain proof needs a numeric sweeps_total reading even though the
  # metric itself is not one of the four PASS criteria above — fail closed
  # if it's missing rather than silently skipping the sustain check.
  if [ "$pass" -eq 1 ] && { [ -z "$sweeps" ] || [ "$sweeps" = "None" ]; }; then
    pass=0
    reasons+=("A-sweeps-metric-missing")
  fi

  summary="A-caughtUp=${a_caught_up:-?} B-caughtUp=${b_caught_up:-?} A-converged=${converged_ok:-?}(${converged:-null}) A-content=${code_a} B-content=${code_b} sweeps=${sweeps:-null}"

  if [ "$pass" -eq 1 ]; then
    if [ -z "$anchor_ts" ]; then
      anchor_ts="$now"
      anchor_sweeps="$sweeps"
      log "PASS ${summary} — anchor set (sustain window opened)"
    else
      elapsed=$((now - anchor_ts))
      if [ "$elapsed" -ge "$SUSTAIN_SECS" ]; then
        if python3 -c "import sys; sys.exit(0 if float(sys.argv[1]) > float(sys.argv[2]) else 1)" "$sweeps" "$anchor_sweeps" 2>/dev/null; then
          log "PASS ${summary} — sustained ${elapsed}s, sweeps advanced ${anchor_sweeps} -> ${sweeps}"
          echo "FLEET QUIESCENT"
          exit 0
        else
          log "PASS ${summary} — elapsed ${elapsed}s>=sustain but sweeps did not advance (anchor=${anchor_sweeps}); waiting for a fresh sweep"
        fi
      else
        log "PASS ${summary} — elapsed ${elapsed}s < sustain ${SUSTAIN_SECS}s, waiting"
      fi
    fi
  else
    if [ -n "$anchor_ts" ]; then
      log "FAIL ${summary} — resetting sustain window (${reasons[*]})"
    else
      log "FAIL ${summary} — (${reasons[*]})"
    fi
    anchor_ts=""
    anchor_sweeps=""
  fi

  sleep "$POLL_SECS"
done
