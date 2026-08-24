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
if [ -n "$shape_now" ]; then
  echo "matrix: live shape $shape_now"
else
  echo "matrix: no live mesh — first scenario will reshape"
fi

reshape_stop() { # <peers-csv> <doorways 0|1> — stop in its OWN session so the
  # matrix (this process's own process group) survives its own reshape.
  # `hc-mesh.sh stop` kills its own process group; called as a plain child it
  # takes the matrix down with it (documented: exit 144).
  local peers="$1" doorways="$2"
  if setsid --help 2>&1 | grep -q -- '-w'; then
    MESH_PEERS="$peers" MESH_DOORWAYS="$doorways" \
      setsid -w "$SCRIPT_DIR/hc-mesh.sh" stop >/dev/null 2>&1 || true
  else
    MESH_PEERS="$peers" MESH_DOORWAYS="$doorways" \
      setsid "$SCRIPT_DIR/hc-mesh.sh" stop >/dev/null 2>&1 || true
    # setsid without -w doesn't wait; poll for the processes it killed.
    local waited=0
    while pgrep -x holochain >/dev/null 2>&1 || pgrep -f "elohim-storag[e].*--http-port" >/dev/null 2>&1; do
      sleep 2
      waited=$((waited + 2))
      [ "$waited" -ge 60 ] && break
    done
  fi
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

      timeline="$MESH_DIR/recovery-timeline.jsonl"
      before_lines=0
      [ -f "$timeline" ] && before_lines="$(wc -l < "$timeline")"
      "$SCRIPT_DIR/hc-mesh-recovery.sh" "$shape" "$rec" \
        --label "scenario=$name" --label "shape=$shape" --label "run=$run" --label "expect=$expect"
      got=$?
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
