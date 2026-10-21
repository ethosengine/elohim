#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export MESH_SCRIPT="$script_dir/hc-mesh.sh"
probe='source "$MESH_SCRIPT" >/dev/null; http_port 0; http_port 2'
actual="$(env -u MESH_STORAGE_BASE_PORT bash -c "$probe")"
[[ "$actual" == $'8090\n8092' ]]
actual="$(MESH_STORAGE_BASE_PORT=18090 bash -c "$probe")"
[[ "$actual" == $'18090\n18092' ]]
for invalid in 0 65536 08 -1 port; do
  if MESH_STORAGE_BASE_PORT="$invalid" bash -c 'source "$MESH_SCRIPT"' >/dev/null 2>&1; then
    echo "FAIL invalid storage port accepted: $invalid" >&2
    exit 1
  fi
done
if MESH_STORAGE_BASE_PORT=65535 bash -c 'source "$MESH_SCRIPT"; http_port 2' >/dev/null 2>&1; then
  echo 'FAIL derived storage port overflow accepted' >&2
  exit 1
fi
echo 'ok mesh storage ports stay separate from provider storage; invalid ports refuse'
actual="$(MESH_CONDUCTOR_BASE_PORT=14444 MESH_P2P_BASE_PORT=19701 MESH_METRICS_BASE_PORT=19464 bash -c 'source "$MESH_SCRIPT" >/dev/null; admin_port 2; app_port 2; p2p_port 2; conductor_metrics_port 2')"
[[ "$actual" == $'14464\n14465\n19703\n19466' ]]
if MESH_CONDUCTOR_BASE_PORT=65535 bash -c 'source "$MESH_SCRIPT"; app_port 0' >/dev/null 2>&1; then
  echo 'FAIL derived conductor app port overflow accepted' >&2
  exit 1
fi
echo 'ok owned mesh conductor, p2p, and metrics listeners can be isolated'
