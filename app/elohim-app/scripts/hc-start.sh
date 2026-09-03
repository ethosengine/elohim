#!/bin/bash
#
# hc-start.sh - Start Elohim P2P Framework
#
# USAGE:
#   ./hc-start.sh [OPTIONS]
#
# OPTIONS:
#   -h, --help       Show this help message
#   -s, --seed       Run sample seed after startup
#   -c, --conductor  Start conductor only (no storage/doorway)
#   -b, --build      Force rebuild all components
#
# ENVIRONMENT VARIABLES:
#   STORAGE_PORT     Storage HTTP port (default: 8090)
#   STORAGE_DIR      Storage data directory, passed through to the
#                    elohim-storage binary (binary default: ~/.local/share/elohim-storage)
#   SEED_LIMIT       Number of items to seed with --seed (default: 200)
#   NETWORK_PROFILE  Conductor network profile (default: isolated)
#                      isolated   - island DHT, no external peers (today's behavior)
#                      join-alpha - join the alpha DHT via the deployed doorway's
#                                   bootstrap + relay; auto-fetches the
#                                   DEPLOYED bundle (scripts/fetch-deployed-dna.sh)
#                                   so DNA hashes match what alpha runs
#   CONDUCTOR_BOOTSTRAP_URL  Bootstrap URL, join-alpha only
#                            (default: https://doorway-alpha.elohim.host/bootstrap)
#   CONDUCTOR_RELAY_URL      Iroh relay URL, join-alpha only
#                            (default: https://relay.alpha.elohim.host)
#   FORCE_LOCAL_HAPP         join-alpha only: =1 installs the locally-built hApp
#                            instead of the fetched deployed bundle (PARTITION
#                            risk if DNA hashes differ — warning printed)
#   DEPLOYED_HAPP_TAG/DEPLOYED_HAPP_BRANCH  Pin the deployed-bundle fetch
#                            (defaults: dev-latest / dev — see fetch-deployed-dna.sh)
#   DOORWAY_AUTH     Doorway auth posture (default: auto)
#                      auto   - secure when mongod is available, else keyless
#                      secure - REQUIRE the secure posture; fail if mongod is absent
#                      keyless- native local-first; no account store, no JWT secret
#   MONGOD_BIN/MONGO_PORT    Account + projection store for the secure posture
#                            (defaults: first mongod on PATH / 27017)
#   CONDUCTOR_RELEASE_CHANNELS  join-alpha only: passed through as
#                            ELOHIM_RELEASE_CHANNELS into ONLY the storage
#                            process's environment (format: channelId or
#                            channelId=observe|apply|canary). Unset = this
#                            peer follows no release channel. See GET
#                            /admin/adoption on the storage peer.
#
# COMPONENTS:
#   1. Holochain Conductor - Cryptographic provenance & agent identity
#   2. elohim-storage      - SQLite content DB + blob storage
#   3. Doorway gateway     - HTTP/WS proxy unifying the stack
#
# The Elohim Protocol is a P2P framework where Holochain provides
# cryptographic identity and provenance. Content lives in elohim-storage.
#
# EXAMPLES:
#   ./hc-start.sh                    # Start full stack (default)
#   ./hc-start.sh --seed             # Start + seed sample content
#   ./hc-start.sh --conductor        # Conductor only (rare, for debugging)
#
# AFTER STARTUP:
#   Health check: curl http://localhost:8888/status
#   Content API:  curl http://localhost:8888/db/stats
#   Stop all:     npm run hc:stop
#

set -e

# ============================================================================
# Configuration
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(cd "$APP_DIR/../.." && pwd)"
HC_DIR="$APP_DIR/../../elohim/holochain"
LOCAL_DEV_DIR="$HC_DIR/local-dev"
HAPP_PATH="$HC_DIR/dna/elohim/workdir/elohim.happ"
HC_PORTS_FILE="$LOCAL_DEV_DIR/.hc_ports"

# Native binaries belong in the governed cargo pool. DNA/WASM builds below
# deliberately remain in-tree because `hc dna pack` canonicalizes ./target.
source "$REPO_ROOT/genesis/agentic/bin/pool-lib.sh"
POOL_FAMILY="$(detect_family "$REPO_ROOT")"
STORAGE_TARGET_DIR="$(slot_path "$POOL_FAMILY" "elohim/elohim-storage" release)"
DOORWAY_TARGET_DIR="$(slot_path "$POOL_FAMILY" "doorway/doorway-service" release)"
mkdir -p "$(readlink -m "$STORAGE_TARGET_DIR")" "$(readlink -m "$DOORWAY_TARGET_DIR")"

# Environment with defaults
: "${STORAGE_PORT:=8090}"
: "${SEED_LIMIT:=200}"
: "${NETWORK_PROFILE:=isolated}"
: "${DOORWAY_AUTH:=auto}"
: "${MONGO_PORT:=27017}"
MONGOD_BIN="${MONGOD_BIN-$(command -v mongod 2>/dev/null || { [ -x "$HOME/bin/mongod" ] && echo "$HOME/bin/mongod"; })}"
MONGO_DIR="$LOCAL_DEV_DIR/mongo"
DOORWAY_STATE_DIR="$LOCAL_DEV_DIR/doorway"
JWT_SECRET_FILE="$DOORWAY_STATE_DIR/jwt-secret"
ADMIN_KEY_FILE="$DOORWAY_STATE_DIR/api-key-admin"

