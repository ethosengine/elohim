#!/usr/bin/env bash
# Source-mode coverage for the persistent household root and reset boundary.
# No conductor, storage, doorway, or mesh port is launched.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../../../.." && pwd)"
mesh_script="$here/../hc-mesh.sh"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

tmp="$(mktemp -d)"
live_pid=""
cleanup() {
  [[ "$live_pid" =~ ^[0-9]+$ ]] && kill "$live_pid" 2>/dev/null || true
  rm -rf "$tmp"
}
trap cleanup EXIT

default_paths="$(env -u MESH_DIR -u MESH_RESET bash -c 'source "$1" >/dev/null; printf "%s|%s\n" "$MESH_DIR" "$LOCAL_DEV_DIR"' _ "$mesh_script")"
default_root="${default_paths%%|*}"
t "implicit mesh root is persistent and workspace-local" \
  '[ "$default_root" = "$repo_root/genesis/local-dev/household-dowell" ]'
t "implicit conductor state belongs to the same reusable fixture" \
  '[ "$default_paths" = "$repo_root/genesis/local-dev/household-dowell|$repo_root/genesis/local-dev/household-dowell/conductors" ]'

binding="$(env -u MESH_DIR -u MESH_RESET bash -c '
  source "$1" >/dev/null
  load_canonical_household_binding || exit
  printf "%s|%s|%s|%s\n" "$CANONICAL_HOUSEHOLD_ID" \
    "$(human_id matthew)" "$(human_id jessica)" "$(human_id james)"
' _ "$mesh_script")"
t "default runtime resolves the canonical Dowell human fixture" \
  '[ "$binding" = "household-dowell|human-matthew-manager|human-jessica-spouse|human-james-son" ]'

invalid_humans="$tmp/invalid-humans.json"
cat > "$invalid_humans" <<'JSON'
{"humans":[
  {"id":"human-matthew-manager","displayName":"Matthew","householdId":"household-dowell"},
  {"id":"human-jessica-spouse","displayName":"Jessica","householdId":"household-dowell"}
]}
JSON
out="$(env -u MESH_DIR -u MESH_RESET CANONICAL_HUMANS_PATH="$invalid_humans" bash -c '
  source "$1" >/dev/null
  CANONICAL_HUMANS_PATH="$2"
  load_canonical_household_binding
' _ "$mesh_script" "$invalid_humans" 2>&1)"; rc=$?
t "default runtime refuses a roster missing from canonical fixture authority" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"REFUSED canonical household binding"* ]] && [[ "$out" == *"james"* ]]'

override="$tmp/explicit"
explicit_root="$(MESH_DIR="$override" bash -c 'source "$1" >/dev/null; printf "%s\n" "$MESH_DIR"' _ "$mesh_script")"
t "an explicit MESH_DIR remains byte-for-byte unchanged" '[ "$explicit_root" = "$override" ]'

export MESH_DIR="$tmp/mesh"
export MESH_PEERS=unit
export MESH_RESET=0
source "$mesh_script" >/dev/null
LOCAL_DEV_DIR="$tmp/conductors"
MONGO_DIR="$MESH_DIR/mongo"
MONGOD_BIN=""
PID_DIR="$MESH_DIR/pids"
PEERS=(unit)
admin_ports_up=0
mesh_ports_up=0
ss() { [ "$admin_ports_up" = 1 ] && printf 'LISTEN\n' || return 1; }
mesh_ports_busy() { [ "$mesh_ports_up" = 1 ]; }
assert_no_live_peer_processes() { return 0; }

write_complete_conductor() {
  mkdir -p "$LOCAL_DEV_DIR/unit/ks" "$LOCAL_DEV_DIR/unit/databases"
  printf '%s\n' "$LOCAL_DEV_DIR/unit" > "$LOCAL_DEV_DIR/.hc"
  printf 'config\n' > "$LOCAL_DEV_DIR/unit/conductor-config.yaml"
  printf 'key\n' > "$LOCAL_DEV_DIR/unit/ks/store_file"
  printf 'db\n' > "$LOCAL_DEV_DIR/unit/databases/conductor.db"
}
write_complete_storage() {
  mkdir -p "$MESH_DIR/unit"
  printf 'db\n' > "$MESH_DIR/unit/content.db"
  printf 'key\n' > "$MESH_DIR/unit/identity.key"
  printf 'iroh-key\n' > "$MESH_DIR/unit/iroh.key"
}

