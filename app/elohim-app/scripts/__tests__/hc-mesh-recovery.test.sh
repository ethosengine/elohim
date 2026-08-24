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
  rm "$tmp/blob/sha256-aa"
  RECOVERY_DOORWAY_A="http://localhost:$port" RECOVERY_DOORWAY_B="http://localhost:$port" RECOVERY_LANDING_PATH="/db/stats" \
    out="$(recovery_predicate "$tmp/snap.json" "$port")"; rc=$?
  t "missing blob bytes fails ONLY P2 (rc=$rc): $out" '[ "$rc" -ne 0 ] && [ "$out" = "P0=1 P1=1 P2=0 P3=1 P4=1" ]'
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

exit "$fail"
