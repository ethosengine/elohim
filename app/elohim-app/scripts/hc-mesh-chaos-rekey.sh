#!/bin/bash
#
# hc-mesh-chaos-rekey.sh — stage the alpha conductor-spin class on the local mesh.
#
# THE CLASS (measured on alpha 2026-08-21, genesis/data/timeline/backlog/alpha-
# conductor-sys-validation-spin-unfetchable-deps.md): every conductor sits pegged
# at its CPU quota because sys-validation wakes every 10s, tries to fetch a
# dependency set NO living chain holds, saturates the DHT read pool on every
# attempt, logs ~1000 lines/s, and sleeps again. The prime suspect for how the
# set became unfetchable is the 2026-07-24 DNA reinstall: it RE-KEYED the
# conductors while the SQLite projections kept referencing the dead
# incarnation's actions. The local mesh does not reproduce it, because the local
# corpus was authored on the mesh and nothing is missing.
#
# So we stage it on purpose. Four phases, each runnable alone:
#
#   author   POST content to ONE peer's storage. The row lands NULL-anchored;
#            that peer's reanchor_backfill sweep then re-authors it THROUGH ITS
#            OWN CONDUCTOR (verified 2026-08-21: dhtAnchorHash uhCkk…,
#            trust:notarized). The ops now live on that peer's source chain.
#   witness   Wait for the OTHER peers to hold the same content and the same
#            anchor — they now carry references to actions authored by a chain
#            we are about to destroy.
#   rekey     Stop that peer's conductor + storage BY EXACT PID, delete its
#             sandbox, regenerate it on the same ports with a NEW agent key, and
#             restart its storage against the new key. Its storage DB is KEPT on
#             purpose — that is the alpha shape: projections referencing a dead
#             incarnation's actions.
#   measure   Run hc-mesh-spin-detector.sh on the survivors and report whether
#             their sys-validation starts accumulating unfetchable dependencies
#             and whether the read-pool saturation log appears.
#
# WHAT THIS SCRIPT WILL NOT DO: it never stops, regenerates, or restarts the
# WHOLE mesh. It touches exactly one peer, by exact pid, never by a pgrep
# pattern that could match the caller's own command line (the self-kill trap
# hc-mesh.sh documents from 2026-08-16). If killing that peer's conductor takes
# its siblings down with it — they share one `hc sandbox run -a` parent — the
# script restarts the siblings FROM THEIR EXISTING SANDBOXES (no regenerate, no
# re-key: their chains and databases are untouched) and says so loudly.
#
# USAGE:
#   ./hc-mesh-chaos-rekey.sh [--peer james] [--count 5] [--phase all|author|witness|rekey|measure]
#                            [--measure-minutes 3] [--anchor-timeout 180]
#                            [--witness-timeout 240] [--tag <run-tag>]
#
#   --peer NAME          household peer to re-key (default james — the smallest
#                        quota on alpha, and index 2 on the local mesh)
#   --count N            content nodes to author on that peer (default 5)
#   --tag TAG            run tag; content ids are chaos-rekey-<TAG>-NN
#                        (default: the UTC timestamp). Reuse it to resume a run.
#   --phase P            run one phase instead of all four
#   --measure-minutes M  detector run length after the re-key (default 3)
#   --method M           reinstall (default) re-keys by wiping only the
#                        conductor's chain + keystore and re-installing the happ
#                        onto the kept wasm-cache — 12s, measured. regenerate
#                        wipes the whole sandbox and re-generates it, which is
#                        cleaner but races a 60s admin-websocket timeout during
#                        the cold wasm install on a loaded host.
#   --keep-storage-db 0  ALSO delete the peer's storage DB (default 1 = keep it,
#                        which is the alpha shape; 0 is the control experiment)
#
# EXIT: 0 = the run completed and the class did NOT reproduce (detector QUIET)
#       1 = the class REPRODUCED (detector SPIN) — the desired staging outcome
#       2 = a setup/probe error; the run did not reach a verdict
#
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
LOCAL_DEV_DIR="${LOCAL_DEV_DIR:-$REPO_ROOT/elohim/holochain/local-dev}"
MESH_DIR="${MESH_DIR:-/tmp/elohim-local-mesh}"
HAPP_PATH="${HAPP_PATH:-$REPO_ROOT/elohim/holochain/dna/elohim/workdir/elohim.happ}"
DOORWAY_PORT="${DOORWAY_PORT:-8888}"
API_KEY="${MESH_API_KEY_ADMIN:-mesh-admin-dev-key}"
MESH_PEERS="${MESH_PEERS:-matthew,jessica,james}"
DETECTOR="$SCRIPT_DIR/hc-mesh-spin-detector.sh"
WORK_DIR="${CHAOS_WORK_DIR:-$MESH_DIR/chaos-rekey}"
SPIN_JSON_DIR="${SPIN_JSON_DIR:-$MESH_DIR/spin}"

