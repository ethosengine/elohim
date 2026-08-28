#!/usr/bin/env bash
# hc-mesh-recovery-matrix.sh — cycle mesh slots through the recovery scenario
# library × recovery shapes × N runs, alternating survivor/recovering roles.
#
#   MESH_RECOVERY_RUNS=3 MESH_RECOVERY_SHAPES=warm,cold \
#   MESH_RECOVERY_SCENARIOS=homo-dual,split-libp2p-iroh \
#   hc-mesh-recovery-matrix.sh
#
# A scenario whose `expect` is no-shared-transport PASSES when recovery FAILS
# with P0..P2 all 0 (nothing crossed) — the honest red the spec demands.
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="${MESH_RECOVERY_LIBRARY:-$SCRIPT_DIR/mesh-recovery-scenarios.tsv}"
RUNS="${MESH_RECOVERY_RUNS:-3}"
IFS=',' read -ra SHAPES <<< "${MESH_RECOVERY_SHAPES:-warm,cold}"
ONLY="${MESH_RECOVERY_SCENARIOS:-}"
# Probe port base for matrix_live_shape's own health checks (mirrors
# hc-mesh.sh's http_port() 8090+i scheme); overridable so tests can point the
# probe at guaranteed-closed ports without touching the real mesh scheme.
MESH_RECOVERY_PROBE_BASE="${MESH_RECOVERY_PROBE_BASE:-8090}"

matrix_scenarios() {
  awk -F '\t' -v only="$ONLY" '
    BEGIN {
      count = split(only, selected, ",")
      for (i = 1; i <= count; i++) wanted[selected[i]] = 1
    }
    /^[[:space:]]*#/ || /^[[:space:]]*$/ { next }
    only == "" || ($1 in wanted)
  ' "$LIB"
}

matrix_recovering_index() { # <run-number>; run 1 -> slot 1, run 2 -> slot 0, …
  echo $(( $1 % 2 ))
}

matrix_live_shape() { # -> "<peers-csv>/<0|1>" describing what's ACTUALLY running, or "" if nothing answers
  # MESH_RECOVERY_LIVE_SHAPE bypasses probing entirely (tests use it).
  if [ -n "${MESH_RECOVERY_LIVE_SHAPE:-}" ]; then
    printf '%s\n' "$MESH_RECOVERY_LIVE_SHAPE"
    return 0
  fi
  local -a candidates
  IFS=',' read -ra candidates <<< "${MESH_PEERS:-matthew,jessica,james}"
  local live="" i=0 name port
  for name in "${candidates[@]}"; do
    port=$((MESH_RECOVERY_PROBE_BASE + i))
    if curl -sf -m 2 "http://localhost:$port/health" >/dev/null 2>&1; then
      live+="${live:+,}$name"
    fi
    i=$((i + 1))
  done
  if [ -z "$live" ]; then
    printf '\n'
    return 0
  fi
  local doorways=0
  curl -sf -m 2 "http://localhost:${DOORWAY_PORT:-8888}/health" >/dev/null 2>&1 && doorways=1
  printf '%s/%s\n' "$live" "$doorways"
}

