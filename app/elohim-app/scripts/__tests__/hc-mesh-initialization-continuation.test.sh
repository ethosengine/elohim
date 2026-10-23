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
# Advance the shell clock rather than sleeping; exercise the actual observer.
# Multiple peers charge their curl time against ONE deadline.
PEERS=(one two three)
SECONDS=0
curl_calls=0
curl() {
  [ "$1" = --fail ] && [ "$2" = --silent ] && [ "$3" = --max-time ] || return 99
  [ "$4" -ge 1 ] && [ "$4" -le 2 ] || return 99
  curl_calls=$((curl_calls+1))
  SECONDS=$((SECONDS + 1))
  [ "$SECONDS" -ge 65 ]
}
sleep() { SECONDS=$((SECONDS + $1)); }
storage_ready_within 900
[ "$SECONDS" -gt 60 ] && [ "$SECONDS" -lt 900 ] && [ "$curl_calls" -gt 3 ]
SECONDS=0
curl() {
  [ "$4" -le 2 ] || return 99
  SECONDS=$((SECONDS + $4))
  return 1
}
refuses storage_ready_within 7
[ "$SECONDS" -eq 7 ]
SECONDS=0
curl() { SECONDS=$((SECONDS + $4)); return 0; }
refuses storage_ready_within 5
[ "$SECONDS" -eq 5 ]
echo 'ok readiness can exceed 60s; requests share deadline and late success cannot pass'
# The second attempt pins only observed hash/size/absence inventory. Fixtures
# hold synthetic credentials, never a real key or live conductor connection.
unset MESH_CONTINUE_INITIALIZATION_STATE_SHA256 MESH_CONTINUE_INITIALIZATION_LOG_SHA256
export MESH_DIR="$tmp/existing"
source "$here/../hc-mesh.sh" >/dev/null
PEERS=(matthew jessica james)
conductor_state_kind() { echo complete; }
archive_state_kind_at() { echo complete; }
live_pids_in_registry() { :; }
mesh_ports_busy() { return 1; }
assert_no_live_peer_processes() { return 0; }
python3 - "$MESH_DIR" <<'PYFIXTURE'
import pathlib,sys,json,base64
r=pathlib.Path(sys.argv[1]); j=r/'initialization-continuation'; (j/'consumed').mkdir(parents=True)
(j/'start.log').write_text('original interrupted boot\n'); (j/'invocation').write_text('original\n')
(r/'logs').mkdir(); (r/'logs/start.log').write_text('REFUSED readiness incomplete\n')
rows=[]
for i,p in enumerate(['matthew','jessica','james']):
 rawagent=b'\x84\x20\x24'+bytes([i+1])*36; rawdna=b'\x84\x2d\x24'+bytes([4])*36
 enc=lambda b:'u'+base64.urlsafe_b64encode(b).decode().rstrip('=')
 agent=enc(rawagent); dna=enc(rawdna)
 rows.append({'peer':p,'agent':agent,'cells':{'infrastructure':[{'dna_hash':dna,'agent_pub_key':agent}], 'lamad':[{'dna_hash':enc(b'\x84\x2d\x24'+bytes([5])*36),'agent_pub_key':agent}]}})
 d=r/p; v=d/'closed-chain-fence/credentials'; v.mkdir(parents=True)
 (d/'content.db').write_bytes(b'db'); (d/'identity.key').write_bytes(b'original-key')
 (r/'logs'/f'{p}.log').write_text('prior storage log\n')
 cred={'signingAgentKey':'84'*39,'keypair':'01'*32,'capSecret':'02'*64,'mintedAt':1}
 (v/(rawdna.hex()+'-'+rawagent.hex()+'.json')).write_text(json.dumps(cred))