PEER="james"
COUNT=5
PHASE="all"
MEASURE_MINUTES=3
ANCHOR_TIMEOUT=180
WITNESS_TIMEOUT=240
KEEP_STORAGE_DB=1
REKEY_METHOD="${REKEY_METHOD:-reinstall}"
TAG=""

while [ $# -gt 0 ]; do
  case "$1" in
    --peer)             PEER="$2"; shift 2 ;;
    --count)            COUNT="$2"; shift 2 ;;
    --tag)              TAG="$2"; shift 2 ;;
    --phase)            PHASE="$2"; shift 2 ;;
    --measure-minutes)  MEASURE_MINUTES="$2"; shift 2 ;;
    --anchor-timeout)   ANCHOR_TIMEOUT="$2"; shift 2 ;;
    --witness-timeout)  WITNESS_TIMEOUT="$2"; shift 2 ;;
    --keep-storage-db)  KEEP_STORAGE_DB="$2"; shift 2 ;;
    --method)           REKEY_METHOD="$2"; shift 2 ;;
    -h|--help)          awk 'NR==1{next} /^#/{sub(/^# ?/,""); print; next} {exit}' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "unknown option: $1 (try --help)" >&2; exit 2 ;;
  esac
done

# Alpha identity model, mirrored from hc-mesh.sh's human_id(): content must be
# authored as the peer's OWN human, or the createdBy provenance is a fiction.
human_id() {
  case "$1" in
    matthew) echo human-matthew-manager ;;
    jessica) echo human-jessica-spouse ;;
    james)   echo human-james-son ;;
    *)       echo "human-$1" ;;
  esac
}

IFS=',' read -ra PEERS <<< "$MESH_PEERS"
PEER_INDEX=-1
for i in "${!PEERS[@]}"; do [ "${PEERS[$i]}" = "$PEER" ] && PEER_INDEX=$i; done
[ "$PEER_INDEX" -ge 0 ] || { echo "peer '$PEER' is not in MESH_PEERS ($MESH_PEERS)" >&2; exit 2; }

admin_port() { echo $((4444 + 10 * $1)); }
app_port()   { echo $((4445 + 10 * $1)); }
http_port()  { echo $((8090 + $1)); }

PEER_ADMIN="$(admin_port "$PEER_INDEX")"
PEER_APP="$(app_port "$PEER_INDEX")"
PEER_HTTP="$(http_port "$PEER_INDEX")"
PEER_SANDBOX="$LOCAL_DEV_DIR/$PEER"
PEER_RUN_LOG="$LOCAL_DEV_DIR/.sandbox_run_log.$PEER"
SHARED_RUN_LOG="$LOCAL_DEV_DIR/.sandbox_run_log"

mkdir -p "$WORK_DIR"
[ -n "$TAG" ] || TAG="$(date -u +%Y%m%dT%H%M%SZ)"
STATE="$WORK_DIR/$TAG.state"
IDS_FILE="$WORK_DIR/$TAG.ids"

say()  { printf '[chaos-rekey %s] %s\n' "$(date -u +%H:%M:%SZ)" "$*"; }
head_() { printf '\n=== %s ===\n' "$*"; }
fail() { echo "FATAL: $*" >&2; exit 2; }

