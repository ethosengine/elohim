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
shape_now=""

reshape_mesh() { # <peers-csv> <doorways 0|1>; the only process-count change
  local peers="$1" doorways="$2"
  echo "=== reshaping mesh: peers=$peers doorways=$doorways ==="
  # stop may return 144 after killing its own process group; start/prologue
  # are the authoritative reshape verdicts.
  MESH_PEERS="$peers" MESH_DOORWAYS="$doorways" "$SCRIPT_DIR/hc-mesh.sh" stop || true
  MESH_PEERS="$peers" MESH_DOORWAYS="$doorways" "$SCRIPT_DIR/hc-mesh.sh" start &&
    MESH_PEERS="$peers" MESH_DOORWAYS="$doorways" "$SCRIPT_DIR/hc-mesh.sh" prologue
}

while IFS=$'\t' read -r name peers doorways t_surv t_rec expect; do
  [ -n "$name" ] || continue
  case "$doorways" in 0|1) ;; *) echo "invalid doorways value for $name: $doorways" >&2; rc=1; continue ;; esac
  case "$expect" in recover|no-shared-transport) ;; *) echo "invalid expectation for $name: $expect" >&2; rc=1; continue ;; esac

  if [ "$shape_now" != "$peers/$doorways" ]; then
    reshape_mesh "$peers" "$doorways" || {
      echo "reshape failed for $name" >&2
      rc=1
      continue
    }
    shape_now="$peers/$doorways"
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
        "$SCRIPT_DIR/hc-mesh-quiesce.sh" >/dev/null 2>&1
        quiesce_log="$(find "$MESH_DIR/quiesce-gate" -maxdepth 1 -type f -name '*.log' -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -1 | cut -d' ' -f2-)"
        if [ -z "$quiesce_log" ]; then
          echo "quiesce produced no local gate log for $name/$shape/run-$run" >&2
          rc=1
        else
          python3 "$REPO_ROOT/genesis/scripts/quiesce-timeline.py" --local "$quiesce_log" \
            --label "scenario=$name" --label "shape=$shape" --label "run=$run" --record \
            >/dev/null || rc=1
        fi
      fi
    done
  done
done < <(matrix_scenarios)

echo
printf 'scenario\tshape\trun\tsurvivor\trecovering\tt_recover\tfailing\tverdict\n'
printf '%s\n' "${rows[@]}"
exit "$rc"