# Options
RUN_SEED=false
CONDUCTOR_ONLY=false
FORCE_BUILD=false

# ============================================================================
# Help
# ============================================================================

show_help() {
    sed -n '/^#/!q;s/^# \?//p' "$0" | tail -n +3
    exit 0
}

# ============================================================================
# Parse Arguments
# ============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            ;;
        -s|--seed)
            RUN_SEED=true
            shift
            ;;
        -c|--conductor)
            CONDUCTOR_ONLY=true
            shift
            ;;
        -b|--build)
            FORCE_BUILD=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# ============================================================================
# Network Profile
# ============================================================================
# isolated   (default) - island DHT; the generate command is byte-identical to
#                        the historical behavior. No external peers.
# join-alpha           - the local conductor joins the alpha DHT via the deployed
#                        doorway's bootstrap endpoint and that doorway's iroh relay.
# NOTE: doorway runtime env vars are a different concern (Tauri native-handoff
# channel) — these CONDUCTOR_* vars are conductor-only.

case "$NETWORK_PROFILE" in
    isolated)
        ;;
    join-alpha)
        : "${CONDUCTOR_BOOTSTRAP_URL:=https://doorway-alpha.elohim.host/bootstrap}"
        ;;
    *)
        echo "Unknown NETWORK_PROFILE: '$NETWORK_PROFILE' (expected: isolated | join-alpha)"
        exit 1
        ;;
esac

# ──────────────────────────────────────────────────────────────────────────────
# Conductor parity (2026-08-28, sovereign-peer T3 rung). Alpha's conductors run
# the ethosengine FORK on the iroh transport (agent URLs are
# https://relay.alpha.elohim.host/…, conductor-config carries relay_url).
# So, like hc-mesh.sh, a matching 0.7 pair (holochain +
# the MATCHING hc — the CLI writes conductor-config.yaml in its own schema) is
# used when one is present, and join-alpha REFUSES a stock conductor unless
# ALLOW_STOCK_JOIN=1 says the skew is deliberate.
#   HOLOCHAIN_BIN=/dir/holding/both        explicit pair
#   MESH_FORK_BIN_DIRS=a:b:c               search list (hc-mesh.sh's convention)
# Default search adds the cargo-pool release slot the fork is built into.
# ──────────────────────────────────────────────────────────────────────────────
: "${CONDUCTOR_RELAY_URL:=https://relay.alpha.elohim.host}"   # doorway-A relay (D2/D7)
# App-interface port. The household mesh owns 4445/4455/4465 (matthew/jessica/james, hc-mesh.sh
# `app_port()`), and a join-alpha workspace conductor is meant to run BESIDE that mesh (T2 and T3
# rungs together), so join-alpha defaults to 4485 — out of the mesh's range. Isolated keeps 4445
# (the app dev proxy's assumption). Consumers read app_port from .hc_ports, never a constant.
if [ "$NETWORK_PROFILE" = "join-alpha" ]; then : "${CONDUCTOR_APP_PORT:=4485}"; else : "${CONDUCTOR_APP_PORT:=4445}"; fi
# Arc factor (fork hc only). 1 = a full peer (parity with the fleet's humans; reads are answered
# LOCALLY because a full-arc node is its own authority, so a fresh joiner sees the fleet's data only
# as gossip fills its rings). 0 = a zero-arc "leecher" whose reads go to the network at once and
# who contributes nothing to gossip. Measured 2026-08-28: the difference between the two is the
# whole of "can the workspace read elohim-host-landing within 3 minutes".
: "${CONDUCTOR_ARC_FACTOR:=1}"
FORK_BIN_DIR=""
_fork_candidates="${HOLOCHAIN_BIN:-}:${MESH_FORK_BIN_DIRS:-}:$REPO_ROOT/.fork-bin:/opt/elohim/fork-bin"
for _d in /projects/.cargo-target-pool/family/*/crates/*/release; do _fork_candidates="$_fork_candidates:$_d"; done
IFS=':' read -ra _fork_dirs <<< "$_fork_candidates"
for _d in "${_fork_dirs[@]}"; do
    [ -n "$_d" ] || continue
    [ -f "$_d" ] && _d="$(dirname "$_d")"
    if [ -x "$_d/holochain" ] && [ -x "$_d/hc" ]; then FORK_BIN_DIR="$_d"; break; fi