matrix_capture_corpus_ref() { # -> sets CORPUS_ROWS/CORPUS_DOCS from the
  # FIRST peer's /db/stats.contentCount and /sync/v1/elohim/docs?limit=1
  # .total (or from MESH_RECOVERY_CORPUS_REF="<rows>/<docs>" in tests), and
  # prints "matrix: corpus reference rows=<n> docs=<n>". This is the reference
  # every survivor is judged against by matrix_survivor_healthy — a fresh
  # capture belongs after any event that regenerates the mesh (initial live
  # shape, or a reshape).
  if [ -n "${MESH_RECOVERY_CORPUS_REF:-}" ]; then
    CORPUS_ROWS="${MESH_RECOVERY_CORPUS_REF%/*}"
    CORPUS_DOCS="${MESH_RECOVERY_CORPUS_REF#*/}"
  else
    local port=$MESH_RECOVERY_PROBE_BASE stats docs
    stats="$(curl -sf -m 3 "http://localhost:$port/db/stats" 2>/dev/null)"
    CORPUS_ROWS="$(printf '%s' "$stats" | python3 -c '
import json, sys
try:
    print(int(json.load(sys.stdin).get("contentCount", 0) or 0))
except Exception:
    print(0)
' 2>/dev/null)"
    [ -z "$CORPUS_ROWS" ] && CORPUS_ROWS=0
    docs="$(curl -sf -m 3 "http://localhost:$port/sync/v1/elohim/docs?limit=1" 2>/dev/null)"
    CORPUS_DOCS="$(printf '%s' "$docs" | python3 -c '
import json, sys
try:
    print(int(json.load(sys.stdin).get("total", 0) or 0))
except Exception:
    print(0)
' 2>/dev/null)"
    [ -z "$CORPUS_DOCS" ] && CORPUS_DOCS=0
  fi
  echo "matrix: corpus reference rows=$CORPUS_ROWS docs=$CORPUS_DOCS"
}

matrix_survivor_healthy() { # <survivor-csv> -> rc 0 iff EVERY named survivor is
  # /health 200 AND contentCount >= corpus_rows*0.9 (integer math) AND
  # docs.total >= corpus_docs*0.9. Ports resolve by the peer's position in
  # MESH_PEERS (the http_port 8090+i scheme), never by position within the
  # survivor-only csv — a lone non-first survivor must still probe ITS port,
  # not the recovering peer's. MESH_RECOVERY_SURVIVOR_STUB=ok|fail bypasses
  # probing entirely for unit tests; unstubbed, a closed port fails bounded
  # by curl's own -m timeout (no retry loop here — the caller decides whether
  # to regenerate and re-check).
  case "${MESH_RECOVERY_SURVIVOR_STUB:-}" in
    ok) echo "matrix: survivor health OK (stub)"; return 0 ;;
    fail) echo "matrix: survivor health FAIL (stub)"; return 1 ;;
  esac
  local csv="$1"
  local -a survs all
  IFS=',' read -ra survs <<< "$csv"
  IFS=',' read -ra all <<< "${MESH_PEERS:-matthew,jessica,james}"
  local ref_rows ref_docs min_rows min_docs
  if [ -n "${MESH_RECOVERY_CORPUS_REF:-}" ]; then
    ref_rows="${MESH_RECOVERY_CORPUS_REF%/*}"
    ref_docs="${MESH_RECOVERY_CORPUS_REF#*/}"
  else
    ref_rows="${CORPUS_ROWS:-0}"
    ref_docs="${CORPUS_DOCS:-0}"
  fi
  min_rows=$(( ref_rows * 9 / 10 ))
  min_docs=$(( ref_docs * 9 / 10 ))
  local name idx i cand port stats cc docs dt
  for name in "${survs[@]}"; do
    idx=-1; i=0
    for cand in "${all[@]}"; do [ "$cand" = "$name" ] && idx=$i; i=$((i + 1)); done
    [ "$idx" -lt 0 ] && idx=0
    port=$((MESH_RECOVERY_PROBE_BASE + idx))
    if ! curl -sf -m 3 "http://localhost:$port/health" >/dev/null 2>&1; then
      echo "matrix: survivor $name unhealthy ($port/health failed)"
      return 1
    fi
    stats="$(curl -sf -m 3 "http://localhost:$port/db/stats" 2>/dev/null)"
    cc="$(printf '%s' "$stats" | python3 -c '
import json, sys
try:
    print(int(json.load(sys.stdin).get("contentCount", 0) or 0))
except Exception:
    print(0)
' 2>/dev/null)"
    [ -z "$cc" ] && cc=0
    if [ "$cc" -lt "$min_rows" ]; then
      echo "matrix: survivor $name unhealthy (contentCount=$cc < $min_rows)"
      return 1
    fi
    docs="$(curl -sf -m 3 "http://localhost:$port/sync/v1/elohim/docs?limit=1" 2>/dev/null)"
    dt="$(printf '%s' "$docs" | python3 -c '