t "no state selects a fresh household" '[ "$(household_start_mode)" = fresh ]'
mesh_ports_up=1
out="$(household_start_mode 2>&1)"; rc=$?
t "fresh recast refuses while another household service remains live" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"process or port is live"* ]]'
mesh_ports_up=0
write_complete_conductor
write_complete_storage
conductor_key_before="$(sha256sum "$LOCAL_DEV_DIR/unit/ks/store_file" | cut -d' ' -f1)"
storage_key_before="$(sha256sum "$MESH_DIR/unit/identity.key" | cut -d' ' -f1)"
t "complete stopped state selects resume" '[ "$(household_start_mode)" = resume ]'
t "resume classification leaves conductor and storage identities unchanged" \
  '[ "$(sha256sum "$LOCAL_DEV_DIR/unit/ks/store_file" | cut -d" " -f1)" = "$conductor_key_before" ] && [ "$(sha256sum "$MESH_DIR/unit/identity.key" | cut -d" " -f1)" = "$storage_key_before" ]'

rm "$MESH_DIR/unit/identity.key"
admin_ports_up=1
out="$(household_start_mode 2>&1)"; rc=$?
t "a running conductor still refuses missing storage identity" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"conductor=complete storage=partial"* ]] && [[ "$out" == *"missing storage state:"*"identity.key"* ]]'
t "a refused partial start preserves the surviving conductor key" \
  '[ "$(sha256sum "$LOCAL_DEV_DIR/unit/ks/store_file" | cut -d" " -f1)" = "$conductor_key_before" ]'

printf 'key\n' > "$MESH_DIR/unit/identity.key"
rm "$MESH_DIR/unit/iroh.key"
out="$(household_start_mode 2>&1)"; rc=$?
t "dual transport refuses a missing iroh identity" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"iroh.key (transport=dual)"* ]]'
mkdir -p "$MESH_DIR/storage-restart"
printf 'ELOHIM_TRANSPORT_BACKEND=libp2p\0' > "$MESH_DIR/storage-restart/unit.environ"
t "a captured libp2p-only runtime does not invent an iroh-key requirement" \
  '[ "$(storage_state_kind)" = complete ]'
rm -rf "$MESH_DIR/storage-restart"
printf 'iroh-key\n' > "$MESH_DIR/unit/iroh.key"
admin_ports_up=0

# A resumed household retains its doorway archive posture. Missing archive
# bytes are not treated as a new empty account store, while an explicit
# archive-less declaration remains a supported layout.
MONGOD_BIN=/bin/true
rm -f "$ARCHIVE_MODE_FILE"
rm -rf "$MONGO_DIR"
out="$(household_start_mode 2>&1)"; rc=$?
t "an unmarked missing archive refuses when mongod is available" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"archive=partial"* ]] && [[ "$out" == *"missing doorway archive declaration"* ]]'
printf 'archive-less\n' > "$ARCHIVE_MODE_FILE"
t "an explicitly archive-less household remains resumable" \
  '[ "$(household_start_mode)" = resume ]'
printf 'archive\n' > "$ARCHIVE_MODE_FILE"
out="$(household_start_mode 2>&1)"; rc=$?
t "an archive-backed household refuses missing Mongo state" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"missing doorway archive state:"*"WiredTiger"* ]]'
mkdir -p "$MONGO_DIR"
printf 'mongo-state\n' > "$MONGO_DIR/WiredTiger"
t "an archive-backed household with its DB and runtime remains resumable" \
  '[ "$(household_start_mode)" = resume ]'
MONGOD_BIN=""

