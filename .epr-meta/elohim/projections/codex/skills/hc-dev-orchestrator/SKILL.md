---
name: hc-dev-orchestrator
description: START and manage the Elohim P2P Framework local DEVELOPMENT STACK — runs conductor (identity/provenance), storage (content), doorway (unified API) as a coordinated service trio. Use when "start the local dev stack", "spin up holochain locally", "why isn't the conductor reachable", "is the doorway alive?", or checking service health during development. NOT for desktop Tauri shell knowledge (use tauri-desktop).
metadata:
  runtime: codex
  sourceRuntime: claude
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/hc-dev-orchestrator.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/hc-dev-orchestrator"
---

# Elohim P2P Framework Orchestrator

This skill manages the local development stack for the Elohim Protocol. The framework combines three core components:

- **Holochain Conductor** - Cryptographic identity and provenance (agent-centric)
- **elohim-storage** - SQLite content database + blob storage (content-centric)
- **Doorway Gateway** - Unified HTTP/WebSocket API

**Note:** This is a P2P framework where Holochain provides the cryptographic/agentic layer. Content lives in elohim-storage, not the DHT.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Elohim P2P Framework                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│   Angular App (4200)                                                  │
│        ↓                                                              │
│   Doorway Gateway (8888)  ←── Unified API for all services           │
│        ↓                   ↓                                          │
│   Holochain Conductor     elohim-storage (8090)                       │
│   └─ Agent identity       └─ SQLite content DB                        │
│   └─ Cryptographic         └─ Blob storage                            │
│      provenance            └─ Import API                              │
│        ↓                                                              │
│   Lair Keystore (in-process dev mode)                                │
│                                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

## Quick Start Commands

```bash
cd /projects/elohim/app/elohim-app    # NOTE: app/elohim-app (post-reorg path)

# Start full stack (conductor + storage + doorway)
pnpm run hc:start

# Start with content seeding
pnpm run hc:start:seed

# Start + Angular dev server
pnpm run dev

# Check status
pnpm run hc:status

# Stop all services
pnpm run hc:stop
```

`hc-start.sh` is the MAINTAINED entry (its HC_DIR math is post-reorg-correct);
`storage-start.sh` standalone still computes a pre-reorg `HC_DIR=../holochain`
and will not find binaries — start storage via `hc-start.sh` or manually (see
the bring-up ladder below).

## Command Reference

| Command | Description |
|---------|-------------|
| `pnpm run hc:start` | Start full stack (conductor + storage + doorway) |
| `pnpm run hc:start:seed` | Full stack + seed sample content |
| `pnpm run hc:start:conductor` | Conductor only (debug mode, rare) |
| `pnpm run hc:stop` | Stop all services |
| `pnpm run hc:reset` | Stop + clear all data + restart |
| `pnpm run hc:status` | Show all service status |
| `pnpm run hc:build` | Build all DNAs |
| `pnpm run hc:build:all` | Rebuild everything (DNAs + binaries) |
| `pnpm run hc:seed` | Seed content to local stack |
| `pnpm run dev` | Start full stack + Angular dev server |

### Component-Specific Commands

| Command | Description |
|---------|-------------|
| `pnpm run storage:start` | Start storage service |
| `pnpm run storage:start:foreground` | Start storage (logs visible) |
| `pnpm run storage:stop` | Stop storage service |
| `pnpm run storage:build` | Build storage binary |
| `pnpm run storage:stats` | Show content database stats |
| `pnpm run doorway:start` | Start doorway |
| `pnpm run doorway:stop` | Stop doorway |
| `pnpm run doorway:build` | Build doorway binary |
| `pnpm run doorway:logs` | Show doorway status |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `STORAGE_PORT` | 8090 | elohim-storage HTTP port |
| `STORAGE_DIR` | /tmp/elohim-storage | Storage data directory |
| `SEED_LIMIT` | 20 | Items to seed with --seed flag |
| `ADMIN_PORT` | (auto) | Override conductor admin port |

## Health Checks

```bash
# Full status
pnpm run hc:status

# Individual checks
curl http://localhost:8888/status        # Doorway (full status)
curl http://localhost:8888/health        # Doorway (quick health)
curl http://localhost:8888/db/stats      # Content database stats
curl http://localhost:8090/health        # Storage direct
curl http://localhost:8090/db/stats      # Storage DB stats
```

## Port Reference

