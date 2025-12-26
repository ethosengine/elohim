#!/bin/bash
# Holochain Development Stack Startup Script (Doorway version)
# Starts sandbox, Doorway gateway, installs hApp

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
HC_DIR="$APP_DIR/../holochain"
LOCAL_DEV_DIR="$HC_DIR/local-dev"
HAPP_PATH="$HC_DIR/dna/elohim/workdir/elohim.happ"
HC_PORTS_FILE="$LOCAL_DEV_DIR/.hc_ports"
DOORWAY_BIN="$HC_DIR/doorway/target/release/doorway"

echo "🚪 Starting Holochain Development Stack (Doorway)..."

# Check if Doorway binary exists
if [ ! -f "$DOORWAY_BIN" ]; then
    echo "⚠️  Doorway binary not found at $DOORWAY_BIN"
    echo "   Building Doorway first..."
    cd "$HC_DIR/doorway"
    RUSTFLAGS="" cargo build --release
    echo "✅ Doorway built"
fi

# Check if hApp file exists
if [ ! -f "$HAPP_PATH" ]; then
    echo "⚠️  hApp not found at $HAPP_PATH"
    echo "   Building hApp first..."
    cd "$HC_DIR/dna/elohim"
    RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown
    hc dna pack . -o workdir/lamad.dna
    hc app pack workdir -o workdir/elohim.happ
    echo "✅ hApp built"
fi

# Function to get admin port from running sandbox or ports file
get_admin_port() {
    if [ -f "$HC_PORTS_FILE" ]; then
        cat "$HC_PORTS_FILE" | grep "admin_port" | grep -o "[0-9]*" | head -1
    fi
}

# Check if conductor is already running
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

    # Start sandbox with PTY for passphrase
    SANDBOX_LOG="$LOCAL_DEV_DIR/.sandbox_log"
    HC_WRAPPER="$LOCAL_DEV_DIR/.hc_wrapper.sh"

    cat > "$HC_WRAPPER" << EOF
#!/bin/bash
exec hc sandbox generate --app-id elohim --in-process-lair -r=4445 "$HAPP_PATH"
EOF
    chmod +x "$HC_WRAPPER"

    rm -f "$SANDBOX_LOG"
    nohup sh -c '(echo "test"; sleep infinity) | socat - EXEC:'"$HC_WRAPPER"',pty,setsid,ctty' > "$SANDBOX_LOG" 2>&1 &
    SANDBOX_PID=$!

    # Wait for conductor to be ready
    echo "⏳ Waiting for conductor to start..."
    for i in {1..45}; do
        if grep -qa '"admin_port"' "$SANDBOX_LOG" 2>/dev/null; then
            ADMIN_PORT=$(grep -ao '"admin_port":[0-9]*' "$SANDBOX_LOG" | grep -o '[0-9]*' | head -1)
            if [ -n "$ADMIN_PORT" ]; then
                echo "admin_port=$ADMIN_PORT" > "$HC_PORTS_FILE"
                echo "app_port=4445" >> "$HC_PORTS_FILE"
                echo "✅ Conductor ready on admin port $ADMIN_PORT"
                break
            fi
        fi
        printf "."
        sleep 1
    done
    echo ""
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

# Wait for conductor to accept connections
echo "⏳ Waiting for conductor to accept connections..."
for i in {1..15}; do
    if hc sandbox call --running "$ADMIN_PORT" list-apps >/dev/null 2>&1; then
        echo "✅ Conductor accepting connections"
        break
    fi
    sleep 1
done

# Stop any existing proxy on 8888
fuser -k 8888/tcp 2>/dev/null || true
sleep 1

# Start Doorway
echo "🚪 Starting Doorway gateway..."
"$DOORWAY_BIN" --dev-mode --listen 0.0.0.0:8888 --conductor-url "ws://localhost:$ADMIN_PORT" &
DOORWAY_PID=$!

# Wait for Doorway to be ready
echo "⏳ Waiting for Doorway to start..."
for i in {1..15}; do
    if curl -s http://localhost:8888/health >/dev/null 2>&1; then
        echo "✅ Doorway ready on port 8888 → conductor:$ADMIN_PORT"
        break
    fi
    sleep 1
done

# Show status
echo ""
echo "🚪 Holochain Stack Status (Doorway):"
echo "   Admin port: $ADMIN_PORT"
echo "   App port: 4445"
echo "   Doorway: http://localhost:8888"
curl -s http://localhost:8888/status 2>/dev/null | head -1 || echo "   Doorway: not responding"
echo ""
hc sandbox call --running "$ADMIN_PORT" list-apps 2>/dev/null | head -5

echo ""
echo "✅ Holochain stack ready with Doorway!"
