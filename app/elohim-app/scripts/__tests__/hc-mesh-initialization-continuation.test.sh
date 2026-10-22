#!/usr/bin/env bash
# Fault fixtures exercise the real admission/reservation and CLI JSON parser.
# No mesh process is launched and no live identity is accessed.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export MESH_DIR="$tmp/mesh" MESH_PEERS=unit MESH_RESET=0
source "$here/../hc-mesh.sh" >/dev/null
PEERS=(unit)
mkdir -p "$LOGDIR" "$PID_DIR"
printf 'reviewed interrupted initialization\n' > "$LOGDIR/start.log"
export MESH_CONTINUE_INITIALIZATION_LOG_SHA256="$(sha256sum "$LOGDIR/start.log" | cut -d' ' -f1)"
conductor_state_kind() { echo complete; }
archive_state_kind_at() { echo complete; }
live_pids_in_registry() { :; }
mesh_ports_busy() { return 1; }
assert_no_live_peer_processes() { return 0; }
ss() { return 1; }
refuses() { if "$@" >/dev/null 2>&1; then echo "FAIL expected refusal: $*"; exit 1; fi; }
[ "$(household_start_mode)" = continue-initialization ]
[ ! -e "$MESH_DIR/initialization-continuation" ]
echo 'ok mode inquiry is read-only'
old="$MESH_CONTINUE_INITIALIZATION_LOG_SHA256"
export MESH_CONTINUE_INITIALIZATION_LOG_SHA256="$(printf wrong | sha256sum | cut -d' ' -f1)"
refuses household_start_mode
export MESH_CONTINUE_INITIALIZATION_LOG_SHA256="$old"
mkdir "$MESH_DIR/unit"
refuses household_start_mode
rmdir "$MESH_DIR/unit"
mkdir "$MESH_DIR/storage-restart"
refuses household_start_mode
rmdir "$MESH_DIR/storage-restart"
touch "$LOGDIR/unit.log"
refuses household_start_mode
rm "$LOGDIR/unit.log"
ln -s "$tmp/missing" "$LOGDIR/unit.log"
refuses household_start_mode
rm "$LOGDIR/unit.log"
ln -s "$tmp/missing" "$MESH_DIR/storage-restart"
refuses household_start_mode
rm "$MESH_DIR/storage-restart"
touch "$PID_DIR/storage-unit"
refuses household_start_mode
rm "$PID_DIR/storage-unit"
mesh_ports_busy() { return 0; }
refuses household_start_mode
mesh_ports_busy() { return 1; }
echo 'ok wrong hash, partial storage, witnesses and live ports refuse'
reserve_initialization_continuation
cmp "$LOGDIR/start.log" "$MESH_DIR/initialization-continuation/start.log"
: > "$LOGDIR/start.log"
[ "$(household_start_mode)" = continue-initialization ]
[ "$(stat -c %a "$MESH_DIR/initialization-continuation")" = 700 ]
refuses reserve_initialization_continuation
mkdir "$MESH_DIR/initialization-continuation/consumed"
refuses household_start_mode
unset MESH_INITIALIZATION_INVOCATION
refuses household_start_mode
echo 'ok retained log survives truncation; consumed or interrupted attempt cannot replay'
# JSON shape is the pinned hc client app_info_to_base64_json contract:
# AppStatus and CellInfo use serde type/value tagged enums; the CLI converts
# native tuple CellId into {dna_hash,agent_pub_key} (hc_client calls.rs:625).
hc() { cat <<'JSON'
[{"installed_app_id":"elohim","status":{"type":"enabled"},"agent_pub_key":"uhCAk-test","cell_info":{"role":[{"type":"provisioned","value":{"cell_id":{"dna_hash":"uhC0k-test","agent_pub_key":"uhCAk-test"}}}]}}]
JSON
}
timeout() {
  [ "$1" = --kill-after=5s ] && [ "$2" = 30s ] || return 1
  shift 2
  "$@"
}
STEWARD_APP_ID=elohim
initialization_app_identities > "$tmp/before"
initialization_app_identities > "$tmp/after"
cmp "$tmp/before" "$tmp/after"
# Exercise malformed raw-API tuple, missing DNA, and mismatched cell agent.
hc() { cat "$tmp/cli-app.json"; }
for cell in '["uhC0k-test","uhCAk-test"]' '{"agent_pub_key":"uhCAk-test"}' '{"dna_hash":"uhC0k-test","agent_pub_key":"uhCAk-other"}'; do
  printf '[{"installed_app_id":"elohim","status":{"type":"enabled"},"agent_pub_key":"uhCAk-test","cell_info":{"role":[{"type":"provisioned","value":{"cell_id":%s}}]}}]' "$cell" > "$tmp/cli-app.json"
  refuses initialization_app_identities
done
printf '%s' '[{"installed_app_id":"elohim","status":{"type":"enabled"},"agent_pub_key":"uhCAk-test","cell_info":{"role":[{"type":"provisioned","value":{"cell_id":{"dna_hash":"uhC0k-changed","agent_pub_key":"uhCAk-test"}}}]}}]' > "$tmp/cli-app.json"
initialization_app_identities > "$tmp/dna-drift"
refuses cmp -s "$tmp/before" "$tmp/dna-drift"
hc() { printf '%s' '[{"installed_app_id":"elohim","status":{"type":"enabled"},"agent_pub_key":"uhCAk-test","cell_info":{}}]'; }
refuses initialization_app_identities
hc() { printf '%s' '[{"installed_app_id":"elohim","status":{"type":"disabled"}}]'; }
refuses initialization_app_identities
hc() { printf '%s' '[]'; }
refuses initialization_app_identities
echo 'ok installed app/cell/agent checks reject missing and disabled apps'
# Completion must compare identities and refuse until durable storage exists.
cp "$tmp/before" "$MESH_DIR/initialization-continuation/apps-before.jsonl"
initialization_app_identities() { cat "$tmp/after"; }
storage_state_kind() { echo partial; }
refuses finish_initialization_continuation
[ ! -e "$MESH_DIR/initialization-continuation/complete" ]
storage_state_kind() { echo complete; }
cp "$tmp/dna-drift" "$tmp/after"
refuses finish_initialization_continuation
[ ! -e "$MESH_DIR/initialization-continuation/complete" ]
cp "$tmp/before" "$tmp/after"
finish_initialization_continuation
[ "$(cat "$MESH_DIR/initialization-continuation/complete")" = "$old" ]
[ "$(household_start_mode)" = resume ]
echo 'ok completion requires durable storage and unchanged identities; completed state resumes normally'
# Healthy sockets cannot race the detached identity comparison/seal.
MONGOD_BIN="" MESH_DOORWAYS_EFFECTIVE=0 MESH_PORTAL=0
curl() { return 0; }
ss() { echo LISTEN; }
unset MESH_CONTINUE_INITIALIZATION_LOG_SHA256
mv "$MESH_DIR/initialization-continuation/complete" "$tmp/complete"
refuses wait_all --timeout 0
printf 'REFUSED installed identities changed\n' > "$LOGDIR/start.log"
refuses wait_all --timeout 0
mv "$tmp/complete" "$MESH_DIR/initialization-continuation/complete"
# A completed receipt must not hide a refusal from this start.
refuses wait_all --timeout 0
: > "$LOGDIR/start.log"
wait_all --timeout 0
printf 'changed retained evidence\n' >> "$MESH_DIR/initialization-continuation/start.log"
refuses wait_all --timeout 0
refuses household_start_mode
echo 'ok healthy services wait for valid retained completion proof, including without opt-in; refusal remains fatal'
