#!/bin/bash
#
# hc-mesh.sh - Local multi-peer Elohim mesh (Eclipse Che / single container)
#
# Brings up the ALPHA-SHAPED topology entirely on loopback, no k8s, no
# external services:
#
#   1 doorway   (dev mode, serves /bootstrap + /signal for the island DHT,
#                proxies all N storage peers)
#   N conductors (hc sandbox, holochain 0.6: --piped + -f pinned admin ports,
#                -n N native multi-sandbox; discovery via the LOCAL doorway)
#   N storages  (release binary, libp2p mesh via mDNS inside the container,
#                each bound to its own conductor)
#
# Verified bring-up 2026-08-16: mesh/upload/propagation/delivery/projection
# probes green via `substrate-verify.ts` against this fleet (same assertion
# set as CI's Dataplane Validation).
#
# USAGE:
#   ./hc-mesh.sh [start|stop|status|probe]
#
# ENVIRONMENT:
#   MESH_PEERS      Peer names, comma-separated (default: matthew,jessica,james)
#   MESH_DIR        Data root (default: /tmp/elohim-local-mesh)
#   DOORWAY_PORT    Doorway HTTP port (default: 8888)
#   STORAGE_BIN     elohim-storage binary (default: pool release slot)
#   DOORWAY_BIN     doorway binary (default: pool debug slot)
#
# LOAD-BEARING FACTS (verified against holochain 0.6.0 / hc 0.6.0):
#   - `hc sandbox --piped` reads the lair passphrase from stdin: the old
#     socat/PTY wrapper is obsolete.
#   - `-f p1,p2,..` pins admin ports; `-r=a1,a2,..` pins app ports; no more
#     log-scraping for dynamic ports.
#   - tx5 REJECTS a signal URL with a path ("parsing tx5 sig url ...
#     InvalidLastSymbol"): the URL must be pathless. The doorway detects a
#     signal request by Host header prefix `signal.`, and `signal.localhost`
#     resolves to loopback out of the box -> ws://signal.localhost:PORT.
#   - `hc sandbox generate` WITHOUT a network section points at the PUBLIC
#     dev-test-bootstrap2.holochain.org. True isolation requires the explicit
#     `network --bootstrap ... webrtc ...` pointing at the local doorway.
#   - Conductor admin/app interfaces bind loopback only; config exposes
#     `danger_bind_addr` when a cross-pod topology ever needs more. Inside
#     one container, loopback is correct.
#   - elohim-storage ignores the HTTP_PORT env var: pass --http-port.
#
set -u

MESH_PEERS="${MESH_PEERS:-matthew,jessica,james}"
MESH_DIR="${MESH_DIR:-/tmp/elohim-local-mesh}"
DOORWAY_PORT="${DOORWAY_PORT:-8888}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
LOCAL_DEV_DIR="$REPO_ROOT/elohim/holochain/local-dev"
HAPP_WORKDIR="$REPO_ROOT/elohim/holochain/dna/elohim/workdir"
HAPP_PATH="$HAPP_WORKDIR/elohim.happ"
POOL="/projects/.cargo-target-pool/family/dev"
STORAGE_BIN="${STORAGE_BIN:-$POOL/elohim__elohim-storage/release/release/elohim-storage}"
DOORWAY_BIN="${DOORWAY_BIN:-$POOL/doorway__doorway-service/dev/debug/doorway}"
LOGDIR="$MESH_DIR/logs"

# Port scheme per peer index i (0-based): admin 4444+10i, app 4445+10i,
# storage http 8090+i, libp2p 9701+i.
IFS=',' read -ra PEERS <<< "$MESH_PEERS"

admin_port() { echo $((4444 + 10 * $1)); }
app_port()   { echo $((4445 + 10 * $1)); }
http_port()  { echo $((8090 + $1)); }
p2p_port()   { echo $((9701 + $1)); }

# Alpha identity model (edgenode template): each storage node self-heals its
# own human's agent_pub_key from its conductor cell key, NULL-only. Without
# this env the saga ch02 finish line (non-null agentPubKey) can never light.
human_id() {
  case "$1" in
    matthew) echo human-matthew-manager ;;
    jessica) echo human-jessica-spouse ;;
    james)   echo human-james-son ;;
    *)       echo "human-$1" ;;
  esac
}

peer_csv() { # name=host:port CSV for substrate-verify / PEER_STORAGE_URLS
  local out="" i=0
  for name in "${PEERS[@]}"; do
    out+="${out:+,}$name=localhost:$(http_port $i)"; i=$((i+1))
  done
  echo "$out"
}

