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
recorded_pid="" port_pid="" stale_pid="" decoy_pid="" fault_pid=""
fixture_pids=()
cleanup() {
  local pid
  for pid in "$recorded_pid" "$port_pid" "$stale_pid" "$decoy_pid" "$fault_pid" "${fixture_pids[@]}"; do
    [[ "$pid" =~ ^[0-9]+$ ]] && kill "$pid" 2>/dev/null || true
  done
  wait 2>/dev/null || true
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

# Nominate only fixture processes: an unconditional sweep must never discover
# a real service on the host running this test. /proc validation stays real.
pgrep() { printf '%s\n' "${fixture_pids[@]}" "$decoy_pid"; }

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

# A prior late joiner is absent from PEERS and has no declared port or PID
# record. Its hc parent and the conductor must still be reaped after the
# non-empty recorded selection has exited and all declared ports are clear.
bin_dir="$tmp/bin"
foreign_dir="$MESH_DIR/conductors-foreign"
mkdir -p "$bin_dir" "$LOCAL_DEV_DIR/unit" "$LOCAL_DEV_DIR/extra" "$foreign_dir/extra"
cp /bin/bash "$bin_dir/holochain"
cp /bin/bash "$bin_dir/hc"
ln -s "$foreign_dir/extra" "$LOCAL_DEV_DIR/foreign-link"
printf '%s\n' "$LOCAL_DEV_DIR/unit" "$LOCAL_DEV_DIR/../conductors/extra" \
  "$foreign_dir/extra" "$LOCAL_DEV_DIR/foreign-link" > "$LOCAL_DEV_DIR/.hc"
shell_body='sleep 300 & child=$!; trap '\''kill "$child" 2>/dev/null; exit 0'\'' TERM INT EXIT; wait "$child"'
(
  cd "$LOCAL_DEV_DIR" || exit 1
  exec "$bin_dir/hc" -c '
    "$1" -c "$2" holochain --config-path ./extra/conductor-config.yaml & child=$!
    printf "%s\n" "$child" > "$3"
    trap '\''exit 0'\'' TERM INT
    wait "$child"
  ' 'hc sandbox fixture run' "$bin_dir/holochain" "$shell_body" "$tmp/extra.pid"
) >/dev/null 2>&1 &
owned_hc=$!
fixture_pids+=("$owned_hc")
for _ in {1..100}; do [ -s "$tmp/extra.pid" ] && break; sleep 0.02; done
extra_pid="$(cat "$tmp/extra.pid")"
fixture_pids+=("$extra_pid")
"$bin_dir/holochain" -c "$shell_body" holochain --config-path "$foreign_dir/extra/conductor-config.yaml" >/dev/null 2>&1 &
foreign_pid=$!
fixture_pids+=("$foreign_pid")
"$bin_dir/holochain" -c "$shell_body" holochain --config-path "$LOCAL_DEV_DIR/extra/conductor-config.yaml.other" >/dev/null 2>&1 &
suffix_pid=$!
fixture_pids+=("$suffix_pid")
bash -c "$shell_body" holochain --config-path "$LOCAL_DEV_DIR/extra/conductor-config.yaml" >/dev/null 2>&1 &
conductor_decoy=$!
fixture_pids+=("$conductor_decoy")
sleep 300 & recorded_pid=$!
record_mesh_pid storage unit "$recorded_pid"
sleep 0.1
t "late-join shutdown starts with a recorded PID and clear declared ports" \
  '[ -n "$(recorded_mesh_pids)" ] && ! mesh_ports_busy && pid_running "$extra_pid" && [ "$(awk '\''{print $4}'\'' "/proc/$extra_pid/stat")" = "$owned_hc" ]'
out="$(stop_all 2>&1)"; rc=$?
t "stop sweeps the unrecorded saved conductor and its hc parent (rc=$rc)" \
  '[ "$rc" -eq 0 ] && ! pid_running "$recorded_pid" && ! pid_running "$extra_pid" && ! pid_running "$owned_hc" && [[ "$out" == *"mesh stopped"* ]]'
t "foreign saved paths, symlink escapes, config suffixes and argv decoys survive" \
  'pid_running "$foreign_pid" && pid_running "$suffix_pid" && pid_running "$conductor_decoy" && pid_running "$decoy_pid"'