# ---------------------------------------------------------------------------
# Process identity — exact pids only. `pgrep -f <pattern>` can match this very
# shell (documented self-kill trap); every lookup below matches on an argv
# substring unique to the target process AND excludes our own pid.
# ---------------------------------------------------------------------------
conductor_pid() { # <peer> -> pid of `holochain --config-path .../<peer>/conductor-config.yaml`
  ps -eo pid=,args= \
    | awk -v me="$$" -v pat="/$1/conductor-config.yaml" \
      '$1 != me && index($0, "holochain ") && index($0, pat) { print $1; exit }'
}
storage_pid() { # <http-port> -> pid of `elohim-storage --http-port <port>`
  ps -eo pid=,args= \
    | awk -v me="$$" -v port="--http-port $1" \
      '$1 != me && index($0, "elohim-storage") && index($0, port) { print $1; exit }'
}
wait_gone() { # <pid> <secs>
  local p="$1" t="$2"
  while [ "$t" -gt 0 ] && kill -0 "$p" 2>/dev/null; do sleep 1; t=$((t - 1)); done
  ! kill -0 "$p" 2>/dev/null
}
port_listening() { ss -tln 2>/dev/null | grep -qE "127.0.0.1:$1 |0.0.0.0:$1 "; }

storage_get() { curl -s -m 10 -H "Authorization: Bearer $API_KEY" "$1"; }
json_field() { python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
except Exception:
    print(''); raise SystemExit(0)
v = d.get(sys.argv[1])
print('' if v is None else v)
" "$1" 2>/dev/null; }

# ===========================================================================
# PHASE 1 — author on the target peer's own chain
# ===========================================================================
phase_author() {
  head_ "phase 1/4: author $COUNT content nodes on $PEER (:$PEER_HTTP)"
  port_listening "$PEER_HTTP" || fail "$PEER storage is not listening on :$PEER_HTTP — is the mesh up?"

  : > "$IDS_FILE"
  for n in $(seq 1 "$COUNT"); do
    local id="chaos-rekey-$TAG-$(printf '%02d' "$n")"
    local code
    code="$(curl -s -m 30 -o /dev/null -w '%{http_code}' -X POST "http://localhost:$PEER_HTTP/db/content" \
      -H "Authorization: Bearer $API_KEY" -H "Content-Type: application/json" \
      -d "{\"id\":\"$id\",\"title\":\"Chaos re-key node $n ($TAG)\",\"contentType\":\"concept\",\"contentFormat\":\"markdown\",\"contentBody\":\"# $id\\n\\nAuthored on $PEER's chain so the re-key makes these ops unfetchable.\",\"reach\":\"commons\",\"createdBy\":\"$(human_id "$PEER")\",\"tags\":[\"chaos-rekey\",\"$TAG\"]}")"
    if [ "$code" = "201" ] || [ "$code" = "200" ]; then
      echo "$id" >> "$IDS_FILE"; say "authored $id (HTTP $code)"
    else
      say "WARN: POST $id returned HTTP $code — not counted"
    fi
  done
  local authored; authored="$(wc -l < "$IDS_FILE")"
  [ "$authored" -gt 0 ] || fail "no content authored on $PEER — nothing to stage"

  # The row lands NULL-anchored; reanchor_backfill re-authors it through THIS
  # peer's conductor on the next reconcile sweep. Anchored == the ops are on
  # this peer's source chain, which is the whole point of the staging.
  say "waiting up to ${ANCHOR_TIMEOUT}s for $PEER's reanchor_backfill to anchor them on ITS OWN conductor"
  local deadline=$((SECONDS + ANCHOR_TIMEOUT)) anchored=0
  while [ "$SECONDS" -lt "$deadline" ]; do
    anchored=0
    while read -r id; do
      local a; a="$(storage_get "http://localhost:$PEER_HTTP/db/content/$id" | json_field dhtAnchorHash)"
      [ -n "$a" ] && anchored=$((anchored + 1))
    done < "$IDS_FILE"
    [ "$anchored" -eq "$authored" ] && break
    sleep 5
  done
  say "anchored on $PEER: $anchored/$authored"
  {
    echo "TAG=$TAG"; echo "PEER=$PEER"; echo "AUTHORED=$authored"; echo "ANCHORED=$anchored"
  } > "$STATE"
  while read -r id; do
    printf '  %-28s anchor=%s\n' "$id" \
      "$(storage_get "http://localhost:$PEER_HTTP/db/content/$id" | json_field dhtAnchorHash)"
  done < "$IDS_FILE"
  [ "$anchored" -gt 0 ] || fail "nothing anchored on $PEER within ${ANCHOR_TIMEOUT}s — the staging precondition is unmet (check $MESH_DIR/logs/$PEER.log for reanchor_backfill)"
}