(j/'apps-before.jsonl').write_text(''.join(json.dumps(v,sort_keys=True,separators=(',',':'))+'\n' for v in rows))
PYFIXTURE
export MESH_CONTINUE_INITIALIZATION_LOG_SHA256="$(sha256sum "$MESH_DIR/initialization-continuation/start.log" | cut -d' ' -f1)"
initialization_existing_state inventory > "$tmp/reviewed.json"
export MESH_CONTINUE_INITIALIZATION_STATE_SHA256="$(sha256sum "$tmp/reviewed.json" | cut -d' ' -f1)"
[ "$(household_start_mode)" = continue-existing-initialization ]
[ ! -e "$MESH_DIR/initialization-continuation/resume-existing" ]
cp "$MESH_DIR/matthew/identity.key" "$tmp/key-copy"
printf changed > "$MESH_DIR/matthew/identity.key"
refuses household_start_mode
cp "$tmp/key-copy" "$MESH_DIR/matthew/identity.key"
ln -s "$tmp/key-copy" "$MESH_DIR/matthew/iroh.key"
refuses household_start_mode
rm "$MESH_DIR/matthew/iroh.key"
credential_file=("$MESH_DIR/matthew/closed-chain-fence/credentials/"*.json)
cp "${credential_file[0]}" "$tmp/credential-copy"
printf '{}' > "${credential_file[0]}"
refuses initialization_existing_state inventory
cp "$tmp/credential-copy" "${credential_file[0]}"
cp "${credential_file[0]}" "${credential_file[0]}.extra"
refuses initialization_existing_state inventory
rm "${credential_file[0]}.extra"
reserve_initialization_existing
cmp "$tmp/reviewed.json" "$MESH_DIR/initialization-continuation/resume-existing/inventory.json"
backup_key="$MESH_DIR/initialization-continuation/resume-existing/backup/matthew/identity.key"
printf tampered > "$backup_key"
refuses household_start_mode
cp "$tmp/key-copy" "$backup_key"
cp "$MESH_DIR/initialization-continuation/resume-existing/inventory.json" "$tmp/saved-inventory"
printf '{}' > "$MESH_DIR/initialization-continuation/resume-existing/inventory.json"
refuses household_start_mode
cp "$tmp/saved-inventory" "$MESH_DIR/initialization-continuation/resume-existing/inventory.json"
: > "$LOGDIR/start.log"
[ "$(household_start_mode)" = continue-existing-initialization ]
refuses reserve_initialization_existing
mkdir "$MESH_DIR/initialization-continuation/resume-existing/consumed"
unset MESH_EXISTING_INITIALIZATION_INVOCATION
refuses household_start_mode
cp "$MESH_DIR/initialization-continuation/apps-before.jsonl" "$tmp/original-apps"
initialization_app_identities() { printf 'changed\n'; }
refuses verify_existing_initialization_apps
cmp "$tmp/original-apps" "$MESH_DIR/initialization-continuation/apps-before.jsonl"
initialization_app_identities() { cat "$tmp/original-apps"; }
verify_existing_initialization_apps
for peer in "${PEERS[@]}"; do printf '%032d' 0 > "$MESH_DIR/$peer/iroh.key"; done
printf short > "$MESH_DIR/matthew/iroh.key"
refuses initialization_existing_state verify
printf '%032d' 0 > "$MESH_DIR/matthew/iroh.key"
python3 - "$MESH_DIR" <<'PYNEWROLE'
import base64,json,pathlib,sys
r=pathlib.Path(sys.argv[1])
raw=lambda s:base64.urlsafe_b64decode(s[1:]+'='*((-len(s[1:]))%4)).hex()
for row in map(json.loads,(r/'initialization-continuation/apps-before.jsonl').read_text().splitlines()):
 v=r/row['peer']/'closed-chain-fence/credentials'; cell=row['cells']['lamad'][0]
 (v/(raw(cell['dna_hash'])+'-'+raw(row['agent'])+'.json')).write_bytes(next(v.iterdir()).read_bytes())
PYNEWROLE
initialization_existing_state verify
printf changed > "${credential_file[0]}"
refuses initialization_existing_state verify
cmp "$tmp/credential-copy" "$MESH_DIR/initialization-continuation/resume-existing/backup/${credential_file[0]#"$MESH_DIR/"}"
cp "$tmp/credential-copy" "${credential_file[0]}"
storage_state_kind() { echo complete; }
finish_initialization_continuation
initialization_continuation_ready
cp "$MESH_DIR/initialization-continuation/resume-existing/after-state.json" "$tmp/saved-after"
printf '{}' > "$MESH_DIR/initialization-continuation/resume-existing/after-state.json"
refuses initialization_continuation_ready
cp "$tmp/saved-after" "$MESH_DIR/initialization-continuation/resume-existing/after-state.json"
# Historical proof is not authority over later live vault/ledger evolution.
printf changed-by-later-authorized-operation > "${credential_file[0]}"
printf '{"later":"closure"}' > "$MESH_DIR/matthew/closed-chain-fence/closed-cells.json"
unset MESH_CONTINUE_INITIALIZATION_STATE_SHA256 MESH_CONTINUE_INITIALIZATION_LOG_SHA256
initialization_continuation_ready
rm "$MESH_DIR/initialization-continuation/resume-existing/complete"
refuses initialization_continuation_ready
echo 'ok reviewed second attempt preserves original evidence, refuses drift/replay, and requires both seals'

# Read bound is applied before loading private bytes.
python3 - "$MESH_DIR/initialization-continuation/apps-before.jsonl" <<'PYBOUND'
import pathlib,sys
pathlib.Path(sys.argv[1]).write_bytes(b'x'*65537)
PYBOUND
refuses initialization_existing_state proof
# Failed post-preflight mode classification preserves the start log and never launches.
(
  printf 'reviewed log retained\n' > "$LOGDIR/start.log"
  cp "$LOGDIR/start.log" "$tmp/log-before-mode-refusal"
  load_canonical_household_binding() { :; }
  maybe_migrate_legacy_mesh_dir() { :; }
  maybe_migrate_legacy_conductors() { :; }
  preflight() { :; }
  household_start_mode() { return 1; }
  export MESH_FOREGROUND=0
  refuses start_detached
  cmp "$LOGDIR/start.log" "$tmp/log-before-mode-refusal"
)
echo 'ok bounded reads, historical proof, prelaunch backups, and post-preflight refusal'
