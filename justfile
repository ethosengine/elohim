# Elohim Dev Workflows
#
# Prerequisites: nix develop ./steward --accept-flake-config
# Usage:         just --list
#
# All recipes assume you are inside the steward nix shell.
# Paths are relative to this justfile (project root).

set dotenv-load := false

# Project root (where this justfile lives)
root := justfile_directory()

# Key directories
hc_dir      := root / "holochain"
app_dir     := root / "elohim-app"
steward_dir := root / "steward"
doorway_dir := root / "doorway"
node_dir    := root / "elohim-node"
genesis_dir := root / "genesis"
sophia_dir  := root / "sophia"

# Derived paths
local_dev   := hc_dir / "local-dev"
ports_file  := local_dev / ".hc_ports"
storage_bin := hc_dir / "target/release/elohim-storage"
doorway_bin := doorway_dir / "target/release/doorway"
happ_path   := hc_dir / "dna/elohim/workdir/elohim.happ"

# Default ports
storage_port := env("STORAGE_PORT", "8090")
doorway_port := "8888"

# ─────────────────────────────────────────────────────────────────────
# Status / Health
# ─────────────────────────────────────────────────────────────────────

# Show health of all services
status:
    @echo "=== Storage (port {{storage_port}}) ==="
    @curl -sf http://localhost:{{storage_port}}/health 2>/dev/null \
        && echo "  UP" && curl -s http://localhost:{{storage_port}}/db/stats 2>/dev/null | head -5 \
        || echo "  DOWN"
    @echo ""
    @echo "=== Doorway (port {{doorway_port}}) ==="
    @curl -sf http://localhost:{{doorway_port}}/health 2>/dev/null \
        && echo "  UP" && curl -s http://localhost:{{doorway_port}}/status 2>/dev/null | head -10 \
        || echo "  DOWN"
    @echo ""
    @echo "=== Session ==="
    @curl -sf http://localhost:{{storage_port}}/session 2>/dev/null \
        || echo "  No active session"
    @echo ""
    @echo "=== Conductor ==="
    @if [ -f "{{ports_file}}" ]; then \
        ADMIN_PORT=$(grep admin_port "{{ports_file}}" | grep -o '[0-9]*' | head -1); \
        if hc sandbox call --running "$ADMIN_PORT" list-apps >/dev/null 2>&1; then \
            echo "  UP (admin: $ADMIN_PORT, app: 4445)"; \
        else \
            echo "  DOWN (stale ports file)"; \
        fi; \
    else \
        echo "  DOWN"; \
    fi

# ─────────────────────────────────────────────────────────────────────
# Steward Desktop
# ─────────────────────────────────────────────────────────────────────

# Build + run steward Tauri app in dev mode
steward-dev:
    cd {{steward_dir}} && npm run tauri:dev

# Production build of steward
steward-build:
    cd {{steward_dir}} && npm run tauri:build

# Production bundle: build storage binary, then steward (which bundles it)
# TAURI_CONFIG injects bundle.resources so elohim-storage is included in the package
steward-bundle:
    just storage-build
    TAURI_CONFIG='{"bundle":{"resources":{"../holochain/target/release/elohim-storage":"bin/"}}}' just steward-build

# ─────────────────────────────────────────────────────────────────────
# Storage
# ─────────────────────────────────────────────────────────────────────

# Build elohim-storage binary (delegates to per-project justfile)
storage-build:
    just --justfile {{hc_dir}}/elohim-storage/justfile --working-directory {{hc_dir}}/elohim-storage build

# Start elohim-storage (wraps existing script)
storage-start:
    cd {{app_dir}} && ./scripts/storage-start.sh

# Stop elohim-storage
storage-stop:
    @fuser -k {{storage_port}}/tcp 2>/dev/null && echo "Stopped elohim-storage" || echo "elohim-storage not running"

# ─────────────────────────────────────────────────────────────────────
# Session Management
# ─────────────────────────────────────────────────────────────────────

# Show current active session
session:
    @curl -sf http://localhost:{{storage_port}}/session 2>/dev/null \
        && echo "" \
        || echo "No active session (storage may not be running)"

# Create a dev session for local testing
session-seed agent_pub_key="uhCAk_dev_agent_placeholder":
    #!/usr/bin/env bash
    echo "Creating dev session..."
    curl -sf -X POST http://localhost:{{storage_port}}/session \
        -H "Content-Type: application/json" \
        -d '{
            "humanId": "dev-human-local",
            "agentPubKey": "{{agent_pub_key}}",
            "doorwayUrl": "http://localhost:{{doorway_port}}",
            "identifier": "dev@local",
            "displayName": "Local Developer"
        }' | head -40
    echo ""

