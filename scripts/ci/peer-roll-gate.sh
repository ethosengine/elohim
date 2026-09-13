#!/usr/bin/env bash
# peer-roll-gate.sh — hold the sequenced conductor roll on ONE peer until that
# peer has actually settled, then release the roll to the next peer.
#
# WHY (backlog/fleet-full-arc-conductor-saturation-and-coordinated-warmup-
# 2026-09-11.md, cure slice 4): the quiesce legs already existed as a
# MEASUREMENT of a finished deploy (scripts/ci/fleet-quiesce-gate.sh). This
# reuses the same read discipline as a DEPLOY PREDICATE, per peer, so the roll
# advances at the substrate's pace instead of the pipeline's.
#
# ── THE TWO LEGS ────────────────────────────────────────────────────────────
#
# 1. CONVERGENCE, from this peer's own `GET <storage>/p2p/status`:
#      projectionReconcile.converged === true
#    OR healedTotal ADVANCING while divergentAnchor FALLS, with sweeps
#    strictly advancing (proof a FRESH sweep produced the movement, not a
#    stale gauge holding its last value — the same discipline
#    fleet-quiesce-gate.sh applies to sweeps_total).
#
#    `caughtUp` IS DELIBERATELY NOT A LEG. The runtime's own doc says so
#    (elohim-storage src/p2p/projection_reconcile.rs: "True when this SWEEP
#    ended ... It is not a convergence signal"), and gaps abandoned at
#    MAX_RETRIES leave `pending` permanently, so caughtUp overstates. It is
#    read and PRINTED as telemetry, never gated on.
#
#    COUNTER-RESET NOTE: healedTotal/sweeps are per-PROCESS-LIFETIME counters.
#    The baseline is therefore taken from the FIRST successful poll after the
#    restart (a reading of the NEW process), never from a pre-roll value. If a
#    later poll reads LOWER than the baseline, the process restarted again
#    (crash-loop) — the baseline is re-anchored and said so, rather than
#    silently comparing across two process lifetimes.
#
#    An unreachable/absent/malformed projectionReconcile is NOT a pass. Absence
#    of evidence is not settlement (same fail-closed rule as the quiesce gate).
#
# 2. CFS THROTTLE, from Prometheus, on this peer's CONDUCTOR container only:
#      rate(container_cpu_cfs_throttled_periods_total{...}[5m])
#      / rate(container_cpu_cfs_periods_total{...}[5m])  <  ROLL_THROTTLE_MAX
#    Labels verified live 2026-09-11 and pinned by the repo alert rules
#    (genesis/orchestrator/manifests/infra/alpha-doorway-alerts.yaml, group
#    elohim-conductor-saturation): container="elohim-conductor" — the
#    elohim-node storage sidecar shares the pod and is deliberately excluded.
#
#    An empty result right after a restart (too few samples for a 5m rate) is
#    NOT a pass — it keeps waiting, which is the behaviour we want anyway.
#
#    DEGRADED MODE, stated plainly: if Prometheus itself is unreachable, the
#    throttle leg is switched OFF for this invocation with a loud banner and
#    the gate runs on the convergence leg alone. It is NOT allowed to silently
#    pass, and it is NOT allowed to wedge: a wrong ROLL_PROM_URL must degrade
#    a sequenced roll to a convergence-gated sequenced roll, never to a
#    budget-burning stall on every peer. Set ROLL_PROM_URL to the cluster's
#    kube-prometheus-stack query endpoint.
#
# ── DEADLINE / BUDGET ───────────────────────────────────────────────────────
#
# On deadline expiry this gate LOGS THE READING AND EXITS 3 — "continue to the
# next peer". A stalled peer must never abort the roll; it is recorded in the
# stage summary and Dataplane Validation judges the fleet afterwards. That is
# the museum's trap #1 discipline applied to a deploy predicate: a no-measure
# outcome is not a failure verdict.
#
# The effective per-peer deadline is FAIR-SHARE CLAMPED against a whole-phase
# budget carried in <state-file>:
#     effective = min(ROLL_PEER_DEADLINE_SECS,
#                     max(ROLL_PEER_MIN_DEADLINE_SECS, budget_left / gates_left))
# Without the clamp, 6 gates x 900s = 90 min of gate wall-clock would push the
# Deploy stage past the pipeline-global 120-min option — and an interrupt at
# that level cannot be caught by a stage's catchError (edge #1406-#1408: a
# HEALTHY deploy read as ABORTED). The clamp also stops the first two peers
# eating the entire budget and leaving the tail ungated.
#
# Usage:
#   peer-roll-gate.sh <peer> <storage-url> <namespace> <conductor-pod> \
#                     <gates-remaining> <state-file>
#
# Env knobs:
#   ROLL_PROM_URL               Prometheus query base (no trailing /api/v1)
#   ROLL_PEER_DEADLINE_SECS     per-peer ceiling (default 900)
#   ROLL_PEER_MIN_DEADLINE_SECS fair-share floor (default 120)
#   ROLL_SEQUENCE_DEADLINE_SECS whole-phase budget seeded on first call (2700)
#   ROLL_POLL_SECS              seconds between polls (default 30)
#   ROLL_THROTTLE_MAX           throttle ratio ceiling (default 0.9)
#   ROLL_THROTTLE_WINDOW        rate window (default 5m)
#   ROLL_CURL_TIMEOUT_SECS      per-request timeout (default 20)
#
# Exit codes:
#   0 — peer settled, roll released to the next peer
#   1 — usage error
#   3 — deadline reached without settlement; CONTINUE to the next peer
set -euo pipefail

