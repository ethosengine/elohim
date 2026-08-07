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
#      (missing/null/unreachable is NOT a pass — keep waiting; never treat
#      a null/absent field as caught up)
#   2. (telemetry only, NOT gating) storage-B pull.caughtUp is read and
#      printed but does not gate: B's pull queue empty-by-design reads
#      caughtUp=false (the all-retired/empty guard, observed 2026-07-31 and
#      live-confirmed constant-False through edge #1283's whole 45-min
#      window) — gating on it would wedge the pipeline forever, and the
#      saga asserts nothing about B's pull. B's gating legs are serving
#      (content 200) only.
#   3. storage-A /metrics: QUIESCED, not perfect (decision 2026-08-07, see
#      genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md):
#        elohim_projection_reconcile_converged_blocked_by{term="divergent_actionable"} == 0
#        AND ...{term="unmeasured"} == 0
#      i.e. the sweep MEASURED and no unadjudicated divergence is outstanding.
#      The pending/failed terms (the content-gap drain backlog) deliberately
#      do NOT gate: with a ~2.9k-gap plateau healing 0-1/sweep, converged==1
#      is unreachable inside any gate deadline — gating on it turned this
#      churn gate into an indefinite measurement embargo. converged itself
#      stays honest and is still read + printed as telemetry; the plateau
#      remains a named red draining under its own workstream. Both series are
#      matched with an EXACT metric-name boundary (name followed by
#      whitespace or '{'), never a bare prefix/startswith match. If the
#      blocked_by series is ABSENT (pre-honesty-floor image), the gate fails
#      closed and keeps waiting — absence of evidence is not quiescence.
#   4. doorway-A and doorway-B GET /db/content/<content-id> both return 200
#      (a 503 catching-up response is NOT a pass — keep waiting)
#
# Scope note: the quiesced predicate is required on storage-A ONLY. The
# saga's honesty fence binds convergence measurement to alpha-A (the
# genesis/primary peer this incident class is measured against); requiring
# it fleet-wide would over-gate B for a property the saga never asserts of
# B. B's legs here are "serving" (200 on content) and "caught up"
# (pull.caughtUp) — not "converged".
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

def labeled_metric_value(text, name, label, value):
    # Same exact-boundary discipline as metric_value, but for one labeled
    # series: `name{...label="value"...} <n>`. Returns None when the series
    # is absent — callers treat None as fail-closed (absence of the
    # honesty-floor gauge is not quiescence).
    pat = re.compile(
        r'^' + re.escape(name) + r'\{[^}]*'
        + re.escape(label) + r'="' + re.escape(value) + r'"[^}]*\}'
    )
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
# QUIESCED predicate (2026-08-07 decision — see header note 3): measured
# sweep + zero actionable divergence. pending/failed deliberately not gating.
BLOCKED_BY = "elohim_projection_reconcile_converged_blocked_by"
actionable = labeled_metric_value(metrics_a, BLOCKED_BY, "term", "divergent_actionable")
unmeasured = labeled_metric_value(metrics_a, BLOCKED_BY, "term", "unmeasured")
quiesced_ok = (
    actionable is not None and actionable == 0
    and unmeasured is not None and unmeasured == 0
)

print(f"A_CAUGHT_UP={a_caught_up}")
print(f"B_CAUGHT_UP={b_caught_up}")
print(f"QUIESCED_OK={quiesced_ok}")
print(f"ACTIONABLE={actionable}")
print(f"UNMEASURED={unmeasured}")
print(f"CONVERGED={converged}")
print(f"SWEEPS={sweeps}")
PYEOF
) || parsed=""

  a_caught_up=$(printf '%s\n' "$parsed" | sed -n 's/^A_CAUGHT_UP=//p')
  b_caught_up=$(printf '%s\n' "$parsed" | sed -n 's/^B_CAUGHT_UP=//p')
  quiesced_ok=$(printf '%s\n' "$parsed" | sed -n 's/^QUIESCED_OK=//p')
  actionable=$(printf '%s\n' "$parsed" | sed -n 's/^ACTIONABLE=//p')
  unmeasured=$(printf '%s\n' "$parsed" | sed -n 's/^UNMEASURED=//p')
  converged=$(printf '%s\n' "$parsed" | sed -n 's/^CONVERGED=//p')
  sweeps=$(printf '%s\n' "$parsed" | sed -n 's/^SWEEPS=//p')

  pass=1
  reasons=()
  [ "$a_caught_up" = "True" ] || { pass=0; reasons+=("A-not-caughtUp"); }
  # B-caughtUp deliberately NOT gating (empty-queue-by-design reads false;
  # see header note 2) — still printed in the summary for telemetry.
  # Quiesced, not perfect (header note 3): actionable==0 AND unmeasured==0,
  # fail-closed on an absent blocked_by series. converged is telemetry only.
  [ "$quiesced_ok" = "True" ] || { pass=0; reasons+=("A-not-quiesced(actionable=${actionable:-null},unmeasured=${unmeasured:-null})"); }
  [ "$code_a" = "200" ] || { pass=0; reasons+=("A-content-${code_a}"); }
  [ "$code_b" = "200" ] || { pass=0; reasons+=("B-content-${code_b}"); }
  # The sustain proof needs a numeric sweeps_total reading even though the
  # metric itself is not one of the four PASS criteria above — fail closed
  # if it's missing rather than silently skipping the sustain check.
  if [ "$pass" -eq 1 ] && { [ -z "$sweeps" ] || [ "$sweeps" = "None" ]; }; then
    pass=0
    reasons+=("A-sweeps-metric-missing")
  fi

  summary="A-caughtUp=${a_caught_up:-?} B-caughtUp=${b_caught_up:-?} A-quiesced=${quiesced_ok:-?}(actionable=${actionable:-null},unmeasured=${unmeasured:-null}) A-converged=${converged:-null} A-content=${code_a} B-content=${code_b} sweeps=${sweeps:-null}"

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