MESH_RESET=1
t "the explicit reset flag selects reset" '[ "$(household_start_mode)" = reset ]'
reset_household_state >/dev/null
t "an explicit stopped reset removes both identity halves" \
  '[ ! -e "$LOCAL_DEV_DIR/unit" ] && [ ! -e "$LOCAL_DEV_DIR/.hc" ] && [ ! -e "$MESH_DIR/unit" ]'

# A matching PID/start-tick record is live kernel evidence and blocks reset.
write_complete_conductor
write_complete_storage
mkdir -p "$PID_DIR"
sleep 300 & live_pid=$!
printf '%s %s\n' "$live_pid" "$(process_start_ticks "$live_pid")" > "$PID_DIR/storage-unit"
out="$(reset_household_state 2>&1)"; rc=$?
t "explicit reset refuses while a recorded process is genuinely live" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"REFUSED MESH_RESET=1"* ]]'
t "a refused live reset preserves both identity halves" \
  '[ -s "$LOCAL_DEV_DIR/unit/ks/store_file" ] && [ -s "$MESH_DIR/unit/identity.key" ]'
kill "$live_pid" 2>/dev/null || true
wait "$live_pid" 2>/dev/null || true
live_pid=""

# Migration is default-only, stopped-only, and never overwrites a destination.
MESH_RESET=0
MESH_DIR_WAS_EXPLICIT=0
INTERIM_MESH_DIR="$tmp/interim"
LEGACY_MESH_DIR="$tmp/legacy"
MESH_DIR="$tmp/persistent"
PID_DIR="$MESH_DIR/pids"
MONGO_DIR="$MESH_DIR/mongo"
mkdir -p "$LEGACY_MESH_DIR/unit"
printf 'legacy\n' > "$LEGACY_MESH_DIR/unit/content.db"
printf '%s\n' "$LEGACY_MESH_DIR/unit/content.db" > "$LEGACY_MESH_DIR/captured.environ"
maybe_migrate_legacy_mesh_dir >/dev/null
t "stopped legacy state moves only into an absent persistent root" \
  '[ -s "$MESH_DIR/unit/content.db" ] && [ -L "$LEGACY_MESH_DIR" ]'
t "the compatibility symlink keeps captured absolute legacy paths usable" \
  '[ "$(cat "$(cat "$MESH_DIR/captured.environ")")" = legacy ]'

rm "$LEGACY_MESH_DIR"
rm -rf "$MESH_DIR"
mkdir -p "$LEGACY_MESH_DIR" "$MESH_DIR"
printf 'old\n' > "$LEGACY_MESH_DIR/witness"
printf 'new\n' > "$MESH_DIR/witness"
out="$(maybe_migrate_legacy_mesh_dir 2>&1)"; rc=$?
t "migration never overwrites when both roots exist" \
  '[ "$rc" -ne 0 ] && [ "$(cat "$LEGACY_MESH_DIR/witness")" = old ] && [ "$(cat "$MESH_DIR/witness")" = new ] && [[ "$out" == *"refusing to overwrite either"* ]]'

rm -rf "$LEGACY_MESH_DIR" "$MESH_DIR"
mkdir -p "$LEGACY_MESH_DIR/pids"
sleep 300 & live_pid=$!
printf '%s %s\n' "$live_pid" "$(process_start_ticks "$live_pid")" > "$LEGACY_MESH_DIR/pids/storage-unit"
out="$(maybe_migrate_legacy_mesh_dir 2>&1)"; rc=$?
t "migration refuses a genuinely live legacy PID" \
  '[ "$rc" -ne 0 ] && [ -d "$LEGACY_MESH_DIR" ] && [ ! -e "$MESH_DIR" ] && [[ "$out" == *"may still be running"* ]]'
kill "$live_pid" 2>/dev/null || true
wait "$live_pid" 2>/dev/null || true
live_pid=""