# Delete active session
session-delete:
    @curl -sf -X DELETE http://localhost:{{storage_port}}/session 2>/dev/null \
        && echo "" \
        || echo "No session to delete (or storage not running)"

# List all sessions (including inactive)
session-list:
    @curl -sf http://localhost:{{storage_port}}/session/all 2>/dev/null \
        && echo "" \
        || echo "Storage not running"

# ─────────────────────────────────────────────────────────────────────
# Doorway Stack (wraps existing npm scripts)
# ─────────────────────────────────────────────────────────────────────

# Start full stack (conductor + storage + doorway)
stack-start:
    cd {{app_dir}} && ./scripts/hc-start.sh

# Start full stack + seed content
stack-start-seed:
    cd {{app_dir}} && ./scripts/hc-start.sh --seed

# Stop all services
stack-stop:
    #!/usr/bin/env bash
    echo "Stopping all Elohim services..."
    pkill -f 'holochain.*conductor' 2>/dev/null || true
    fuser -k {{doorway_port}}/tcp 2>/dev/null || true
    fuser -k {{storage_port}}/tcp 2>/dev/null || true
    rm -f {{local_dev}}/.hc_live_*
    echo "All services stopped"

# Stop + clear data + restart
stack-reset:
    #!/usr/bin/env bash
    echo "Resetting Elohim stack..."
    just stack-stop
    rm -rf {{local_dev}}/.hc* {{local_dev}}/.sandbox* {{local_dev}}/conductor-data /tmp/elohim-storage
    echo "Data cleared, restarting..."
    just stack-start

# ─────────────────────────────────────────────────────────────────────
# Build
# ─────────────────────────────────────────────────────────────────────

# Build all Holochain DNAs + pack hApp (delegates to per-project justfiles)
dna-build: dna-lamad dna-imagodei dna-infrastructure
    #!/usr/bin/env bash
    set -e
    WORKDIR="{{hc_dir}}/dna/elohim/workdir"
    echo "Packing elohim.happ..."
    hc app pack "$WORKDIR" -o "$WORKDIR/elohim.happ"
    echo "DNAs built (lamad + imagodei + infrastructure)"

# Build lamad DNA
dna-lamad:
    just --justfile {{hc_dir}}/dna/elohim/justfile --working-directory {{hc_dir}}/dna/elohim pack

# Build imagodei DNA
dna-imagodei:
    just --justfile {{hc_dir}}/dna/imagodei/justfile --working-directory {{hc_dir}}/dna/imagodei pack

# Build infrastructure DNA
dna-infrastructure:
    just --justfile {{hc_dir}}/dna/infrastructure/justfile --working-directory {{hc_dir}}/dna/infrastructure pack

# Build node-registry DNA
dna-node-registry:
    just --justfile {{hc_dir}}/dna/node-registry/justfile --working-directory {{hc_dir}}/dna/node-registry pack

# Build doorway gateway (delegates to per-project justfile)
doorway-build:
    just --justfile {{doorway_dir}}/justfile --working-directory {{doorway_dir}} build

# Build elohim-node P2P runtime (delegates to per-project justfile)
node-build:
    just --justfile {{node_dir}}/justfile --working-directory {{node_dir}} build

# Build sophia assessment UMD bundle
sophia-build:
    cd {{sophia_dir}} && pnpm install && pnpm build && pnpm build:umd

# Verify sophia build artifacts exist
sophia-check:
    bash {{root}}/scripts/check-sophia.sh

# ─────────────────────────────────────────────────────────────────────
# Content
# ─────────────────────────────────────────────────────────────────────

# Seed content to local stack
seed:
    cd {{genesis_dir}}/seeder && npx tsx src/seed.ts

# Validate seed data without writing
seed-dry-run:
    cd {{genesis_dir}}/seeder && npx tsx src/seed.ts --dry-run

# ─────────────────────────────────────────────────────────────────────
# Angular Dev
# ─────────────────────────────────────────────────────────────────────

# Start Angular dev server (proxy to doorway)
app-dev:
    cd {{app_dir}} && npx ng serve --proxy-config proxy.conf.mjs --disable-host-check

# Production build of elohim-app
app-build:
    cd {{app_dir}} && npx ng build