usage() {
  echo "Usage: $(basename "$0") <peer> <storage-url> <namespace> <conductor-pod> <gates-remaining> <state-file>" >&2
}

if [ "$#" -lt 6 ]; then
  usage
  exit 1
fi

PEER="$1"
STORAGE_URL="${2%/}"
NAMESPACE="$3"
POD="$4"
GATES_REMAINING="$5"
STATE_FILE="$6"

for v in "$PEER" "$STORAGE_URL" "$NAMESPACE" "$POD" "$GATES_REMAINING" "$STATE_FILE"; do
  if [ -z "$v" ]; then usage; exit 1; fi
done

PROM_URL="${ROLL_PROM_URL:-http://kube-prom-stack-kube-prometheus-prometheus.observability.svc.cluster.local:9090}"
PROM_URL="${PROM_URL%/}"
PEER_DEADLINE="${ROLL_PEER_DEADLINE_SECS:-900}"
MIN_DEADLINE="${ROLL_PEER_MIN_DEADLINE_SECS:-120}"
SEQ_BUDGET="${ROLL_SEQUENCE_DEADLINE_SECS:-2700}"
POLL_SECS="${ROLL_POLL_SECS:-30}"
THROTTLE_MAX="${ROLL_THROTTLE_MAX:-0.9}"
WINDOW="${ROLL_THROTTLE_WINDOW:-5m}"
CURL_TIMEOUT="${ROLL_CURL_TIMEOUT_SECS:-20}"

SUMMARY_FILE="${STATE_FILE}.summary"

log() {
  printf 'peer-roll-gate[%s %s]: %s\n' "$PEER" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1"
}

# ── budget state ────────────────────────────────────────────────────────────
# Plain key=value read with sed, never `source` — the state file is ours but a
# sourced file is an arbitrary-code seam we do not need.
BUDGET_LEFT="$SEQ_BUDGET"
if [ -f "$STATE_FILE" ]; then
  from_file=$(sed -n 's/^BUDGET_LEFT=//p' "$STATE_FILE" | tail -n 1)
  if [ -n "$from_file" ]; then BUDGET_LEFT="$from_file"; fi
fi

# Non-numeric or <1 collapses to 1 — a bad gates-remaining must never reach
# awk and divide the budget by zero.
case "$GATES_REMAINING" in
  ''|*[!0-9]*) gates_left=1 ;;
  *) gates_left="$GATES_REMAINING"; [ "$gates_left" -ge 1 ] || gates_left=1 ;;
esac

case "$BUDGET_LEFT" in
  ''|*[!0-9]*) BUDGET_LEFT="$SEQ_BUDGET" ;;
esac

DEADLINE=$(awk -v budget="$BUDGET_LEFT" -v gates="$gates_left" \
               -v ceil="$PEER_DEADLINE" -v floor="$MIN_DEADLINE" 'BEGIN{
  share = budget / gates;
  if (share < floor) share = floor;
  if (share > ceil) share = ceil;
  if (share < 1) share = 1;
  printf "%d", share;
}')

