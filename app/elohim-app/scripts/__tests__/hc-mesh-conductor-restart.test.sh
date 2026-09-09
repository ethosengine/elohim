#!/usr/bin/env bash
# Source-mode coverage for conductor restart PID ownership. No mesh is started.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
pid_running() {
  [ -r "/proc/$1/stat" ] && [ "$(awk '{print $3}' "/proc/$1/stat" 2>/dev/null)" != Z ]
}

tmp="$(mktemp -d)"
pids=()
cleanup() {
  local pid
  for pid in "${pids[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  for pid in "${pids[@]}"; do
    wait "$pid" 2>/dev/null || true
  done
  rm -rf "$tmp"
}
trap cleanup EXIT

export MESH_DIR="$tmp/mesh"
export MESH_PEERS=unit
source "$here/../hc-mesh.sh" >/dev/null
LOCAL_DEV_DIR="$tmp/household"
foreign_dir="$tmp/foreign"
bin_dir="$tmp/bin"
mkdir -p "$LOCAL_DEV_DIR/unit" "$foreign_dir/unit" "$bin_dir"
: > "$LOCAL_DEV_DIR/unit/conductor-config.yaml"
: > "$foreign_dir/unit/conductor-config.yaml"
cp /bin/bash "$bin_dir/holochain"
cp /bin/bash "$bin_dir/hc"
cp "$(command -v tail)" "$bin_dir/awk"

start_shell_fixture() { # <exe> <cwd> <argv...>
  local exe="$1" cwd="$2"
  shift 2
  (
    cd "$cwd" || exit 1
    exec "$exe" -c \
      'sleep 300 & child=$!; trap '\''kill "$child" 2>/dev/null; exit 0'\'' TERM INT EXIT; wait "$child"' \
      "$@"
  ) >/dev/null 2>&1 &
  started_pid=$!
  pids+=("$started_pid")
}

# Two owned conductors: a direct holochain names this household's config, and
# an hc supervisor runs from this household root with the expected argv shape.
start_shell_fixture "$bin_dir/holochain" "$tmp" holochain \
  --config-path "$LOCAL_DEV_DIR/unit/conductor-config.yaml"
owned_holochain=$started_pid
start_shell_fixture "$bin_dir/hc" "$LOCAL_DEV_DIR" hc sandbox fixture run
owned_hc=$started_pid

# Executable names alone do not establish ownership when config/cwd belongs to
# a different household.
start_shell_fixture "$bin_dir/holochain" "$tmp" holochain \
  --config-path "$foreign_dir/unit/conductor-config.yaml"
foreign_holochain=$started_pid
start_shell_fixture "$bin_dir/hc" "$foreign_dir" hc sandbox fixture run
foreign_hc=$started_pid

# These argv strings nominate candidates for the old process-text search, but
# /proc/exe proves neither process is a conductor executable.
bash -c 'exec -a "hc sandbox fixture run" sleep 300' >/dev/null 2>&1 &
sleep_decoy=$!
pids+=("$sleep_decoy")
: > "$tmp/hc sandbox fixture run"
(
  cd "$LOCAL_DEV_DIR" || exit 1
  exec "$bin_dir/awk" -f "$tmp/hc sandbox fixture run"
) >/dev/null 2>&1 &
awk_decoy=$!
pids+=("$awk_decoy")

# Allow every exec to settle before reading /proc and pgrep.
for _ in {1..100}; do
  ready=1
  for pid in "${pids[@]}"; do pid_running "$pid" || ready=0; done
  [ "$ready" -eq 1 ] && break
  sleep 0.02
done

set -o pipefail
selected="$(conductor_restart_pids | sort -n)"; selected_rc=$?
t "helper succeeds under inherited pipefail" '[ "$selected_rc" -eq 0 ]'
t "no conductor candidates is a clean empty selection" \
  '( fallback_pattern_pids() { return 0; }; empty="$(conductor_restart_pids)"; [ $? -eq 0 ] && [ -z "$empty" ]; )'
t "owned holochain config is selected" 'grep -qx "$owned_holochain" <<<"$selected"'
t "owned hc supervisor cwd is selected" 'grep -qx "$owned_hc" <<<"$selected"'
t "only the two owned conductor processes are selected" '[ "$(wc -w <<<"$selected")" -eq 2 ]'
t "foreign-household conductor executables are excluded" \
  '! grep -qx "$foreign_holochain" <<<"$selected" && ! grep -qx "$foreign_hc" <<<"$selected"'
t "argv-only sleep and awk decoys are excluded" \
  '! grep -qx "$sleep_decoy" <<<"$selected" && ! grep -qx "$awk_decoy" <<<"$selected"'
t "selection does not stop any fixture process" \
  'for pid in "${pids[@]}"; do pid_running "$pid" || exit 1; done'

exit "$fail"
