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
#   MONGOD_BIN      mongod binary (default: first of $PATH mongod, ~/bin/mongod);
#                   empty/absent => the doorways run WITHOUT an archive (inert
#                   warm-shell store, memory-only projection) exactly as before
#   MONGO_PORT      loopback mongod port for the doorways' projection archive
#                   (default: 27017 — the doorway's own MONGODB_URI default)
#
#   Dev-tier pacing profile (see the block below the port-scheme helpers —
#   minutes-quiesce plan W3): MESH_RECONCILE_SECS, MESH_CONTEST_BACKOFF,
#   MESH_HEAL_MISSING_BACKOFF, MESH_EVIDENCE_ABSENT_BACKOFF,
#   MESH_HEAD_CORPUS_DIGEST override the storage peers' reconcile/backoff
#   cadence; the conductor sandboxes also get a fixed kitsune2 gossip
#   acceleration patch (k2Gossip initiate=1000ms) after generate, before run.
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
# mongod backs the doorways' Mongo-side projection archive (app_file_cache /
# warm-shell ShellArchive / DoorwayResolver store). Without it every doorway
# constructs an INERT WarmShellStore and a memory-only projection — which is
# precisely the production shape 18a65fd0d found un-wired, so the archive leg
# of the boot-order self-heal family could never be proven on this mesh.
# Loopback-only, per-doorway database (doorway-a / doorway-b) so A and B do
# not share an archive and mask each other's boot-order gaps.
MONGOD_BIN="${MONGOD_BIN-$(command -v mongod 2>/dev/null || { [ -x "$HOME/bin/mongod" ] && echo "$HOME/bin/mongod"; })}"
MONGO_PORT="${MONGO_PORT:-27017}"
MONGO_DIR="$MESH_DIR/mongo"
LOGDIR="$MESH_DIR/logs"

# Port scheme per peer index i (0-based): admin 4444+10i, app 4445+10i,
# storage http 8090+i, libp2p 9701+i.
IFS=',' read -ra PEERS <<< "$MESH_PEERS"

admin_port() { echo $((4444 + 10 * $1)); }
app_port()   { echo $((4445 + 10 * $1)); }
http_port()  { echo $((8090 + $1)); }
p2p_port()   { echo $((9701 + $1)); }