import json, sys
try:
    print(int(json.load(sys.stdin).get("total", 0) or 0))
except Exception:
    print(0)
' 2>/dev/null)"
    [ -z "$dt" ] && dt=0
    if [ "$dt" -lt "$min_docs" ]; then
      echo "matrix: survivor $name unhealthy (docs.total=$dt < $min_docs)"
      return 1
    fi
  done
  echo "matrix: survivor health OK ($csv)"
  return 0
}

matrix_mark_remaining_fail() { # <name> <from-shape> <from-run> — appends
  # FAIL(survivor-unhealthy) rows for every shape×run at/after
  # <from-shape>,<from-run> (SHAPES order, 1..RUNS) for a scenario abandoned
  # mid-flight because survivors never came back healthy after a regenerate.
  local nm="$1" from_shape="$2" from_run="$3" started=0 sh rn
  for sh in "${SHAPES[@]}"; do
    [ "$sh" = "$from_shape" ] && started=1
    [ "$started" -eq 1 ] || continue
    for rn in $(seq 1 "$RUNS"); do
      if [ "$sh" = "$from_shape" ] && [ "$rn" -lt "$from_run" ]; then continue; fi
      rows+=("$nm"$'\t'"$sh"$'\t'"$rn"$'\t'"-"$'\t'"-"$'\t'"-"$'\t'"survivor-unhealthy"$'\t'"FAIL(survivor-unhealthy)")
    done
  done
}

reshape_verify() { # <peers-csv> <doorways 0|1> -> rc 0 iff the mesh is actually
  # SERVING (every peer /health + /db/stats contentCount>0, and — when
  # doorways=1 — both doorways answer /db/content/elohim-host-landing).
  # A reshape is judged by this, never by the prologue seeder's exit code
  # (its post-flight probe is a known false red against SSR HTML).
  local peers="$1" doorways="$2"
  case "${MESH_RECOVERY_RESHAPE_VERIFY_STUB:-}" in
    ok)
      echo "matrix: reshape verified peers=$peers doorways=$doorways in 0s (stub)"
      return 0
      ;;
    fail)
      echo "matrix: reshape NOT serving after 0s (stub forced failure)"
      return 1
      ;;
  esac
  local max="${MESH_RECOVERY_RESHAPE_VERIFY_SECS:-180}" interval=5
  local -a plist
  IFS=',' read -ra plist <<< "$peers"
  local start_ts elapsed which ok i name port stats cc remaining sleep_for
  start_ts=$(date +%s)
  while :; do
    ok=1
    which=""
    i=0
    for name in "${plist[@]}"; do
      port=$((MESH_RECOVERY_PROBE_BASE + i))
      if ! curl -sf -m 3 "http://localhost:$port/health" >/dev/null 2>&1; then
        ok=0
        which="$name/health"
        break
      fi
      stats="$(curl -sf -m 3 "http://localhost:$port/db/stats" 2>/dev/null)"
      if [ -z "$stats" ]; then
        ok=0
        which="$name/db/stats unreachable"
        break
      fi
      cc="$(printf '%s' "$stats" | python3 -c '
import json, sys
try:
    print(int(json.load(sys.stdin).get("contentCount", 0) or 0))
except Exception:
    print(0)