done
if [ "$NETWORK_PROFILE" = "join-alpha" ]; then
    echo ""
    echo "   ── device-peer preflight ───────────────────────────────────────"
    if [ -n "$FORK_BIN_DIR" ]; then
        echo "   ✓ fork holochain+hc pair: $FORK_BIN_DIR"
    else
        echo "   ✗ fork holochain+hc pair: none found (refused below unless ALLOW_STOCK_JOIN=1)"
    fi
    echo "   ✓ CONDUCTOR_APP_PORT=$CONDUCTOR_APP_PORT (default 4485; household mesh owns 4445/4455/4465)"
    echo "   ✓ CONDUCTOR_ARC_FACTOR=$CONDUCTOR_ARC_FACTOR"
    if [ "${FORCE_LOCAL_HAPP:-0}" = "1" ]; then
        echo "   ✓ hApp source: local build (FORCE_LOCAL_HAPP=1)"
    else
        echo "   ✓ hApp source: fetched deployed bundle (scripts/fetch-deployed-dna.sh)"
    fi
    if [ "$DOORWAY_AUTH" = "secure" ] && { [ -z "$MONGOD_BIN" ] || [ ! -x "$MONGOD_BIN" ]; }; then
        echo "   ✗ DOORWAY_AUTH=secure but no mongod found (set MONGOD_BIN)"
    elif [ -n "$MONGOD_BIN" ] && [ -x "$MONGOD_BIN" ]; then
        echo "   ✓ DOORWAY_AUTH=$DOORWAY_AUTH (mongod resolves: $MONGOD_BIN)"
    else
        echo "   ✓ DOORWAY_AUTH=$DOORWAY_AUTH (no mongod found — will run keyless)"
    fi
    if [ -n "${CONDUCTOR_RELEASE_CHANNELS:-}" ]; then
        echo "   ✓ CONDUCTOR_RELEASE_CHANNELS=$CONDUCTOR_RELEASE_CHANNELS"
    else
        echo "   ✗ CONDUCTOR_RELEASE_CHANNELS unset (not following any release channel)"
    fi
    echo "   ─────────────────────────────────────────────────────────────"
fi

if [ -n "$FORK_BIN_DIR" ]; then
    export HC_HOLOCHAIN_PATH="$FORK_BIN_DIR/holochain"
    export PATH="$FORK_BIN_DIR:$PATH"
    echo "   🔧 conductor: FORK pair $FORK_BIN_DIR ($("$FORK_BIN_DIR/holochain" --version 2>/dev/null | head -1))"
elif [ "$NETWORK_PROFILE" = "join-alpha" ] && [ "${ALLOW_STOCK_JOIN:-0}" != "1" ]; then
    echo ""
    echo "   ❌ join-alpha refused: no fork conductor pair found (holochain + hc in one dir)."
    echo "      Alpha's conductors run the iroh transport; the stock $(holochain --version 2>/dev/null | head -1) on PATH"
    echo "      would publish itself, be listed, and never connect (connections: [] — measured"
    echo "      2026-08-28). Point HOLOCHAIN_BIN at a dir holding BOTH binaries, e.g."
    echo "      HOLOCHAIN_BIN=/projects/.cargo-target-pool/family/dev/crates/dev/release"
    echo "      or set ALLOW_STOCK_JOIN=1 to accept a listed-but-unconnected peer on purpose."
    exit 1
fi

# ============================================================================
# Functions
# ============================================================================

get_admin_port() {
    local port
    if [ -f "$HC_PORTS_FILE" ]; then
        port=$(grep "admin_port" "$HC_PORTS_FILE" | grep -o "[0-9]*" | head -1)
        # Staleness guard: the ports file survives conductor crashes/restarts.
        # Only trust the recorded port if something actually accepts a TCP
        # connection there; otherwise return nothing so callers regenerate.
        if [ -n "$port" ] && timeout 1 bash -c "exec 3<>/dev/tcp/127.0.0.1/$port" 2>/dev/null; then
            echo "$port"
        fi
    fi
}

# ============================================================================
# Main
# ============================================================================

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "🔷 Elohim P2P Framework Startup"
echo "════════════════════════════════════════════════════════════════"
echo ""
if [ "$CONDUCTOR_ONLY" = true ]; then
    echo "   Mode: Conductor only (debug mode)"
else
    echo "   Mode: Full stack (Conductor + Storage + Doorway)"
fi
echo ""

# ──────────────────────────────────────────────────────────────────────────────
# Step 1: Build hApp if needed
# ──────────────────────────────────────────────────────────────────────────────
if [ "$NETWORK_PROFILE" = "join-alpha" ] && [ "${FORCE_LOCAL_HAPP:-0}" != "1" ]; then
    # join-alpha installs the FETCHED deployed bundle (DNA-hash parity with
    # alpha), so the locally-built hApp would never be installed — skip the
    # multi-minute WASM build. FORCE_LOCAL_HAPP=1 restores local build+install.
    echo "⏭️  Skipping local DNA build: NETWORK_PROFILE=join-alpha installs the"
    echo "   fetched deployed bundle (set FORCE_LOCAL_HAPP=1 to build + install local)"
    echo ""