log "gate opening — deadline=${DEADLINE}s (ceiling=${PEER_DEADLINE}s, budget-left=${BUDGET_LEFT}s over ${gates_left} remaining gate(s), floor=${MIN_DEADLINE}s) poll=${POLL_SECS}s throttle-max=${THROTTLE_MAX} window=${WINDOW}"

# ── Prometheus reachability probe (once) ────────────────────────────────────
THROTTLE_LEG="on"
if ! curl -fsS --max-time "$CURL_TIMEOUT" -G "${PROM_URL}/api/v1/query" \
      --data-urlencode 'query=up' >/dev/null 2>&1; then
  THROTTLE_LEG="degraded"
  log "DEGRADED — Prometheus at ${PROM_URL} is not answering; the CFS-throttle leg is OFF for this peer and the gate runs on the convergence leg alone. Set ROLL_PROM_URL to the cluster's kube-prometheus-stack query endpoint to restore it."
fi

THROTTLE_QUERY="rate(container_cpu_cfs_throttled_periods_total{namespace=\"${NAMESPACE}\",container=\"elohim-conductor\",pod=\"${POD}\"}[${WINDOW}]) / rate(container_cpu_cfs_periods_total{namespace=\"${NAMESPACE}\",container=\"elohim-conductor\",pod=\"${POD}\"}[${WINDOW}])"

start_ts=$(date +%s)
deadline_ts=$((start_ts + DEADLINE))

base_healed=""
base_divergent=""
base_sweeps=""

last_summary="no-observation"
outcome="DEADLINE"