| Service | Port | Protocol | Description |
|---------|------|----------|-------------|
| Angular | 4200 | HTTP | Dev server (`ng serve`) |
| Doorway | 8888 | HTTP/WS | Unified gateway API |
| Conductor App | 4445 | WebSocket | Holochain app interface |
| Conductor Admin | dynamic | WebSocket | Saved in `.hc_ports` |
| Storage | 8090 | HTTP/WS | Content DB + blobs |

## Content Database

elohim-storage provides a SQLite content database:

```bash
# List content
curl "http://localhost:8888/db/content?limit=10"

# Get content by ID
curl http://localhost:8888/db/content/manifesto

# List learning paths — there is NO /db/paths route; paths are ContentNodes
curl "http://localhost:8888/db/content?contentType=path&limit=100"

# Get a path (steps live in the body: sections[].items[].ref = "epr:{id}")
curl http://localhost:8888/db/content/elohim-protocol

# Database stats
curl http://localhost:8888/db/stats
```

## Blob Storage

```bash
# Upload blob
curl -X PUT "http://localhost:8090/blob/sha256-abc123" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @file.bin

# Get blob
curl http://localhost:8888/blob/sha256-abc123

# Check blob exists
curl -I http://localhost:8888/blob/sha256-abc123
```

## Seeding Content

```bash
# Quick seed (from app/elohim-app)
pnpm run hc:seed

# Or with options (from seeder directory)
cd genesis/seeder
DOORWAY_URL=http://localhost:8888 \
STORAGE_URL=http://localhost:8090 \
npx tsx src/seed.ts --limit 50

# Dry run (validate without writing)
pnpm run seed:dry-run

# Validate schema only
pnpm run seed:validate
```

## Troubleshooting

### Services Not Starting
```bash
# Check what's using ports
fuser 8888/tcp 8090/tcp 4445/tcp

# Kill stuck processes
pnpm run hc:stop
fuser -k 8888/tcp 8090/tcp

# Fresh start
pnpm run hc:reset
pnpm run hc:start
```

### Content Database 503 Errors
If you see `503 Service Unavailable` on `/db/*` endpoints:
```bash
# Ensure storage has ENABLE_CONTENT_DB=true
pnpm run storage:start:foreground
# Look for: "SQLite content database enabled"
```

### Conductor Not Responding
```bash
# Check ports file
cat ../../elohim/holochain/local-dev/.hc_ports

# Test conductor
ADMIN_PORT=$(cat ../../elohim/holochain/local-dev/.hc_ports | grep admin_port | grep -o '[0-9]*')
hc sandbox call --running $ADMIN_PORT list-apps
```

### Doorway Missing Storage
```bash
# Check doorway status
curl http://localhost:8888/status | jq '.storage'
# Should show: "configured": true, "healthy": true

# Restart doorway with storage
pnpm run doorway:stop
pnpm run doorway:start
```

## Verified bring-up ladder (render-proof grade, verified 2026-06-09)

The full path from cold tree to a screenshot-able stack — each rung names the
trap it clears. This is the /deliver station's bring-up asset: reuse it, don't
rediscover it.

1. **Binaries.** `hc-start.sh` expects `elohim/elohim-storage/target/release/elohim-storage`
   and `doorway/doorway-service/target/release/doorway`. Native release builds on
   the /projects volume hit the fingerprint-ENOENT quirk — build with a /tmp
   target and symlink the binary to the expected path:
   ```bash
   cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' \
     CARGO_TARGET_DIR=/tmp/cargo-target-elohim-storage cargo build --release
   mkdir -p target/release && ln -sf /tmp/cargo-target-elohim-storage/release/elohim-storage target/release/
   ```
2. **Stale-process trap.** `hc-start.sh` skips storage when ANYTHING answers
   `:8090/health` — including a days-old binary from a previous session. Before
   trusting "already running": `ls -l /proc/$(fuser 8090/tcp 2>/dev/null | tr -d ' ')/exe`
   and check its mtime/path. Old binary on the port = old wire shapes
   (snake_case graph responses were the 2026-06-09 tell). Kill and relaunch:
   ```bash
   fuser -k 8090/tcp; ADMIN_PORT=$(grep admin_port elohim/holochain/local-dev/.hc_ports | grep -o '[0-9]*' | head -1)
   cd elohim/elohim-storage && HOLOCHAIN_ADMIN_URL="ws://localhost:$ADMIN_PORT" HTTP_PORT=8090 \
     ENABLE_IMPORT_API=true ENABLE_CONTENT_DB=true nohup ./target/release/elohim-storage > /tmp/storage.log 2>&1 &
   ```