elif [ ! -f "$HAPP_PATH" ] || [ "$FORCE_BUILD" = true ]; then
    echo "┌──────────────────────────────────────────────────────────────┐"
    echo "│ Building Holochain DNAs                                       │"
    echo "└──────────────────────────────────────────────────────────────┘"

    WORKDIR="$HC_DIR/dna/elohim/workdir"
    mkdir -p "$WORKDIR"

    echo "📦 Building lamad DNA..."
    cd "$HC_DIR/dna/elohim"
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
    hc dna pack . -o "$WORKDIR/lamad.dna"

    echo "📦 Building imagodei DNA..."
    cd "$HC_DIR/dna/imagodei"
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
    hc dna pack . -o "$WORKDIR/imagodei.dna"

    echo "📦 Building infrastructure DNA..."
    cd "$HC_DIR/dna/infrastructure"
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
    hc dna pack . -o "$WORKDIR/infrastructure.dna"

    echo "📦 Building mishpat DNA..."
    cd "$HC_DIR/dna/mishpat"
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
    hc dna pack . -o "$WORKDIR/mishpat.dna"

    echo "📦 Building node-registry DNA..."
    cd "$HC_DIR/dna/node-registry"
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
    # happ.yaml references ../../node-registry/node-registry.dna (peer-level), so
    # output the bundle next to the DNA's own dna.yaml rather than under elohim/workdir/.
    hc dna pack . -o "node-registry.dna"

    echo "📦 Packing elohim.happ..."
    hc app pack "$WORKDIR" -o "$WORKDIR/elohim.happ"

    echo "✅ DNAs built (lamad + imagodei + infrastructure + mishpat + node-registry)"
    echo ""
fi

# ──────────────────────────────────────────────────────────────────────────────
# Step 2: Start Conductor
# ──────────────────────────────────────────────────────────────────────────────
echo "┌──────────────────────────────────────────────────────────────┐"
echo "│ Step 1: Holochain Conductor                                   │"
echo "└──────────────────────────────────────────────────────────────┘"

ADMIN_PORT=$(get_admin_port)
CONDUCTOR_RUNNING=false

if [ -n "$ADMIN_PORT" ] && hc sandbox call --running "$ADMIN_PORT" list-apps >/dev/null 2>&1; then
    echo "   ✅ Conductor already running on port $ADMIN_PORT"
    CONDUCTOR_RUNNING=true
    if [ "$NETWORK_PROFILE" = "join-alpha" ]; then
        echo "   ⚠️  NETWORK_PROFILE=join-alpha requested, but the network profile"
        echo "      only applies at sandbox generate time. This conductor keeps"
        echo "      whatever network config it was generated with."
        echo "      To re-generate: npm run hc:stop, then re-run with the profile."
    fi
fi

if [ "$CONDUCTOR_RUNNING" = false ]; then
    echo "   🚀 Starting Holochain sandbox..."
    mkdir -p "$LOCAL_DEV_DIR"
    cd "$LOCAL_DEV_DIR"

    rm -f "$HC_PORTS_FILE"

    SANDBOX_LOG="$LOCAL_DEV_DIR/.sandbox_log"
    HC_WRAPPER="$LOCAL_DEV_DIR/.hc_wrapper.sh"

    if [ "$NETWORK_PROFILE" = "join-alpha" ]; then
        echo ""
        echo "   ╔══════════════════════════════════════════════════════════════╗"
        echo "   ║  🌐 NETWORK_PROFILE=join-alpha — JOINING THE ALPHA DHT         ║"
        echo "   ╚══════════════════════════════════════════════════════════════╝"
        echo "   Bootstrap: $CONDUCTOR_BOOTSTRAP_URL"
        echo "   Relay:     $CONDUCTOR_RELAY_URL"
        echo "   Arc:       target_arc_factor=$CONDUCTOR_ARC_FACTOR  (1 = full peer, 0 = zero-arc reader; fork only)"
        echo ""
        # DNA-hash parity: install the DEPLOYED bundle (what alpha actually
        # runs), not the locally-built one — mismatched DNA hashes land this
        # peer on a PARTITIONED DHT (same network endpoints, different DHTs —
        # peers never see each other's data). FORCE_LOCAL_HAPP=1 opts out.
        JOIN_HAPP_PATH="$LOCAL_DEV_DIR/deployed-bundles/elohim.happ"
        if [ "${FORCE_LOCAL_HAPP:-0}" = "1" ]; then
            JOIN_HAPP_PATH="$HAPP_PATH"
            echo "   ⚠️  FORCE_LOCAL_HAPP=1: installing the LOCALLY BUILT hApp"
            echo "      ($HAPP_PATH)."
            echo "      If its DNA hashes differ from the deployed alpha bundles,"
            echo "      this peer lands on a PARTITIONED DHT (same network"
            echo "      endpoints, different DHTs — peers will never see each"
            echo "      other's data). Verify hashes with:"
            echo "      scripts/fetch-deployed-dna.sh (prints the deployed hashes)"
            echo ""
        elif "$SCRIPT_DIR/fetch-deployed-dna.sh"; then
            echo "   ✅ Installing the DEPLOYED bundle (DNA-hash parity with alpha):"
            echo "      $JOIN_HAPP_PATH"
            echo ""
        else
            echo ""
            echo "   ❌ join-alpha refused: could not fetch the deployed bundle,"
            echo "      and installing the locally-built hApp risks a PARTITIONED"
            echo "      DHT. Either fix connectivity and re-run, or accept the"
            echo "      risk explicitly with FORCE_LOCAL_HAPP=1."
            exit 1
        fi
        # Holochain 0.7 has one transport. The matching hc CLI writes bootstrap,
        # relay, and arc settings in the conductor's 0.7 schema.
        NETWORK_TAIL="network --bootstrap \"$CONDUCTOR_BOOTSTRAP_URL\" --target-arc-factor $CONDUCTOR_ARC_FACTOR quic \"$CONDUCTOR_RELAY_URL\""
        cat > "$HC_WRAPPER" << EOF
