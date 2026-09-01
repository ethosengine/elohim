#!/usr/bin/env bash
# Sourced-mode fixture for the orphaned-live-conductor guard. It uses a real
# process with a real open file, but no Holochain binary or mesh ports.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

tmp="$(mktemp -d)"
fixture_pid=""
cleanup() {
  [[ "$fixture_pid" =~ ^[0-9]+$ ]] && kill "$fixture_pid" 2>/dev/null || true
  rm -rf "$tmp"
}
trap cleanup EXIT

export MESH_DIR="$tmp/mesh"
export MESH_PEERS=unit
source "$here/../hc-mesh.sh" >/dev/null

sandbox="$tmp/sandbox"
mkdir -p "$sandbox/databases"
: > "$sandbox/databases/wasm"
bash -c 'exec 9<> "$1"; exec sleep 300' _ "$sandbox/databases/wasm" &
fixture_pid=$!
sleep 0.1

out="$(check_conductor_data_root unit "$fixture_pid" "$sandbox" 2>&1)"; rc=$?
t "a live process with a linked sandbox is healthy" \
  '[ "$rc" -eq 0 ] && [ -z "$out" ]'

rm -rf "$sandbox"
out="$(check_conductor_data_root unit "$fixture_pid" "$sandbox" 2>&1)"; rc=$?
t "deleting a live process sandbox names orphaned-data-root" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"state=orphaned-data-root"* ]]'
t "orphan diagnostics name the denied path and its mode" \
  '[[ "$out" == *"sandbox=$sandbox mode=missing"* ]] && [[ "$out" == *"handle=/proc/$fixture_pid/fd/9"* ]]'

conductor_pid_for_index() { echo "$fixture_pid"; }
ss() { return 1; }
curl() { return 1; }
mesh_footprint() { :; }
hc() { echo 'holochain_cli 0.6.0'; }
holochain() { echo 'holochain 0.6.0'; }
out="$(status_all 2>&1)"; rc=$?
t "status returns unhealthy and names the orphan" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"conductor data roots:"* ]] && [[ "$out" == *"state=orphaned-data-root"* ]]'

out="$(mesh_coordswap --not-reached 2>&1)"; rc=$?
t "coordswap preflight refuses the orphan" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"REFUSING coordswap: state=orphaned-data-root"* ]]'
t "refusal leaves the live process untouched and names remediation" \
  'kill -0 "$fixture_pid" && [[ "$out" == *"hc-mesh.sh stop && ./hc-mesh.sh start"* ]]'

exit "$fail"