3. **Seed.** Full: `pnpm run hc:seed` (from app/elohim-app). Scoped:
   `npx tsx src/seed.ts --content-only --ids=a,b,c` / `--paths-only` (from
   genesis/seeder, with DOORWAY_URL/STORAGE_URL). Two seeder facts: bulk create
   is **skip-on-exists** (stale rows keep their old content/type — DELETE the
   row or update SQL directly, then re-seed), and the paths phase's step
   relationships are rejected (`relationship_type 'step'` not in storage's
   vocabulary — known, backlogged; path-step membership still works via the
   path body).
4. **Provenance gate (the documented anchor gap).** Bulk-seeded rows have
   `dht_anchor_hash` NULL → `GET /db/content/{id}` 404s while the graph/tag
   routes still work. Dev recovery — backfill the live DB (Che data dir is
   `$XDG_DATA_HOME/elohim-storage/content.db`, i.e. `/nix/xdg/data/...`):
   ```sql
   UPDATE content SET dht_anchor_hash='uhCkk-local-dev-backfill' WHERE dht_anchor_hash IS NULL;
   ```
5. **Wire facts for verification probes.** There is NO `/db/paths` route —
   paths are ContentNodes (`/db/content?contentType=path`), body shape
   `sections[].items[].ref` (`epr:{id}`), step index = 0-based global item
   index. Graph route: `/db/relationships/graph/{id}?computed=true&depth=N`.
6. **Render topology (two bundles, two dev servers).** The shell
   (`app/elohim-app`, :4200) routes `/epr/{id}` (it lazy-loads the lamad
   content-viewer) but NOT `/lamad/*`; the lamad bundle serves those:
   ```bash
   cd app/lamad && pnpm exec ng serve --port 4300 --serve-path /lamad --proxy-config ../elohim-app/proxy.conf.mjs
   ```
   Shell `pnpm start` runs a `prestart` that needs `wasm-pack`; when absent and
   `elohim/elohim-cache-core/pkg/` already exists, bypass:
   `pnpm exec ng serve --proxy-config proxy.conf.mjs --disable-host-check`.
   Screenshots: `cd genesis/a2o && pnpm look <url> --wait-testid <id> --out <slug>`.
7. **Known local noise.** The shell dev server's SSR pass logs
   `ReferenceError: document is not defined` per request (500 → CSR fallback);
   pages render fine client-side, but a2o After-hooks asserting zero console
   errors trip on it locally — absent on alpha (static serve).

## Key File Locations

| File | Purpose |
|------|---------|
| `app/elohim-app/scripts/hc-start.sh` | Main startup script (the MAINTAINED path math) |
| `app/elohim-app/scripts/storage-start.sh` | Storage startup — STALE pre-reorg HC_DIR; prefer hc-start.sh |
| `app/elohim-app/scripts/hc-build.sh` | Multi-DNA build |
| `elohim/holochain/local-dev/.hc_ports` | Dynamic port configuration |
| `elohim/elohim-storage/target/release/elohim-storage` | Storage binary (symlink from /tmp build — rung 1) |
| `doorway/doorway-service/target/release/doorway` | Doorway binary (hc-start's expected path) |
| `$XDG_DATA_HOME/elohim-storage/content.db` | Live SQLite (Che: /nix/xdg/data/elohim-storage/) |

## Multi-DNA Architecture

The Elohim hApp provides identity and provenance:

| DNA | Purpose | Zomes |
|-----|---------|-------|
| `lamad` | Learning provenance | content_store, content_store_integrity |
| `imagodei` | Identity/sovereignty | identity, session |
| `infrastructure` | Network services | doorway_registry |

## Common Workflows

### Full Development Session
```bash
# Start everything + Angular
pnpm run dev

# Or step by step
pnpm run hc:start
npm start
```

### Content Development
```bash
# Start stack
pnpm run hc:start

# Edit content in genesis/data/lamad/
# Then seed
pnpm run hc:seed

# Verify
curl http://localhost:8888/db/stats
```

### Debugging Storage
```bash
# Start conductor only
pnpm run hc:start:conductor

# Start storage in foreground (see all logs)
pnpm run storage:start:foreground

# In another terminal, start doorway
pnpm run doorway:start
```

### Connect to Remote Environment
```bash
# Dev environment
export DOORWAY_URL='https://doorway-alpha.elohim.host'
export HOLOCHAIN_ADMIN_URL='wss://doorway-alpha.elohim.host/admin?apiKey=dev-elohim-auth-2024'
```
