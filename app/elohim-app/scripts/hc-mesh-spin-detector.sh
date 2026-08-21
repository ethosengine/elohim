#!/bin/bash
#
# hc-mesh-spin-detector.sh — measure a conductor spin-loop, on ANY mesh.
#
# The alpha fleet's conductors sit pegged at their CPU quota (whatever the
# quota is: 2, 3, 4 or 8 cores) while doing no useful work. The mechanism —
# measured 2026-08-21, genesis/data/timeline/backlog/alpha-conductor-sys-
# validation-spin-unfetchable-deps.md — is sys-validation waking every 10s,
# attempting to fetch a permanently-unfetchable dependency set, saturating the
# DHT read pool on every attempt, logging ~1000 lines/s, and sleeping again.
#
# This script is the MEASURE for that class, so it can be reproduced at a desk
# and proven fixed. It reports, over N cycles of W seconds:
#
#   - per-conductor CPU in CORES, from /proc/<pid>/stat utime+stime deltas
#     (a spin is a constant amplitude: flat at the ceiling, cycle after cycle)
#   - conductor log-rate spectroscopy in lines/s, TOTAL and by message class:
#       saturated   "Database read connection is saturated"  (read-pool oversubscription)
#       nopeers     "No peers to fetch record" / NoPeersForLocation
#       sysval      "Sys validation sleeping for Ns, with F fetched of N missing dependencies"
#       other       everything else
#   - the latest missing-dependency count, per DNA where the line carries a
#     DNA hash, and the fetched-count (0 fetched, forever, is the tell)
#   - a VERDICT line: SPIN when the missing-dependency count is > 0 and NOT
#     decreasing across >= 3 cycles AND saturated lines/s is over threshold;
#     QUIET otherwise.
#
#   - per-conductor THREAD COUNT, and the hottest threads by name. `perf` is not
#     installable in this container (no perf/kernel-tools package in any
#     configured repo, checked 2026-08-21), so this is the profiler substitute:
#     /proc/<pid>/task/<tid>/{stat,comm} gives per-thread CPU and thread names,
#     which is enough to say WHICH worker is burning the core even though it
#     cannot say which function inside it.
#   - each storage peer's own convergence state, read-only, from GET /p2p/status:
#     connected peers, whether projection-reconcile reports itself caught up and
#     converged, and its divergent-anchor count. A conductor spinning while its
#     storage peer insists it is caught up is a different (and worse) finding
#     than one spinning while its peer openly reports divergence.
#
# A one-line JSON summary is printed last (and optionally written to a file)
# so an a2o step can assert on the verdict without parsing the human report.
#
# LOG ATTRIBUTION, honestly: `hc sandbox run -a` multiplexes every conductor's
# stdout into ONE file (elohim/holochain/local-dev/.sandbox_run_log) with no
# per-conductor prefix, so log rates from that file are MESH-WIDE, not
# per-conductor — the report labels them as such. Where per-conductor logs do
# exist (alpha: one file/stream per pod), pass them with repeated
# `--log <name>=<path>` and every rate is attributed to that name. CPU is
# always per-conductor, because /proc is.
#
# USAGE:
#   ./hc-mesh-spin-detector.sh [options]
#
#     --cycles N        measurement cycles (default 3; >=3 is required for a
#                       non-decreasing-trend verdict)
#     --window SECS     seconds per cycle (default 20)
#     --minutes M       convenience: run for ~M minutes at the current window
#                       (overrides --cycles)
#     --log NAME=PATH   a conductor log stream to watch; repeatable. Default:
#                       mesh=<local-dev>/.sandbox_run_log
#     --pid NAME=PID    a conductor process to watch; repeatable. Default:
#                       auto-discovered from `holochain --config-path .../<name>/...`
#     --sat-threshold R saturated lines/s above which the read pool counts as
#                       oversubscribed (default 5.0; local-mesh baseline is
#                       ~0.26 lines/s TOTAL, alpha is 600-950/s of THIS class)
#     --cpu-threshold C cores per conductor above which CPU is flagged hot in
#                       the report (default 0.8; informational, not the verdict)
#     --storage N=URL   a storage peer to probe read-only for convergence state;
#                       repeatable. Default: derived from the discovered
#                       conductor names on the standard mesh port scheme
#                       (matthew :8090, jessica :8091, james :8092).
#     --no-storage      skip the storage probes entirely
#     --top-threads N   report the N hottest THREADS per conductor (default 3)
#     --json-out PATH   also write the one-line JSON summary here
#     --quiet           suppress per-cycle rows; print the summary + verdict only
#
# EXIT: 0 when the verdict is QUIET, 1 when it is SPIN, 2 on a usage/probe
# error. (A detector that exits 0 on a red measure cannot gate anything.)
#
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
LOCAL_DEV_DIR="${LOCAL_DEV_DIR:-$REPO_ROOT/elohim/holochain/local-dev}"

