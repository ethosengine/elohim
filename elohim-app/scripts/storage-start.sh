#!/bin/bash
#
# storage-start.sh - Start elohim-storage blob storage service
#
# USAGE:
#   ./storage-start.sh [OPTIONS]
#
# OPTIONS:
#   -h, --help      Show this help message
#   -b, --build     Force rebuild before starting
#   -f, --foreground  Run in foreground (not backgrounded)
#
# ENVIRONMENT VARIABLES:
#   STORAGE_PORT    HTTP port (default: 8090)
#   STORAGE_DIR     Data directory (default: /tmp/elohim-storage)
#   ADMIN_PORT      Override conductor admin port (auto-detected from .hc_ports)
#
# EXAMPLES:
#   ./storage-start.sh                          # Start with defaults
#   STORAGE_PORT=9000 ./storage-start.sh        # Custom port
#   ./storage-start.sh --build                  # Rebuild before starting
#   ./storage-start.sh -f                       # Run in foreground (for debugging)
#
# REQUIRES:
#   - Holochain conductor running (npm run hc:start)
#   - Rust toolchain for building
#
# PROVIDES:
#   - Blob storage API on http://localhost:${STORAGE_PORT}/
#   - Import API at /import/queue, /import/status/{id}
#   - WebSocket progress at /import/progress
#   - Health check at /health
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
STORAGE_BIN="$HC_DIR/target/release/elohim-storage"

# Environment with defaults
: "${STORAGE_PORT:=8090}"
: "${STORAGE_DIR:=/tmp/elohim-storage}"

# Options
FORCE_BUILD=false
FOREGROUND=false

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
        -b|--build)
            FORCE_BUILD=true
            shift
            ;;
        -f|--foreground)
            FOREGROUND=true
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

echo "📦 elohim-storage startup"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Detect admin port (can be overridden with ADMIN_PORT env var)
if [ -n "$ADMIN_PORT" ]; then
    echo "   Using ADMIN_PORT from environment: $ADMIN_PORT"
elif [ -f "$HC_PORTS_FILE" ]; then
    ADMIN_PORT=$(grep "admin_port" "$HC_PORTS_FILE" | grep -o "[0-9]*" | head -1)
    echo "   Detected admin port from .hc_ports: $ADMIN_PORT"
else
    echo "❌ No conductor found!"
    echo ""
    echo "   Start conductor first:  npm run hc:start"
    echo "   Or set ADMIN_PORT:      ADMIN_PORT=12345 $0"
    exit 1
fi

echo "   Storage port: $STORAGE_PORT"
echo "   Storage dir:  $STORAGE_DIR"
echo ""

# Build if needed or forced
if [ "$FORCE_BUILD" = true ] || [ ! -f "$STORAGE_BIN" ]; then
    echo "🔨 Building elohim-storage..."
    cd "$HC_DIR/elohim-storage"
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
    echo "✅ Build complete"
    echo ""
fi

# Stop any existing process
if fuser "$STORAGE_PORT/tcp" 2>/dev/null; then
    echo "⚠️  Port $STORAGE_PORT in use, stopping..."
    fuser -k "$STORAGE_PORT/tcp" 2>/dev/null || true
    sleep 1
fi

# Set up environment
export HOLOCHAIN_ADMIN_URL="ws://localhost:$ADMIN_PORT"
export STORAGE_DIR="$STORAGE_DIR"
export HTTP_PORT="$STORAGE_PORT"
export ENABLE_IMPORT_API=true

echo "🚀 Starting elohim-storage..."
echo "   HOLOCHAIN_ADMIN_URL=$HOLOCHAIN_ADMIN_URL"
echo "   STORAGE_DIR=$STORAGE_DIR"
echo "   HTTP_PORT=$HTTP_PORT"
echo "   ENABLE_IMPORT_API=$ENABLE_IMPORT_API"
echo ""

if [ "$FOREGROUND" = true ]; then
    # Run in foreground
    exec "$STORAGE_BIN"
else
    # Run in background
    "$STORAGE_BIN" &
    STORAGE_PID=$!

    # Wait for ready
    echo "⏳ Waiting for storage service..."
    for i in {1..15}; do
        if curl -s "http://localhost:$STORAGE_PORT/health" >/dev/null 2>&1; then
            echo ""
            echo "✅ elohim-storage ready!"
            echo ""
            echo "   Health:   curl http://localhost:$STORAGE_PORT/health"
            echo "   Import:   POST http://localhost:$STORAGE_PORT/import/queue"
            echo "   Progress: ws://localhost:$STORAGE_PORT/import/progress"
            echo ""
            echo "   PID: $STORAGE_PID"
            echo "   Stop: fuser -k $STORAGE_PORT/tcp"
            exit 0
        fi
        printf "."
        sleep 1
    done

    echo ""
    echo "❌ Storage failed to start"
    exit 1
fi