t "complete ownership sweep clears bookkeeping" '[ ! -d "$PID_DIR" ]'

# Simulate an unkillable owned process without sending a signal. Stop must
# retain its receipt and name the survivor, never claim a successful shutdown.
sleep 300 & recorded_pid=$!
record_mesh_pid storage unit "$recorded_pid"
out="$(terminate_mesh_pids() { :; }; stop_all 2>&1)"; rc=$?
t "surviving owner refuses stop and retains bookkeeping" \
  '[ "$rc" -ne 0 ] && pid_running "$recorded_pid" && [ -f "$PID_DIR/storage-unit" ] && [[ "$out" == *"owned process pid=$recorded_pid remains"* ]] && [[ "$out" != *"mesh stopped"* ]]'
kill "$recorded_pid" 2>/dev/null || true
wait "$recorded_pid" 2>/dev/null || true
recorded_pid=""

# Fault stops must prove the saved pathname can recover the running process
# before replacing an existing receipt or signalling anything.
MESH_TRANSPORT_BACKEND_EFFECTIVE=libp2p
fault_bin="$tmp/storage-fixture"
cp /bin/bash "$fault_bin"
"$fault_bin" -c 'sleep 300 & child=$!; trap '\''kill "$child" 2>/dev/null'\'' EXIT; trap '\''exit 0'\'' TERM; wait "$child"' & fault_pid=$!
sleep 0.1
record_mesh_pid storage unit "$fault_pid"
capture_dir="$MESH_DIR/storage-restart"
mkdir -p "$capture_dir"
printf 'known-good-env' > "$capture_dir/unit.environ"
printf 'known-good-exe' > "$capture_dir/unit.exe"
receipts_unchanged() {
  [ "$(cat "$capture_dir/unit.environ")" = known-good-env ] &&
    [ "$(cat "$capture_dir/unit.exe")" = known-good-exe ]
}

# Atomic pathname replacement leaves /proc/PID/exe pointing at deleted bytes.
cp /bin/bash "$tmp/replacement"
printf 'different bytes' >> "$tmp/replacement"
mv "$tmp/replacement" "$fault_bin"
out="$(stop_storage unit 2>&1)"; rc=$?
t "replaced executable refuses fault without killing or overwriting receipts" \
  '[ "$rc" -ne 0 ] && pid_running "$fault_pid" && receipts_unchanged && [[ "$out" == *"differs from the running bytes"* ]]'
rm "$fault_bin"
out="$(stop_storage unit 2>&1)"; rc=$?
t "missing executable refuses fault and preserves known-good recovery" \
  '[ "$rc" -ne 0 ] && pid_running "$fault_pid" && receipts_unchanged'

# Restore identical bytes, then require dual: bash lacks the iroh marker.
cp /bin/bash "$fault_bin"
MESH_TRANSPORT_BACKEND_EFFECTIVE=dual
out="$(stop_storage unit 2>&1)"; rc=$?
t "incompatible transport refuses fault before receipt mutation" \
  '[ "$rc" -ne 0 ] && pid_running "$fault_pid" && receipts_unchanged && [[ "$out" == *"p2p-iroh marker"* ]]'
MESH_TRANSPORT_BACKEND_EFFECTIVE=libp2p

# The PID/proc identity functions do not need Python; fail the actual capture.
python3() { return 1; }
out="$(stop_storage unit 2>&1)"; rc=$?
unset -f python3
t "failed environment capture refuses fault and preserves prior receipt" \
  '[ "$rc" -ne 0 ] && pid_running "$fault_pid" && receipts_unchanged && [[ "$out" == *"could not be captured"* ]]'

out="$(stop_storage unit 2>&1)"; rc=$?
t "matching executable and captured environment permit owned fault" \
  '[ "$rc" -eq 0 ] && ! pid_running "$fault_pid" && [ -s "$capture_dir/unit.environ" ] && [ "$(cat "$capture_dir/unit.exe")" = "$fault_bin" ]'
kill "$fault_pid" 2>/dev/null || true
wait "$fault_pid" 2>/dev/null || true
fault_pid=""

exit "$fail"