' 2>/dev/null)"
      [ -z "$cc" ] && cc=0
      if [ "$cc" -le 0 ]; then
        ok=0
        which="$name/db/stats contentCount=$cc"
        break
      fi
      i=$((i + 1))
    done
    if [ "$ok" -eq 1 ] && [ "$doorways" = "1" ]; then
      if ! curl -sf -m 3 "http://localhost:${DOORWAY_PORT:-8888}/db/content/elohim-host-landing" >/dev/null 2>&1; then
        ok=0
        which="doorway-A /db/content/elohim-host-landing"
      elif ! curl -sf -m 3 "http://localhost:${DOORWAY_B_PORT:-8889}/db/content/elohim-host-landing" >/dev/null 2>&1; then
        ok=0
        which="doorway-B /db/content/elohim-host-landing"
      fi
    fi
    elapsed=$(( $(date +%s) - start_ts ))
    if [ "$ok" -eq 1 ]; then
      echo "matrix: reshape verified peers=$peers doorways=$doorways in ${elapsed}s"
      return 0
    fi
    if [ "$elapsed" -ge "$max" ]; then
      echo "matrix: reshape NOT serving after ${elapsed}s ($which)"
      return 1
    fi
    remaining=$((max - elapsed))
    sleep_for="$interval"
    [ "$remaining" -lt "$interval" ] && sleep_for="$remaining"
    sleep "$sleep_for"
  done
}

if [ "${RECOVERY_MATRIX_SOURCE_ONLY:-0}" = "1" ]; then
  return 0 2>/dev/null || exit 0
fi

if [ ! -r "$LIB" ]; then
  echo "recovery scenario library is not readable: $LIB" >&2
  exit 2
fi
case "$RUNS" in
  ''|*[!0-9]*|0) echo "MESH_RECOVERY_RUNS must be a positive integer (got '$RUNS')" >&2; exit 2 ;;
esac
for matrix_shape in "${SHAPES[@]}"; do
  case "$matrix_shape" in
    warm|cold) ;;
    *) echo "invalid recovery shape '$matrix_shape' (expected warm or cold)" >&2; exit 2 ;;
  esac
done
scenario_count="$(matrix_scenarios | wc -l)"
if [ "$scenario_count" -eq 0 ]; then
  echo "no recovery scenarios matched MESH_RECOVERY_SCENARIOS='$ONLY'" >&2
  exit 2
fi

rc=0
rows=()
# Per-distinct-shape (peers/doorways) reshape bookkeeping: bounded retries
# instead of a regenerate loop when the mesh never comes up serving.
declare -A reshape_fail_count=()
declare -A reshape_given_up=()
MESH_RECOVERY_RESHAPE_RETRIES="${MESH_RECOVERY_RESHAPE_RETRIES:-1}"
# Seed shape_now from what's ACTUALLY running so the first scenario reshapes
# only when the live mesh doesn't already match it — not unconditionally.
shape_now="$(matrix_live_shape)"
# Corpus reference (Fix wave 3): captured once the mesh is confirmed serving
# — here if it's already live, or after the first scenario's reshape below
# otherwise — and re-captured after every later reshape. matrix_survivor_healthy
# judges every subsequent run against this reference so the matrix never
# alternates onto a survivor that only LOOKS healthy because it's empty.
CORPUS_ROWS=""
CORPUS_DOCS=""
if [ -n "$shape_now" ]; then
  echo "matrix: live shape $shape_now"
  matrix_capture_corpus_ref
  if [ "${CORPUS_ROWS:-0}" -eq 0 ] && [ "${CORPUS_DOCS:-0}" -eq 0 ]; then
    echo "matrix: corpus reference is empty (0/0) — treating as no live mesh, first scenario will reshape"
    shape_now=""
    CORPUS_ROWS=""
    CORPUS_DOCS=""
  fi
else
  echo "matrix: no live mesh — first scenario will reshape"
fi

reshape_stop() { # <peers-csv> <doorways 0|1>
  local peers="$1" doorways="$2"
  # Stop owns exact recorded PIDs and configured listener ports, so it is a
  # normal synchronous child; no separate session/self-kill workaround needed.
  MESH_PEERS="$peers" MESH_DOORWAYS="$doorways" \
    "$SCRIPT_DIR/hc-mesh.sh" stop >/dev/null 2>&1 || true
}