#!/bin/bash
export PATH="$PATH"
${HC_HOLOCHAIN_PATH:+export HC_HOLOCHAIN_PATH="$HC_HOLOCHAIN_PATH"}
exec hc sandbox generate --app-id elohim --in-process-lair -r=$CONDUCTOR_APP_PORT "$JOIN_HAPP_PATH" $NETWORK_TAIL
EOF
    else
        cat > "$HC_WRAPPER" << EOF
#!/bin/bash
exec hc sandbox generate --app-id elohim --in-process-lair -r=$CONDUCTOR_APP_PORT "$HAPP_PATH"
EOF
    fi
    chmod +x "$HC_WRAPPER"

    rm -f "$SANDBOX_LOG"
    # WHY the weird socat line (rationale harvested from the retired
    # elohim/holochain/docs/claude.md, 2026-06-11):
    #   - lair keystore demands an interactive TTY; a plain backgrounded
    #     `hc sandbox generate --in-process-lair` dies with
    #     "No such device or address (os error 6)".
    #   - socat's EXEC:...,pty,setsid,ctty gives the wrapper a real PTY +
    #     session; `(echo "test"; ...)` feeds the keystore passphrase, and
    #     `sleep infinity` holds stdin open so lair never sees EOF.
    #   - PTY output carries null bytes — any parsing of $SANDBOX_LOG must
    #     use `grep -a` (see the admin_port wait loop below).
    nohup sh -c '(echo "test"; sleep infinity) | socat - EXEC:'"$HC_WRAPPER"',pty,setsid,ctty' > "$SANDBOX_LOG" 2>&1 &

    echo -n "   ⏳ Waiting for conductor"
    for i in {1..45}; do
        if grep -qa '"admin_port"' "$SANDBOX_LOG" 2>/dev/null; then
            ADMIN_PORT=$(grep -ao '"admin_port":[0-9]*' "$SANDBOX_LOG" | grep -o '[0-9]*' | head -1)
            if [ -n "$ADMIN_PORT" ]; then
                echo "admin_port=$ADMIN_PORT" > "$HC_PORTS_FILE"
                echo "app_port=$CONDUCTOR_APP_PORT" >> "$HC_PORTS_FILE"
                echo ""
                echo "   ✅ Conductor ready (admin: $ADMIN_PORT, app: $CONDUCTOR_APP_PORT)"
                break
            fi
        fi
        printf "."
        sleep 1
    done
    sleep 2
fi

ADMIN_PORT=$(get_admin_port)
if [ -z "$ADMIN_PORT" ]; then
    echo "   ❌ Could not start conductor. Check $LOCAL_DEV_DIR/.sandbox_log"
    exit 1
fi

# Wait for connections
for i in {1..15}; do
    if hc sandbox call --running "$ADMIN_PORT" list-apps >/dev/null 2>&1; then
        break
    fi
    sleep 1
done

# If conductor only mode, stop here
if [ "$CONDUCTOR_ONLY" = true ]; then
    echo ""
    echo "════════════════════════════════════════════════════════════════"
    echo "   Conductor running on admin port $ADMIN_PORT"
    echo "   App interface on port $CONDUCTOR_APP_PORT"
    echo ""
    echo "   To start full stack: npm run hc:start"
    echo "════════════════════════════════════════════════════════════════"
    exit 0
fi

# ──────────────────────────────────────────────────────────────────────────────
# Step 3: Start elohim-storage
# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "┌──────────────────────────────────────────────────────────────┐"
echo "│ Step 2: elohim-storage (Content DB + Blobs)                   │"
echo "└──────────────────────────────────────────────────────────────┘"

# elohim-storage now lives at elohim/elohim-storage (peer-level), not under elohim/holochain.
# NOTE: this is the CRATE directory, deliberately NOT named STORAGE_DIR — the
# elohim-storage binary reads STORAGE_DIR from the environment as its DATA
# directory (src/main.rs), so a user-set STORAGE_DIR must pass through untouched.
STORAGE_CRATE_DIR="$HC_DIR/../elohim-storage"
STORAGE_BIN="$STORAGE_TARGET_DIR/release/elohim-storage"

# Build if needed
if [ ! -f "$STORAGE_BIN" ] || [ "$FORCE_BUILD" = true ]; then
    echo "   🔨 Building elohim-storage..."
    cd "$STORAGE_CRATE_DIR"
    CARGO_TARGET_DIR="$STORAGE_TARGET_DIR" \
      RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
    echo "   ✅ Build complete"
