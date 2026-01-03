#!/bin/bash
#
# hc-start-full.sh - Start complete Holochain development stack
#
# USAGE:
#   ./hc-start-full.sh [OPTIONS]
#
# OPTIONS:
#   -h, --help       Show this help message
#   -s, --seed       Run sample seed after startup
#   -n, --no-storage Skip elohim-storage (conductor + doorway only)
#   -b, --build      Force rebuild all components
#
# ENVIRONMENT VARIABLES:
#   STORAGE_PORT     Storage HTTP port (default: 8090)
#   STORAGE_DIR      Storage data directory (default: /tmp/elohim-storage)
#   SEED_LIMIT       Number of items to seed with --seed (default: 20)
#
# COMPONENTS:
#   1. Holochain Conductor (admin: dynamic, app: 4445)
#   2. elohim-storage service (port: 8090) - blob storage + import API
#   3. Doorway gateway (port: 8888) - proxy to conductor + storage
#
# EXAMPLES:
#   ./hc-start-full.sh                    # Start full stack
#   ./hc-start-full.sh --seed             # Start + seed sample content
#   ./hc-start-full.sh --no-storage       # Without storage (for basic testing)
#   SEED_LIMIT=100 ./hc-start-full.sh -s  # Seed 100 items
#
# AFTER STARTUP:
#   Test import: cd genesis/seeder && npm run seed -- --limit 20
#   Health check: curl http://localhost:8888/status
#   Stop all: npm run hc:stop
#

set -e

# ============================================================================
# Configuration
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
HC_DIR="$APP_DIR/../holochain"
LOCAL_DEV_DIR="$HC_DIR/local-dev"
HC_PORTS_FILE="$LOCAL_DEV_DIR/.hc_ports"

# Environment with defaults
: "${STORAGE_PORT:=8090}"
: "${SEED_LIMIT:=20}"

# Options
RUN_SEED=false
SKIP_STORAGE=false
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
        -n|--no-storage)
            SKIP_STORAGE=true
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
# Main
# ============================================================================

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "🔷 Holochain Full Stack Startup"
echo "════════════════════════════════════════════════════════════════"
echo ""
if [ "$SKIP_STORAGE" = true ]; then
    echo "   Mode: Conductor + Doorway (no storage)"
else
    echo "   Mode: Conductor + Doorway + Storage"
fi
echo ""

# ──────────────────────────────────────────────────────────────────────────────
# Step 1: Conductor + Basic Doorway
# ──────────────────────────────────────────────────────────────────────────────
echo "┌──────────────────────────────────────────────────────────────┐"
echo "│ Step 1: Starting Conductor                                    │"
echo "└──────────────────────────────────────────────────────────────┘"

"$SCRIPT_DIR/hc-start.sh"

# Read admin port
if [ -f "$HC_PORTS_FILE" ]; then
    ADMIN_PORT=$(grep "admin_port" "$HC_PORTS_FILE" | grep -o "[0-9]*" | head -1)
else
    echo "❌ Conductor failed to start"
    exit 1
fi

echo ""
echo "   ✅ Conductor ready on admin port $ADMIN_PORT"

# ──────────────────────────────────────────────────────────────────────────────
# Step 2: elohim-storage (optional)
# ──────────────────────────────────────────────────────────────────────────────
if [ "$SKIP_STORAGE" = false ]; then
    echo ""
    echo "┌──────────────────────────────────────────────────────────────┐"
    echo "│ Step 2: Starting elohim-storage                              │"
    echo "└──────────────────────────────────────────────────────────────┘"

    BUILD_FLAG=""
    if [ "$FORCE_BUILD" = true ]; then
        BUILD_FLAG="--build"
    fi

    ADMIN_PORT="$ADMIN_PORT" STORAGE_PORT="$STORAGE_PORT" "$SCRIPT_DIR/storage-start.sh" $BUILD_FLAG

    echo ""
    echo "   ✅ Storage ready on port $STORAGE_PORT"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Step 3: Restart Doorway with storage URL
# ──────────────────────────────────────────────────────────────────────────────
if [ "$SKIP_STORAGE" = false ]; then
    echo ""
    echo "┌──────────────────────────────────────────────────────────────┐"
    echo "│ Step 3: Reconfiguring Doorway with storage                   │"
    echo "└──────────────────────────────────────────────────────────────┘"

    DOORWAY_BIN="$HC_DIR/doorway/target/release/doorway"

    # Stop existing doorway
    fuser -k 8888/tcp 2>/dev/null || true
    sleep 1

    # Start doorway with storage URL
    echo "   Connecting doorway to storage service..."
    ELOHIM_STORAGE_URL="http://localhost:$STORAGE_PORT" \
    "$DOORWAY_BIN" \
        --dev-mode \
        --listen 0.0.0.0:8888 \
        --conductor-url "ws://localhost:$ADMIN_PORT" \
        --storage-url "http://localhost:$STORAGE_PORT" &

    # Wait for doorway
    for i in {1..10}; do
        if curl -s http://localhost:8888/health >/dev/null 2>&1; then
            echo "   ✅ Doorway ready with storage integration"
            break
        fi
        sleep 1
    done
fi

# ──────────────────────────────────────────────────────────────────────────────
# Step 4: Optional seeding
# ──────────────────────────────────────────────────────────────────────────────
if [ "$RUN_SEED" = true ]; then
    echo ""
    echo "┌──────────────────────────────────────────────────────────────┐"
    echo "│ Step 4: Running sample seed ($SEED_LIMIT items)               │"
    echo "└──────────────────────────────────────────────────────────────┘"

    cd "$HC_DIR/../genesis/seeder"
    DOORWAY_URL="http://localhost:8888" \
    HOLOCHAIN_ADMIN_URL="ws://localhost:$ADMIN_PORT" \
    npx tsx src/seed.ts --dir ../data/lamad/content --type content --limit "$SEED_LIMIT"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Final Status
# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════════════"
echo "🔷 Stack Status"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "┌─────────────┬───────────────────────────────────────────────┐"
echo "│ Service     │ Endpoint                                      │"
echo "├─────────────┼───────────────────────────────────────────────┤"
echo "│ Conductor   │ ws://localhost:$ADMIN_PORT (admin)             │"
echo "│             │ ws://localhost:4445 (app)                      │"
echo "│ Doorway     │ http://localhost:8888                         │"
if [ "$SKIP_STORAGE" = false ]; then
echo "│ Storage     │ http://localhost:$STORAGE_PORT                 │"
fi
echo "└─────────────┴───────────────────────────────────────────────┘"
echo ""
echo "📋 Quick Commands:"
echo ""
echo "   # Health checks"
echo "   curl http://localhost:8888/status"
if [ "$SKIP_STORAGE" = false ]; then
echo "   curl http://localhost:$STORAGE_PORT/health"
fi
echo ""
echo "   # Run seeder"
echo "   cd genesis/seeder && DOORWAY_URL=http://localhost:8888 \\"
echo "     HOLOCHAIN_ADMIN_URL=ws://localhost:$ADMIN_PORT npx tsx src/seed.ts --limit 20"
echo ""
echo "   # Stop everything"
echo "   npm run hc:stop"
echo ""
echo "✅ Full stack ready!"