CYCLES=3
WINDOW=20
MINUTES=""
SAT_THRESHOLD="${SPIN_SAT_THRESHOLD:-5.0}"
CPU_THRESHOLD="${SPIN_CPU_THRESHOLD:-0.8}"
JSON_OUT=""
QUIET=0
PROBE_STORAGE=1
TOP_THREADS="${SPIN_TOP_THREADS:-3}"
declare -a LOG_NAMES=() LOG_PATHS=() PID_NAMES=() PID_VALUES=()
declare -a STORAGE_NAMES=() STORAGE_URLS=()

while [ $# -gt 0 ]; do
  case "$1" in
    --cycles)        CYCLES="$2"; shift 2 ;;
    --window)        WINDOW="$2"; shift 2 ;;
    --minutes)       MINUTES="$2"; shift 2 ;;
    --log)           LOG_NAMES+=("${2%%=*}"); LOG_PATHS+=("${2#*=}"); shift 2 ;;
    --pid)           PID_NAMES+=("${2%%=*}"); PID_VALUES+=("${2#*=}"); shift 2 ;;
    --sat-threshold) SAT_THRESHOLD="$2"; shift 2 ;;
    --cpu-threshold) CPU_THRESHOLD="$2"; shift 2 ;;
    --json-out)      JSON_OUT="$2"; shift 2 ;;
    --storage)       STORAGE_NAMES+=("${2%%=*}"); STORAGE_URLS+=("${2#*=}"); shift 2 ;;
    --no-storage)    PROBE_STORAGE=0; shift ;;
    --top-threads)   TOP_THREADS="$2"; shift 2 ;;
    --quiet)         QUIET=1; shift ;;
    -h|--help)       awk 'NR==1{next} /^#/{sub(/^# ?/,""); print; next} {exit}' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "unknown option: $1 (try --help)" >&2; exit 2 ;;
  esac
done

if [ -n "$MINUTES" ]; then
  CYCLES=$(awk -v m="$MINUTES" -v w="$WINDOW" 'BEGIN{c=int((m*60)/w); print (c<1?1:c)}')
fi

CLK_TCK="$(getconf CLK_TCK 2>/dev/null || echo 100)"

