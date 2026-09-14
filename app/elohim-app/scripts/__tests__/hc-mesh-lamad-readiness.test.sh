#!/usr/bin/env bash
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
export MESH_DIR="$tmp/mesh" MESH_PEERS=unit
source "$here/../hc-mesh.sh" >/dev/null
PEERS=(unit); http_port() { echo 18090; }; sleep() { SECONDS=$((SECONDS + 2)); }
mock_body='{"headActionHash":"uhCkk-ready","record":"eA=="}'; mock_status=200
curl() {
  local out="" arg
  while [ $# -gt 0 ]; do arg="$1"; shift; [ "$arg" = -o ] && { out="$1"; shift; }; done
  printf '%s' "$mock_body" > "$out"; printf '%s' "$mock_status"
}
fail=0
check() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
check "strict structural response passes" 'wait_for_lamad_call_readiness 1 >/dev/null 2>&1'
mock_body='{"error":"CellDisabled"}'; mock_status=503
check "CellDisabled refuses at the shared deadline" '! wait_for_lamad_call_readiness 1 >/dev/null 2>&1'
mock_body='{"error":"false 200"}'; mock_status=200
check "false-200 error JSON refuses" '! wait_for_lamad_call_readiness 1 >/dev/null 2>&1'
mock_body='{"headActionHash":"uhCkk-ready"}'
check "missing record refuses" '! wait_for_lamad_call_readiness 1 >/dev/null 2>&1'
PEERS=()
check "missing peer configuration refuses" '! wait_for_lamad_call_readiness 1 >/dev/null 2>&1'
MESH_ALLOW_NO_PROLOGUE=1
check "explicit no-prologue mode bypasses the gate" 'wait_for_lamad_call_readiness 1 >/dev/null 2>&1'
exit "$fail"