# ---------------------------------------------------------------------------
# dev-tier pacing profile (declared preproduction stakes — see minutes-quiesce
# plan W3: genesis/docs/superpowers/plans/2026-08-16-minutes-quiesce-fixture-
# trust-swarm-plan.md §3). Same reconcile/backoff machinery elohim-storage
# runs everywhere — never a parallel dev path — just tuned to a declared
# preproduction cadence so the 3-peer local mesh converges in minutes
# instead of the ~90min baseline (.claude/shifts/2026-08-16T04-15-local-
# mesh-saga-delivery.journal.md). Every knob overrides via its own MESH_*
# var; defaults below are the dev-tier stakes, never applied silently in
# prod (this file is dev-only). Names verified against elohim-storage/src/
# {main.rs,config.rs} 2026-08-16 — these are the exact env vars the storage
# binary reads, not aliases:
#   PROJECTION_RECONCILE_SECS            reconcile sweep cadence (prod default 300s)
#   CONTEST_BACKOFF_SECONDS               contest-backoff ladder rung (prod default 3600s)
#   HEAL_MISSING_BACKOFF_SECONDS          heal-missing backoff rung (prod default 600s)
#   ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS   evidence-absent backoff rung (prod default 86400s)
#   ELOHIM_HEAD_CORPUS_DIGEST             T5 digest-requester flip (prod default off/0)
#   ELOHIM_NETWORK_STAKES                 T10 declared-stakes operator-config leg (prod:
#                                          unset -> Bootstrap fail-closed default). The
#                                          explicit Simulacra declaration for THIS local
#                                          preproduction mesh only — Simulacra is never a
#                                          default and never derived from DEV_MODE; it is
#                                          reached only by this positive declaration
#                                          (trust::manifest_resolver::ManifestStakesResolver,
#                                          T10 runtime half, §3 W2 task Q6).
# ---------------------------------------------------------------------------
PROJECTION_RECONCILE_SECS="${MESH_RECONCILE_SECS:-30}"
CONTEST_BACKOFF_SECONDS="${MESH_CONTEST_BACKOFF:-120}"
HEAL_MISSING_BACKOFF_SECONDS="${MESH_HEAL_MISSING_BACKOFF:-60}"
ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS="${MESH_EVIDENCE_ABSENT_BACKOFF:-600}"
ELOHIM_HEAD_CORPUS_DIGEST="${MESH_HEAD_CORPUS_DIGEST:-1}"
# Adopt-before-author pre-flight ON for the mesh: without it cross-peer head
# divergence has no adopt discharge and accumulates into contest grind (the
# 2026-08-16 measure's 23→81 actionable plateau); the overnight converged run
# had it live (adopt_peer canonical links). Prod default stays off.
ELOHIM_ADOPT_BEFORE_AUTHOR="${MESH_ADOPT_BEFORE_AUTHOR:-1}"
# Serialize adopt/contest declares on the mesh: concurrent declares race the
# conductor source-chain head ("bundle head has moved", 2026-08-16 measure) and
# every collision costs a fallback + next-sweep retry — fanout 1 lands first-try.
# Declared profile difference vs alpha (6); attributed in the transfer ratio.
ADOPT_CONTEST_FANOUT="${MESH_ADOPT_FANOUT:-1}"
# Explicit preproduction Simulacra declaration for the local mesh's storage peers
# (never a default — see the comment block above).
ELOHIM_NETWORK_STAKES="${MESH_NETWORK_STAKES:-simulacra}"

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
  # mongod: exact dbpath identity, never a bare "mongod" pattern.
  kill $(pgrep -f "mongod --dbpath $MESH_DIR/mong[o]") 2>/dev/null
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
  printf "doorwayB :%s " "${DOORWAY_B_PORT:-8889}"
  curl -s -m 2 "http://localhost:${DOORWAY_B_PORT:-8889}/health" >/dev/null && echo UP || echo down
  printf "mongod   :%s " "$MONGO_PORT"
  if (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null; then echo "UP (archive-backed doorways)"; else echo "down (doorways run archive-less: inert warm shell)"; fi
  echo
  echo "probe env:  PEER_STORAGE_URLS=\"$(peer_csv)\" INTERNAL_DOORWAY_URL=\"localhost:$DOORWAY_PORT\" E2E_DOORWAY_B=\"http://localhost:${DOORWAY_B_PORT:-8889}\""
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

  # 0. mongod — the doorways' projection archive. Must be listening BEFORE a
  #    doorway boots: AppState::init_projection binds the archive at startup
  #    (bind_warm_shell_to_archive), and a doorway that boots archive-less
  #    stays inert for its whole life. Optional: no binary => skip, and the
  #    doorways degrade to today's memory-only shape (status says so).
  if [ -n "$MONGOD_BIN" ] && [ -x "$MONGOD_BIN" ]; then
    if ! (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null; then
      mkdir -p "$MONGO_DIR"
      "$MONGOD_BIN" --dbpath "$MONGO_DIR" --bind_ip 127.0.0.1 --port "$MONGO_PORT" \
        --fork --logpath "$LOGDIR/mongod.log" >/dev/null 2>&1 \
        || echo "WARN: mongod failed to start (see $LOGDIR/mongod.log) — doorways will run archive-less" >&2
      for _ in $(seq 1 20); do
        (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null && break; sleep 1
      done
      echo "mongod up on :$MONGO_PORT (dbpath $MONGO_DIR)"
    else
      echo "mongod already up on :$MONGO_PORT"
    fi
  else
    echo "mongod not found (MONGOD_BIN unset/absent) — doorways will run archive-less (inert warm shell)"
  fi

  # 1. Doorway first: it is the island DHT's bootstrap + signal home.
  if ! curl -s -m 2 "http://localhost:$DOORWAY_PORT/health" >/dev/null; then
    local i=0 primary="" extras=""
    for name in "${PEERS[@]}"; do
      if [ $i -eq 0 ]; then primary="http://127.0.0.1:$(http_port $i)"
      else extras+="${extras:+,}http://127.0.0.1:$(http_port $i)"; fi
      i=$((i+1))
    done
    # DOORWAY_ID models alpha's alpha-elohim-host: the EPR router filters
    # projections by doorway_id, so an unset id matches ZERO seeded rows.
    # SSR env mirrors genesis/orchestrator/manifests/doorway/alpha.yaml: the
    # RendererRegistry materializes each slug's server bundle from the
    # substrate (serverBlobHash staged via scripts/ci/stage-spa-blob.sh); an
    # unstaged slug degrades to CSR with x-ssr-skipped. The landing browser
    # bundle carries only index.csr.html, so WITHOUT SSR the / mount 404s.
    DOORWAY_ID="${DOORWAY_ID:-alpha-elohim-host}" \
    MONGODB_URI="mongodb://127.0.0.1:$MONGO_PORT" MONGODB_DB="doorway-a" \
    ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
    SSR_BUNDLE_PATH="${SSR_BUNDLE_PATH:-$REPO_ROOT/app/elohim-app/dist/elohim-app/server/main.server.mjs}" \
    SSR_BUNDLE_SLUG="${SSR_BUNDLE_SLUG:-elohim-host-landing}" \
    SSR_BUNDLE_SLUGS="${SSR_BUNDLE_SLUGS:-elohim-host-landing,lamad-spa}" \
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

  # 1b. Doorway B (apex/elohim.host stand-in): jessica-primary, NO bootstrap/
  # signal (A owns discovery — two mem-bootstrap doorways would partition the
  # island DHT). Gives the saga's cross-doorway legs a LOCAL target instead of
  # bleeding to the live production doorway (E2E_DOORWAY_B).
  DOORWAY_B_PORT="${DOORWAY_B_PORT:-8889}"
  if ! curl -s -m 2 "http://localhost:$DOORWAY_B_PORT/health" >/dev/null; then
    DOORWAY_ID="${DOORWAY_B_ID:-apex-elohim-host}" \
    MONGODB_URI="mongodb://127.0.0.1:$MONGO_PORT" MONGODB_DB="doorway-b" \
    ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
    SSR_BUNDLE_PATH="${SSR_BUNDLE_PATH:-$REPO_ROOT/app/elohim-app/dist/elohim-app/server/main.server.mjs}" \
    SSR_BUNDLE_SLUG="${SSR_BUNDLE_SLUG:-elohim-host-landing}" \
    SSR_BUNDLE_SLUGS="${SSR_BUNDLE_SLUGS:-elohim-host-landing,lamad-spa}" \
    nohup "$DOORWAY_BIN" --dev-mode --listen "0.0.0.0:$DOORWAY_B_PORT" \
      --conductor-url "ws://localhost:$(admin_port 1)" \
      --storage-url "http://127.0.0.1:$(http_port 1)" \
      --storage-urls "http://127.0.0.1:$(http_port 0),http://127.0.0.1:$(http_port 2)" \
      > "$LOGDIR/doorway-b.log" 2>&1 &
    for _ in $(seq 1 20); do
      curl -s -m 2 "http://localhost:$DOORWAY_B_PORT/health" >/dev/null && break; sleep 1
    done
    echo "doorway B up on :$DOORWAY_B_PORT (apex stand-in, jessica-primary)"
  else
    echo "doorway B already up on :$DOORWAY_B_PORT"
  fi

  # 2. Conductors: hc sandbox generate (installs the happ + writes each
  #    sandbox's conductor-config.yaml, but does NOT launch the conductor —
  #    only `-r`/`run` does that per `hc sandbox generate --help`), THEN
  #    patch the dev-tier gossip config into each written config, THEN run.
  #    Splitting generate from run (the old code combined them via
  #    `generate -r=$rports`) creates the window the pacing profile needs:
  #    the conductor must not boot until the patch has landed, or its first
  #    kitsune2 gossip round starts at prod cadence.
  if [ "$(ss -tln | grep -cE "127.0.0.1:$(admin_port 0) ")" -eq 0 ]; then
    cd "$LOCAL_DEV_DIR" || exit 2
    rm -rf .hc .sandbox_log .sandbox_run_log "${PEERS[@]}"
    local fports="" rports=""
    local i=0
    for _ in "${PEERS[@]}"; do
      fports+="${fports:+,}$(admin_port $i)"; rports+="${rports:+,}$(app_port $i)"; i=$((i+1))
    done

    echo -n "generating ${#PEERS[@]} conductor sandboxes (cold install can take ~2-4 min)"
    timeout 300 sh -c "echo test | hc sandbox --piped -f $fports generate -n ${#PEERS[@]} \
      --app-id elohim --in-process-lair --root \"\$PWD\" -d $MESH_PEERS \
      \"$HAPP_PATH\" network --bootstrap http://localhost:$DOORWAY_PORT/bootstrap \
      webrtc ws://signal.localhost:$DOORWAY_PORT" > .sandbox_log 2>&1
    gen_status=$?
    echo " done"
    if [ "$gen_status" -ne 0 ] || grep -qa "Payload: Could not" .sandbox_log; then
      echo "conductor generate failed (exit=$gen_status) — see $LOCAL_DEV_DIR/.sandbox_log"
      exit 1
    fi

    # -------------------------------------------------------------------
    # dev-tier gossip acceleration (declared preproduction stakes — see
    # minutes-quiesce plan W3). Holochain 0.6's ConductorConfig.network.
    # advanced JSON passes straight through to kitsune2
    # (holochain-conductor crates/holochain_conductor_api/src/config/
    # conductor.rs with_gossip_*_interval_ms; the k2Gossip module name and
    # camelCase keys are its wire shape). initiateIntervalMs /
    # minInitiateIntervalMs at 1000ms match what SweetConductorConfig::
    # standard() sets for upstream tests (sweettest/sweet_conductor_
    # config.rs:82-84) — vs prod defaults of 120_000ms / 300_000ms
    # (elohim/kitsune2 crates/gossip/src/config.rs). initialInitiateIntervalMs
    # (first-round-only interval; same struct, same file — confirmed present
    # under `#[serde(rename_all = "camelCase")]` before wiring it here) is
    # set explicitly for clarity — its default is already 1000ms.
    # Boot-verified 2026-08-16: a scratch single-sandbox conductor patched
    # with this exact block came up clean (admin port live, `list-apps`
    # returned the installed happ, no config-parse errors) before this loop
    # was wired to run for real.
    # -------------------------------------------------------------------
    for name in "${PEERS[@]}"; do
      python3 - "$LOCAL_DEV_DIR/$name/conductor-config.yaml" <<'PYEOF'
import sys
import yaml

path = sys.argv[1]
with open(path) as f:
    cfg = yaml.safe_load(f)

network = cfg.setdefault("network", {})
advanced = network.get("advanced") or {}
k2gossip = advanced.get("k2Gossip") or {}
k2gossip["initiateIntervalMs"] = 1000
k2gossip["minInitiateIntervalMs"] = 1000
k2gossip["initialInitiateIntervalMs"] = 1000
advanced["k2Gossip"] = k2gossip
network["advanced"] = advanced

with open(path, "w") as f:
    yaml.safe_dump(cfg, f, default_flow_style=False, sort_keys=False)
PYEOF
      if [ $? -ne 0 ]; then
        echo "gossip-config patch failed for $name — see $LOCAL_DEV_DIR/$name/conductor-config.yaml"
        exit 1
      fi
    done
    echo "dev-tier gossip config patched into ${#PEERS[@]} conductor-config.yaml (k2Gossip initiate=1000ms)"

    nohup sh -c "echo test | hc sandbox --piped -f $fports run -a -p=$rports" > .sandbox_run_log 2>&1 &
    echo -n "waiting for ${#PEERS[@]} conductors to boot"
    for _ in $(seq 1 90); do
      [ "$(ss -tln | grep -cE "127.0.0.1:($(echo "$fports" | tr ',' '|')) ")" -ge ${#PEERS[@]} ] && break
      grep -qa "Payload: Could not" .sandbox_run_log && { echo; echo "conductor run failed — see $LOCAL_DEV_DIR/.sandbox_run_log"; exit 1; }
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
      PROJECTION_RECONCILE_SECS="$PROJECTION_RECONCILE_SECS" \
      CONTEST_BACKOFF_SECONDS="$CONTEST_BACKOFF_SECONDS" \
      HEAL_MISSING_BACKOFF_SECONDS="$HEAL_MISSING_BACKOFF_SECONDS" \
      ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS="$ELOHIM_EVIDENCE_ABSENT_BACKOFF_SECS" \
      ELOHIM_HEAD_CORPUS_DIGEST="$ELOHIM_HEAD_CORPUS_DIGEST" \
      ELOHIM_ADOPT_BEFORE_AUTHOR="$ELOHIM_ADOPT_BEFORE_AUTHOR" \
      ADOPT_CONTEST_FANOUT="$ADOPT_CONTEST_FANOUT" \
      ELOHIM_NETWORK_STAKES="$ELOHIM_NETWORK_STAKES" \
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