reshape_wait_ports_free() { # bounded wait (60s) until no mesh port is bound
  local waited=0 max=60 lines busy i
  while :; do
    lines="$(ss -tln 2>/dev/null)"
    busy=0
    for i in 0 1 2 3 4 5 6 7 8 9; do
      echo "$lines" | grep -q ":$((4444 + 10 * i)) " && busy=1
      echo "$lines" | grep -q ":$((8090 + i)) " && busy=1
    done
    echo "$lines" | grep -qE ':(8888|8889) ' && busy=1
    [ "$busy" -eq 0 ] && return 0
    [ "$waited" -ge "$max" ] && {
      echo "reshape: mesh ports still busy after ${max}s; proceeding anyway" >&2
      return 0
    }
    sleep 2
    waited=$((waited + 2))
  done
}

reshape_mesh() { # <peers-csv> <doorways 0|1>; the only process-count change
  local peers="$1" doorways="$2"
  echo "=== reshaping mesh: peers=$peers doorways=$doorways ==="
  reshape_stop "$peers" "$doorways"
  reshape_wait_ports_free
  MESH_PEERS="$peers" MESH_DOORWAYS="$doorways" "$SCRIPT_DIR/hc-mesh.sh" start || return 1
  local prologue_rc
  MESH_PEERS="$peers" MESH_DOORWAYS="$doorways" "$SCRIPT_DIR/hc-mesh.sh" prologue
  prologue_rc=$?
  echo "matrix: prologue exit $prologue_rc (advisory — the seeder post-flight is a known false red; the mesh is judged by reshape_verify)"
  reshape_verify "$peers" "$doorways"
}

matrix_reshape_fail_rows() { # <name> — append FAIL(reshape) rows for every
  # shape×run of this scenario so the table accounts for a scenario the
  # matrix never ran (reshape verify red, or a shape already given up on).
  local name="$1"
  local fr_shape fr_run
  for fr_shape in "${SHAPES[@]}"; do
    for fr_run in $(seq 1 "$RUNS"); do
      rows+=("$name"$'\t'"$fr_shape"$'\t'"$fr_run"$'\t'"-"$'\t'"-"$'\t'"-"$'\t'"reshape"$'\t'"FAIL(reshape)")
    done
  done
}

