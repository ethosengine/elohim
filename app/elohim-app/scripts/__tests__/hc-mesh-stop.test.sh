#!/usr/bin/env bash
# Source-mode coverage for PID/port-scoped mesh shutdown. No real mesh needed.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
pid_running() {
  [ -r "/proc/$1/stat" ] && [ "$(awk '{print $3}' "/proc/$1/stat" 2>/dev/null)" != Z ]
}

tmp="$(mktemp -d)"
recorded_pid="" port_pid="" stale_pid="" decoy_pid=""
cleanup() {
  local pid
  for pid in "$recorded_pid" "$port_pid" "$stale_pid" "$decoy_pid"; do
    [[ "$pid" =~ ^[0-9]+$ ]] && kill "$pid" 2>/dev/null || true
  done
  rm -rf "$tmp"
}
trap cleanup EXIT

export MESH_DIR="$tmp/mesh"
export MESH_PEERS=unit
export DOORWAY_PORT=18888
export DOORWAY_B_PORT=18889
export DOORWAY_A_HEALTH_PORT=18079
export DOORWAY_B_HEALTH_PORT=18089
export MONGO_PORT=17017
source "$here/../hc-mesh.sh" >/dev/null

# A launch record carries both PID and process-start identity, and resolves
# only while that exact process is alive.
sleep 300 & recorded_pid=$!
record_mesh_pid storage unit "$recorded_pid"
recorded="$(recorded_mesh_pids)"
t "recorded PID resolves while start identity matches" \
  '[ "$recorded" = "$recorded_pid" ] && [ "$(wc -w < "$PID_DIR/storage-unit")" -eq 2 ]'

# A stale/reused PID file must be discarded without touching the live process
# that currently owns that numeric PID.
sleep 300 & stale_pid=$!
mkdir -p "$PID_DIR"
printf '%s %s\n' "$stale_pid" 0 > "$PID_DIR/storage-stale"
recorded="$(recorded_mesh_pids)"
t "stale start identity is discarded, not trusted" \
  '! grep -qw "$stale_pid" <<<"$recorded" && [ ! -e "$PID_DIR/storage-stale" ] && pid_running "$stale_pid"'

# Stub only the listener inventory: it exposes port_pid on one configured mesh
# port and stops reporting it as soon as the process is dead/zombie.
sleep 300 & port_pid=$!
ss() {
  if [[ "$*" == *"sport = :$DOORWAY_PORT"* ]] && pid_running "$port_pid"; then
    case "${2:-}" in
      -ltnp) printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:* users:(("sleep",pid=%s,fd=3))\n' "$DOORWAY_PORT" "$port_pid" ;;
      -ltn)  printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:*\n' "$DOORWAY_PORT" ;;
    esac
  fi
}

port_resolved="$(listener_pids_for_ports "$DOORWAY_PORT")"
t "configured listener port resolves its owning PID" '[ "$port_resolved" = "$port_pid" ]'

# This argv triggers the legacy pgrep pattern, but /proc/exe proves it is only
# sleep. The validated fallback must never nominate it, which preserves shells
# and tools whose command text happens to mention a service path.
bash -c 'exec -a "elohim-storage --http-port 65500" sleep 300' & decoy_pid=$!
sleep 0.1
fallback="$(fallback_pattern_pids)"
t "pattern fallback rejects argv-only service-name matches" \
  '! grep -qw "$decoy_pid" <<<"$fallback" && pid_running "$decoy_pid"'

out="$(stop_all 2>&1)"; rc=$?
t "stop terminates both recorded and configured-port owners (rc=$rc)" \
  '[ "$rc" -eq 0 ] && ! pid_running "$recorded_pid" && ! pid_running "$port_pid"'
t "stop leaves an argv-only decoy and its caller alive" \
  'pid_running "$decoy_pid" && [[ "$out" != *"process-name fallback"* ]]'
t "stop clears the PID registry after shutdown" '[ ! -d "$PID_DIR" ]'

exit "$fail"
