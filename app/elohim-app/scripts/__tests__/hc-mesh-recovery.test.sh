#!/usr/bin/env bash
# Source-only unit coverage for recovery helpers. Task 5 extends this file
# with the recovery snapshot/predicate tests when that dependency lands.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

# recovery_snapshot / recovery_predicate against a stub storage peer served by
# python's http.server on a free port (no mesh needed).
(
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"; kill $srv 2>/dev/null' EXIT
  mkdir -p "$tmp/db/content" "$tmp/sync/v1/elohim" "$tmp/blob" "$tmp/p2p"
  echo '{"items":[{"id":"a","blobHash":"sha256-aa"},{"id":"b"}]}' > "$tmp/db/content/index.html"
  echo '{"id":"a","blobHash":"sha256-aa"}' > "$tmp/db/content/a"
  echo '{"contentCount":2}' > "$tmp/db/stats"
  echo '{"total":2}' > "$tmp/sync/v1/elohim/docs"
  echo 'bytes' > "$tmp/blob/sha256-aa"
  echo '{"pull":{"caughtUp":true,"failed":0}}' > "$tmp/p2p/status"
  port=$(python3 -c 'import socket;s=socket.socket();s.bind(("",0));print(s.getsockname()[1])')
  ( cd "$tmp" && python3 -m http.server "$port" >/dev/null 2>&1 ) & srv=$!
  sleep 1
  set +e; RECOVERY_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery.sh"; set -e 2>/dev/null; set +e
  recovery_snapshot "$port" > "$tmp/snap.json"
  t "snapshot captured 1 blob-bearing row" '[ "$(python3 -c "import json;print(len(json.load(open(\"$tmp/snap.json\"))[\"rows\"]))")" = 1 ]'
  RECOVERY_DOORWAY_A="http://localhost:$port" RECOVERY_DOORWAY_B="http://localhost:$port" RECOVERY_LANDING_PATH="/db/stats" \
    out="$(recovery_predicate "$tmp/snap.json" "$port")"; rc=$?
  t "all legs pass on a matching peer (rc=$rc): $out" '[ "$rc" -eq 0 ] && [ "$out" = "P0=1 P1=1 P2=1 P3=1 P4=1" ]'
  # MESH_DOORWAYS=0 (no doorways to serve through, e.g. a single-doorway or
  # doorway-less scenario): P4 is skipped ("-"), not failed, and does not
  # block the overall pass/fail decision.
  RECOVERY_DOORWAY_A="http://localhost:$port" RECOVERY_DOORWAY_B="http://localhost:$port" RECOVERY_LANDING_PATH="/db/stats" \
    out="$(MESH_DOORWAYS=0 recovery_predicate "$tmp/snap.json" "$port")"; rc=$?
  t "MESH_DOORWAYS=0 skips P4 without failing (rc=$rc): $out" '[ "$rc" -eq 0 ] && [ "$out" = "P0=1 P1=1 P2=1 P3=1 P4=-" ]'
  rm "$tmp/blob/sha256-aa"
  RECOVERY_DOORWAY_A="http://localhost:$port" RECOVERY_DOORWAY_B="http://localhost:$port" RECOVERY_LANDING_PATH="/db/stats" \
    out="$(recovery_predicate "$tmp/snap.json" "$port")"; rc=$?
  t "missing blob bytes fails ONLY P2 (rc=$rc): $out" '[ "$rc" -ne 0 ] && [ "$out" = "P0=1 P1=1 P2=0 P3=1 P4=1" ]'
  # receipt_max (Critical-1 fix): reads the CONDUCTOR's log via LOCAL_DEV_DIR —
  # never elohim-storage's own log — preferring the per-peer
  # $LOCAL_DEV_DIR/.sandbox_run_log.<peer> and falling back to the shared
  # mesh-wide $LOCAL_DEV_DIR/.sandbox_run_log, printing the literal "null"
  # (never 0.0) when neither carries an in-window sample. The per-peer
  # assertion below also proves ANSI-stripping and since-window honoring in
  # one shot (an older, larger-valued line outside the window must not win
  # over a smaller in-window value) — mirrors the real tracing-formatter
  # shape (`\x1b[3melapsed_s\x1b[0m=\x1b[0m<n> ... recv_validation_receipt_received`).
  export LOCAL_DEV_DIR="$tmp/local-dev"; mkdir -p "$LOCAL_DEV_DIR"
  since_epoch=1755972000
  old_epoch=$((since_epoch - 3600))
  new_epoch=$((since_epoch + 60))
  old_ts="$(date -u -d "@$old_epoch" +%Y-%m-%dT%H:%M:%S)"
  new_ts="$(date -u -d "@$new_epoch" +%Y-%m-%dT%H:%M:%S)"
  printf '%s.000000Z  INFO \033[2mconductor\033[0m: \033[3melapsed_s\033[0m=\033[0m99.0 \033[3ma\033[0m=\033[0m"recv_validation_receipt_received"\n' "$old_ts" > "$LOCAL_DEV_DIR/.sandbox_run_log.jessica"
  printf '%s.123456Z  INFO \033[2mconductor\033[0m: \033[3melapsed_s\033[0m=\033[0m5.018487491 \033[3ma\033[0m=\033[0m"recv_validation_receipt_received"\n' "$new_ts" >> "$LOCAL_DEV_DIR/.sandbox_run_log.jessica"
  rout="$(receipt_max jessica "$since_epoch")"
  t "receipt_max reads the per-peer conductor log, strips ANSI, honors the since-window (got $rout)" '[ "$rout" = "5.0" ]'
  rm -f "$LOCAL_DEV_DIR/.sandbox_run_log.jessica"
  printf '%s.123456Z  INFO \033[2mconductor\033[0m: \033[3melapsed_s\033[0m=\033[0m7.5 \033[3ma\033[0m=\033[0m"recv_validation_receipt_received"\n' "$new_ts" > "$LOCAL_DEV_DIR/.sandbox_run_log"
  rout="$(receipt_max jessica "$since_epoch")"
  t "receipt_max falls back to the shared mesh-wide log when no per-peer log exists (got $rout)" '[ "$rout" = "7.5" ]'
  rm -f "$LOCAL_DEV_DIR/.sandbox_run_log"
  rout="$(receipt_max jessica "$since_epoch")"; rrc=$?
  t "receipt_max prints null (not 0.0), rc 0, when neither log exists (rc=$rrc, got $rout)" '[ "$rrc" -eq 0 ] && [ "$rout" = "null" ]'
  exit "$fail"
) || fail=1