stop_all() {
  # Kill by exact binary identity, NEVER by a pattern that could match the
  # caller's own command line (self-kill class, hit twice on 2026-08-16).
  kill $(pgrep -x holochain) 2>/dev/null
  kill $(pgrep -f "[h]c sandbox") 2>/dev/null
  kill $(pgrep -f "elohim-storag[e]") 2>/dev/null
  kill $(pgrep -f "(debug|release)/doorwa[y]") 2>/dev/null
  sleep 1
  echo "mesh stopped"
}

status_all() {
  echo "conductors:"; ss -tln 2>/dev/null | grep -E "127.0.0.1:44[0-9]{2} " || echo "  (none)"
  local i=0
  for name in "${PEERS[@]}"; do
    printf "  %-8s admin=%s app=%s  storage=" "$name" "$(admin_port $i)" "$(app_port $i)"
    curl -s -m 2 "http://localhost:$(http_port $i)/health" >/dev/null && echo "UP :$(http_port $i)" || echo "down"
    i=$((i+1))
  done
  printf "doorway  :%s " "$DOORWAY_PORT"
  curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null && echo UP || echo down
  echo
  echo "probe env:  PEER_STORAGE_URLS=\"$(peer_csv)\" INTERNAL_DOORWAY_URL=\"localhost:$DOORWAY_PORT\""
}

probe_all() {
  cd "$REPO_ROOT/genesis/a2o" || exit 2
  mkdir -p "$MESH_DIR/reports"
  local failures=0
  for cmd in mesh upload propagation delivery projection federation resilience; do
    PEER_STORAGE_URLS="$(peer_csv)" \
    INTERNAL_DOORWAY_URL="localhost:$DOORWAY_PORT" \
    REPORT_DIR="$MESH_DIR/reports" \
    STATE_DIR="$MESH_DIR/state" \
    CONTENT_PATH="$REPO_ROOT/genesis/docs/content/elohim-protocol/manifesto.md" \
    BUILD_TAG="local-mesh" \
    pnpm exec tsx scripts/substrate-verify.ts "$cmd" || failures=$((failures+1))
  done
  echo "probe complete: $failures subcommand(s) with failures (reports: $MESH_DIR/reports)"
}

