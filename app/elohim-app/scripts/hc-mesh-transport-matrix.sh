#!/usr/bin/env bash
# Repeat one household contract under libp2p, iroh, and dual, preserving the
# existing sprint-report envelope for every sample and rendering a derived
# ValueFlows Process view over those witnesses.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
A2O_DIR="$REPO_ROOT/genesis/a2o"
MESH_SCRIPT="$SCRIPT_DIR/hc-mesh.sh"
REPORTS_DIR="${MESH_MATRIX_REPORTS_DIR:-$A2O_DIR/reports/transport-matrix}"
RUNS="${MESH_MATRIX_RUNS:-3}"
SCOPE_OVERRIDE="${MESH_MATRIX_SCOPE:-}"
IFS=',' read -r -a MODES <<<"${MESH_MATRIX_MODES:-libp2p,iroh,dual}"

[[ "$RUNS" =~ ^[1-9][0-9]*$ ]] || {
  echo "MESH_MATRIX_RUNS must be a positive integer" >&2
  exit 2
}
for mode in "${MODES[@]}"; do
  case "$mode" in
    libp2p|iroh|dual) ;;
    *) echo "MESH_MATRIX_MODES contains unsupported mode '$mode'" >&2; exit 2 ;;
  esac
done

set +e
# shellcheck source=hc-mesh.sh
source "$MESH_SCRIPT"
set -e
MATRIX_DOORWAY_URL="http://localhost:$DOORWAY_PORT"

if [[ "${MESH_MATRIX_DRY_RUN:-0}" == "1" ]]; then
  echo "transport matrix (dry run)"
  echo "  modes: ${MODES[*]}"
  echo "  repetitions: $RUNS"
  echo "  scope: ${SCOPE_OVERRIDE:-@concern:transport-parity and @transport:<mode>}"
  echo "  reports: $REPORTS_DIR"
  exit 0
fi

if ! curl -sf -m 2 "http://localhost:$(http_port 0)/health" >/dev/null; then
  echo "household mesh is not running; run 'just mesh start' and 'just mesh prologue' first" >&2
  exit 2
fi

wait_for_storage_stability() {
  local deadline=$((SECONDS + 90))
  local stable=0
  while [[ "$SECONDS" -lt "$deadline" ]]; do
    local up=0 index=0
    for _peer in "${PEERS[@]}"; do
      curl -sf -m 3 "http://localhost:$(http_port "$index")/health" >/dev/null && up=$((up + 1))
      index=$((index + 1))
    done
    if [[ "$up" -eq "${#PEERS[@]}" ]]; then
      stable=$((stable + 1))
      [[ "$stable" -ge 3 ]] && return 0
    else
      stable=0
    fi
    sleep 2
  done
  return 1
}

wait_for_doorway_upstream() {
  local deadline=$((SECONDS + 90))
  local status
  while [[ "$SECONDS" -lt "$deadline" ]]; do
    # A collection read may be served from doorway cache while the upstream
    # breaker is still open. A unique missing content id must traverse the same
    # storage proxy used by POST /db/content; 404 is the healthy probe result.
    status="$(curl -s -o /dev/null -w '%{http_code}' -m 5 \
      "$MATRIX_DOORWAY_URL/db/content/__transport_matrix_probe_${SECONDS}")"
    if [[ "$status" == "404" || "$status" == "200" ]]; then
      return 0
    fi
    sleep 3
  done
  return 1
}

matrix_id="$(date -u +%Y%m%dT%H%M%SZ)-$(git -C "$REPO_ROOT" rev-parse --short=8 HEAD 2>/dev/null || echo nogit)"
matrix_dir="$REPORTS_DIR/$matrix_id"
mkdir -p "$matrix_dir"
report_args=()
matrix_rc=0

for mode in "${MODES[@]}"; do
  echo
  echo "=== transport $mode: restarting household storage peers ==="
  export MESH_TRANSPORT_BACKEND="$mode"
  restart_rc=0
  MESH_RESTART_APPLY_PROFILE=1 "$MESH_SCRIPT" storage-restart || restart_rc=$?
  if [[ "$restart_rc" -ne 0 ]]; then
    echo "transport $mode: restart reported a transient failure; applying the matrix stability gate" >&2
  fi
  if ! wait_for_storage_stability; then
    echo "transport $mode: storage peers did not remain healthy" >&2
    matrix_rc=1
    continue
  fi

  observed="$(mesh_transport_backend_from_status)"
  if [[ "$observed" != "$mode" ]]; then
    echo "transport $mode: live status classified as $observed; refusing mislabeled samples" >&2
    matrix_rc=1
    continue
  fi
  if ! wait_for_doorway_upstream; then
    echo "transport $mode: doorway upstream circuit did not recover" >&2
    matrix_rc=1
    continue
  fi

  for repetition in $(seq 1 "$RUNS"); do
    run_scope="${SCOPE_OVERRIDE:-@concern:transport-parity and @transport:$mode}"
    run_id="$matrix_id-$mode-r$repetition"
    cucumber_json="$matrix_dir/cucumber-$mode-r$repetition.json"
    sprint_json="$matrix_dir/sprint-report-$mode-r$repetition.json"
    sprint_md="$matrix_dir/sprint-report-$mode-r$repetition.md"
    echo "=== transport $mode: sample $repetition/$RUNS ==="
    run_rc=0
    CUCUMBER_JSON_REPORT="$cucumber_json" \
      A2O_RUN_ID="$run_id" \
      A2O_SPRINT_REPORT_JSON="$sprint_json" \
      A2O_SPRINT_REPORT_MD="$sprint_md" \
      E2E_EXPECTED_TRANSPORT="$mode" \
      just --justfile "$REPO_ROOT/justfile" test mesh "$run_scope" || run_rc=$?
    if [[ -f "$sprint_json" ]]; then
      report_args+=(--report "$sprint_json")
      stable_report="$A2O_DIR/reports/sprint-report-household-$run_id.json"
      report_rel="$(realpath --relative-to="$A2O_DIR/reports" "$sprint_json")"
      ln -sfn "$report_rel" "$stable_report"
    else
      echo "transport $mode sample $repetition left no sprint-report witness" >&2
      run_rc=1
    fi
    [[ "$run_rc" -eq 0 ]] || matrix_rc="$run_rc"
  done
done

if [[ "${#report_args[@]}" -eq 0 ]]; then
  echo "transport matrix produced no run witnesses" >&2
  exit 1
fi

matrix_md="$matrix_dir/transport-matrix.md"
(
  cd "$A2O_DIR"
  node --import tsx scripts/render-transport-matrix.ts \
    "${report_args[@]}" \
    --out "$matrix_md"
)
ln -sfn "$matrix_id/transport-matrix.md" "$REPORTS_DIR/latest.md"
echo
echo "transport matrix: $matrix_md"
echo "latest view: $REPORTS_DIR/latest.md"
exit "$matrix_rc"