fi

# Check if already running
if curl -s "http://localhost:$STORAGE_PORT/health" >/dev/null 2>&1; then
    echo "   ✅ Storage already running on port $STORAGE_PORT"
else
    # Stop any existing process
    fuser -k "$STORAGE_PORT/tcp" 2>/dev/null || true
    sleep 1

    # Start storage with content database enabled
    export HOLOCHAIN_ADMIN_URL="ws://localhost:$ADMIN_PORT"
    export ENABLE_IMPORT_API=true
    export ENABLE_CONTENT_DB=true

    # NOTE: the binary ignores the HTTP_PORT env var — the port must be the
    # --http-port flag (verified 2026-08-16; the old export was a silent no-op
    # that only worked because 8090 is also the binary's default).
    #
    # Release-channel passthrough (T3 device-peer rung): CONDUCTOR_RELEASE_CHANNELS
    # is the operator-facing knob (matches the CONDUCTOR_* naming of the rest of
    # this join-alpha section); elohim-storage's runtime-config reads
    # ELOHIM_RELEASE_CHANNELS (services/release_adoption/state.rs). The rename
    # rides a command prefix, not `export`, so it lands ONLY in the storage
    # process's environment — never globally in this shell (the doorway launch
    # below must not inherit it).
    if [ -n "${CONDUCTOR_RELEASE_CHANNELS:-}" ]; then
        echo "   following: $CONDUCTOR_RELEASE_CHANNELS"
        ELOHIM_RELEASE_CHANNELS="$CONDUCTOR_RELEASE_CHANNELS" "$STORAGE_BIN" --http-port "$STORAGE_PORT" &
    else
        echo "   following: (none — set CONDUCTOR_RELEASE_CHANNELS=<channel>=observe to ride a release channel)"
        "$STORAGE_BIN" --http-port "$STORAGE_PORT" &
    fi

    echo -n "   ⏳ Waiting for storage"
    for i in {1..15}; do
        if curl -s "http://localhost:$STORAGE_PORT/health" >/dev/null 2>&1; then
            echo ""
            STATS=$(curl -s "http://localhost:$STORAGE_PORT/db/stats" 2>/dev/null || echo "{}")
            CONTENT_COUNT=$(echo "$STATS" | grep -o '"content_count":[0-9]*' | grep -o '[0-9]*' || echo "0")
            echo "   ✅ Storage ready (port: $STORAGE_PORT, content: $CONTENT_COUNT items)"
            break
        fi
        printf "."
        sleep 1
    done
fi

# ──────────────────────────────────────────────────────────────────────────────
# Step 2.5: Elohim Agent SDK (Inference Sidecar)
# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "┌──────────────────────────────────────────────────────────────┐"
echo "│ Step 2.5: Elohim Agent SDK (Inference Sidecar)                │"
echo "└──────────────────────────────────────────────────────────────┘"

AGENT_SDK_DIR="$APP_DIR/../../elohim/elohim-agent/elohim-agent-sdk"
AGENT_SDK_PORT="${ELOHIM_AGENT_PORT:-8095}"

if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "   ⚠️  ANTHROPIC_API_KEY not set — sidecar skipped (gate falls back to PassThrough)"
else
    # Check if already running
    if curl -s "http://localhost:$AGENT_SDK_PORT/health" >/dev/null 2>&1; then
        echo "   ✅ Agent SDK already running (port: $AGENT_SDK_PORT)"
    else
        # Build if needed
        if [ ! -d "$AGENT_SDK_DIR/dist" ] || [ "$FORCE_BUILD" = true ]; then
            echo "   🔨 Building agent SDK..."
            cd "$AGENT_SDK_DIR"
            pnpm build
            echo "   ✅ Build complete"
        fi

        # Start sidecar
        cd "$AGENT_SDK_DIR"
        ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
        ELOHIM_AGENT_PORT="$AGENT_SDK_PORT" \
        pnpm start &

        echo -n "   ⏳ Waiting for agent SDK"
        for i in {1..10}; do
            if curl -s "http://localhost:$AGENT_SDK_PORT/health" >/dev/null 2>&1; then
                echo ""
                echo "   ✅ Agent SDK ready (port: $AGENT_SDK_PORT)"
                break
            fi
            printf "."
            sleep 1
        done
    fi
fi

# ──────────────────────────────────────────────────────────────────────────────
# Step 4: Start Doorway
# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "┌──────────────────────────────────────────────────────────────┐"
echo "│ Step 3: Doorway Gateway                                       │"
echo "└──────────────────────────────────────────────────────────────┘"

# doorway-service is the Rust crate; `doorway/` is the umbrella with subprojects.
DOORWAY_DIR="$APP_DIR/../../doorway/doorway-service"
DOORWAY_BIN="$DOORWAY_TARGET_DIR/release/doorway"

