#!/bin/bash
# Holochain Development Stack Startup Script
# Starts sandbox, Doorway gateway, installs hApp

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
HC_DIR="$APP_DIR/../holochain"
LOCAL_DEV_DIR="$HC_DIR/local-dev"
HAPP_PATH="$HC_DIR/dna/lamad-spike/workdir/lamad-spike.happ"
HC_PORTS_FILE="$LOCAL_DEV_DIR/.hc_ports"

echo "🔷 Starting Holochain Development Stack..."

# Check if hApp file exists
if [ ! -f "$HAPP_PATH" ]; then
    echo "⚠️  hApp not found at $HAPP_PATH"
    echo "   Building hApp first..."
    cd "$HC_DIR/dna/lamad-spike"
    cargo build --release --target wasm32-unknown-unknown
    hc dna pack . -o workdir/lamad_spike.dna
    hc app pack workdir -o workdir/lamad-spike.happ
    echo "✅ hApp built"
fi

# Function to get admin port from running sandbox or ports file
get_admin_port() {
    # Try to read from saved ports file
    if [ -f "$HC_PORTS_FILE" ]; then
        cat "$HC_PORTS_FILE" | grep "admin_port" | grep -o "[0-9]*" | head -1
    fi
}

# Check if conductor is already running (check saved port or common ports)
ADMIN_PORT=$(get_admin_port)
CONDUCTOR_RUNNING=false

if [ -n "$ADMIN_PORT" ] && hc sandbox call --running "$ADMIN_PORT" list-apps >/dev/null 2>&1; then
    echo "✅ Holochain conductor already running on port $ADMIN_PORT"
    CONDUCTOR_RUNNING=true
fi

if [ "$CONDUCTOR_RUNNING" = false ]; then
    echo "🚀 Starting Holochain sandbox..."
    mkdir -p "$LOCAL_DEV_DIR"
    cd "$LOCAL_DEV_DIR"

    # Remove old ports file
    rm -f "$HC_PORTS_FILE"

    # Start sandbox and capture output to get the actual port
    SANDBOX_LOG="$LOCAL_DEV_DIR/.sandbox_log"
    HC_WRAPPER="$LOCAL_DEV_DIR/.hc_wrapper.sh"

    # Create wrapper script for socat (handles passphrase prompt via PTY)
    # -r=4445 means: run conductor with app interface on port 4445
    # --in-process-lair uses embedded lair keystore
    cat > "$HC_WRAPPER" << EOF
#!/bin/bash
exec hc sandbox generate --app-id elohim --in-process-lair -r=4445 "$HAPP_PATH"
EOF
    chmod +x "$HC_WRAPPER"

    # Use socat with PTY to handle the interactive lair passphrase prompt
    # (echo "test"; sleep infinity) provides passphrase then keeps stdin open
    rm -f "$SANDBOX_LOG"
    nohup sh -c '(echo "test"; sleep infinity) | socat - EXEC:'"$HC_WRAPPER"',pty,setsid,ctty' > "$SANDBOX_LOG" 2>&1 &
    SANDBOX_PID=$!

    # Wait for conductor to be ready and extract admin port
    echo "⏳ Waiting for conductor to start..."
    for i in {1..45}; do
        # Look for the admin_port in the log (JSON format from "Conductor launched")
        # Use grep -a to handle binary data (null bytes from PTY)
        if grep -qa '"admin_port"' "$SANDBOX_LOG" 2>/dev/null; then
            ADMIN_PORT=$(grep -ao '"admin_port":[0-9]*' "$SANDBOX_LOG" | grep -o '[0-9]*' | head -1)
            if [ -n "$ADMIN_PORT" ]; then
                echo "admin_port=$ADMIN_PORT" > "$HC_PORTS_FILE"
                echo "app_port=4445" >> "$HC_PORTS_FILE"
                echo "✅ Conductor ready on admin port $ADMIN_PORT"
                break
            fi
        fi
        # Show progress dots
        printf "."
        sleep 1
    done
    echo ""

    # Give it a moment to fully initialize
    sleep 2
fi

# Re-read admin port
ADMIN_PORT=$(get_admin_port)
if [ -z "$ADMIN_PORT" ]; then
    echo "❌ Could not determine admin port. Check $LOCAL_DEV_DIR/.sandbox_log"
    exit 1
fi

export HOLOCHAIN_ADMIN_PORT="$ADMIN_PORT"
echo "📍 Admin port: $ADMIN_PORT"

# Wait for conductor to actually be accepting connections
echo "⏳ Waiting for conductor to accept connections..."
for i in {1..15}; do
    if hc sandbox call --running "$ADMIN_PORT" list-apps >/dev/null 2>&1; then
        echo "✅ Conductor accepting connections"
        break
    fi
    sleep 1
done

# Check if Doorway is already running with correct admin port
DOORWAY_BIN="$HC_DIR/doorway/target/release/doorway"
PROXY_STATUS=$(curl -s http://localhost:8888/status 2>/dev/null || echo "")
DOORWAY_CONDUCTOR_PORT=$(echo "$PROXY_STATUS" | grep -o '"conductorUrl":"ws://localhost:[0-9]*"' | grep -o '[0-9]*' || echo "0")

if [ -n "$PROXY_STATUS" ] && [ "$DOORWAY_CONDUCTOR_PORT" = "$ADMIN_PORT" ]; then
    echo "✅ Doorway already running with correct admin port $ADMIN_PORT"
else
    if [ -n "$PROXY_STATUS" ]; then
        echo "⚠️  Doorway running with wrong conductor port ($DOORWAY_CONDUCTOR_PORT vs $ADMIN_PORT), restarting..."
    fi

    # Check if Doorway binary exists
    if [ ! -f "$DOORWAY_BIN" ]; then
        echo "⚠️  Doorway binary not found, building..."
        cd "$HC_DIR/doorway"
        RUSTFLAGS="" cargo build --release
        echo "✅ Doorway built"
    fi

    # Stop any existing process on 8888
    fuser -k 8888/tcp 2>/dev/null || true
    sleep 1

    echo "🚪 Starting Doorway gateway with admin port $ADMIN_PORT..."
    "$DOORWAY_BIN" --dev-mode --listen 0.0.0.0:8888 --conductor-url "ws://localhost:$ADMIN_PORT" &

    # Wait for Doorway to be ready
    echo "⏳ Waiting for Doorway to start..."
    for i in {1..15}; do
        if curl -s http://localhost:8888/health >/dev/null 2>&1; then
            echo "✅ Doorway ready on port 8888 → conductor:$ADMIN_PORT"
            break
        fi
        sleep 1
    done
fi

# Check if hApp is installed
APPS=$(hc sandbox call --running "$ADMIN_PORT" list-apps 2>/dev/null || echo "")
if echo "$APPS" | grep -q "elohim"; then
    echo "✅ elohim hApp installed"
else
    echo "⚠️  elohim hApp not found - sandbox may have started without hApp"
    echo "   Ensure $HAPP_PATH exists and restart with: npm run hc:stop && npm run start:hc"
fi

# Show status
echo ""
echo "🔷 Holochain Stack Status:"
echo "   Admin port: $ADMIN_PORT"
echo "   App port: 4445"
curl -s http://localhost:8888/status 2>/dev/null | head -1 || echo "   Doorway: not responding"
echo ""
hc sandbox call --running "$ADMIN_PORT" list-apps 2>/dev/null | head -5

echo ""
echo "✅ Holochain stack ready! Starting Angular..."