# ===========================================================================
# PHASE 2 — the other peers take references to those actions
# ===========================================================================
phase_witness() {
  head_ "phase 2/4: wait for the other peers to hold $PEER's content"
  [ -s "$IDS_FILE" ] || fail "no ids file for tag $TAG — run the author phase first"

  local others=() i=0
  for name in "${PEERS[@]}"; do
    [ "$name" != "$PEER" ] && others+=("$name=$(http_port $i)")
    i=$((i + 1))
  done
  say "witnesses: ${others[*]}"

  local deadline=$((SECONDS + WITNESS_TIMEOUT))
  while [ "$SECONDS" -lt "$deadline" ]; do
    local all_ok=1
    for spec in "${others[@]}"; do
      local port="${spec#*=}"
      while read -r id; do
        local a; a="$(storage_get "http://localhost:$port/db/content/$id" | json_field dhtAnchorHash)"
        [ -n "$a" ] || all_ok=0
      done < "$IDS_FILE"
    done
    [ "$all_ok" -eq 1 ] && break
    sleep 5
  done

  local held_total=0
  for spec in "${others[@]}"; do
    local name="${spec%%=*}" port="${spec#*=}" held=0
    while read -r id; do
      local a; a="$(storage_get "http://localhost:$port/db/content/$id" | json_field dhtAnchorHash)"
      [ -n "$a" ] && { held=$((held + 1)); held_total=$((held_total + 1)); }
    done < "$IDS_FILE"
    say "$name holds $held/$(wc -l < "$IDS_FILE") of $PEER's anchored nodes"
    echo "WITNESS_$name=$held" >> "$STATE"
  done
  [ "$held_total" -gt 0 ] || say "WARN: no witness peer holds the content yet — the re-key will still run, but the dependency references may be thinner than alpha's"
}

