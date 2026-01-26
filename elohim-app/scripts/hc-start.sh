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
#   STORAGE_DIR      Storage data directory (default: /tmp/elohim-storage)
#   SEED_LIMIT       Number of items to seed with --seed (default: 20)
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
HC_DIR="$APP_DIR/../holochain"
LOCAL_DEV_DIR="$HC_DIR/local-dev"
HAPP_PATH="$HC_DIR/dna/elohim/workdir/elohim.happ"
HC_PORTS_FILE="$LOCAL_DEV_DIR/.hc_ports"

# Environment with defaults
: "${STORAGE_PORT:=8090}"
: "${SEED_LIMIT:=20}"

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
# Functions
# ============================================================================

get_admin_port() {
    if [ -f "$HC_PORTS_FILE" ]; then
        grep "admin_port" "$HC_PORTS_FILE" | grep -o "[0-9]*" | head -1
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
if [ ! -f "$HAPP_PATH" ] || [ "$FORCE_BUILD" = true ]; then
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

    echo "📦 Packing elohim.happ..."
    hc app pack "$WORKDIR" -o "$WORKDIR/elohim.happ"

    echo "✅ DNAs built (lamad + imagodei + infrastructure)"
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
fi

if [ "$CONDUCTOR_RUNNING" = false ]; then
    echo "   🚀 Starting Holochain sandbox..."
    mkdir -p "$LOCAL_DEV_DIR"
    cd "$LOCAL_DEV_DIR"

    rm -f "$HC_PORTS_FILE"

    SANDBOX_LOG="$LOCAL_DEV_DIR/.sandbox_log"
    HC_WRAPPER="$LOCAL_DEV_DIR/.hc_wrapper.sh"

    cat > "$HC_WRAPPER" << EOF
#!/bin/bash
exec hc sandbox generate --app-id elohim --in-process-lair -r=4445 "$HAPP_PATH"
EOF
    chmod +x "$HC_WRAPPER"

    rm -f "$SANDBOX_LOG"
    nohup sh -c '(echo "test"; sleep infinity) | socat - EXEC:'"$HC_WRAPPER"',pty,setsid,ctty' > "$SANDBOX_LOG" 2>&1 &

    echo -n "   ⏳ Waiting for conductor"
    for i in {1..45}; do
        if grep -qa '"admin_port"' "$SANDBOX_LOG" 2>/dev/null; then
            ADMIN_PORT=$(grep -ao '"admin_port":[0-9]*' "$SANDBOX_LOG" | grep -o '[0-9]*' | head -1)
            if [ -n "$ADMIN_PORT" ]; then
                echo "admin_port=$ADMIN_PORT" > "$HC_PORTS_FILE"
                echo "app_port=4445" >> "$HC_PORTS_FILE"
                echo ""
                echo "   ✅ Conductor ready (admin: $ADMIN_PORT, app: 4445)"
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
    echo "   App interface on port 4445"
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

STORAGE_BIN="$HC_DIR/target/release/elohim-storage"

# Build if needed
if [ ! -f "$STORAGE_BIN" ] || [ "$FORCE_BUILD" = true ]; then
    echo "   🔨 Building elohim-storage..."
    cd "$HC_DIR/elohim-storage"
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
    export HTTP_PORT="$STORAGE_PORT"
    export ENABLE_IMPORT_API=true
    export ENABLE_CONTENT_DB=true

    "$STORAGE_BIN" &

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
# Step 4: Start Doorway
# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "┌──────────────────────────────────────────────────────────────┐"
echo "│ Step 3: Doorway Gateway                                       │"
echo "└──────────────────────────────────────────────────────────────┘"

DOORWAY_BIN="$APP_DIR/../doorway/target/release/doorway"

# Build if needed
if [ ! -f "$DOORWAY_BIN" ] || [ "$FORCE_BUILD" = true ]; then
    echo "   🔨 Building doorway..."
    cd "$APP_DIR/../doorway"
    RUSTFLAGS="" cargo build --release
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

    # Start with storage URL
    "$DOORWAY_BIN" \
        --dev-mode \
        --listen 0.0.0.0:8888 \
        --conductor-url "ws://localhost:$ADMIN_PORT" \
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

    cd "$HC_DIR/../genesis/seeder"
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
echo "│             │ ws://localhost:4445  (app)                     │"
printf "│ Storage     │ http://localhost:%-4s (content DB + blobs)    │\n" "$STORAGE_PORT"
echo "│ Doorway     │ http://localhost:8888 (unified API)           │"
echo "└─────────────┴────────────────────────────────────────────────┘"
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