while :; do
  now=$(date +%s)

  status_raw=$(curl -fsS --max-time "$CURL_TIMEOUT" "${STORAGE_URL}/p2p/status" 2>/dev/null) || status_raw=""

  throttle_raw=""
  if [ "$THROTTLE_LEG" = "on" ]; then
    throttle_raw=$(curl -fsS --max-time "$CURL_TIMEOUT" -G "${PROM_URL}/api/v1/query" \
                     --data-urlencode "query=${THROTTLE_QUERY}" 2>/dev/null) || throttle_raw=""
  fi

  parsed=$(STATUS="$status_raw" THROTTLE="$throttle_raw" python3 - <<'PYEOF'
import json, os

def emit(k, v):
    print("%s=%s" % (k, v))

raw = os.environ.get("STATUS", "")
doc = None
if raw:
    try:
        doc = json.loads(raw)
    except Exception:
        doc = None

pr = doc.get("projectionReconcile") if isinstance(doc, dict) else None
if not isinstance(pr, dict):
    emit("PR_OK", "0")
    emit("CONVERGED", "unknown")
    emit("CAUGHT_UP", "unknown")
    emit("HEALED", "")
    emit("DIVERGENT", "")
    emit("SWEEPS", "")
else:
    def as_int(key):
        v = pr.get(key)
        if isinstance(v, bool) or not isinstance(v, (int, float)):
            return None
        return int(v)

    healed = as_int("healedTotal")
    divergent = as_int("divergentAnchor")
    sweeps = as_int("sweeps")
    if healed is None or divergent is None or sweeps is None:
        # A projectionReconcile block missing its counters is malformed, not
        # settled — fail closed exactly like an absent block.
        emit("PR_OK", "0")
        emit("CONVERGED", "unknown")
        emit("CAUGHT_UP", "unknown")
        emit("HEALED", "")
        emit("DIVERGENT", "")
        emit("SWEEPS", "")
    else:
        emit("PR_OK", "1")
        emit("CONVERGED", "1" if pr.get("converged") is True else "0")
        emit("CAUGHT_UP", "1" if pr.get("caughtUp") is True else "0")
        emit("HEALED", healed)
        emit("DIVERGENT", divergent)
        emit("SWEEPS", sweeps)

traw = os.environ.get("THROTTLE", "")
ratio = None
if traw:
    try:
        tdoc = json.loads(traw)
        if tdoc.get("status") == "success":
            results = tdoc.get("data", {}).get("result", [])
            if results:
                val = float(results[0]["value"][1])
                # NaN (0/0 on a freshly restarted container) is not a reading.
                if val == val:
                    ratio = val
    except Exception:
        ratio = None

emit("RATIO", "" if ratio is None else "%.4f" % ratio)
PYEOF
) || parsed=""

  pr_ok=$(printf '%s\n' "$parsed" | sed -n 's/^PR_OK=//p')
  converged=$(printf '%s\n' "$parsed" | sed -n 's/^CONVERGED=//p')
  caught_up=$(printf '%s\n' "$parsed" | sed -n 's/^CAUGHT_UP=//p')
  healed=$(printf '%s\n' "$parsed" | sed -n 's/^HEALED=//p')
  divergent=$(printf '%s\n' "$parsed" | sed -n 's/^DIVERGENT=//p')
  sweeps=$(printf '%s\n' "$parsed" | sed -n 's/^SWEEPS=//p')
  ratio=$(printf '%s\n' "$parsed" | sed -n 's/^RATIO=//p')

  converge_ok=0
  converge_why="no-status"
  if [ "$pr_ok" = "1" ]; then
    if [ "$converged" = "1" ]; then
      converge_ok=1
      converge_why="converged"
    else
      if [ -z "$base_healed" ]; then
        base_healed="$healed"; base_divergent="$divergent"; base_sweeps="$sweeps"
        converge_why="baseline-anchored(healed=${healed},divergent=${divergent},sweeps=${sweeps})"
      elif [ "$healed" -lt "$base_healed" ] || [ "$sweeps" -lt "$base_sweeps" ]; then
        # Counters went backwards: the peer's process restarted under us. Two
        # process lifetimes are not comparable — re-anchor, never subtract.
        base_healed="$healed"; base_divergent="$divergent"; base_sweeps="$sweeps"
        converge_why="counter-reset-reanchored(healed=${healed},divergent=${divergent},sweeps=${sweeps})"
      elif [ "$sweeps" -gt "$base_sweeps" ] && [ "$healed" -gt "$base_healed" ] && [ "$divergent" -lt "$base_divergent" ]; then
        converge_ok=1
        converge_why="healing(healed ${base_healed}->${healed}, divergentAnchor ${base_divergent}->${divergent}, sweeps ${base_sweeps}->${sweeps})"
      else
        converge_why="no-movement(healed ${base_healed}->${healed}, divergentAnchor ${base_divergent}->${divergent}, sweeps ${base_sweeps}->${sweeps})"
      fi
    fi
  fi

  throttle_ok=0
  throttle_why="no-series"
  if [ "$THROTTLE_LEG" = "degraded" ]; then
    throttle_ok=1
    throttle_why="leg-degraded(prometheus-unreachable)"
  elif [ -n "$ratio" ]; then
    if awk -v r="$ratio" -v m="$THROTTLE_MAX" 'BEGIN{exit !(r < m)}'; then
      throttle_ok=1
      throttle_why="ratio=${ratio}<${THROTTLE_MAX}"
    else
      throttle_why="ratio=${ratio}>=${THROTTLE_MAX}"
    fi
  fi

  elapsed=$((now - start_ts))
  last_summary="converge=${converge_why} throttle=${throttle_why} caughtUp=${caught_up:-?}(telemetry-only)"

  if [ "$converge_ok" -eq 1 ] && [ "$throttle_ok" -eq 1 ]; then
    outcome="RELEASED"
    log "RELEASED after ${elapsed}s — ${last_summary}"
    break
  fi

  log "holding (${elapsed}s/${DEADLINE}s) — ${last_summary}"

  if [ "$now" -ge "$deadline_ts" ]; then
    outcome="DEADLINE"
    log "DEADLINE ${DEADLINE}s reached — ${last_summary} — CONTINUING to the next peer. This peer is recorded as unsettled in the roll summary; Dataplane Validation's fleet-quiesce gate judges the fleet. A stalled peer does not abort the roll."
    break
  fi

  sleep "$POLL_SECS"
done

end_ts=$(date +%s)
spent=$((end_ts - start_ts))
new_budget=$(awk -v b="$BUDGET_LEFT" -v s="$spent" 'BEGIN{ v=b-s; if (v<0) v=0; printf "%d", v }')
printf 'BUDGET_LEFT=%s\n' "$new_budget" > "$STATE_FILE"
printf '%s|%s|%ss|%s|throttle-leg=%s\n' "$PEER" "$outcome" "$spent" "$last_summary" "$THROTTLE_LEG" >> "$SUMMARY_FILE"

log "budget: spent ${spent}s, ${new_budget}s left for the remaining gates"

if [ "$outcome" = "RELEASED" ]; then
  exit 0
fi
exit 3