# ===========================================================================
# PHASE 3 — re-key the target peer
# ===========================================================================
phase_rekey() {
  head_ "phase 3/4: re-key $PEER (new agent key, same ports, storage DB kept=$KEEP_STORAGE_DB)"
  [ -f "$HAPP_PATH" ] || fail "happ bundle not found at $HAPP_PATH"

  local cpid spid
  cpid="$(conductor_pid "$PEER")"; spid="$(storage_pid "$PEER_HTTP")"
  [ -n "$cpid" ] || fail "no conductor process for $PEER"
  [ -n "$spid" ] || fail "no storage process on :$PEER_HTTP"
  say "target pids: conductor=$cpid storage=$spid"

  # Capture the storage process's EXACT environment and cwd before killing it —
  # the restart must differ in AGENT_PUBKEY and nothing else. Reconstructing the
  # env from hc-mesh.sh would silently drift the moment that script changes.
  local envfile="$WORK_DIR/$TAG.$PEER.environ" cwd
  cp "/proc/$spid/environ" "$envfile" || fail "could not read /proc/$spid/environ"
  cwd="$(readlink "/proc/$spid/cwd")"
  local storage_bin; storage_bin="$(readlink "/proc/$spid/exe")"
  say "captured storage env ($(tr -cd '\0' < "$envfile" | wc -c) vars), cwd=$cwd, bin=$storage_bin"

  # Sibling liveness BEFORE, so we can prove what our kill did and undo it.
  local siblings=()
  for name in "${PEERS[@]}"; do
    [ "$name" = "$PEER" ] && continue
    siblings+=("$name:$(conductor_pid "$name")")
  done
  say "sibling conductors before: ${siblings[*]}"

  say "stopping $PEER storage (pid $spid) and conductor (pid $cpid) — exact pids, no patterns"
  kill "$spid" 2>/dev/null; wait_gone "$spid" 15 || { kill -9 "$spid" 2>/dev/null; wait_gone "$spid" 10; }
  kill "$cpid" 2>/dev/null; wait_gone "$cpid" 20 || { kill -9 "$cpid" 2>/dev/null; wait_gone "$cpid" 10; }
  say "$PEER stopped"

  # Did the shared `hc sandbox run -a` parent take the siblings with it? If so,
  # restart them FROM THEIR EXISTING SANDBOXES: no generate, no re-key, their
  # chains and DBs are untouched. Loud, because it changes what the measure means.
  sleep 3
  local dead=() dead_idx=()
  for entry in "${siblings[@]}"; do
    local name="${entry%%:*}"
    if [ -z "$(conductor_pid "$name")" ]; then
      dead+=("$name")
      local j=0; for n2 in "${PEERS[@]}"; do [ "$n2" = "$name" ] && dead_idx+=("$j"); j=$((j+1)); done
    fi
  done
  if [ "${#dead[@]}" -gt 0 ]; then
    say "NOTE: killing $PEER's conductor also took down: ${dead[*]} (they share one 'hc sandbox run -a' parent)."
    say "      restarting them from their EXISTING sandboxes — no regenerate, no re-key."
    local f="" p="" idx=""
    for k in "${dead_idx[@]}"; do
      f+="${f:+,}$(admin_port "$k")"; p+="${p:+,}$(app_port "$k")"; idx+="${idx:+ }$k"
    done
    ( cd "$LOCAL_DEV_DIR" && nohup sh -c "echo test | hc sandbox --piped -f $f run $idx -p=$p" \
        >> "$SHARED_RUN_LOG" 2>&1 & )
    local dl=$((SECONDS + 120))
    while [ "$SECONDS" -lt "$dl" ]; do
      local up=1
      for name in "${dead[@]}"; do [ -z "$(conductor_pid "$name")" ] && up=0; done
      [ "$up" -eq 1 ] && break
      sleep 3
    done
    for name in "${dead[@]}"; do say "  sibling $name conductor pid now: $(conductor_pid "$name" || echo NONE)"; done
  else
    say "siblings survived the kill — good, the measure stays clean"
  fi

  # --- re-key this peer's conductor ----------------------------------------
  # Two methods, both a TRUE re-key (a fresh keystore mints a new agent key):
  #
  #   reinstall (default) — delete only databases/ and ks/, keep wasm-cache and
  #     the conductor config, then install+enable the happ on the running
  #     conductor. Measured 2026-08-21: 12s, because the DNAs' compiled wasm is
  #     already there. It is also the closer analogue of the alpha event, which
  #     re-keyed conductors without changing the DNAs.
  #   regenerate — delete the whole sandbox and `hc sandbox generate` it again.
  #     Cleaner in principle, but its install runs inside the conductor's 60s
  #     admin-websocket request timeout, and on a loaded host (load average 79,
  #     with the mesh and two repair agents running) a COLD wasm compile does not
  #     fit: measured 65s, "Websocket error: Timeout", twice in a row. Kept as a
  #     fallback, with retries, for when a pristine sandbox is wanted on a quiet
  #     box.
  if [ "$KEEP_STORAGE_DB" = "0" ]; then
    say "ALSO deleting $PEER's storage DB ($MESH_DIR/$PEER) — control experiment, not the alpha shape"
    rm -rf "${MESH_DIR:?}/$PEER"
  else
    say "KEEPING $PEER's storage DB ($MESH_DIR/$PEER) — its projections now reference a DEAD incarnation's actions (the alpha shape)"
  fi

  if [ "$REKEY_METHOD" = "reinstall" ]; then
    [ -d "$PEER_SANDBOX" ] || fail "$PEER's sandbox is missing at $PEER_SANDBOX — use --method regenerate"
    say "deleting $PEER's chain + keystore (databases/, ks/), KEEPING wasm-cache"
    rm -rf "$PEER_SANDBOX/databases" "$PEER_SANDBOX/ks"
  else
    say "deleting $PEER's whole sandbox: $PEER_SANDBOX (chain + keystore + DHT db + wasm-cache)"
    local attempt=1 gen_rc=1
    while [ "$attempt" -le 3 ]; do
      say "hc sandbox generate for $PEER (attempt $attempt/3; a cold wasm install races a 60s admin timeout)"
      rm -rf "$PEER_SANDBOX"
      ( cd "$LOCAL_DEV_DIR" && timeout 300 sh -c "echo test | hc sandbox --piped -f $PEER_ADMIN generate -n 1 --app-id elohim --in-process-lair --root \"\$PWD\" -d $PEER \"$HAPP_PATH\" network --bootstrap http://localhost:$DOORWAY_PORT/bootstrap webrtc ws://signal.localhost:$DOORWAY_PORT" ) > "$WORK_DIR/$TAG.generate.$attempt.log" 2>&1
      gen_rc=$?
      [ "$gen_rc" -eq 0 ] && break
      say "  attempt $attempt failed (exit $gen_rc) — see $WORK_DIR/$TAG.generate.$attempt.log"
      attempt=$((attempt + 1))
    done
    [ "$gen_rc" -eq 0 ] || fail "hc sandbox generate for $PEER failed 3x — the host is likely too loaded for a cold wasm install; use --method reinstall"
  fi

  # `hc sandbox generate` APPENDS to ./.hc; rewrite it so the index order stays
  # [peer0, peer1, ...] and `run <index>` keeps meaning what it says.
  ( cd "$LOCAL_DEV_DIR" && { for name in "${PEERS[@]}"; do echo "$LOCAL_DEV_DIR/$name"; done > .hc; } )
  say ".hc rewritten in peer order: $(tr '\n' ' ' < "$LOCAL_DEV_DIR/.hc")"

  # Keep the dev-tier gossip pacing hc-mesh.sh applies — a conductor without it
  # starts its first kitsune2 round at PROD cadence (120s/300s), and the re-keyed
  # peer would look partitioned for reasons that are not the class. Idempotent.
  python3 - "$PEER_SANDBOX/conductor-config.yaml" <<'GOSSIPEOF'
import sys, yaml
path = sys.argv[1]
with open(path) as f:
    cfg = yaml.safe_load(f)
network = cfg.setdefault("network", {})
advanced = network.get("advanced") or {}
k2 = advanced.get("k2Gossip") or {}
k2["initiateIntervalMs"] = 1000
k2["minInitiateIntervalMs"] = 1000
k2["initialInitiateIntervalMs"] = 1000
advanced["k2Gossip"] = k2
network["advanced"] = advanced
with open(path, "w") as f:
    yaml.safe_dump(cfg, f, default_flow_style=False, sort_keys=False)
GOSSIPEOF
  [ $? -eq 0 ] || fail "gossip-config patch failed for $PEER"
  say "dev-tier gossip patch applied (k2Gossip initiate=1000ms)"

  # Run the re-keyed peer on its OWN log stream: `hc sandbox run -a` multiplexes
  # every conductor into one prefix-less file, so a separate stream is the only
  # way the detector can attribute this peer's log rate to this peer.
  say "starting $PEER's conductor on its own log stream ($PEER_RUN_LOG)"
  ( cd "$LOCAL_DEV_DIR" && nohup sh -c "echo test | hc sandbox --piped -f $PEER_ADMIN run $PEER_INDEX -p=$PEER_APP" > "$PEER_RUN_LOG" 2>&1 & )
  local dl=$((SECONDS + 180))
  while [ "$SECONDS" -lt "$dl" ]; do port_listening "$PEER_ADMIN" && break; sleep 3; done
  port_listening "$PEER_ADMIN" || fail "$PEER's conductor did not come up on :$PEER_ADMIN — see $PEER_RUN_LOG"
  say "$PEER conductor up (admin :$PEER_ADMIN, pid $(conductor_pid "$PEER"))"

  # reinstall: the conductor is now up with a fresh keystore and NO apps.
  # Installing without --agent-key mints a new one — that IS the re-key.
  if [ "$REKEY_METHOD" = "reinstall" ]; then
    local iattempt=1 irc=1
    while [ "$iattempt" -le 3 ]; do
      ( cd "$LOCAL_DEV_DIR" && timeout 280 sh -c "echo test | hc sandbox --piped call --running $PEER_ADMIN install-app --app-id elohim \"$HAPP_PATH\"" ) > "$WORK_DIR/$TAG.install.$iattempt.log" 2>&1
      irc=$?
      [ "$irc" -eq 0 ] && break
      say "  install-app attempt $iattempt/3 failed (exit $irc) — see $WORK_DIR/$TAG.install.$iattempt.log"
      iattempt=$((iattempt + 1)); sleep 5
    done
    [ "$irc" -eq 0 ] || fail "install-app on $PEER failed 3x — see $WORK_DIR/$TAG.install.*.log"
    say "happ installed on $PEER's new incarnation (attempt $iattempt)"
    ( cd "$LOCAL_DEV_DIR" && timeout 120 sh -c "echo test | hc sandbox --piped call --running $PEER_ADMIN enable-app elohim" ) > "$WORK_DIR/$TAG.enable.log" 2>&1 || fail "enable-app on $PEER failed — see $WORK_DIR/$TAG.enable.log"
    say "happ enabled on $PEER"
  fi

  # --- new agent key, then storage back on it -------------------------------
  local newkey="" dl2=$((SECONDS + 120))
  while [ "$SECONDS" -lt "$dl2" ]; do
    newkey="$(hc sandbox call --running "$PEER_ADMIN" list-apps 2>/dev/null \
      | grep -o '"agent_pub_key":"[^"]*"' | head -1 | cut -d'"' -f4)"
    [ -n "$newkey" ] && break
    sleep 3
  done
  [ -n "$newkey" ] || fail "could not read $PEER's NEW agent key from :$PEER_ADMIN"
  local oldkey; oldkey="$(tr '\0' '\n' < "$envfile" | sed -n 's/^AGENT_PUBKEY=//p')"
  say "agent key re-keyed: ${oldkey:0:20}... -> ${newkey:0:20}..."
  [ "$oldkey" != "$newkey" ] || say "WARN: the agent key did NOT change — the re-key did not take, and this run stages nothing"
  { echo "OLD_AGENT=$oldkey"; echo "NEW_AGENT=$newkey"; } >> "$STATE"

  cat > "$WORK_DIR/$TAG.relaunch.py" <<'PYEOF'
"""Re-exec the peer's storage with its captured environment, AGENT_PUBKEY replaced.

Rebuilding the env from hc-mesh.sh would drift the moment that script changes;
/proc/<pid>/environ is what the process ACTUALLY had.
"""
import os, sys
envfile, newkey, binpath, port, cwd = sys.argv[1:6]
with open(envfile, "rb") as f:
    raw = f.read().decode("utf-8", "replace")
env = dict(p.split("=", 1) for p in raw.split("\0") if "=" in p)
env["AGENT_PUBKEY"] = newkey
os.chdir(cwd)
os.execve(binpath, [binpath, "--http-port", port], env)
PYEOF
  say "restarting $PEER storage on the new key"
  nohup python3 "$WORK_DIR/$TAG.relaunch.py" "$envfile" "$newkey" "$storage_bin" "$PEER_HTTP" "$cwd" \
    >> "$MESH_DIR/logs/$PEER.log" 2>&1 &
  local dl3=$((SECONDS + 120))
  while [ "$SECONDS" -lt "$dl3" ]; do
    curl -s -m 3 "http://localhost:$PEER_HTTP/health" >/dev/null && break; sleep 3
  done
  curl -s -m 3 "http://localhost:$PEER_HTTP/health" >/dev/null \
    || fail "$PEER storage did not come back on :$PEER_HTTP — see $MESH_DIR/logs/$PEER.log"
  say "$PEER storage back up on :$PEER_HTTP (pid $(storage_pid "$PEER_HTTP"))"
  echo "REKEYED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$STATE"
}

# ===========================================================================
# PHASE 4 — measure
# ===========================================================================
phase_measure() {
  head_ "phase 4/4: measure for ${MEASURE_MINUTES} minutes"
  [ -x "$DETECTOR" ] || fail "spin detector not found/executable at $DETECTOR"
  local logs=(--log "mesh=$SHARED_RUN_LOG")
  [ -f "$PEER_RUN_LOG" ] && logs+=(--log "$PEER=$PEER_RUN_LOG")
  # One predictable home for the machine-readable summary, so an a2o step (or an
  # operator comparing two runs) can find it without knowing this script's
  # internal work-dir layout: $MESH_DIR/spin/<tag>-<phase>.json
  mkdir -p "$SPIN_JSON_DIR"
  "$DETECTOR" "${logs[@]}" --minutes "$MEASURE_MINUTES" \
    --json-out "$SPIN_JSON_DIR/$TAG-measure.json" | tee "$WORK_DIR/$TAG.after.txt"
  local rc=${PIPESTATUS[0]}
  cp -f "$SPIN_JSON_DIR/$TAG-measure.json" "$WORK_DIR/$TAG.after.json" 2>/dev/null
  say "spin summary: $SPIN_JSON_DIR/$TAG-measure.json"

  head_ "post-re-key content honesty on $PEER (:$PEER_HTTP)"
  # Alpha's honest alternative to a spin: the peer either RE-ADOPTS its old
  # content under the new key, or says plainly that it cannot fetch it. A 200
  # with a live anchor is re-adoption; a 404/410 is an honest miss. Silence
  # while the conductor grinds is the failure this whole exercise is about.
  if [ -s "$IDS_FILE" ]; then
    while read -r id; do
      local code body anchor trust
      body="$(curl -s -m 10 -w '\n%{http_code}' -H "Authorization: Bearer $API_KEY" \
        "http://localhost:$PEER_HTTP/db/content/$id")"
      code="$(echo "$body" | tail -1)"; body="$(echo "$body" | sed '$d')"
      anchor="$(echo "$body" | json_field dhtAnchorHash)"
      trust="$(echo "$body" | json_field trust)"
      printf '  %-28s HTTP %-4s anchor=%-52s trust=%s\n' "$id" "$code" "${anchor:-<null>}" "${trust:-<none>}"
    done < "$IDS_FILE"
  fi
  echo "DETECTOR_EXIT=$rc"
  return "$rc"
}

# ===========================================================================
head_ "chaos re-key run tag=$TAG peer=$PEER (index $PEER_INDEX: admin=$PEER_ADMIN app=$PEER_APP http=$PEER_HTTP)"
say "artifacts: $WORK_DIR/$TAG.*"
RC=0
case "$PHASE" in
  author)  phase_author ;;
  witness) phase_witness ;;
  rekey)   phase_rekey ;;
  measure) phase_measure; RC=$? ;;
  all)     phase_author; phase_witness; phase_rekey; phase_measure; RC=$? ;;
  *) fail "unknown phase '$PHASE' (author|witness|rekey|measure|all)" ;;
esac

head_ "run complete (tag=$TAG)"
[ -f "$STATE" ] && cat "$STATE"
# Only a run that actually MEASURED can pronounce on the class. A staging-only
# phase reporting "did not reproduce" would be a measure that never ran claiming
# a green — the exact lossy-measure shape the CI museum warns about.
case "$PHASE" in
  all|measure)
    if [ "$RC" -eq 1 ]; then
      echo "OUTCOME: the class REPRODUCED — the detector's verdict is SPIN."
    else
      echo "OUTCOME: the class did NOT reproduce — the detector's verdict is QUIET."
    fi ;;
  *) echo "OUTCOME: staging phase '$PHASE' completed; no verdict (run --phase measure to get one)." ;;
esac
exit "$RC"