rm -rf "$LEGACY_MESH_DIR" "$MESH_DIR"
mkdir -p "$LEGACY_MESH_DIR"
printf 'prior-state\n' > "$LEGACY_MESH_DIR/witness"
out="$(household_start_mode 2>&1)"; rc=$?
t "standalone state check does not misclassify unmigrated legacy state as fresh" \
  '[ "$rc" -ne 0 ] && [[ "$out" == *"awaits ordinary-start migration"* ]]'

rm -rf "$LEGACY_MESH_DIR" "$MESH_DIR"
mkdir -p "$LEGACY_MESH_DIR/pids"
printf '%s 0\n' "$$" > "$LEGACY_MESH_DIR/pids/stale"
printf 'state\n' > "$LEGACY_MESH_DIR/witness"
maybe_migrate_legacy_mesh_dir >/dev/null
t "a stale PID number does not block stopped migration" \
  '[ -s "$MESH_DIR/witness" ] && [ -L "$LEGACY_MESH_DIR" ]'

rm "$LEGACY_MESH_DIR"
rm -rf "$MESH_DIR"
mkdir -p "$LEGACY_MESH_DIR"
printf 'state\n' > "$LEGACY_MESH_DIR/witness"
MESH_DIR_WAS_EXPLICIT=1
maybe_migrate_legacy_mesh_dir >/dev/null
t "explicit MESH_DIR disables legacy migration" \
  '[ -s "$LEGACY_MESH_DIR/witness" ] && [ ! -e "$MESH_DIR" ]'

MESH_DIR_WAS_EXPLICIT=0
rm -rf "$LEGACY_MESH_DIR" "$MESH_DIR"
rm -f "$INTERIM_MESH_DIR"
mkdir -p "$INTERIM_MESH_DIR"
printf 'interim-state\n' > "$INTERIM_MESH_DIR/witness"
maybe_migrate_legacy_mesh_dir >/dev/null
t "the prior generic workspace root migrates to the canonical fixture root" \
  '[ "$(cat "$MESH_DIR/witness")" = interim-state ] && [ -L "$INTERIM_MESH_DIR" ] && [ -L "$LEGACY_MESH_DIR" ]'

# Historical conductor state shared a directory with the unrelated single-peer
# developer stack. Move only the configured household peers and retain old
# per-peer readers as links; partial and competing states fail closed.
write_conductor_at() {
  local root="$1" name="$2"
  mkdir -p "$root/$name/ks" "$root/$name/databases"
  printf 'config\n' > "$root/$name/conductor-config.yaml"
  printf 'key-%s\n' "$name" > "$root/$name/ks/store_file"
  printf 'db\n' > "$root/$name/databases/conductor.db"
}
MESH_PEERS=unit
PEERS=(unit)
LEGACY_CONDUCTOR_DIR="$tmp/old-local-dev"
LOCAL_DEV_DIR="$MESH_DIR/conductors"
rm -rf "$MESH_DIR" "$LEGACY_CONDUCTOR_DIR"
write_conductor_at "$LEGACY_CONDUCTOR_DIR" unit
write_conductor_at "$LEGACY_CONDUCTOR_DIR" unrelated
printf '%s\n' "$LEGACY_CONDUCTOR_DIR/unit" "$LEGACY_CONDUCTOR_DIR/unrelated" > "$LEGACY_CONDUCTOR_DIR/.hc"
maybe_migrate_legacy_conductors >/dev/null
t "complete household conductor state moves under the fixture" \
  '[ -s "$LOCAL_DEV_DIR/unit/ks/store_file" ] && [ -L "$LEGACY_CONDUCTOR_DIR/unit" ]'
t "conductor migration preserves unrelated single-peer state and roster" \
  '[ -s "$LEGACY_CONDUCTOR_DIR/unrelated/ks/store_file" ] && grep -Fxq "$LEGACY_CONDUCTOR_DIR/unrelated" "$LEGACY_CONDUCTOR_DIR/.hc"'
t "migrated conductor roster points at the persistent fixture" \
  '[ "$(cat "$LOCAL_DEV_DIR/.hc")" = "$LOCAL_DEV_DIR/unit" ]'