# --- conductor discovery -----------------------------------------------------
# Match the exact holochain process identity, never a loose pattern (the
# self-match trap the mesh scripts document): the conductor's own argv carries
# `--config-path <root>/<peer>/conductor-config.yaml`, which names the peer.
if [ ${#PID_NAMES[@]} -eq 0 ]; then
  while read -r pid args; do
    [ -n "$pid" ] || continue
    name="$(echo "$args" | grep -oE '/[^/ ]+/conductor-config\.yaml' | head -1 | cut -d/ -f2)"
    [ -n "$name" ] || name="pid$pid"
    PID_NAMES+=("$name"); PID_VALUES+=("$pid")
  done < <(ps -eo pid=,args= | grep -E '[h]olochain .*--config-path' | awk '{pid=$1; $1=""; print pid, $0}')
fi

if [ ${#LOG_NAMES[@]} -eq 0 ]; then
  LOG_NAMES+=("mesh"); LOG_PATHS+=("$LOCAL_DEV_DIR/.sandbox_run_log")
fi

# Storage peers default to the mesh's standard port scheme, derived from the
# conductor names we just discovered — so the common case needs no flags, and an
# unusual topology stays expressible with --storage.
if [ "$PROBE_STORAGE" -eq 1 ] && [ ${#STORAGE_NAMES[@]} -eq 0 ]; then
  IFS=',' read -ra _MESH_PEERS <<< "${MESH_PEERS:-matthew,jessica,james}"
  for i in "${!_MESH_PEERS[@]}"; do
    for pn in "${PID_NAMES[@]}"; do
      if [ "$pn" = "${_MESH_PEERS[$i]}" ]; then
        STORAGE_NAMES+=("$pn"); STORAGE_URLS+=("http://localhost:$((8090 + i))")
      fi
    done
  done
fi

# Read-only convergence probe. Never writes, never retries hard, and treats an
# unreachable peer as a REPORTED fact rather than a failure — a peer that cannot
# answer during a spin measurement is itself a finding.
storage_probe() { # <url> -> "connectedPeers caughtUp converged divergentAnchor pullCaughtUp"
  curl -s -m 5 "$1/p2p/status" 2>/dev/null | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    print("- - - - -"); raise SystemExit(0)
pr = d.get("projectionReconcile") or {}
pull = d.get("pull") or {}
def s(v):
    return "-" if v is None else str(v).lower()
print(s(d.get("connectedPeers")), s(pr.get("caughtUp")), s(pr.get("converged")),
      s(pr.get("divergentAnchor")), s(pull.get("caughtUp")))
' 2>/dev/null || echo "- - - - -"
}

if [ ${#PID_NAMES[@]} -eq 0 ]; then
  echo "no conductor processes found (looked for: holochain --config-path ...). Is the mesh up?" >&2
  echo "  hint: pass them explicitly with --pid <name>=<pid>" >&2
  exit 2
fi

for i in "${!LOG_PATHS[@]}"; do
  [ -f "${LOG_PATHS[$i]}" ] || { echo "log stream '${LOG_NAMES[$i]}' not found at ${LOG_PATHS[$i]}" >&2; exit 2; }
done

# --- classification ----------------------------------------------------------
# Reads the bytes appended to a log since the last offset and emits ONE line of
# counters. Strips ANSI (holochain colorizes its structured output).
#   total sat nopeers sysval other missing fetched dna_pairs...
classify() { # <path> <from_offset> <to_offset>
  local path="$1" from="$2" to="$3" len=$((3 > 2 ? $3 - $2 : 0))
  if [ "$len" -le 0 ]; then echo "0 0 0 0 0 - - "; return; fi
  tail -c "+$((from + 1))" "$path" 2>/dev/null | head -c "$len" | awk '
    BEGIN { total=0; sat=0; nop=0; sys=0; oth=0; miss="-"; fetch="-" }
    {
      line=$0
      gsub(/\033\[[0-9;]*m/, "", line)
      total++
      if (line ~ /read connection is saturated/) { sat++ }
      else if (line ~ /No peers to fetch record|NoPeersForLocation/) { nop++ }
      else if (line ~ /Sys validation sleeping/) {
        sys++
        # "Sys validation sleeping for 10s, with 0 fetched of 873 missing dependencies"
        if (match(line, /with [0-9]+ fetched of [0-9]+ missing/)) {
          seg = substr(line, RSTART, RLENGTH)
          split(seg, w, " ")
          fetch = w[2]; miss = w[5]
          dna = "-"
          if (match(line, /uhC0k[A-Za-z0-9_+\/=-]+/)) dna = substr(line, RSTART, RLENGTH)
          per[dna] = miss
        }
      }
      else { oth++ }
    }
    END {
      printf "%d %d %d %d %d %s %s", total, sat, nop, sys, oth, miss, fetch
      for (d in per) printf " %s=%s", d, per[d]
      printf "\n"
    }'
}

thread_count() { ls "/proc/$1/task" 2>/dev/null | wc -l; }

# Per-thread CPU accounting — the stand-in for a profiler we cannot install.
# Emits "<tid> <ticks> <comm>" per line for every thread of <pid>. The comm goes
# LAST because thread names contain spaces: holochain names eight of its threads
# "RTC worker", which with comm in the middle shifts the tick count into the
# name field, and `set -u` then evaluates "worker" as an unbound variable inside
# the arithmetic. Trailing-field-absorbs-the-rest is the only safe layout.
thread_ticks() { # <pid>
  local d
  for d in /proc/"$1"/task/*; do
    [ -d "$d" ] || continue
    local tid comm stat
    tid="${d##*/}"
    comm="$(cat "$d/comm" 2>/dev/null)" || continue
    stat="$(cat "$d/stat" 2>/dev/null)" || continue
    echo "$tid $(echo "$stat" | awk '{ rest = substr($0, index($0, ")") + 2); n = split(rest, f, " "); print f[12] + f[13] }') ${comm:-unknown}"
  done
}

cpu_ticks() { # <pid> -> utime+stime, or "" when the process is gone
  local stat
  stat="$(cat "/proc/$1/stat" 2>/dev/null)" || return 1
  # after the (comm) field, which may contain spaces: fields 14,15 are utime,stime
  echo "$stat" | awk '{ rest = substr($0, index($0, ")") + 2); n = split(rest, f, " "); print f[12] + f[13] }'
}

# --- baseline snapshot -------------------------------------------------------
declare -A PREV_TICKS PREV_OFF
declare -A CPU_SUM CPU_MAX CPU_LAST
declare -A THREADS_LAST THREADS_MAX
declare -A PREV_TT SUM_TT TT_COMM
declare -A STORE_BEFORE STORE_AFTER
declare -A TOT_SUM SAT_SUM NOP_SUM SYS_SUM
declare -A MISS_SERIES MISS_LAST FETCH_LAST DNA_LAST

for i in "${!PID_NAMES[@]}"; do
  n="${PID_NAMES[$i]}"; pid="${PID_VALUES[$i]}"
  t="$(cpu_ticks "$pid")" || t=""
  PREV_TICKS["$n"]="${t:-0}"
  CPU_SUM["$n"]=0; CPU_MAX["$n"]=0
  THREADS_LAST["$n"]="$(thread_count "$pid")"; THREADS_MAX["$n"]="${THREADS_LAST[$n]}"
  while read -r tid ticks comm; do
    [ -n "$tid" ] || continue
    PREV_TT["$n|$tid"]="$ticks"; SUM_TT["$n|$tid"]=0; TT_COMM["$n|$tid"]="$comm"
  done < <(thread_ticks "$pid")
done

for i in "${!STORAGE_NAMES[@]}"; do
  STORE_BEFORE["${STORAGE_NAMES[$i]}"]="$(storage_probe "${STORAGE_URLS[$i]}")"
done
for i in "${!LOG_NAMES[@]}"; do
  n="${LOG_NAMES[$i]}"
  PREV_OFF["$n"]="$(stat -c %s "${LOG_PATHS[$i]}" 2>/dev/null || echo 0)"
  TOT_SUM["$n"]=0; SAT_SUM["$n"]=0; NOP_SUM["$n"]=0; SYS_SUM["$n"]=0
  MISS_SERIES["$n"]=""; MISS_LAST["$n"]="-"; FETCH_LAST["$n"]="-"; DNA_LAST["$n"]=""
done

START_ISO="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "hc-mesh-spin-detector: ${CYCLES} cycles x ${WINDOW}s = $((CYCLES * WINDOW))s  (started $START_ISO)"
echo "  conductors: $(for i in "${!PID_NAMES[@]}"; do printf '%s(%s) ' "${PID_NAMES[$i]}" "${PID_VALUES[$i]}"; done)"
echo "  log streams: $(for i in "${!LOG_NAMES[@]}"; do printf '%s=%s ' "${LOG_NAMES[$i]}" "${LOG_PATHS[$i]}"; done)"
echo "  thresholds: saturated > ${SAT_THRESHOLD} lines/s (verdict), cpu > ${CPU_THRESHOLD} cores (flag)"
if [ ${#STORAGE_NAMES[@]} -gt 0 ]; then
  echo "  storage peers (read-only /p2p/status): $(for i in "${!STORAGE_NAMES[@]}"; do printf '%s=%s ' "${STORAGE_NAMES[$i]}" "${STORAGE_URLS[$i]}"; done)"
fi
echo

for cycle in $(seq 1 "$CYCLES"); do
  sleep "$WINDOW"

  cpu_row=""
  for i in "${!PID_NAMES[@]}"; do
    n="${PID_NAMES[$i]}"; p="${PID_VALUES[$i]}"
    t="$(cpu_ticks "$p")" || t=""
    if [ -z "$t" ]; then
      cores="gone"
      CPU_LAST["$n"]="gone"
    else
      cores="$(awk -v d="$((t - ${PREV_TICKS[$n]}))" -v hz="$CLK_TCK" -v w="$WINDOW" 'BEGIN{printf "%.2f", d/hz/w}')"
      PREV_TICKS["$n"]="$t"
      CPU_LAST["$n"]="$cores"
      tc="$(thread_count "$p")"
      THREADS_LAST["$n"]="$tc"
      [ "$tc" -gt "${THREADS_MAX[$n]:-0}" ] && THREADS_MAX["$n"]="$tc"
      while read -r tid ticks comm; do
        [ -n "$tid" ] || continue
        prev="${PREV_TT[$n|$tid]:-$ticks}"
        SUM_TT["$n|$tid"]=$(( ${SUM_TT[$n|$tid]:-0} + ticks - prev ))
        PREV_TT["$n|$tid"]="$ticks"; TT_COMM["$n|$tid"]="$comm"
      done < <(thread_ticks "$p")
      CPU_SUM["$n"]="$(awk -v a="${CPU_SUM[$n]}" -v b="$cores" 'BEGIN{printf "%.4f", a+b}')"
      awk -v a="${CPU_MAX[$n]}" -v b="$cores" 'BEGIN{exit !(b>a)}' && CPU_MAX["$n"]="$cores"
    fi
    cpu_row+="$(printf '%s=%s ' "$n" "$cores")"
  done

  log_row=""
  for i in "${!LOG_NAMES[@]}"; do
    n="${LOG_NAMES[$i]}"; path="${LOG_PATHS[$i]}"
    now_off="$(stat -c %s "$path" 2>/dev/null || echo 0)"
    from="${PREV_OFF[$n]}"
    # truncation/rotation: never read a negative span, restart from 0
    [ "$now_off" -lt "$from" ] && from=0
    read -r total sat nop sys oth miss fetch dnas <<< "$(classify "$path" "$from" "$now_off")"
    PREV_OFF["$n"]="$now_off"
    TOT_SUM["$n"]=$(( ${TOT_SUM[$n]} + total ))
    SAT_SUM["$n"]=$(( ${SAT_SUM[$n]} + sat ))
    NOP_SUM["$n"]=$(( ${NOP_SUM[$n]} + nop ))
    SYS_SUM["$n"]=$(( ${SYS_SUM[$n]} + sys ))
    if [ "$miss" != "-" ]; then
      MISS_LAST["$n"]="$miss"; FETCH_LAST["$n"]="$fetch"
      MISS_SERIES["$n"]="${MISS_SERIES[$n]}${MISS_SERIES[$n]:+,}$miss"
      [ -n "${dnas:-}" ] && DNA_LAST["$n"]="$dnas"
    else
      # No sys-validation line this cycle: the counter is unchanged, not absent.
      # Carry the last known value so the trend series stays comparable.
      MISS_SERIES["$n"]="${MISS_SERIES[$n]}${MISS_SERIES[$n]:+,}${MISS_LAST[$n]}"
    fi
    rate="$(awk -v c="$total" -v w="$WINDOW" 'BEGIN{printf "%.2f", c/w}')"
    srate="$(awk -v c="$sat" -v w="$WINDOW" 'BEGIN{printf "%.2f", c/w}')"
    log_row+="$(printf '%s: %s l/s (sat %s nopeers %d sysval %d other %d) missing=%s fetched=%s ' \
      "$n" "$rate" "$srate" "$nop" "$sys" "$oth" "${MISS_LAST[$n]}" "${FETCH_LAST[$n]}")"
  done

  [ "$QUIET" -eq 1 ] || printf 'cycle %d/%d  cpu(cores): %s\n           logs: %s\n' \
    "$cycle" "$CYCLES" "$cpu_row" "$log_row"
done

for i in "${!STORAGE_NAMES[@]}"; do
  STORE_AFTER["${STORAGE_NAMES[$i]}"]="$(storage_probe "${STORAGE_URLS[$i]}")"
done

# --- verdict -----------------------------------------------------------------
# SPIN requires BOTH legs, per stream:
#   (a) missing-deps > 0 and the series is NOT decreasing across >= 3 cycles
#       (a draining backlog is work; a flat or growing one is a spin)
#   (b) saturated lines/s over threshold (the read pool is being hammered)
VERDICT="QUIET"
declare -A STREAM_VERDICT STREAM_REASON
for i in "${!LOG_NAMES[@]}"; do
  n="${LOG_NAMES[$i]}"
  total_s=$((CYCLES * WINDOW))
  sat_rate="$(awk -v c="${SAT_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')"
  series="${MISS_SERIES[$n]}"
  miss_ok=0; reason=""
  if [ "${MISS_LAST[$n]}" = "-" ] || [ "${MISS_LAST[$n]}" = "0" ]; then
    reason="no missing dependencies reported"
  elif [ "$CYCLES" -lt 3 ]; then
    reason="fewer than 3 cycles — trend not established"
  else
    # non-decreasing across the whole series (allow equal; a plateau IS the spin)
    if awk -v s="$series" 'BEGIN{n=split(s,a,","); for(i=2;i<=n;i++){ if(a[i]=="-"||a[i-1]=="-") continue; if(a[i]+0 < a[i-1]+0) exit 1 } exit 0}'; then
      miss_ok=1; reason="missing-deps non-decreasing: $series"
    else
      reason="missing-deps decreasing (draining): $series"
    fi
  fi
  sat_ok=0
  awk -v r="$sat_rate" -v t="$SAT_THRESHOLD" 'BEGIN{exit !(r>t)}' && sat_ok=1
  if [ "$miss_ok" -eq 1 ] && [ "$sat_ok" -eq 1 ]; then
    STREAM_VERDICT["$n"]="SPIN"; VERDICT="SPIN"
    STREAM_REASON["$n"]="$reason; saturated ${sat_rate}/s > ${SAT_THRESHOLD}/s"
  else
    STREAM_VERDICT["$n"]="QUIET"
    STREAM_REASON["$n"]="$reason; saturated ${sat_rate}/s vs threshold ${SAT_THRESHOLD}/s"
  fi
done

echo
echo "── summary (${CYCLES} cycles x ${WINDOW}s) ──"
for i in "${!PID_NAMES[@]}"; do
  n="${PID_NAMES[$i]}"
  if [ "${CPU_LAST[$n]:-}" = "gone" ]; then
    printf '  cpu  %-10s GONE (process exited during the measure)\n' "$n"
  else
    avg="$(awk -v s="${CPU_SUM[$n]}" -v c="$CYCLES" 'BEGIN{printf "%.2f", s/c}')"
    hot=""; awk -v a="$avg" -v t="$CPU_THRESHOLD" 'BEGIN{exit !(a>t)}' && hot="  <-- hot"
    printf '  cpu  %-10s avg %s cores, max %s cores, %s threads (max %s)%s\n' \
      "$n" "$avg" "${CPU_MAX[$n]}" "${THREADS_LAST[$n]:-?}" "${THREADS_MAX[$n]:-?}" "$hot"
    # Hottest threads — which worker is burning it, since perf is unavailable here.
    top="$(for key in "${!SUM_TT[@]}"; do
             [ "${key%%|*}" = "$n" ] || continue
             echo "${SUM_TT[$key]} ${key##*|} ${TT_COMM[$key]}"
           done | sort -rn | head -"$TOP_THREADS")"
    while read -r ticks tid comm; do
      [ -n "${ticks:-}" ] || continue
      [ "$ticks" -gt 0 ] || continue
      printf '       %-10s thread %-7s %-18s %s cores\n' "" "$tid" "$comm" \
        "$(awk -v d="$ticks" -v hz="$CLK_TCK" -v s="$((CYCLES * WINDOW))" 'BEGIN{printf "%.2f", d/hz/s}')"
    done <<< "$top"
  fi
done
for i in "${!LOG_NAMES[@]}"; do
  n="${LOG_NAMES[$i]}"; total_s=$((CYCLES * WINDOW))
  printf '  log  %-10s %s l/s total | saturated %s/s | nopeers %s/s | sysval %s/s | missing=%s fetched=%s%s\n' \
    "$n" \
    "$(awk -v c="${TOT_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')" \
    "$(awk -v c="${SAT_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')" \
    "$(awk -v c="${NOP_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')" \
    "$(awk -v c="${SYS_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')" \
    "${MISS_LAST[$n]}" "${FETCH_LAST[$n]}" \
    "$([ -n "${DNA_LAST[$n]}" ] && printf ' | per-dna: %s' "${DNA_LAST[$n]}")"
  printf '       %-10s trend [%s] -> %s (%s)\n' "$n" "${MISS_SERIES[$n]}" "${STREAM_VERDICT[$n]}" "${STREAM_REASON[$n]}"
done
if [ ${#STORAGE_NAMES[@]} -gt 0 ]; then
  for i in "${!STORAGE_NAMES[@]}"; do
    n="${STORAGE_NAMES[$i]}"
    read -r b_peers b_caught b_conv b_div b_pull <<< "${STORE_BEFORE[$n]:-- - - - -}"
    read -r a_peers a_caught a_conv a_div a_pull <<< "${STORE_AFTER[$n]:-- - - - -}"
    printf '  peer %-10s connected %s->%s | reconcile caughtUp %s->%s converged %s->%s | divergentAnchor %s->%s | pull caughtUp %s->%s\n' \
      "$n" "$b_peers" "$a_peers" "$b_caught" "$a_caught" "$b_conv" "$a_conv" "$b_div" "$a_div" "$b_pull" "$a_pull"
  done
fi
echo
echo "VERDICT: $VERDICT"

# --- one-line JSON (a2o asserts on this) -------------------------------------
json="{\"tool\":\"hc-mesh-spin-detector\",\"startedAt\":\"$START_ISO\",\"cycles\":$CYCLES,\"windowSecs\":$WINDOW"
json+=",\"satThreshold\":$SAT_THRESHOLD,\"verdict\":\"$VERDICT\",\"conductors\":{"
sep=""
for i in "${!PID_NAMES[@]}"; do
  n="${PID_NAMES[$i]}"
  if [ "${CPU_LAST[$n]:-}" = "gone" ]; then
    json+="$sep\"$n\":{\"pid\":${PID_VALUES[$i]},\"state\":\"gone\"}"
  else
    avg="$(awk -v s="${CPU_SUM[$n]}" -v c="$CYCLES" 'BEGIN{printf "%.2f", s/c}')"
    json+="$sep\"$n\":{\"pid\":${PID_VALUES[$i]},\"avgCores\":$avg,\"maxCores\":${CPU_MAX[$n]}"
    json+=",\"threads\":${THREADS_LAST[$n]:-0},\"maxThreads\":${THREADS_MAX[$n]:-0},\"topThreads\":["
    tsep=""
    while read -r ticks tid comm; do
      [ -n "${ticks:-}" ] || continue
      [ "$ticks" -gt 0 ] || continue
      json+="$tsep{\"tid\":$tid,\"comm\":\"$comm\",\"cores\":$(awk -v d="$ticks" -v hz="$CLK_TCK" -v s="$((CYCLES * WINDOW))" 'BEGIN{printf "%.2f", d/hz/s}')}"
      tsep=","
    done <<< "$(for key in "${!SUM_TT[@]}"; do
                  [ "${key%%|*}" = "$n" ] || continue
                  echo "${SUM_TT[$key]} ${key##*|} ${TT_COMM[$key]}"
                done | sort -rn | head -"$TOP_THREADS")"
    json+="]}"
  fi
  sep=","
done
json+="},\"logStreams\":{"
sep=""
for i in "${!LOG_NAMES[@]}"; do
  n="${LOG_NAMES[$i]}"; total_s=$((CYCLES * WINDOW))
  miss="${MISS_LAST[$n]}"; [ "$miss" = "-" ] && miss="null"
  fetch="${FETCH_LAST[$n]}"; [ "$fetch" = "-" ] && fetch="null"
  json+="$sep\"$n\":{\"linesPerSec\":$(awk -v c="${TOT_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')"
  json+=",\"saturatedPerSec\":$(awk -v c="${SAT_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')"
  json+=",\"noPeersPerSec\":$(awk -v c="${NOP_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')"
  json+=",\"sysValPerSec\":$(awk -v c="${SYS_SUM[$n]}" -v s="$total_s" 'BEGIN{printf "%.2f", c/s}')"
  json+=",\"missingDeps\":$miss,\"fetched\":$fetch"
  json+=",\"missingSeries\":\"${MISS_SERIES[$n]}\",\"verdict\":\"${STREAM_VERDICT[$n]}\"}"
  sep=","
done
json+="}"
json+=",\"storagePeers\":{"
sep=""
for i in "${!STORAGE_NAMES[@]}"; do
  n="${STORAGE_NAMES[$i]}"
  jstore() { # <probe-tuple> -> JSON object; "-" becomes null, never a fake value
    read -r peers caught conv div pull <<< "$1"
    local o="{"
    o+="\"connectedPeers\":$([ "$peers" = "-" ] && echo null || echo "$peers")"
    o+=",\"reconcileCaughtUp\":$([ "$caught" = "-" ] && echo null || echo "$caught")"
    o+=",\"reconcileConverged\":$([ "$conv" = "-" ] && echo null || echo "$conv")"
    o+=",\"divergentAnchor\":$([ "$div" = "-" ] && echo null || echo "$div")"
    o+=",\"pullCaughtUp\":$([ "$pull" = "-" ] && echo null || echo "$pull")"
    echo "$o}"
  }
  json+="$sep\"$n\":{\"url\":\"${STORAGE_URLS[$i]}\""
  json+=",\"before\":$(jstore "${STORE_BEFORE[$n]:-- - - - -}")"
  json+=",\"after\":$(jstore "${STORE_AFTER[$n]:-- - - - -}")}"
  sep=","
done
json+="}}"
echo "$json"
[ -n "$JSON_OUT" ] && { mkdir -p "$(dirname "$JSON_OUT")"; echo "$json" > "$JSON_OUT"; }

[ "$VERDICT" = "SPIN" ] && exit 1
exit 0