# Build if needed
if [ ! -f "$DOORWAY_BIN" ] || [ "$FORCE_BUILD" = true ]; then
    echo "   🔨 Building doorway..."
    cd "$DOORWAY_DIR"
    CARGO_TARGET_DIR="$DOORWAY_TARGET_DIR" RUSTFLAGS="" cargo build --release
    echo "   ✅ Build complete"
fi

# Check status
PROXY_STATUS=$(curl -s http://localhost:8888/status 2>/dev/null || echo "")
STORAGE_CONFIGURED=$(echo "$PROXY_STATUS" | grep -o '"configured":true' || echo "")

if [ -n "$PROXY_STATUS" ] && [ -n "$STORAGE_CONFIGURED" ]; then
    echo "   ✅ Doorway already running with storage integration"
else
    # Stop existing doorway
    fuser -k 8888/tcp 2>/dev/null || true
    sleep 1

    # ------------------------------------------------------------------
    # Auth posture. Two honest shapes, chosen by what this box actually has
    # — never by a mode flag. See doorway `native_local_first_operator`.
    #
    #   secure  — this workspace has its OWN doorway identity: a persistent
    #             per-workspace JWT_SECRET, an account store, and the hApp
    #             bundle the chaperone provisions from. The browser logs in
    #             and reaches the conductor through POST /hc/connect. The
    #             conductor admin socket requires a credential, exactly as on
    #             the deployed fleet. This is the mode that makes the devspace
    #             a protocol-compliant peer rather than a hole.
    #
    #   keyless — native local-first: no account store, no doorway identity.
    #             The doorway grants the conductor-operator level to loopback
    #             callers (and ONLY loopback, and only pre-coordination), so
    #             the admin-WS flow still works with nothing configured.
    #
    # `--dev-mode` is passed ONLY in the keyless posture, and it no longer
    # decides anything about authorization. It survives as one honest thing:
    # a startup-time DECLARATION that this is a developer's box, which the
    # config validator requires before it will let a doorway run with no
    # signing secret at all (`!dev_mode && jwt_secret.is_none()` => refuse to
    # start). That is fail-closed and stays. What it must never again do is
    # decide a per-request grant — that is why the secure posture below does
    # not pass it, and why the fleet's `DEV_MODE: "true"` is now inert for
    # the conductor socket.
    # ------------------------------------------------------------------
    mkdir -p "$DOORWAY_STATE_DIR"
    DOORWAY_POSTURE=keyless
    if [ "$DOORWAY_AUTH" != "keyless" ]; then
        if [ -n "$MONGOD_BIN" ] && [ -x "$MONGOD_BIN" ]; then
            DOORWAY_POSTURE=secure
        elif [ "$DOORWAY_AUTH" = "secure" ]; then
            echo "   ❌ DOORWAY_AUTH=secure but no mongod found (set MONGOD_BIN)." >&2
            echo "      The secure posture needs an account store to log in against." >&2
            exit 1
        else
            echo "   ℹ️  No mongod found — starting keyless (native local-first)."
            echo "      Install mongod or set MONGOD_BIN for the secure posture."
        fi
    fi

    DOORWAY_ENV=()
    if [ "$DOORWAY_POSTURE" = "secure" ]; then
        # mongod must be listening BEFORE the doorway boots: the projection
        # archive is bound at startup and a doorway that boots archive-less
        # stays inert for its whole life.
        if ! (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null; then
            mkdir -p "$MONGO_DIR"
            "$MONGOD_BIN" --dbpath "$MONGO_DIR" --bind_ip 127.0.0.1 --port "$MONGO_PORT" \
                --fork --logpath "$LOCAL_DEV_DIR/mongod.log" >/dev/null 2>&1 \
                || { echo "   ❌ mongod failed to start (see $LOCAL_DEV_DIR/mongod.log)" >&2; exit 1; }
            for _ in $(seq 1 20); do
                (exec 3<>"/dev/tcp/127.0.0.1/$MONGO_PORT") 2>/dev/null && break; sleep 1
            done
            echo "   ✅ mongod up on :$MONGO_PORT"
        else
            echo "   ✅ mongod already up on :$MONGO_PORT"
        fi

        # Per-workspace secrets, generated once and persisted. NOT shared with
        # any other workspace: a doorway that signs with the publicly-known dev
        # placeholder (JwtValidator::new_dev) can have its tokens forged by
        # anyone, which would make the chaperone security theatre.
        if [ ! -s "$JWT_SECRET_FILE" ]; then
            head -c 48 /dev/urandom | base64 | tr -d '\n' > "$JWT_SECRET_FILE"
            chmod 600 "$JWT_SECRET_FILE"
            echo "   🔑 Generated this workspace's doorway JWT secret"
        fi
        if [ ! -s "$ADMIN_KEY_FILE" ]; then
            head -c 32 /dev/urandom | base64 | tr -d '\n' > "$ADMIN_KEY_FILE"
            chmod 600 "$ADMIN_KEY_FILE"
        fi

        DOORWAY_ENV+=(
            "MONGODB_URI=mongodb://127.0.0.1:$MONGO_PORT"
            "MONGODB_DB=doorway-dev"
            "JWT_SECRET=$(cat "$JWT_SECRET_FILE")"
            "API_KEY_ADMIN=$(cat "$ADMIN_KEY_FILE")"
            "HAPP_BUNDLE_PATH=$HAPP_PATH"
            "DOORWAY_ID=${DOORWAY_ID:-workspace-local}"
        )
    fi

    # Keyless declares itself with --dev-mode (see above); secure never does.
    DOORWAY_FLAGS=()
    [ "$DOORWAY_POSTURE" = "keyless" ] && DOORWAY_FLAGS+=(--dev-mode)

    # Start with storage URL
    # `hc sandbox` picks a RANDOM admin port and pins the app interface with
    # `-r=4445`, so the doorway's default "admin = app port - 1" derivation
    # cannot find it. Pass BOTH explicitly rather than let it guess: the app
    # interface as --conductor-url, the real admin socket as
    # --conductor-admin-url. Before the secure posture existed nothing here
    # exercised the admin path from the doorway side (the browser drove the
    # admin socket itself), so the mis-derivation was invisible.
    env "${DOORWAY_ENV[@]}" "$DOORWAY_BIN" "${DOORWAY_FLAGS[@]}" \
        --listen 0.0.0.0:8888 \
        --conductor-url "ws://localhost:$CONDUCTOR_APP_PORT" \
        --conductor-admin-url "ws://localhost:$ADMIN_PORT" \
        --storage-url "http://localhost:$STORAGE_PORT" &

    echo -n "   ⏳ Waiting for doorway"
    for i in {1..10}; do
        if curl -s http://localhost:8888/health >/dev/null 2>&1; then
            echo ""
            echo "   ✅ Doorway ready (port: 8888)"
            break
        fi
        printf "."
        sleep 1
    done
fi

# ──────────────────────────────────────────────────────────────────────────────
# Step 5: Optional seeding
# ──────────────────────────────────────────────────────────────────────────────
if [ "$RUN_SEED" = true ]; then
    echo ""
    echo "┌──────────────────────────────────────────────────────────────┐"
    echo "│ Step 4: Seeding content ($SEED_LIMIT items)                   │"
    echo "└──────────────────────────────────────────────────────────────┘"

    cd "$HC_DIR/../../genesis/seeder"
    DOORWAY_URL="http://localhost:8888" \
    STORAGE_URL="http://localhost:$STORAGE_PORT" \
    HOLOCHAIN_ADMIN_URL="ws://localhost:$ADMIN_PORT" \
    npx tsx src/seed.ts --limit "$SEED_LIMIT"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Final Status
# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════════════"
echo "🔷 Elohim P2P Framework Ready"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "┌─────────────┬────────────────────────────────────────────────┐"
echo "│ Component   │ Endpoint                                       │"
echo "├─────────────┼────────────────────────────────────────────────┤"
printf "│ Conductor   │ ws://localhost:%-5s (admin)                   │\n" "$ADMIN_PORT"
echo "│             │ ws://localhost:$CONDUCTOR_APP_PORT  (app)                     │"
printf "│ Storage     │ http://localhost:%-4s (content DB + blobs)    │\n" "$STORAGE_PORT"
if [ -n "$ANTHROPIC_API_KEY" ]; then
printf "│ Agent SDK   │ http://localhost:%-4s (inference sidecar)     │\n" "${ELOHIM_AGENT_PORT:-8095}"
else
echo "│ Agent SDK   │ (skipped — no ANTHROPIC_API_KEY)              │"
fi
echo "│ Doorway     │ http://localhost:8888 (unified API)           │"
echo "└─────────────┴────────────────────────────────────────────────┘"
echo ""
case "${DOORWAY_POSTURE:-unknown}" in
  secure)
    echo "🔐 Doorway auth: SECURE (this workspace has its own doorway identity)"
    echo "   • Register/log in in the app — the doorway is its own identity provider."
    echo "   • The browser then reaches the conductor via POST /hc/connect (chaperone);"
    echo "     the conductor admin socket requires a credential, as on the fleet."
    echo "   • Secrets: $DOORWAY_STATE_DIR (per-workspace, generated once)"
    ;;
  keyless)
    echo "🔓 Doorway auth: KEYLESS (native local-first — no account store)"
    echo "   • Loopback callers are this conductor's operator; off-box callers get nothing."
    echo "   • Install mongod (or set MONGOD_BIN) for the secure posture."
    ;;
  *)
    echo "ℹ️  Doorway auth: unchanged (doorway was already running)"
    ;;
esac
echo ""
echo "📋 Quick Commands:"
echo ""
echo "   # Health & status"
echo "   curl http://localhost:8888/status"
echo "   curl http://localhost:8888/db/stats"
echo ""
echo "   # Content API"
echo "   curl http://localhost:8888/db/content?limit=10"
echo "   curl http://localhost:8888/db/paths"
echo ""
echo "   # Seed content"
echo "   npm run hc:seed"
echo ""
echo "   # Stop everything"
echo "   npm run hc:stop"
echo ""
echo "✅ Ready for development!"