t "compatibility links make repeated migration checks idempotent" \
  'maybe_migrate_legacy_conductors >/dev/null'

rm "$LEGACY_CONDUCTOR_DIR/unit"
rm -rf "$LOCAL_DEV_DIR"
mkdir -p "$LEGACY_CONDUCTOR_DIR/unit/ks"
printf 'key\n' > "$LEGACY_CONDUCTOR_DIR/unit/ks/store_file"
out="$(maybe_migrate_legacy_conductors 2>&1)"; rc=$?
t "partial prior conductor state refuses without moving bytes" \
  '[ "$rc" -ne 0 ] && [ -s "$LEGACY_CONDUCTOR_DIR/unit/ks/store_file" ] && [ ! -e "$LOCAL_DEV_DIR/unit" ] && [[ "$out" == *"prior household state is partial"* ]]'

rm -rf "$LEGACY_CONDUCTOR_DIR/unit" "$LOCAL_DEV_DIR"
write_conductor_at "$LEGACY_CONDUCTOR_DIR" unit
write_conductor_at "$LOCAL_DEV_DIR" unit
out="$(maybe_migrate_legacy_conductors 2>&1)"; rc=$?
t "conductor migration refuses to overwrite persistent state" \
  '[ "$rc" -ne 0 ] && [ "$(cat "$LEGACY_CONDUCTOR_DIR/unit/ks/store_file")" = key-unit ] && [ "$(cat "$LOCAL_DEV_DIR/unit/ks/store_file")" = key-unit ] && [[ "$out" == *"both exist"* ]]'

rm -f "$INTERIM_MESH_DIR" "$LEGACY_MESH_DIR"
rm -rf "$MESH_DIR"
mkdir -p "$LEGACY_MESH_DIR"
printf 'identity-bearing-state\n' > "$LEGACY_MESH_DIR/witness"
MESH_PEERS="$DEFAULT_HOUSEHOLD_PEERS"
IFS=',' read -ra PEERS <<< "$MESH_PEERS"
CANONICAL_HUMANS_PATH="$invalid_humans"
out="$(start_detached 2>&1)"; rc=$?
t "canonical binding failure precedes migration and launch" \
  '[ "$rc" -ne 0 ] && [ "$(cat "$LEGACY_MESH_DIR/witness")" = identity-bearing-state ] && [ ! -e "$MESH_DIR" ] && [[ "$out" == *"REFUSED canonical household binding"* ]]'

# Exercise the detached subprocess boundary without launching network services.
# Replace only the child entrypoint; retain its actual environment handling.
probe="$tmp/reexec-probe.sh"
cat > "$probe" <<'BASH'
source "$MESH_TEST_SCRIPT" >/dev/null
printf '%s|%s\n' "$MESH_DIR_WAS_EXPLICIT" "$MESH_DIR" > "$MESH_TEST_RESULT"
BASH
export MESH_TEST_SCRIPT="$mesh_script" MESH_TEST_RESULT="$tmp/reexec-result"
for explicit in 0 1; do
  (
    load_canonical_household_binding() { return 0; }
    maybe_migrate_legacy_mesh_dir() { return 0; }
    maybe_migrate_legacy_conductors() { return 0; }
    preflight() { return 0; }
    setsid() {
      shift # nohup
      local -a child=("$@")
      child[${#child[@]}-2]="$probe"
      "${child[@]}"
    }
    disown() { return 0; }
    MESH_DIR="$tmp/detached-$explicit"
    MESH_DIR_WAS_EXPLICIT="$explicit"
    MESH_FOREGROUND=0
    LOGDIR="$MESH_DIR/logs"
    start_detached >/dev/null
    wait
  )
  expected="$repo_root/genesis/local-dev/household-dowell"
  [ "$explicit" = 1 ] && expected="$tmp/detached-$explicit"
  t "detached launch preserves root selection (explicit=$explicit)" \
    '[ "$(cat "$MESH_TEST_RESULT")" = "$explicit|$expected" ]'
done

exit "$fail"