start_all() {
  mkdir -p "$MESH_DIR" "$LOGDIR" "$LOCAL_DEV_DIR"

  # Peer policy: the storage binary loads ./config/peer-policy.toml relative to
  # ITS CWD; a missing file silently disables the whole heartbeat + signal-
  # subscriber + genesis-self-heal block ("PeerStatus heartbeat disabled" —
  # found 2026-08-16 as the root of NULL agent keys + zero peer-status rows on
  # the mesh). Generate a minimal local policy and pass it explicitly.
  POLICY_FILE="$MESH_DIR/peer-policy.toml"
  if [ ! -f "$POLICY_FILE" ]; then
    cat > "$POLICY_FILE" <<'EOF'
[pool]
accept_general_traffic = "auto"
min_free_storage_pct = 5
require_conductor_healthy = true

[stewardship]
accept_new_reserves = "auto"
max_storage_pct = 80

[network]
# All peers share one netns locally — conductor stays loopback-only, no
# forwarders (they would EADDRINUSE across the three peers). The bind/port
# fields are required by the TOML schema even when the switch is off.
expose_conductor_externally = false
conductor_external_bind = "0.0.0.0:8445"
conductor_internal_port = 4445
conductor_admin_external_bind = "0.0.0.0:8444"
conductor_admin_internal_port = 4444
EOF
  fi

  for bin in "$STORAGE_BIN" "$DOORWAY_BIN"; do
    [ -x "$bin" ] || { echo "missing binary: $bin (build it first — see CLAUDE.md pool-slot paths)"; exit 1; }
  done

  # Repack the happ when any DNA is newer than the bundle (stale-bundle trap:
  # elohim.happ predated lamad.dna by 3 months on 2026-08-16).
  if [ ! -f "$HAPP_PATH" ] || [ -n "$(find "$HAPP_WORKDIR" -name '*.dna' -newer "$HAPP_PATH" 2>/dev/null)" ]; then
    echo "repacking elohim.happ (DNA newer than bundle)"
    (cd "$HAPP_WORKDIR" && hc app pack . -o elohim.happ) || exit 1
  fi

  # 1. Doorway first: it is the island DHT's bootstrap + signal home.
  if ! curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null; then
    local i=0 primary="" extras=""
    for name in "${PEERS[@]}"; do
      if [ $i -eq 0 ]; then primary="http://localhost:$(http_port $i)"
      else extras+="${extras:+,}http://localhost:$(http_port $i)"; fi
      i=$((i+1))
    done
    # DOORWAY_ID models alpha's alpha-elohim-host: the EPR router filters
    # projections by doorway_id, so an unset id matches ZERO seeded rows.
    DOORWAY_ID="${DOORWAY_ID:-alpha-elohim-host}" \
    nohup "$DOORWAY_BIN" --dev-mode --listen "0.0.0.0:$DOORWAY_PORT" \
      --conductor-url "ws://localhost:$(admin_port 0)" \
      --storage-url "$primary" ${extras:+--storage-urls "$extras"} \
      --bootstrap-enabled --signal-enabled > "$LOGDIR/doorway.log" 2>&1 &
    for _ in $(seq 1 20); do
      curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null && break; sleep 1
    done
    echo "doorway up on :$DOORWAY_PORT (bootstrap+signal enabled)"
  else
    echo "doorway already up on :$DOORWAY_PORT"
  fi

  # 2. Conductors: one hc sandbox generate, N sandboxes, pinned ports,
  #    discovery via the local doorway.
  if [ "$(ss -tln | grep -cE "127.0.0.1:$(admin_port 0) ")" -eq 0 ]; then
    cd "$LOCAL_DEV_DIR" || exit 2
    rm -rf .hc .sandbox_log "${PEERS[@]}"
    local fports="" rports=""
    local i=0
    for _ in "${PEERS[@]}"; do
      fports+="${fports:+,}$(admin_port $i)"; rports+="${rports:+,}$(app_port $i)"; i=$((i+1))
    done
    nohup sh -c "echo test | hc sandbox --piped -f $fports generate -n ${#PEERS[@]} \
      --app-id elohim --in-process-lair -r=$rports --root \"\$PWD\" -d $MESH_PEERS \
      \"$HAPP_PATH\" network --bootstrap http://localhost:$DOORWAY_PORT/bootstrap \
      webrtc ws://signal.localhost:$DOORWAY_PORT" > .sandbox_log 2>&1 &
    echo -n "waiting for ${#PEERS[@]} conductors (cold install can take ~2-4 min)"
    for _ in $(seq 1 90); do
      [ "$(ss -tln | grep -cE "127.0.0.1:($(echo "$fports" | tr ',' '|')) ")" -ge ${#PEERS[@]} ] && break
      grep -qa "Payload: Could not" .sandbox_log && { echo; echo "conductor failed — see $LOCAL_DEV_DIR/.sandbox_log"; exit 1; }
      printf "."; sleep 3
    done
    echo " up"
  else
    echo "conductors already up"
  fi

  # 3. Storage peers: one per conductor, agent key read from its conductor.
  local i=0
  for name in "${PEERS[@]}"; do
    if ! curl -s -m 2 "http://localhost:$(http_port $i)/health" >/dev/null; then
      local agent
      agent=$(hc sandbox call --running "$(admin_port $i)" list-apps 2>/dev/null \
        | grep -o '"agent_pub_key":"[^"]*"' | head -1 | cut -d'"' -f4)
      mkdir -p "$MESH_DIR/$name"
      HOLOCHAIN_ADMIN_URL="ws://localhost:$(admin_port $i)" \
      HOLOCHAIN_APP_URL="ws://localhost:$(app_port $i)" \
      STORAGE_DIR="$MESH_DIR/$name" \
      ENABLE_CONTENT_DB=true ENABLE_IMPORT_API=true \
      ENABLE_P2P=true P2P_PORT="$(p2p_port $i)" \
      AGENT_PUBKEY="$agent" RELAY_MODE=server \
      GENESIS_SELF_HEAL_IDENTITY=1 SELF_HUMAN_ID="$(human_id "$name")" \
      HOUSEHOLD_ID=household-dowell \
      DEVICE_ARCHETYPE=device-family-node-base \
      ELOHIM_STORAGE_PEER_POLICY_PATH="$MESH_DIR/peer-policy.toml" \
      nohup "$STORAGE_BIN" --http-port "$(http_port $i)" > "$LOGDIR/$name.log" 2>&1 &
      echo "storage $name: http=$(http_port $i) p2p=$(p2p_port $i) agent=${agent:0:16}..."
    else
      echo "storage $name already up on :$(http_port $i)"
    fi
    i=$((i+1))
  done

  for _ in $(seq 1 30); do
    local ok=0 j=0
    for _n in "${PEERS[@]}"; do
      curl -s -m 2 "http://localhost:$(http_port $j)/health" >/dev/null && ok=$((ok+1)); j=$((j+1))
    done
    [ "$ok" -ge ${#PEERS[@]} ] && break; sleep 2
  done

  echo
  status_all
  echo
  echo "next: ./hc-mesh.sh probe   # run the CI Dataplane Validation probes here"
}

case "${1:-start}" in
  start)  start_all ;;
  stop)   stop_all ;;
  status) status_all ;;
  probe)  probe_all ;;
  *) echo "usage: hc-mesh.sh [start|stop|status|probe]"; exit 2 ;;
esac
