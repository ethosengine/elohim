#!/usr/bin/env bash
# Exercise the real late-join lifecycle with a storage-owned app interface.
# Source-mode fakes never launch a mesh or touch an incumbent data root.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export MESH_DIR="$tmp/mesh"
source "$here/../hc-mesh.sh" >/dev/null
LOCAL_DEV_DIR="$tmp/local-dev"
PID_DIR="$MESH_DIR/pids"
LOGDIR="$MESH_DIR/logs"
STORAGE_BIN=/bin/true
HAPP_PATH="$tmp/fixture.happ"
MESH_CONDUCTOR_LAUNCH=ark
touch "$HAPP_PATH"
guard_conductor_data_roots() { return 0; }
listener_pids_for_ports() { return 0; }
assert_launch_prerequisites() { return 0; }
assert_storage_transport_capability() { return 0; }
mesh_network_args() { echo isolated; }
patch_mesh_gossip_config() { return 0; }
record_listener_pid() { return 0; }
refresh_mesh_pidfiles() { return 0; }
timeout() {
  mkdir -p "$LOCAL_DEV_DIR/daniel"
  printf '%s\n' "$LOCAL_DEV_DIR/daniel" >> "$LOCAL_DEV_DIR/.hc"
  touch "$LOCAL_DEV_DIR/daniel/conductor-config.yaml"
}
launch_ark_conductor() { touch "$case_dir/admin-started"; }
start_storage_peer() {
  [ -f "$case_dir/admin-started" ] || return 1
  touch "$case_dir/storage-started"
}
# Advance the lifecycle's monotonic loop clock instead of waiting three minutes.
sleep() { SECONDS=$((SECONDS + 181)); }
ss() {
  case "$*" in
    *4474*) [ "$scenario" != no-admin ] && [ -f "$case_dir/admin-started" ] && echo listening ;;
    *4475*) [ "$scenario" != no-app ] && [ -f "$case_dir/storage-started" ] && echo listening ;;
    *) echo listening ;;
  esac
}
curl() {
  case "$*" in
    *8093/health*)
      [ "$scenario" != no-health ] && [ -f "$case_dir/storage-started" ] || return 7
      # A real curl returns success for HTTP 503 unless --fail/-f is enabled.
      if [ "$scenario" = http-error ]; then
        case "$*" in *-f*|*--fail*) return 22 ;; esac
      fi
      return 0 ;;
    *8093/p2p/status*) printf '{"irohNodeId":"%064d"}\n' 1 ;;
    *) return 0 ;;
  esac
}
fail=0
for scenario in ready no-admin no-app no-health http-error; do
  case_dir="$tmp/$scenario"
  mkdir -p "$case_dir" "$LOCAL_DEV_DIR" "$PID_DIR" "$LOGDIR"
  rm -rf "$LOCAL_DEV_DIR/daniel"
  printf '%s\n' "$LOCAL_DEV_DIR/matthew" "$LOCAL_DEV_DIR/jessica" "$LOCAL_DEV_DIR/james" > "$LOCAL_DEV_DIR/.hc"
  touch "$MESH_DIR/peer-policy.toml"
  PEERS=(matthew jessica james)
  join_peer daniel > "$case_dir/result.log" 2>&1
  rc=$?
  if [ "$scenario" = ready ]; then
    if [ "$rc" -eq 0 ] && grep -q '^JOINED_PEER name=daniel ' "$case_dir/result.log"; then
      echo 'ok storage can attach the app interface after the admin interface is ready'
    else
      echo 'FAIL storage-owned app interface never reached readiness'
      cat "$case_dir/result.log"
      fail=1
    fi
  elif [ "$rc" -ne 0 ] && ! grep -q '^JOINED_PEER ' "$case_dir/result.log"; then
    echo "ok $scenario refuses a successful join receipt"
  else
    echo "FAIL $scenario accepted an incomplete late joiner"
    fail=1
  fi
  if [ "$scenario" = no-admin ] && [ -f "$case_dir/storage-started" ]; then
    echo 'FAIL storage launched without its admin prerequisite'
    fail=1
  fi
done
exit "$fail"