# Pre-kill capture (Critical-2): recovery_capture_peer mirrors restart_storage's
# live branch in hc-mesh.sh — captures /proc/<pid>/environ + /proc/<pid>/exe
# BEFORE anything is killed or wiped — and refuses (rc 5) rather than proceed
# with no live pid and no prior capture to fall back on.
(
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
  export MESH_DIR="$tmp/mesh"
  set +e; RECOVERY_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery.sh"; set -e 2>/dev/null; set +e
  recovery_capture_peer testpeer "$$"; cap_rc=$?
  t "recovery_capture_peer captures a live pid's environ (rc=$cap_rc)" \
    '[ "$cap_rc" -eq 0 ] && [ -s "$MESH_DIR/storage-restart/testpeer.environ" ]'
  t "recovery_capture_peer captures a live pid's exe as an executable path" \
    '[ -x "$(cat "$MESH_DIR/storage-restart/testpeer.exe" 2>/dev/null)" ]'
  # Sentinel proves the refusal path touches nothing under MESH_DIR.
  echo MARKER > "$MESH_DIR/sentinel"
  recovery_capture_peer nopid-nocapture ""; ref_rc=$?
  t "recovery_capture_peer refuses with no live pid and no capture (rc=$ref_rc)" \
    '[ "$ref_rc" -eq 5 ] && [ ! -s "$MESH_DIR/storage-restart/nopid-nocapture.environ" ]'
  t "refusal path leaves the rest of MESH_DIR untouched (sentinel intact)" \
    '[ "$(cat "$MESH_DIR/sentinel")" = MARKER ]'
  exit "$fail"
) || fail=1

# Record writer tolerates null receipt values (Important-1) instead of raising
# inside float(...) and silently dropping the whole JSONL line, and yields
# "unknown" for a missing zome verdict — factored out of main so this is
# testable without a live recovery.
(
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
  set +e; RECOVERY_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery.sh"; set -e 2>/dev/null; set +e
  out="$tmp/recovery-timeline.jsonl"
  recovery_write_record "$out" warm jessica matthew iroh libp2p 1 42 3 "" '{}' null null per-peer mesh-wide ""
  t "recovery_write_record: null receipts serialize as JSON null (not a raised exception)" \
    '[ "$(python3 -c "import json;d=json.load(open(\"$out\"));print(json.dumps(d[\"conductor_receipt_max_s\"]))")" = "{\"recovering\": null, \"survivor\": null}" ]'
  t "recovery_write_record: a missing zome verdict yields zome_path=unknown" \
    '[ "$(python3 -c "import json;print(json.load(open(\"$out\"))[\"zome_path\"])")" = "unknown" ]'
  exit "$fail"
) || fail=1