while IFS=$'\t' read -r name peers doorways t_surv t_rec expect; do
  [ -n "$name" ] || continue
  case "$doorways" in 0|1) ;; *) echo "invalid doorways value for $name: $doorways" >&2; rc=1; continue ;; esac
  case "$expect" in recover|no-shared-transport) ;; *) echo "invalid expectation for $name: $expect" >&2; rc=1; continue ;; esac

  if [ "$shape_now" != "$peers/$doorways" ]; then
    shape_key="$peers/$doorways"
    if [ "${reshape_given_up[$shape_key]:-0}" = "1" ]; then
      echo "matrix: shape $shape_key already given up on — skipping reshape, recording FAIL(reshape) for $name" >&2
      matrix_reshape_fail_rows "$name"
      rc=1
      continue
    fi
    if ! reshape_mesh "$peers" "$doorways"; then
      attempt=$(( ${reshape_fail_count[$shape_key]:-0} + 1 ))
      reshape_fail_count[$shape_key]="$attempt"
      max_attempts=$(( MESH_RECOVERY_RESHAPE_RETRIES + 1 ))
      if [ "$attempt" -ge "$max_attempts" ]; then
        reshape_given_up[$shape_key]=1
        echo "matrix: giving up on shape $shape_key after $attempt attempts" >&2
      else
        echo "matrix: reshape failed for $name (attempt $attempt/$max_attempts on shape $shape_key) — will retry when the next scenario needs it" >&2
      fi
      matrix_reshape_fail_rows "$name"
      rc=1
      continue
    fi
    reshape_fail_count[$shape_key]=0
    shape_now="$shape_key"
    matrix_capture_corpus_ref
  fi

  export MESH_PEERS="$peers" MESH_DOORWAYS="$doorways"
  set +e
  # shellcheck source=hc-mesh.sh
  source "$SCRIPT_DIR/hc-mesh.sh" >/dev/null 2>&1
  mesh_source_rc=$?
  set -u
  if [ "$mesh_source_rc" -ne 0 ]; then
    echo "could not load mesh interface for $name" >&2
    rc=1
    continue
  fi

  for shape in "${SHAPES[@]}"; do
    for run in $(seq 1 "$RUNS"); do
      # Two peers alternate roles. In fanout scenarios the last peer always
      # recovers and every preceding peer survives.
      if [ "${#PEERS[@]}" -eq 2 ]; then
        ri="$(matrix_recovering_index "$run")"
      else
        ri=$(( ${#PEERS[@]} - 1 ))
      fi
      rec="${PEERS[$ri]}"
      surv=""
      pt=""
      for k in "${!PEERS[@]}"; do
        if [ "$k" -eq "$ri" ]; then
          pt+="${pt:+,}${PEERS[$k]}=$t_rec"
        else
          pt+="${pt:+,}${PEERS[$k]}=$t_surv"
          surv+="${surv:+,}${PEERS[$k]}"
        fi
      done
      export MESH_PEER_TRANSPORTS="$pt"

      echo
      echo "=== $name · $shape · run $run · survivor=$surv($t_surv) recovering=$rec($t_rec) ==="
      # Survivors must be serving in their declared plane before the loser
      # returns. A restart error is visible but the recovery predicate remains
      # the final verdict.
      for s1 in ${surv//,/ }; do
        MESH_RESTART_APPLY_PROFILE=1 "$SCRIPT_DIR/hc-mesh.sh" storage-restart "$s1" \
          >/dev/null 2>&1 || echo "  survivor $s1 restart non-zero; continuing"
      done
      sleep 10

      # Survivor health gate (Fix wave 3): a FAILED recovery leaves the loser
      # empty; the matrix alternates roles next run, so the empty loser would
      # become this run's survivor and every leg would pass against nothing.
      # Refuse that here — regenerate the current shape and re-check once
      # (honouring the same retry bookkeeping the top-of-scenario reshape
      # uses) rather than measure against an unhealthy survivor.
      if ! matrix_survivor_healthy "$surv"; then
        echo "matrix: survivor(s) $surv unhealthy before $name/$shape/run-$run" >&2
        shape_key="$peers/$doorways"
        reshaped_ok=0
        if [ "${reshape_given_up[$shape_key]:-0}" = "1" ]; then
          echo "matrix: shape $shape_key already given up on — skipping regenerate" >&2
        elif reshape_mesh "$peers" "$doorways"; then
          reshape_fail_count[$shape_key]=0
          shape_now="$shape_key"
          matrix_capture_corpus_ref
          if matrix_survivor_healthy "$surv"; then
            reshaped_ok=1
          else
            echo "matrix: survivor(s) $surv still unhealthy after regenerate" >&2
          fi
        else
          attempt=$(( ${reshape_fail_count[$shape_key]:-0} + 1 ))
          reshape_fail_count[$shape_key]="$attempt"
          max_attempts=$(( MESH_RECOVERY_RESHAPE_RETRIES + 1 ))
          if [ "$attempt" -ge "$max_attempts" ]; then
            reshape_given_up[$shape_key]=1
            echo "matrix: giving up on shape $shape_key after $attempt attempts" >&2
          fi
        fi
        if [ "$reshaped_ok" -ne 1 ]; then
          echo "matrix: marking remaining runs of $name FAIL(survivor-unhealthy)" >&2
          matrix_mark_remaining_fail "$name" "$shape" "$run"
          rc=1
          break 2
        fi
      fi

      timeline="${RECOVERY_TIMELINE:-$(cd "$SCRIPT_DIR/../../.." && pwd)/genesis/a2o/reports/recovery/recovery-timeline.jsonl}"  # durable home (see hc-mesh-recovery.sh)
      before_lines=0
      [ -f "$timeline" ] && before_lines="$(wc -l < "$timeline")"
      "$SCRIPT_DIR/hc-mesh-recovery.sh" "$shape" "$rec" \
        --label "scenario=$name" --label "shape=$shape" --label "run=$run" --label "expect=$expect"
      got=$?

      if [ "$got" -eq 6 ]; then
        # Should be unreachable behind the survivor-health gate above, but
        # honour it: no timeline record was written (nothing was measured).
        echo "recovery refused a vacuous measurement for $name/$shape/run-$run (rc=6)" >&2
        rows+=("$name"$'\t'"$shape"$'\t'"$run"$'\t'"$surv=$t_surv"$'\t'"$rec=$t_rec"$'\t'"-"$'\t'"vacuous"$'\t'"FAIL(vacuous)")
        rc=1
        continue
      fi

      after_lines=0
      [ -f "$timeline" ] && after_lines="$(wc -l < "$timeline")"

      if [ "$after_lines" -le "$before_lines" ]; then
        echo "recovery produced no timeline record for $name/$shape/run-$run" >&2
        legs="no-record"
        secs="-"
        verdict=FAIL
      else
        last="$(tail -1 "$timeline")"
        parsed="$(python3 -c '
import json, sys
record = json.loads(sys.argv[1])
print(",".join(record["failing_legs"]), record["time_to_recover_s"])
' "$last")"
        parse_rc=$?
        if [ "$parse_rc" -ne 0 ]; then
          echo "invalid recovery timeline record for $name/$shape/run-$run" >&2
          legs="invalid-record"
          secs="-"
          verdict=FAIL
        else
          read -r legs secs <<< "$parsed"
          case "$expect" in
            recover)
              verdict=$([ "$got" -eq 0 ] && echo PASS || echo FAIL)
              ;;
            no-shared-transport)
              if [ "$got" -ne 0 ] && [[ "$legs" == *P0* && "$legs" == *P1* && "$legs" == *P2* ]]; then
                verdict="PASS(expected-red)"
              else
                verdict=FAIL
              fi
              ;;
          esac
        fi
      fi

      [[ "$verdict" == PASS* ]] || rc=1
      rows+=("$name"$'\t'"$shape"$'\t'"$run"$'\t'"$surv=$t_surv"$'\t'"$rec=$t_rec"$'\t'"${secs}s"$'\t'"$legs"$'\t'"$verdict")

      # Doorway-less runs cannot take a quiesce measure: the gate needs both.
      [ "$doorways" = "0" ] && continue
      if [ "${MESH_RECOVERY_QUIESCE:-0}" = "1" ]; then
        # Quiesce is a measurement side-channel, not a recovery expectation: the
        # matrix's exit code is exit 0 iff every run matched its scenario's
        # expect — a quiesce plumbing failure warns on stderr and records
        # nothing, but never taints rc. The verdict column (and rc) stay
        # exactly what the recovery run above decided.
        "$SCRIPT_DIR/hc-mesh-quiesce.sh" >/dev/null 2>&1
        quiesce_log="$(find "$MESH_DIR/quiesce-gate" -maxdepth 1 -type f -name '*.log' -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -1 | cut -d' ' -f2-)"
        if [ -z "$quiesce_log" ]; then
          echo "quiesce produced no local gate log for $name/$shape/run-$run (measurement only, verdict unaffected)" >&2
        else
          python3 "$REPO_ROOT/genesis/scripts/quiesce-timeline.py" --local "$quiesce_log" \
            --label "scenario=$name" --label "shape=$shape" --label "run=$run" --record \
            >/dev/null || echo "quiesce-timeline record failed for $name/$shape/run-$run (measurement only, verdict unaffected)" >&2
        fi
      fi
    done
  done
done < <(matrix_scenarios)

echo
printf 'scenario\tshape\trun\tsurvivor\trecovering\tt_recover\tfailing\tverdict\n'
printf '%s\n' "${rows[@]}"
exit "$rc"