# Matrix: library parsing and role alternation are deterministic.
(
  set +e
  RECOVERY_MATRIX_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery-matrix.sh"
  t "library has 8 scenarios" '[ "$(matrix_scenarios | wc -l)" -eq 8 ]'
  t "fanout-2 declares 3 peers, no doorways" \
    '[ "$(matrix_scenarios | awk -F"\t" "\$1==\"fanout-2\"{print \$2\"/\"\$3}")" = "matthew,jessica,james/0" ]'
  t "run 1 recovers peer[1], run 2 recovers peer[0]" \
    '[ "$(matrix_recovering_index 1)" = 1 ] && [ "$(matrix_recovering_index 2)" = 0 ]'
  exit "$fail"
) || fail=1

# Scenario selection is exact and preserves library order.
(
  set +e
  MESH_RECOVERY_SCENARIOS=fanout-2,homo-dual RECOVERY_MATRIX_SOURCE_ONLY=1 \
    source "$here/../hc-mesh-recovery-matrix.sh"
  t "scenario filter is exact and library ordered" \
    '[ "$(matrix_scenarios | cut -f1 | paste -sd, -)" = "homo-dual,fanout-2" ]'
  exit "$fail"
) || fail=1

(
  set +e
  MESH_RECOVERY_SCENARIOS=does-not-exist RECOVERY_MATRIX_SOURCE_ONLY=1 \
    source "$here/../hc-mesh-recovery-matrix.sh"
  t "unknown scenario selection yields no rows" '[ "$(matrix_scenarios | wc -l)" -eq 0 ]'
  exit "$fail"
) || fail=1

# matrix_live_shape: an explicit override bypasses probing entirely (tests use it).
(
  set +e
  MESH_RECOVERY_LIVE_SHAPE="matthew,jessica/1" RECOVERY_MATRIX_SOURCE_ONLY=1 \
    source "$here/../hc-mesh-recovery-matrix.sh"
  out="$(matrix_live_shape)"; rc=$?
  t "matrix_live_shape honors MESH_RECOVERY_LIVE_SHAPE override (rc=$rc): '$out'" \
    '[ "$rc" -eq 0 ] && [ "$out" = "matthew,jessica/1" ]'
  exit "$fail"
) || fail=1

# matrix_live_shape: real probing path against guaranteed-closed ports (no
# override) reports no live mesh — this is the "first scenario reshapes"
# fallback the brief requires, exercised without touching any real mesh port.
(
  set +e
  unset MESH_RECOVERY_LIVE_SHAPE
  # export (not just prefix the source command): matrix_live_shape is called
  # AFTER source returns, and bash does not retain command-prefixed
  # assignments once the prefixed command (source) completes.
  export MESH_PEERS=nobody-a,nobody-b DOORWAY_PORT=1 MESH_RECOVERY_PROBE_BASE=65000
  RECOVERY_MATRIX_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery-matrix.sh"
  out="$(matrix_live_shape)"; rc=$?
  t "matrix_live_shape probes closed ports and reports no live mesh (rc=$rc): '$out'" \
    '[ "$rc" -eq 0 ] && [ -z "$out" ]'
  exit "$fail"
) || fail=1

# reshape_verify: a reshape is judged by whether the mesh SERVES, never by the
# prologue seeder's exit code (its post-flight probe is a known false red).
# MESH_RECOVERY_RESHAPE_VERIFY_STUB bypasses probing entirely for unit tests.
(
  set +e
  RECOVERY_MATRIX_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery-matrix.sh"
  out="$(MESH_RECOVERY_RESHAPE_VERIFY_STUB=ok reshape_verify x,y 1)"; rc=$?
  t "reshape_verify STUB=ok passes (rc=$rc): $out" \
    '[ "$rc" -eq 0 ] && [[ "$out" == *"reshape verified"* ]]'
  out="$(MESH_RECOVERY_RESHAPE_VERIFY_STUB=fail reshape_verify x,y 1)"; rc=$?
  t "reshape_verify STUB=fail fails (rc=$rc): $out" \
    '[ "$rc" -ne 0 ] && [[ "$out" == *"NOT serving"* ]]'
  exit "$fail"
) || fail=1

# reshape_verify: unstubbed against guaranteed-closed ports must be bounded by
# MESH_RECOVERY_RESHAPE_VERIFY_SECS, not hang or loop forever.
(
  set +e
  RECOVERY_MATRIX_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery-matrix.sh"
  unset MESH_RECOVERY_RESHAPE_VERIFY_STUB
  start="$(date +%s)"
  out="$(MESH_RECOVERY_PROBE_BASE=65000 MESH_RECOVERY_RESHAPE_VERIFY_SECS=6 reshape_verify x,y 1)"; rc=$?
  elapsed=$(( $(date +%s) - start ))
  t "reshape_verify bounded wait fails within ~6s on closed ports (rc=$rc, elapsed=${elapsed}s): $out" \
    '[ "$rc" -ne 0 ] && [ "$elapsed" -le 15 ] && [[ "$out" == *"NOT serving"* ]]'
  exit "$fail"
) || fail=1

exit "$fail"
